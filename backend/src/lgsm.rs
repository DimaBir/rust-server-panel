use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::Mutex;

use crate::config::AppConfig;
use crate::monitor::{GameMonitor, SystemMonitor};
use crate::rcon::RconClient;

/// Mutex to prevent concurrent LinuxGSM operations.
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
struct CommandResult {
    success: bool,
    output: String,
    action: String,
}

#[derive(Debug, Serialize)]
struct ServerStatus {
    /// Whether the game process is running (from RCON)
    game_online: bool,
    /// Game metrics if available
    players: u32,
    max_players: u32,
    fps: f64,
    hostname: String,
    map_name: String,
    uptime: u64,
    /// System metrics
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
    pub wipe_type: String, // "map" or "full"
    pub seed: Option<String>,
}

/// Run a LinuxGSM command and capture output.
async fn run_lgsm_command(script: &str, action: &str) -> anyhow::Result<String> {
    tracing::info!("Running LGSM command: {} {}", script, action);

    let output = Command::new(script)
        .arg(action)
        .output()
        .await?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let combined = if stderr.is_empty() {
        stdout
    } else {
        format!("{}\n{}", stdout, stderr)
    };

    if !output.status.success() {
        tracing::warn!("LGSM command '{}' exited with status: {}", action, output.status);
    }

    Ok(combined)
}

/// POST /api/server/start
pub async fn server_start(
    config: web::Data<AppConfig>,
    lgsm_lock: web::Data<Arc<LgsmLock>>,
) -> HttpResponse {
    let _guard = lgsm_lock.lock.lock().await;
    match run_lgsm_command(&config.paths.lgsm_script, "start").await {
        Ok(output) => HttpResponse::Ok().json(CommandResult {
            success: true,
            output,
            action: "start".to_string(),
        }),
        Err(e) => HttpResponse::InternalServerError().json(CommandResult {
            success: false,
            output: e.to_string(),
            action: "start".to_string(),
        }),
    }
}

/// POST /api/server/stop
pub async fn server_stop(
    config: web::Data<AppConfig>,
    lgsm_lock: web::Data<Arc<LgsmLock>>,
) -> HttpResponse {
    let _guard = lgsm_lock.lock.lock().await;
    match run_lgsm_command(&config.paths.lgsm_script, "stop").await {
        Ok(output) => HttpResponse::Ok().json(CommandResult {
            success: true,
            output,
            action: "stop".to_string(),
        }),
        Err(e) => HttpResponse::InternalServerError().json(CommandResult {
            success: false,
            output: e.to_string(),
            action: "stop".to_string(),
        }),
    }
}

/// POST /api/server/restart
pub async fn server_restart(
    config: web::Data<AppConfig>,
    lgsm_lock: web::Data<Arc<LgsmLock>>,
) -> HttpResponse {
    let _guard = lgsm_lock.lock.lock().await;
    match run_lgsm_command(&config.paths.lgsm_script, "restart").await {
        Ok(output) => HttpResponse::Ok().json(CommandResult {
            success: true,
            output,
            action: "restart".to_string(),
        }),
        Err(e) => HttpResponse::InternalServerError().json(CommandResult {
            success: false,
            output: e.to_string(),
            action: "restart".to_string(),
        }),
    }
}

/// POST /api/server/update
pub async fn server_update(
    config: web::Data<AppConfig>,
    lgsm_lock: web::Data<Arc<LgsmLock>>,
) -> HttpResponse {
    let _guard = lgsm_lock.lock.lock().await;
    match run_lgsm_command(&config.paths.lgsm_script, "update").await {
        Ok(output) => HttpResponse::Ok().json(CommandResult {
            success: true,
            output,
            action: "update".to_string(),
        }),
        Err(e) => HttpResponse::InternalServerError().json(CommandResult {
            success: false,
            output: e.to_string(),
            action: "update".to_string(),
        }),
    }
}

/// POST /api/server/backup
pub async fn server_backup(
    config: web::Data<AppConfig>,
    lgsm_lock: web::Data<Arc<LgsmLock>>,
) -> HttpResponse {
    let _guard = lgsm_lock.lock.lock().await;
    match run_lgsm_command(&config.paths.lgsm_script, "backup").await {
        Ok(output) => HttpResponse::Ok().json(CommandResult {
            success: true,
            output,
            action: "backup".to_string(),
        }),
        Err(e) => HttpResponse::InternalServerError().json(CommandResult {
            success: false,
            output: e.to_string(),
            action: "backup".to_string(),
        }),
    }
}

/// POST /api/server/save - RCON server.save
pub async fn server_save(
    rcon: web::Data<Arc<RconClient>>,
) -> HttpResponse {
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

/// POST /api/server/wipe
pub async fn server_wipe(
    body: web::Json<WipeRequest>,
    config: web::Data<AppConfig>,
    lgsm_lock: web::Data<Arc<LgsmLock>>,
) -> HttpResponse {
    let _guard = lgsm_lock.lock.lock().await;

    let server_dir = format!("{}/server/rustserver", config.paths.server_files);

    // Stop the server first
    if let Err(e) = run_lgsm_command(&config.paths.lgsm_script, "stop").await {
        tracing::warn!("Failed to stop server before wipe: {}", e);
    }

    let mut deleted_files = Vec::new();
    let mut errors = Vec::new();

    // Delete .sav and .map files
    if let Ok(entries) = std::fs::read_dir(&server_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                let should_delete = match body.wipe_type.as_str() {
                    "full" => ext == "sav" || ext == "map" || ext == "db",
                    _ => ext == "sav" || ext == "map", // "map" wipe type
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

    // Update seed in server.cfg if provided
    if let Some(ref seed) = body.seed {
        if let Err(e) = update_server_seed(&config.paths.server_cfg, seed) {
            errors.push(format!("Failed to update seed: {}", e));
        }
    }

    // Start the server again
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

/// Update the server.seed value in server.cfg.
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

/// GET /api/server/status - combined server + system info.
pub async fn server_status(
    rcon: web::Data<Arc<RconClient>>,
    sys_monitor: web::Data<Arc<SystemMonitor>>,
    game_monitor: web::Data<Arc<GameMonitor>>,
) -> HttpResponse {
    // Get latest system snapshot
    let sys_history = sys_monitor.history.read().await;
    let sys = sys_history.latest().cloned();
    drop(sys_history);

    // Get latest game snapshot
    let game_history = game_monitor.history.read().await;
    let game = game_history.latest().cloned();
    drop(game_history);

    // If no game snapshot, try RCON directly
    let (game_online, players, max_players, fps, hostname, map_name, uptime) =
        if let Some(ref g) = game {
            (
                g.online,
                g.players,
                g.max_players,
                g.fps,
                g.hostname.clone(),
                g.map_name.clone(),
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
                    info.uptime,
                ),
                Err(_) => (false, 0, 0, 0.0, String::new(), String::new(), 0),
            }
        };

    let status = ServerStatus {
        game_online,
        players,
        max_players,
        fps,
        hostname,
        map_name,
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
