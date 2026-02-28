use actix_web::{web, HttpRequest, HttpResponse};
use actix_ws::Message;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::time::{interval, Duration};

use crate::auth::validate_token;
use crate::config::AppConfig;
use crate::monitor::{GameSnapshot, SystemMonitor, SystemSnapshot};
use crate::registry::ServerRegistry;

#[derive(Debug, Deserialize)]
pub struct WsTokenQuery {
    pub token: String,
}

/// Combined stats payload pushed over the monitor WebSocket.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MonitorPayload {
    system: Option<SystemSnapshot>,
    game: Option<GameSnapshot>,
}

/// GET /ws/{server_id}/console
pub async fn ws_console(
    req: HttpRequest,
    stream: web::Payload,
    path: web::Path<String>,
    query: web::Query<WsTokenQuery>,
    config: web::Data<AppConfig>,
    registry: web::Data<Arc<ServerRegistry>>,
) -> Result<HttpResponse, actix_web::Error> {
    let server_id = path.into_inner();

    if let Err(e) = validate_token(&query.token, &config.auth.jwt_secret) {
        tracing::debug!("WebSocket console auth failed: {}", e);
        return Ok(HttpResponse::Unauthorized().body("Invalid or expired token"));
    }

    let rcon = match registry.get_rcon(&server_id).await {
        Some(r) => r,
        None => return Ok(HttpResponse::NotFound().body("Server not found")),
    };

    let (response, mut session, mut msg_stream) = actix_ws::handle(&req, stream)?;

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

/// GET /ws/{server_id}/monitor
pub async fn ws_monitor(
    req: HttpRequest,
    stream: web::Payload,
    path: web::Path<String>,
    query: web::Query<WsTokenQuery>,
    config: web::Data<AppConfig>,
    sys_monitor: web::Data<Arc<SystemMonitor>>,
    registry: web::Data<Arc<ServerRegistry>>,
) -> Result<HttpResponse, actix_web::Error> {
    let server_id = path.into_inner();

    if let Err(e) = validate_token(&query.token, &config.auth.jwt_secret) {
        tracing::debug!("WebSocket monitor auth failed: {}", e);
        return Ok(HttpResponse::Unauthorized().body("Invalid or expired token"));
    }

    let game_monitor = match registry.get_game_monitor(&server_id).await {
        Some(m) => m,
        None => return Ok(HttpResponse::NotFound().body("Server not found")),
    };

    let (response, mut session, mut msg_stream) = actix_ws::handle(&req, stream)?;

    let sys_monitor = sys_monitor.into_inner().clone();

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
