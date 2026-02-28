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

/// Cache for RustMaps image URLs (keyed by "size_seed").
pub struct MapImageCache {
    cache: RwLock<HashMap<String, String>>,
}

impl MapImageCache {
    pub fn new() -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
        }
    }
}

/// Fetch the map image URL from the RustMaps page HTML.
async fn fetch_rustmaps_image_url(world_size: u32, seed: u32) -> Option<String> {
    let page_url = format!("https://rustmaps.com/map/{}_{}", world_size, seed);
    let html = reqwest::get(&page_url).await.ok()?.text().await.ok()?;
    // Look for the map_icons.png URL in the HTML
    // Pattern: https://content.rustmaps.com/maps/{ver}/{hash}/map_icons.png
    for segment in html.split("https://content.rustmaps.com/maps/") {
        if let Some(end) = segment.find("/map_icons.png") {
            let path = &segment[..end];
            return Some(format!(
                "https://content.rustmaps.com/maps/{}/map_icons.png",
                path
            ));
        }
    }
    None
}

/// GET /api/servers/{server_id}/map
pub async fn get_map_info(
    server_id: web::Path<String>,
    registry: web::Data<Arc<ServerRegistry>>,
    map_cache: web::Data<Arc<MapImageCache>>,
) -> HttpResponse {
    let def = match registry.get_definition(&server_id).await {
        Some(d) => d,
        None => {
            return HttpResponse::NotFound().json(ErrorBody {
                error: "Server not found".to_string(),
            })
        }
    };

    // Try to get live seed/worldSize from RCON convar queries
    let (seed, world_size) = if let Some(rcon) = registry.get_rcon(&server_id).await {
        let seed_raw = rcon.execute("server.seed").await.unwrap_or_default();
        let ws_raw = rcon.execute("server.worldsize").await.unwrap_or_default();
        let parse_convar = |raw: &str| -> Option<u32> {
            raw.rsplit(':').next()
                .map(|s| s.trim().trim_matches('"').trim())
                .and_then(|s| s.parse::<u32>().ok())
        };
        let seed = parse_convar(&seed_raw).filter(|&s| s > 0).unwrap_or(def.seed);
        let ws = parse_convar(&ws_raw).filter(|&s| s > 0).unwrap_or(def.world_size);
        (seed, ws)
    } else {
        (def.seed, def.world_size)
    };

    // Look up cached image URL or fetch from RustMaps
    let cache_key = format!("{}_{}", world_size, seed);
    let image_url = {
        let cache = map_cache.cache.read().await;
        cache.get(&cache_key).cloned()
    };

    let image_url = match image_url {
        Some(url) => url,
        None => {
            let url = fetch_rustmaps_image_url(world_size, seed)
                .await
                .unwrap_or_default();
            if !url.is_empty() {
                let mut cache = map_cache.cache.write().await;
                cache.insert(cache_key, url.clone());
            }
            url
        }
    };

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
