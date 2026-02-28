use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::Mutex;

use crate::monitor::SystemMonitor;
use crate::registry::ServerRegistry;

/// Mutex to prevent concurrent LinuxGSM operations per server.
pub struct LgsmLock {
    pub lock: Mutex<()>,
}

impl LgsmLock {
    pub fn new() -> Self {
        Self {
            lock: Mutex::new(()),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CommandResult {
    success: bool,
    output: String,
    action: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ServerStatus {
    online: bool,
    players: u32,
    max_players: u32,
    fps: f64,
    hostname: String,
    map: String,
    entities: u64,
    uptime: u64,
    cpu_percent: f32,
    mem_used: u64,
    mem_total: u64,
    mem_percent: f32,
    disk_used: u64,
    disk_total: u64,
    disk_percent: f32,
}

#[derive(Debug, Deserialize)]
pub struct WipeRequest {
    #[serde(rename = "type")]
    pub wipe_type: String,
    pub seed: Option<String>,
}

/// Run a LinuxGSM command and capture output.
async fn run_lgsm_command(script: &str, action: &str) -> anyhow::Result<String> {
    tracing::info!("Running LGSM command: {} {}", script, action);

    let output = Command::new(script).arg(action).output().await?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let combined = if stderr.is_empty() {
        stdout
    } else {
        format!("{}\n{}", stdout, stderr)
    };

    if !output.status.success() {
        tracing::warn!(
            "LGSM command '{}' exited with status: {}",
            action,
            output.status
        );
    }

    Ok(combined)
}

async fn lgsm_action(
    server_id: web::Path<String>,
    registry: web::Data<Arc<ServerRegistry>>,
    action: &str,
) -> HttpResponse {
    let config = match registry.get_config(&server_id).await {
        Some(c) => c,
        None => {
            return HttpResponse::NotFound()
                .json(serde_json::json!({"error": "Server not found"}))
        }
    };
    let lgsm_lock = match registry.get_lgsm_lock(&server_id).await {
        Some(l) => l,
        None => {
            return HttpResponse::NotFound()
                .json(serde_json::json!({"error": "Server runtime not found"}))
        }
    };

    let _guard = lgsm_lock.lock.lock().await;
    match run_lgsm_command(&config.paths.lgsm_script, action).await {
        Ok(output) => HttpResponse::Ok().json(CommandResult {
            success: true,
            output,
            action: action.to_string(),
        }),
        Err(e) => HttpResponse::InternalServerError().json(CommandResult {
            success: false,
            output: e.to_string(),
            action: action.to_string(),
        }),
    }
}

pub async fn server_start(
    server_id: web::Path<String>,
    registry: web::Data<Arc<ServerRegistry>>,
) -> HttpResponse {
    lgsm_action(server_id, registry, "start").await
}

pub async fn server_stop(
    server_id: web::Path<String>,
    registry: web::Data<Arc<ServerRegistry>>,
) -> HttpResponse {
    lgsm_action(server_id, registry, "stop").await
}

pub async fn server_restart(
    server_id: web::Path<String>,
    registry: web::Data<Arc<ServerRegistry>>,
) -> HttpResponse {
    lgsm_action(server_id, registry, "restart").await
}

pub async fn server_update(
    server_id: web::Path<String>,
    registry: web::Data<Arc<ServerRegistry>>,
) -> HttpResponse {
    lgsm_action(server_id, registry, "update").await
}

pub async fn server_backup(
    server_id: web::Path<String>,
    registry: web::Data<Arc<ServerRegistry>>,
) -> HttpResponse {
    lgsm_action(server_id, registry, "backup").await
}

pub async fn server_force_update(
    server_id: web::Path<String>,
    registry: web::Data<Arc<ServerRegistry>>,
) -> HttpResponse {
    lgsm_action(server_id, registry, "force-update").await
}

pub async fn server_validate(
    server_id: web::Path<String>,
    registry: web::Data<Arc<ServerRegistry>>,
) -> HttpResponse {
    lgsm_action(server_id, registry, "validate").await
}

pub async fn server_check_update(
    server_id: web::Path<String>,
    registry: web::Data<Arc<ServerRegistry>>,
) -> HttpResponse {
    lgsm_action(server_id, registry, "check-update").await
}

pub async fn server_monitor_check(
    server_id: web::Path<String>,
    registry: web::Data<Arc<ServerRegistry>>,
) -> HttpResponse {
    lgsm_action(server_id, registry, "monitor").await
}

pub async fn server_details(
    server_id: web::Path<String>,
    registry: web::Data<Arc<ServerRegistry>>,
) -> HttpResponse {
    lgsm_action(server_id, registry, "details").await
}

pub async fn server_update_lgsm(
    server_id: web::Path<String>,
    registry: web::Data<Arc<ServerRegistry>>,
) -> HttpResponse {
    lgsm_action(server_id, registry, "update-lgsm").await
}

pub async fn server_full_wipe(
    server_id: web::Path<String>,
    registry: web::Data<Arc<ServerRegistry>>,
) -> HttpResponse {
    lgsm_action(server_id, registry, "full-wipe").await
}

pub async fn server_map_wipe(
    server_id: web::Path<String>,
    registry: web::Data<Arc<ServerRegistry>>,
) -> HttpResponse {
    lgsm_action(server_id, registry, "map-wipe").await
}

/// POST /api/servers/{server_id}/save - RCON server.save
pub async fn server_save(
    server_id: web::Path<String>,
    registry: web::Data<Arc<ServerRegistry>>,
) -> HttpResponse {
    let rcon = match registry.get_rcon(&server_id).await {
        Some(r) => r,
        None => {
            return HttpResponse::NotFound()
                .json(serde_json::json!({"error": "Server not found"}))
        }
    };
    match rcon.save().await {
        Ok(output) => HttpResponse::Ok().json(CommandResult {
            success: true,
            output,
            action: "save".to_string(),
        }),
        Err(e) => HttpResponse::InternalServerError().json(CommandResult {
            success: false,
            output: e.to_string(),
            action: "save".to_string(),
        }),
    }
}

/// POST /api/servers/{server_id}/wipe
pub async fn server_wipe(
    server_id: web::Path<String>,
    body: web::Json<WipeRequest>,
    registry: web::Data<Arc<ServerRegistry>>,
) -> HttpResponse {
    let config = match registry.get_config(&server_id).await {
        Some(c) => c,
        None => {
            return HttpResponse::NotFound()
                .json(serde_json::json!({"error": "Server not found"}))
        }
    };
    let lgsm_lock = match registry.get_lgsm_lock(&server_id).await {
        Some(l) => l,
        None => {
            return HttpResponse::NotFound()
                .json(serde_json::json!({"error": "Server runtime not found"}))
        }
    };

    let _guard = lgsm_lock.lock.lock().await;

    let server_dir = format!("{}/server/rustserver", config.paths.server_files);

    if let Err(e) = run_lgsm_command(&config.paths.lgsm_script, "stop").await {
        tracing::warn!("Failed to stop server before wipe: {}", e);
    }

    let mut deleted_files = Vec::new();
    let mut errors = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&server_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                let should_delete = match body.wipe_type.as_str() {
                    "full" => ext == "sav" || ext == "map" || ext == "db",
                    _ => ext == "sav" || ext == "map",
                };
                if should_delete {
                    match std::fs::remove_file(&path) {
                        Ok(()) => {
                            deleted_files.push(path.display().to_string());
                        }
                        Err(e) => {
                            errors.push(format!("Failed to delete {}: {}", path.display(), e));
                        }
                    }
                }
            }
        }
    }

    if let Some(ref seed) = body.seed {
        if let Err(e) = update_server_seed(&config.paths.server_cfg, seed) {
            errors.push(format!("Failed to update seed: {}", e));
        }
    }

    let start_output = run_lgsm_command(&config.paths.lgsm_script, "start")
        .await
        .unwrap_or_else(|e| format!("Failed to start server: {}", e));

    let output = format!(
        "Wipe type: {}\nDeleted files: {}\nErrors: {}\nServer start: {}",
        body.wipe_type,
        if deleted_files.is_empty() {
            "none".to_string()
        } else {
            deleted_files.join(", ")
        },
        if errors.is_empty() {
            "none".to_string()
        } else {
            errors.join(", ")
        },
        start_output
    );

    HttpResponse::Ok().json(CommandResult {
        success: errors.is_empty(),
        output,
        action: "wipe".to_string(),
    })
}

fn update_server_seed(cfg_path: &str, seed: &str) -> anyhow::Result<()> {
    let content = std::fs::read_to_string(cfg_path)?;
    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();

    let mut found = false;
    for line in &mut lines {
        if line.starts_with("server.seed") {
            *line = format!("server.seed \"{}\"", seed);
            found = true;
            break;
        }
    }
    if !found {
        lines.push(format!("server.seed \"{}\"", seed));
    }

    std::fs::write(cfg_path, lines.join("\n"))?;
    Ok(())
}

/// GET /api/servers/{server_id}/status
pub async fn server_status(
    server_id: web::Path<String>,
    registry: web::Data<Arc<ServerRegistry>>,
    sys_monitor: web::Data<Arc<SystemMonitor>>,
) -> HttpResponse {
    let rcon = match registry.get_rcon(&server_id).await {
        Some(r) => r,
        None => {
            return HttpResponse::NotFound()
                .json(serde_json::json!({"error": "Server not found"}))
        }
    };

    let sys_history = sys_monitor.history.read().await;
    let sys = sys_history.latest().cloned();
    drop(sys_history);

    let game_monitor = registry.get_game_monitor(&server_id).await;
    let game = if let Some(ref gm) = game_monitor {
        let game_history = gm.history.read().await;
        game_history.latest().cloned()
    } else {
        None
    };

    let (online, players, max_players, fps, hostname, map, entities, uptime) =
        if let Some(ref g) = game {
            (
                g.online,
                g.players,
                g.max_players,
                g.fps,
                g.hostname.clone(),
                g.map.clone(),
                g.entities,
                g.uptime,
            )
        } else {
            match rcon.server_info().await {
                Ok(info) => (
                    true,
                    info.players,
                    info.max_players,
                    info.framerate,
                    info.hostname,
                    info.map,
                    info.entity_count,
                    info.uptime,
                ),
                Err(_) => (false, 0, 0, 0.0, String::new(), String::new(), 0, 0),
            }
        };

    let status = ServerStatus {
        online,
        players,
        max_players,
        fps,
        hostname,
        map,
        entities,
        uptime,
        cpu_percent: sys.as_ref().map(|s| s.cpu_percent).unwrap_or(0.0),
        mem_used: sys.as_ref().map(|s| s.mem_used).unwrap_or(0),
        mem_total: sys.as_ref().map(|s| s.mem_total).unwrap_or(0),
        mem_percent: sys.as_ref().map(|s| s.mem_percent).unwrap_or(0.0),
        disk_used: sys.as_ref().map(|s| s.disk_used).unwrap_or(0),
        disk_total: sys.as_ref().map(|s| s.disk_total).unwrap_or(0),
        disk_percent: sys.as_ref().map(|s| s.disk_percent).unwrap_or(0.0),
    };

    HttpResponse::Ok().json(status)
}
