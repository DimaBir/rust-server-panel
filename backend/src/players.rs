use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::rcon::RconClient;

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: String,
}

#[derive(Debug, Serialize)]
struct SuccessBody {
    success: bool,
    message: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KickRequest {
    pub steam_id: String,
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BanRequest {
    pub steam_id: String,
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnbanRequest {
    pub steam_id: String,
}

/// GET /api/servers/{server_id}/players
pub async fn list_players(
    server_id: web::Path<String>,
    rcon_clients: web::Data<HashMap<String, Arc<RconClient>>>,
) -> HttpResponse {
    let rcon = match rcon_clients.get(server_id.as_str()) {
        Some(r) => r,
        None => return HttpResponse::NotFound().json(ErrorBody { error: "Server not found".to_string() }),
    };

    match rcon.player_list().await {
        Ok(players) => HttpResponse::Ok().json(serde_json::json!({ "players": players })),
        Err(e) => HttpResponse::InternalServerError().json(ErrorBody {
            error: format!("Failed to get player list: {}", e),
        }),
    }
}

/// POST /api/servers/{server_id}/players/kick
pub async fn kick_player(
    server_id: web::Path<String>,
    body: web::Json<KickRequest>,
    rcon_clients: web::Data<HashMap<String, Arc<RconClient>>>,
) -> HttpResponse {
    let rcon = match rcon_clients.get(server_id.as_str()) {
        Some(r) => r,
        None => return HttpResponse::NotFound().json(ErrorBody { error: "Server not found".to_string() }),
    };

    let reason = body.reason.as_deref().unwrap_or("Kicked by admin");
    match rcon.kick(&body.steam_id, reason).await {
        Ok(msg) => HttpResponse::Ok().json(SuccessBody {
            success: true,
            message: format!("Kicked {}: {}", body.steam_id, msg),
        }),
        Err(e) => HttpResponse::InternalServerError().json(ErrorBody {
            error: format!("Failed to kick player: {}", e),
        }),
    }
}

/// POST /api/servers/{server_id}/players/ban
pub async fn ban_player(
    server_id: web::Path<String>,
    body: web::Json<BanRequest>,
    rcon_clients: web::Data<HashMap<String, Arc<RconClient>>>,
) -> HttpResponse {
    let rcon = match rcon_clients.get(server_id.as_str()) {
        Some(r) => r,
        None => return HttpResponse::NotFound().json(ErrorBody { error: "Server not found".to_string() }),
    };

    let reason = body.reason.as_deref().unwrap_or("Banned by admin");
    match rcon.ban(&body.steam_id, reason).await {
        Ok(msg) => HttpResponse::Ok().json(SuccessBody {
            success: true,
            message: format!("Banned {}: {}", body.steam_id, msg),
        }),
        Err(e) => HttpResponse::InternalServerError().json(ErrorBody {
            error: format!("Failed to ban player: {}", e),
        }),
    }
}

/// POST /api/servers/{server_id}/players/unban
pub async fn unban_player(
    server_id: web::Path<String>,
    body: web::Json<UnbanRequest>,
    rcon_clients: web::Data<HashMap<String, Arc<RconClient>>>,
) -> HttpResponse {
    let rcon = match rcon_clients.get(server_id.as_str()) {
        Some(r) => r,
        None => return HttpResponse::NotFound().json(ErrorBody { error: "Server not found".to_string() }),
    };

    match rcon.unban(&body.steam_id).await {
        Ok(msg) => HttpResponse::Ok().json(SuccessBody {
            success: true,
            message: format!("Unbanned {}: {}", body.steam_id, msg),
        }),
        Err(e) => HttpResponse::InternalServerError().json(ErrorBody {
            error: format!("Failed to unban player: {}", e),
        }),
    }
}
