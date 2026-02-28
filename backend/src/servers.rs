use actix_web::{web, HttpResponse};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;

use crate::config::GameServerConfig;
use crate::monitor::GameMonitor;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ServerListEntry {
    id: String,
    name: String,
    online: bool,
}

/// GET /api/servers — list all configured servers with online status.
pub async fn list_servers(
    server_configs: web::Data<Vec<GameServerConfig>>,
    game_monitors: web::Data<HashMap<String, Arc<GameMonitor>>>,
) -> HttpResponse {
    let mut entries = Vec::new();

    for config in server_configs.iter() {
        let online = if let Some(monitor) = game_monitors.get(&config.id) {
            let history = monitor.history.read().await;
            history.latest().map(|s| s.online).unwrap_or(false)
        } else {
            false
        };

        entries.push(ServerListEntry {
            id: config.id.clone(),
            name: config.name.clone(),
            online,
        });
    }

    HttpResponse::Ok().json(entries)
}
