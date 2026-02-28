use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::config::{GameServerConfig, PathsConfig, RconConfig};
use crate::lgsm::LgsmLock;
use crate::monitor::GameMonitor;
use crate::rcon::RconClient;

/// Source of a server definition: either from config.yaml or dynamically created.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ServerSource {
    Static,
    Dynamic,
}

/// Provisioning status for dynamically created servers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProvisioningStatus {
    Ready,
    Installing,
    Downloading,
    InstallingOxide,
    Configuring,
    Error,
}

/// Server type: vanilla or modded.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ServerType {
    Vanilla,
    Modded,
}

/// Extended server definition with provisioning info.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerDefinition {
    pub id: String,
    pub name: String,
    pub server_type: ServerType,
    pub source: ServerSource,
    pub provisioning_status: ProvisioningStatus,
    pub provisioning_log: Vec<String>,
    pub game_port: u16,
    pub rcon_port: u16,
    pub query_port: u16,
    pub max_players: u32,
    pub world_size: u32,
    pub seed: u32,
    pub hostname: String,
    pub rcon_password: String,
    pub base_path: String,
    pub created_at: DateTime<Utc>,
}

impl ServerDefinition {
    /// Convert to a GameServerConfig for compatibility with existing handler code.
    pub fn to_game_server_config(&self) -> GameServerConfig {
        let base_dir = format!("{}/rustserver-{}", self.base_path, self.id);
        GameServerConfig {
            id: self.id.clone(),
            name: self.name.clone(),
            rcon: RconConfig {
                host: "127.0.0.1".to_string(),
                port: self.rcon_port,
                password: self.rcon_password.clone(),
            },
            paths: PathsConfig {
                lgsm_script: format!("{}/rustserver", base_dir),
                server_files: format!("{}/serverfiles", base_dir),
                oxide_plugins: format!("{}/serverfiles/oxide/plugins", base_dir),
                oxide_config: format!("{}/serverfiles/oxide/config", base_dir),
                server_cfg: format!(
                    "{}/serverfiles/server/rustserver/cfg/server.cfg",
                    base_dir
                ),
                server_log: format!("{}/log/console/rustserver-console.log", base_dir),
                base_dir,
            },
        }
    }

    /// Create a ServerDefinition from a static GameServerConfig.
    pub fn from_static_config(config: &GameServerConfig) -> Self {
        Self {
            id: config.id.clone(),
            name: config.name.clone(),
            server_type: ServerType::Vanilla,
            source: ServerSource::Static,
            provisioning_status: ProvisioningStatus::Ready,
            provisioning_log: Vec::new(),
            game_port: 28015,
            rcon_port: config.rcon.port,
            query_port: 27015,
            max_players: 100,
            world_size: 4000,
            seed: 0,
            hostname: config.name.clone(),
            rcon_password: config.rcon.password.clone(),
            base_path: config
                .paths
                .base_dir
                .rsplit('/')
                .skip(1)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<Vec<_>>()
                .join("/"),
            created_at: Utc::now(),
        }
    }
}

/// Per-server runtime state: RCON client, game monitor, LGSM lock, collector handle.
pub struct ServerRuntime {
    pub rcon: Arc<RconClient>,
    pub game_monitor: Arc<GameMonitor>,
    pub lgsm_lock: Arc<LgsmLock>,
    pub collector_handle: Option<tokio::task::JoinHandle<()>>,
}

/// Central shared registry replacing the separate HashMaps.
pub struct ServerRegistry {
    pub definitions: RwLock<Vec<ServerDefinition>>,
    pub runtimes: RwLock<HashMap<String, ServerRuntime>>,
    /// Original static configs from config.yaml, keyed by server id.
    pub static_configs: HashMap<String, GameServerConfig>,
}

impl ServerRegistry {
    pub fn new(
        definitions: Vec<ServerDefinition>,
        static_configs: HashMap<String, GameServerConfig>,
    ) -> Self {
        Self {
            definitions: RwLock::new(definitions),
            runtimes: RwLock::new(HashMap::new()),
            static_configs,
        }
    }

    /// Resolve a server by ID, returning its GameServerConfig.
    /// For static servers, returns the original config from config.yaml.
    /// For dynamic servers, generates paths from the definition.
    pub async fn get_config(&self, server_id: &str) -> Option<GameServerConfig> {
        // Check static configs first
        if let Some(config) = self.static_configs.get(server_id) {
            return Some(config.clone());
        }
        // Fall back to dynamic definition
        let defs = self.definitions.read().await;
        defs.iter()
            .find(|d| d.id == server_id)
            .map(|d| d.to_game_server_config())
    }

    /// Get all GameServerConfigs.
    pub async fn all_configs(&self) -> Vec<GameServerConfig> {
        let defs = self.definitions.read().await;
        defs.iter()
            .map(|d| {
                self.static_configs
                    .get(&d.id)
                    .cloned()
                    .unwrap_or_else(|| d.to_game_server_config())
            })
            .collect()
    }

    /// Get all server definitions.
    pub async fn all_definitions(&self) -> Vec<ServerDefinition> {
        let defs = self.definitions.read().await;
        defs.clone()
    }

    /// Get a specific server definition.
    pub async fn get_definition(&self, server_id: &str) -> Option<ServerDefinition> {
        let defs = self.definitions.read().await;
        defs.iter().find(|d| d.id == server_id).cloned()
    }

    /// Get the RCON client for a server.
    pub async fn get_rcon(&self, server_id: &str) -> Option<Arc<RconClient>> {
        let runtimes = self.runtimes.read().await;
        runtimes.get(server_id).map(|r| r.rcon.clone())
    }

    /// Get the game monitor for a server.
    pub async fn get_game_monitor(&self, server_id: &str) -> Option<Arc<GameMonitor>> {
        let runtimes = self.runtimes.read().await;
        runtimes.get(server_id).map(|r| r.game_monitor.clone())
    }

    /// Get the LGSM lock for a server.
    pub async fn get_lgsm_lock(&self, server_id: &str) -> Option<Arc<LgsmLock>> {
        let runtimes = self.runtimes.read().await;
        runtimes.get(server_id).map(|r| r.lgsm_lock.clone())
    }
}
