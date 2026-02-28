use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::config::AppConfig;
use crate::provisioner;
use crate::registry::{
    ProvisioningStatus, ServerDefinition, ServerRegistry, ServerSource, ServerType,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ServerListEntry {
    id: String,
    name: String,
    online: bool,
    server_type: String,
    game_port: u16,
    rcon_port: u16,
    query_port: u16,
    max_players: u32,
    world_size: u32,
    seed: u32,
    provisioning_status: String,
    source: String,
    players: Option<u32>,
    created_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateServerRequest {
    pub name: String,
    pub server_type: String,
    pub max_players: Option<u32>,
    pub world_size: Option<u32>,
    pub seed: Option<u32>,
    pub hostname: Option<String>,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: String,
}

#[derive(Debug, Serialize)]
struct SuccessBody {
    success: bool,
    message: String,
}

fn status_to_string(status: &ProvisioningStatus) -> String {
    match status {
        ProvisioningStatus::Ready => "ready",
        ProvisioningStatus::Installing => "installing",
        ProvisioningStatus::Downloading => "downloading",
        ProvisioningStatus::InstallingOxide => "installing_oxide",
        ProvisioningStatus::Configuring => "configuring",
        ProvisioningStatus::Error => "error",
    }
    .to_string()
}

fn source_to_string(source: &ServerSource) -> String {
    match source {
        ServerSource::Static => "static",
        ServerSource::Dynamic => "dynamic",
    }
    .to_string()
}

fn type_to_string(st: &ServerType) -> String {
    match st {
        ServerType::Vanilla => "vanilla",
        ServerType::Modded => "modded",
    }
    .to_string()
}

/// GET /api/servers — list all servers with extended info.
pub async fn list_servers(registry: web::Data<Arc<ServerRegistry>>) -> HttpResponse {
    let defs = registry.all_definitions().await;
    let mut entries = Vec::new();

    for def in &defs {
        let (online, players, live_max_players) = if let Some(monitor) = registry.get_game_monitor(&def.id).await {
            let history = monitor.history.read().await;
            if let Some(snap) = history.latest() {
                (snap.online, Some(snap.players), if snap.max_players > 0 { Some(snap.max_players) } else { None })
            } else {
                (false, None, None)
            }
        } else {
            (false, None, None)
        };

        entries.push(ServerListEntry {
            id: def.id.clone(),
            name: def.name.clone(),
            online,
            server_type: type_to_string(&def.server_type),
            game_port: def.game_port,
            rcon_port: def.rcon_port,
            query_port: def.query_port,
            max_players: live_max_players.unwrap_or(def.max_players),
            world_size: def.world_size,
            seed: def.seed,
            provisioning_status: status_to_string(&def.provisioning_status),
            source: source_to_string(&def.source),
            players,
            created_at: def.created_at.to_rfc3339(),
        });
    }

    HttpResponse::Ok().json(entries)
}

/// POST /api/servers — create a new server.
pub async fn create_server(
    body: web::Json<CreateServerRequest>,
    registry: web::Data<Arc<ServerRegistry>>,
    config: web::Data<AppConfig>,
) -> HttpResponse {
    // Validate
    let defs = registry.all_definitions().await;
    if defs.len() >= config.provisioning.max_servers {
        return HttpResponse::BadRequest().json(ErrorBody {
            error: format!("Maximum of {} servers reached", config.provisioning.max_servers),
        });
    }

    let server_type = match body.server_type.to_lowercase().as_str() {
        "vanilla" => ServerType::Vanilla,
        "modded" => ServerType::Modded,
        _ => {
            return HttpResponse::BadRequest().json(ErrorBody {
                error: "Invalid server type. Use 'vanilla' or 'modded'".to_string(),
            })
        }
    };

    // Generate unique ID
    let id = format!(
        "srv-{}",
        uuid::Uuid::new_v4().to_string().split('-').next().unwrap()
    );

    // Allocate ports
    let (game_port, rcon_port, query_port) =
        provisioner::allocate_ports(&defs, &config.provisioning);

    // Generate random RCON password
    let rcon_password: String = (0..16)
        .map(|_| {
            let idx = rand::random::<u8>() % 36;
            if idx < 10 {
                (b'0' + idx) as char
            } else {
                (b'a' + idx - 10) as char
            }
        })
        .collect();

    let seed = body.seed.unwrap_or_else(|| rand::random::<u32>() % 999999 + 1);
    let world_size = body.world_size.unwrap_or(4000);
    let max_players = body.max_players.unwrap_or(100);
    let hostname = body
        .hostname
        .clone()
        .unwrap_or_else(|| body.name.clone());

    let def = ServerDefinition {
        id: id.clone(),
        name: body.name.clone(),
        server_type,
        source: ServerSource::Dynamic,
        provisioning_status: ProvisioningStatus::Installing,
        provisioning_log: Vec::new(),
        game_port,
        rcon_port,
        query_port,
        max_players,
        world_size,
        seed,
        hostname,
        rcon_password,
        base_path: config.provisioning.base_path.clone(),
        created_at: chrono::Utc::now(),
    };

    // Add to registry
    {
        let mut defs = registry.definitions.write().await;
        defs.push(def.clone());
    }

    // Save dynamic servers to disk
    {
        let defs = registry.definitions.read().await;
        let dynamic: Vec<_> = defs
            .iter()
            .filter(|d| d.source == ServerSource::Dynamic)
            .cloned()
            .collect();
        if let Err(e) = crate::persistence::save_servers(&dynamic) {
            tracing::error!("Failed to save servers: {}", e);
        }
    }

    // Spawn provisioning task
    let registry_clone = registry.into_inner().as_ref().clone();
    let config_clone = config.into_inner().as_ref().clone();
    let def_clone = def.clone();
    tokio::spawn(async move {
        provisioner::provision_server(def_clone, registry_clone, config_clone).await;
    });

    HttpResponse::Created().json(serde_json::json!({
        "id": id,
        "name": body.name,
        "status": "installing",
    }))
}

/// DELETE /api/servers/{server_id} — remove a dynamic server.
pub async fn delete_server(
    server_id: web::Path<String>,
    registry: web::Data<Arc<ServerRegistry>>,
) -> HttpResponse {
    let server_id = server_id.into_inner();

    // Check if server exists and is dynamic
    let def = match registry.get_definition(&server_id).await {
        Some(d) => d,
        None => {
            return HttpResponse::NotFound().json(ErrorBody {
                error: "Server not found".to_string(),
            })
        }
    };

    if def.source == ServerSource::Static {
        return HttpResponse::BadRequest().json(ErrorBody {
            error: "Cannot delete a static server (defined in config.yaml)".to_string(),
        });
    }

    // Remove runtime (stop collector)
    {
        let mut runtimes = registry.runtimes.write().await;
        if let Some(runtime) = runtimes.remove(&server_id) {
            if let Some(handle) = runtime.collector_handle {
                handle.abort();
            }
        }
    }

    // Remove definition
    {
        let mut defs = registry.definitions.write().await;
        defs.retain(|d| d.id != server_id);
    }

    // Save updated dynamic servers
    {
        let defs = registry.definitions.read().await;
        let dynamic: Vec<_> = defs
            .iter()
            .filter(|d| d.source == ServerSource::Dynamic)
            .cloned()
            .collect();
        if let Err(e) = crate::persistence::save_servers(&dynamic) {
            tracing::error!("Failed to save servers: {}", e);
        }
    }

    HttpResponse::Ok().json(SuccessBody {
        success: true,
        message: format!("Server '{}' deleted", server_id),
    })
}

/// GET /api/servers/{server_id}/provision-status
pub async fn provision_status(
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

    HttpResponse::Ok().json(serde_json::json!({
        "status": status_to_string(&def.provisioning_status),
        "log": def.provisioning_log,
    }))
}
