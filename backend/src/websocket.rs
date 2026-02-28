use actix_web::{web, HttpRequest, HttpResponse};
use actix_ws::Message;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::time::{interval, Duration};

use crate::auth::validate_token;
use crate::config::AppConfig;
use crate::monitor::{GameMonitor, GameSnapshot, SystemMonitor, SystemSnapshot};
use crate::rcon::RconClient;

#[derive(Debug, Deserialize)]
pub struct WsTokenQuery {
    pub token: String,
}

/// Combined stats payload pushed over the monitor WebSocket.
#[derive(Debug, Serialize)]
struct MonitorPayload {
    system: Option<SystemSnapshot>,
    game: Option<GameSnapshot>,
}

/// GET /ws/console - RCON bridge WebSocket.
///
/// Accepts a WebSocket upgrade with JWT auth via `?token=` query param.
/// Text messages from the browser are forwarded to RCON as commands,
/// and RCON responses are sent back to the browser.
pub async fn ws_console(
    req: HttpRequest,
    stream: web::Payload,
    query: web::Query<WsTokenQuery>,
    config: web::Data<AppConfig>,
    rcon: web::Data<Arc<RconClient>>,
) -> Result<HttpResponse, actix_web::Error> {
    // Validate JWT from query param
    if let Err(e) = validate_token(&query.token, &config.auth.jwt_secret) {
        tracing::debug!("WebSocket console auth failed: {}", e);
        return Ok(HttpResponse::Unauthorized().body("Invalid or expired token"));
    }

    let (response, mut session, mut msg_stream) = actix_ws::handle(&req, stream)?;

    let rcon = rcon.into_inner().clone();

    // Spawn a task to handle incoming messages
    actix_web::rt::spawn(async move {
        while let Some(Ok(msg)) = msg_stream.next().await {
            match msg {
                Message::Text(text) => {
                    let cmd = text.to_string();
                    tracing::debug!("RCON WS command: {}", cmd);

                    match rcon.execute(&cmd).await {
                        Ok(response_text) => {
                            if session.text(response_text).await.is_err() {
                                break;
                            }
                        }
                        Err(e) => {
                            let err_msg = format!("Error: {}", e);
                            if session.text(err_msg).await.is_err() {
                                break;
                            }
                        }
                    }
                }
                Message::Ping(bytes) => {
                    if session.pong(&bytes).await.is_err() {
                        break;
                    }
                }
                Message::Close(_) => {
                    break;
                }
                _ => {}
            }
        }

        let _ = session.close(None).await;
        tracing::debug!("RCON WebSocket session closed");
    });

    Ok(response)
}

/// GET /ws/monitor - Live stats push WebSocket.
///
/// Accepts a WebSocket upgrade with JWT auth via `?token=` query param.
/// Every 5 seconds, pushes JSON with the latest system and game stats.
pub async fn ws_monitor(
    req: HttpRequest,
    stream: web::Payload,
    query: web::Query<WsTokenQuery>,
    config: web::Data<AppConfig>,
    sys_monitor: web::Data<Arc<SystemMonitor>>,
    game_monitor: web::Data<Arc<GameMonitor>>,
) -> Result<HttpResponse, actix_web::Error> {
    // Validate JWT from query param
    if let Err(e) = validate_token(&query.token, &config.auth.jwt_secret) {
        tracing::debug!("WebSocket monitor auth failed: {}", e);
        return Ok(HttpResponse::Unauthorized().body("Invalid or expired token"));
    }

    let (response, mut session, mut msg_stream) = actix_ws::handle(&req, stream)?;

    let sys_monitor = sys_monitor.into_inner().clone();
    let game_monitor = game_monitor.into_inner().clone();

    // Spawn a task that pushes stats every 5 seconds
    actix_web::rt::spawn(async move {
        let mut tick = interval(Duration::from_secs(5));

        loop {
            tokio::select! {
                _ = tick.tick() => {
                    let sys_history = sys_monitor.history.read().await;
                    let system = sys_history.latest().cloned();
                    drop(sys_history);

                    let game_history = game_monitor.history.read().await;
                    let game = game_history.latest().cloned();
                    drop(game_history);

                    let payload = MonitorPayload { system, game };

                    match serde_json::to_string(&payload) {
                        Ok(json) => {
                            if session.text(json).await.is_err() {
                                break;
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to serialize monitor payload: {}", e);
                        }
                    }
                }
                msg = msg_stream.next() => {
                    match msg {
                        Some(Ok(Message::Ping(bytes))) => {
                            if session.pong(&bytes).await.is_err() {
                                break;
                            }
                        }
                        Some(Ok(Message::Close(_))) | None => {
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }

        let _ = session.close(None).await;
        tracing::debug!("Monitor WebSocket session closed");
    });

    Ok(response)
}
