use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use tokio::sync::{Mutex, oneshot};
use tokio::time::{timeout, Duration};
use tokio_tungstenite::tungstenite::Message;

use crate::config::RconConfig;

/// RCON request packet sent to the Rust game server.
#[derive(Debug, Serialize)]
struct RconRequest {
    #[serde(rename = "Identifier")]
    identifier: i32,
    #[serde(rename = "Message")]
    message: String,
    #[serde(rename = "Name")]
    name: String,
}

/// RCON response packet received from the Rust game server.
#[derive(Debug, Deserialize, Clone)]
pub struct RconResponse {
    #[serde(rename = "Identifier")]
    pub identifier: i32,
    #[serde(rename = "Message")]
    pub message: String,
    #[serde(rename = "Type")]
    #[serde(default)]
    pub msg_type: String,
}

/// Parsed server info from the "serverinfo" RCON command.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo {
    #[serde(default, alias = "Hostname")]
    pub hostname: String,
    #[serde(default, alias = "Players")]
    pub players: u32,
    #[serde(default, alias = "MaxPlayers")]
    pub max_players: u32,
    #[serde(default, alias = "Queued")]
    pub queued: u32,
    #[serde(default, alias = "Joining")]
    pub joining: u32,
    #[serde(default, alias = "EntityCount")]
    pub entity_count: u64,
    #[serde(default, alias = "Framerate")]
    pub framerate: f64,
    #[serde(default, alias = "Uptime")]
    pub uptime: u64,
    #[serde(default, alias = "Map")]
    pub map: String,
    #[serde(default, alias = "GameTime")]
    pub game_time: String,
    #[serde(default, alias = "SaveCreatedTime")]
    pub save_created_time: String,
    #[serde(default, alias = "Seed")]
    pub seed: u32,
    #[serde(default, alias = "WorldSize")]
    pub world_size: u32,
}

/// Parsed player entry from the "playerlist" RCON command.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Player {
    #[serde(default, alias = "SteamID")]
    pub steam_id: String,
    #[serde(default, alias = "DisplayName")]
    pub display_name: String,
    #[serde(default, alias = "Address")]
    pub address: String,
    #[serde(default, alias = "Ping")]
    pub ping: i32,
    #[serde(default, alias = "ConnectedSeconds")]
    pub connected_seconds: f64,
    #[serde(default, alias = "Health")]
    pub health: f64,
    #[serde(default, alias = "VoiationLevel")]
    pub violation_level: f64,
}

type WsSink =
    futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, Message>;

struct PendingRequest {
    sender: oneshot::Sender<String>,
}

struct RconInner {
    sink: Option<WsSink>,
    pending: std::collections::HashMap<i32, PendingRequest>,
}

/// WebSocket RCON client for the Rust game server.
/// The Rust game server uses WebSocket RCON on port 28016.
/// Protocol: connect to ws://{host}:{port}/{password}
pub struct RconClient {
    config: RconConfig,
    inner: Arc<Mutex<RconInner>>,
    next_id: AtomicI32,
    reader_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl RconClient {
    pub fn new(config: RconConfig) -> Self {
        Self {
            config,
            inner: Arc::new(Mutex::new(RconInner {
                sink: None,
                pending: std::collections::HashMap::new(),
            })),
            next_id: AtomicI32::new(1),
            reader_handle: Mutex::new(None),
        }
    }

    /// Connect (or reconnect) to the RCON WebSocket.
    pub async fn connect(&self) -> anyhow::Result<()> {
        // Close existing connection
        {
            let mut inner = self.inner.lock().await;
            inner.sink = None;
            inner.pending.clear();
        }

        // Abort existing reader task
        {
            let mut handle = self.reader_handle.lock().await;
            if let Some(h) = handle.take() {
                h.abort();
            }
        }

        let url = format!(
            "ws://{}:{}/{}",
            self.config.host, self.config.port, self.config.password
        );
        tracing::info!("Connecting to RCON at ws://{}:{}/***", self.config.host, self.config.port);

        let (ws_stream, _) = tokio_tungstenite::connect_async(&url).await?;
        let (sink, stream) = ws_stream.split();

        {
            let mut inner = self.inner.lock().await;
            inner.sink = Some(sink);
        }

        // Spawn reader task to route responses to pending requests
        let inner_clone = self.inner.clone();
        let handle = tokio::spawn(async move {
            Self::reader_loop(stream, inner_clone).await;
        });

        {
            let mut h = self.reader_handle.lock().await;
            *h = Some(handle);
        }

        tracing::info!("RCON connected successfully");
        Ok(())
    }

    async fn reader_loop(
        mut stream: futures_util::stream::SplitStream<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
        >,
        inner: Arc<Mutex<RconInner>>,
    ) {
        while let Some(msg) = stream.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Ok(response) = serde_json::from_str::<RconResponse>(&text) {
                        let mut guard = inner.lock().await;
                        if let Some(pending) = guard.pending.remove(&response.identifier) {
                            let _ = pending.sender.send(response.message);
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    tracing::warn!("RCON WebSocket closed by server");
                    break;
                }
                Err(e) => {
                    tracing::error!("RCON WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }
        tracing::info!("RCON reader loop ended");
    }

    /// Check if connected (has an active sink).
    pub async fn is_connected(&self) -> bool {
        let inner = self.inner.lock().await;
        inner.sink.is_some()
    }

    /// Execute an RCON command and wait for the response.
    pub async fn execute(&self, cmd: &str) -> anyhow::Result<String> {
        // Try to connect if not connected
        if !self.is_connected().await {
            self.connect().await?;
        }

        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let request = RconRequest {
            identifier: id,
            message: cmd.to_string(),
            name: "WebRcon".to_string(),
        };

        let json = serde_json::to_string(&request)?;
        let (tx, rx) = oneshot::channel();

        {
            let mut inner = self.inner.lock().await;
            inner.pending.insert(id, PendingRequest { sender: tx });
            if let Some(ref mut sink) = inner.sink {
                sink.send(Message::Text(json)).await?;
            } else {
                anyhow::bail!("RCON not connected");
            }
        }

        // Wait for response with timeout
        match timeout(Duration::from_secs(10), rx).await {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(_)) => anyhow::bail!("RCON response channel closed"),
            Err(_) => {
                // Clean up pending request on timeout
                let mut inner = self.inner.lock().await;
                inner.pending.remove(&id);
                anyhow::bail!("RCON command timed out after 10 seconds")
            }
        }
    }

    /// Get parsed server info.
    pub async fn server_info(&self) -> anyhow::Result<ServerInfo> {
        let response = self.execute("serverinfo").await?;
        let info: ServerInfo = serde_json::from_str(&response)
            .map_err(|e| anyhow::anyhow!("Failed to parse serverinfo: {} (raw: {})", e, response))?;
        Ok(info)
    }

    /// Get parsed player list.
    pub async fn player_list(&self) -> anyhow::Result<Vec<Player>> {
        let response = self.execute("playerlist").await?;
        let players: Vec<Player> = serde_json::from_str(&response)
            .map_err(|e| anyhow::anyhow!("Failed to parse playerlist: {} (raw: {})", e, response))?;
        Ok(players)
    }

    /// Kick a player by Steam ID or name.
    pub async fn kick(&self, target: &str, reason: &str) -> anyhow::Result<String> {
        self.execute(&format!("kick {} \"{}\"", target, reason)).await
    }

    /// Ban a player by Steam ID or name.
    pub async fn ban(&self, target: &str, reason: &str) -> anyhow::Result<String> {
        self.execute(&format!("ban {} \"{}\"", target, reason)).await
    }

    /// Unban a player by Steam ID.
    pub async fn unban(&self, steam_id: &str) -> anyhow::Result<String> {
        self.execute(&format!("unban {}", steam_id)).await
    }

    /// Send a message to all players.
    pub async fn say(&self, message: &str) -> anyhow::Result<String> {
        self.execute(&format!("say \"{}\"", message)).await
    }

    /// Trigger a world save.
    pub async fn save(&self) -> anyhow::Result<String> {
        self.execute("server.save").await
    }

    /// Reload an Oxide plugin.
    pub async fn oxide_reload(&self, plugin_name: &str) -> anyhow::Result<String> {
        self.execute(&format!("oxide.reload {}", plugin_name)).await
    }

    /// Load an Oxide plugin.
    pub async fn oxide_load(&self, plugin_name: &str) -> anyhow::Result<String> {
        self.execute(&format!("oxide.load {}", plugin_name)).await
    }

    /// Unload an Oxide plugin.
    pub async fn oxide_unload(&self, plugin_name: &str) -> anyhow::Result<String> {
        self.execute(&format!("oxide.unload {}", plugin_name)).await
    }
}
