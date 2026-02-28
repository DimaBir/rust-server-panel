use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::registry::ServerRegistry;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerPosition {
    pub steam_id: String,
    pub display_name: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePositionsBody {
    pub players: Vec<PlayerPosition>,
    pub token: String,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: String,
}

pub struct PositionStore {
    pub positions: RwLock<HashMap<String, Vec<PlayerPosition>>>,
}

impl PositionStore {
    pub fn new() -> Self {
        Self {
            positions: RwLock::new(HashMap::new()),
        }
    }
}

/// GET /api/servers/{server_id}/map
pub async fn get_map_info(
    server_id: web::Path<String>,
    registry: web::Data<Arc<ServerRegistry>>,
) -> HttpResponse {
    let def = match registry.get_definition(&server_id).await {
        Some(d) => d,
        None => {
            return HttpResponse::NotFound().json(ErrorBody {
                error: "Server not found".to_string(),
            })
        }
    };

    // Try to get live seed/worldSize from RCON serverinfo
    let (seed, world_size) = if let Some(rcon) = registry.get_rcon(&server_id).await {
        match rcon.server_info().await {
            Ok(info) if info.seed > 0 => (info.seed, if info.world_size > 0 { info.world_size } else { def.world_size }),
            _ => (def.seed, def.world_size),
        }
    } else {
        (def.seed, def.world_size)
    };

    let image_url = format!(
        "https://content.rustmaps.com/maps/{}_{}/map_icons.png",
        world_size, seed
    );

    HttpResponse::Ok().json(serde_json::json!({
        "seed": seed,
        "worldSize": world_size,
        "imageUrl": image_url,
    }))
}

/// GET /api/servers/{server_id}/positions
pub async fn get_positions(
    server_id: web::Path<String>,
    store: web::Data<Arc<PositionStore>>,
    registry: web::Data<Arc<ServerRegistry>>,
) -> HttpResponse {
    // Verify server exists
    if registry.get_definition(&server_id).await.is_none() {
        return HttpResponse::NotFound().json(ErrorBody {
            error: "Server not found".to_string(),
        });
    }

    let positions = store.positions.read().await;
    let players = positions
        .get(server_id.as_str())
        .cloned()
        .unwrap_or_default();

    HttpResponse::Ok().json(serde_json::json!({
        "players": players,
    }))
}

/// POST /api/servers/{server_id}/positions
/// Authenticated via RCON password in body (not JWT).
pub async fn update_positions(
    server_id: web::Path<String>,
    body: web::Json<UpdatePositionsBody>,
    store: web::Data<Arc<PositionStore>>,
    registry: web::Data<Arc<ServerRegistry>>,
) -> HttpResponse {
    // Verify server exists and token matches RCON password
    let def = match registry.get_definition(&server_id).await {
        Some(d) => d,
        None => {
            return HttpResponse::NotFound().json(ErrorBody {
                error: "Server not found".to_string(),
            })
        }
    };

    if body.token != def.rcon_password {
        return HttpResponse::Unauthorized().json(ErrorBody {
            error: "Invalid token".to_string(),
        });
    }

    let mut positions = store.positions.write().await;
    positions.insert(server_id.into_inner(), body.players.clone());

    HttpResponse::Ok().json(serde_json::json!({
        "success": true,
    }))
}
