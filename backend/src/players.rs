use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::registry::ServerRegistry;

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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModeratorRequest {
    pub steam_id: String,
    pub display_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveModeratorRequest {
    pub steam_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GiveItemRequest {
    pub steam_id: String,
    pub item: String,
    pub amount: u32,
}

/// GET /api/servers/{server_id}/players
pub async fn list_players(
    server_id: web::Path<String>,
    registry: web::Data<Arc<ServerRegistry>>,
) -> HttpResponse {
    let rcon = match registry.get_rcon(&server_id).await {
        Some(r) => r,
        None => {
            return HttpResponse::NotFound().json(ErrorBody {
                error: "Server not found".to_string(),
            })
        }
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
    registry: web::Data<Arc<ServerRegistry>>,
) -> HttpResponse {
    let rcon = match registry.get_rcon(&server_id).await {
        Some(r) => r,
        None => {
            return HttpResponse::NotFound().json(ErrorBody {
                error: "Server not found".to_string(),
            })
        }
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
    registry: web::Data<Arc<ServerRegistry>>,
) -> HttpResponse {
    let rcon = match registry.get_rcon(&server_id).await {
        Some(r) => r,
        None => {
            return HttpResponse::NotFound().json(ErrorBody {
                error: "Server not found".to_string(),
            })
        }
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
    registry: web::Data<Arc<ServerRegistry>>,
) -> HttpResponse {
    let rcon = match registry.get_rcon(&server_id).await {
        Some(r) => r,
        None => {
            return HttpResponse::NotFound().json(ErrorBody {
                error: "Server not found".to_string(),
            })
        }
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

/// POST /api/servers/{server_id}/players/moderator
pub async fn add_moderator(
    server_id: web::Path<String>,
    body: web::Json<ModeratorRequest>,
    registry: web::Data<Arc<ServerRegistry>>,
) -> HttpResponse {
    let rcon = match registry.get_rcon(&server_id).await {
        Some(r) => r,
        None => {
            return HttpResponse::NotFound().json(ErrorBody {
                error: "Server not found".to_string(),
            })
        }
    };

    let cmd = format!(
        "moderatorid {} \"{}\" \"Added via panel\"",
        body.steam_id, body.display_name
    );
    match rcon.execute(&cmd).await {
        Ok(msg) => {
            let _ = rcon.execute("server.writecfg").await;
            HttpResponse::Ok().json(SuccessBody {
                success: true,
                message: format!("Added moderator {}: {}", body.steam_id, msg),
            })
        }
        Err(e) => HttpResponse::InternalServerError().json(ErrorBody {
            error: format!("Failed to add moderator: {}", e),
        }),
    }
}

/// POST /api/servers/{server_id}/players/remove-moderator
pub async fn remove_moderator(
    server_id: web::Path<String>,
    body: web::Json<RemoveModeratorRequest>,
    registry: web::Data<Arc<ServerRegistry>>,
) -> HttpResponse {
    let rcon = match registry.get_rcon(&server_id).await {
        Some(r) => r,
        None => {
            return HttpResponse::NotFound().json(ErrorBody {
                error: "Server not found".to_string(),
            })
        }
    };

    match rcon.execute(&format!("removemoderator {}", body.steam_id)).await {
        Ok(msg) => {
            let _ = rcon.execute("server.writecfg").await;
            HttpResponse::Ok().json(SuccessBody {
                success: true,
                message: format!("Removed moderator {}: {}", body.steam_id, msg),
            })
        }
        Err(e) => HttpResponse::InternalServerError().json(ErrorBody {
            error: format!("Failed to remove moderator: {}", e),
        }),
    }
}

/// POST /api/servers/{server_id}/players/give
pub async fn give_item(
    server_id: web::Path<String>,
    body: web::Json<GiveItemRequest>,
    registry: web::Data<Arc<ServerRegistry>>,
) -> HttpResponse {
    let rcon = match registry.get_rcon(&server_id).await {
        Some(r) => r,
        None => {
            return HttpResponse::NotFound().json(ErrorBody {
                error: "Server not found".to_string(),
            })
        }
    };

    let cmd = format!(
        "inventory.giveto {} {} {}",
        body.steam_id, body.item, body.amount
    );
    match rcon.execute(&cmd).await {
        Ok(msg) => HttpResponse::Ok().json(SuccessBody {
            success: true,
            message: format!("Gave {} x{} to {}: {}", body.item, body.amount, body.steam_id, msg),
        }),
        Err(e) => HttpResponse::InternalServerError().json(ErrorBody {
            error: format!("Failed to give item: {}", e),
        }),
    }
}
