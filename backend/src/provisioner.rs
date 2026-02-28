use std::sync::Arc;

use crate::config::{AppConfig, ProvisioningConfig};
use crate::lgsm::LgsmLock;
use crate::monitor::GameMonitor;
use crate::rcon::RconClient;
use crate::registry::{
    ProvisioningStatus, ServerDefinition, ServerRegistry, ServerRuntime, ServerSource, ServerType,
};

/// The non-root user that runs LinuxGSM commands inside the container.
const GAME_USER: &str = "gameserver";

/// Allocate the next free ports based on existing definitions.
pub fn allocate_ports(
    existing: &[ServerDefinition],
    config: &ProvisioningConfig,
) -> (u16, u16, u16) {
    let mut max_slot: u16 = 0;
    for def in existing {
        if def.game_port >= config.port_range_start {
            let slot = (def.game_port - config.port_range_start) / config.port_offset + 1;
            if slot > max_slot {
                max_slot = slot;
            }
        }
    }
    let game_port = config.port_range_start + max_slot * config.port_offset;
    let rcon_port = game_port + 1;
    let query_port = game_port - 1000; // e.g., 28015 -> 27015

    (game_port, rcon_port, query_port)
}

/// Helper: run a shell command as the game user and return (success, stdout, stderr).
async fn run_as_user(cmd: &str) -> Result<std::process::Output, std::io::Error> {
    tokio::process::Command::new("su")
        .args(["-", GAME_USER, "-c", cmd])
        .output()
        .await
}

/// Format command output for logging.
fn format_output(output: &std::process::Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let exit_code = output
        .status
        .code()
        .map(|c| c.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let mut msg = format!("Exit code: {}", exit_code);
    let stdout_trimmed = stdout.trim();
    let stderr_trimmed = stderr.trim();
    if !stdout_trimmed.is_empty() {
        // Only include last 2000 chars to avoid huge log entries
        let s = if stdout_trimmed.len() > 2000 {
            &stdout_trimmed[stdout_trimmed.len() - 2000..]
        } else {
            stdout_trimmed
        };
        msg.push_str(&format!("\nSTDOUT: {}", s));
    }
    if !stderr_trimmed.is_empty() {
        let s = if stderr_trimmed.len() > 2000 {
            &stderr_trimmed[stderr_trimmed.len() - 2000..]
        } else {
            stderr_trimmed
        };
        msg.push_str(&format!("\nSTDERR: {}", s));
    }
    msg
}

/// Run the full provisioning pipeline for a new server.
pub async fn provision_server(
    def: ServerDefinition,
    registry: Arc<ServerRegistry>,
    config: AppConfig,
) {
    let server_id = def.id.clone();
    let base_dir = format!("{}/rustserver-{}", def.base_path, def.id);

    tracing::info!("Starting provisioning for server '{}'", server_id);

    // Step 1: Create directory and set ownership
    update_status(
        &registry,
        &server_id,
        ProvisioningStatus::Installing,
        "Creating server directory...",
    )
    .await;

    if let Err(e) = std::fs::create_dir_all(&base_dir) {
        update_status(
            &registry,
            &server_id,
            ProvisioningStatus::Error,
            &format!("Failed to create directory: {}", e),
        )
        .await;
        return;
    }

    // chown the directory to the game user
    let chown_result = tokio::process::Command::new("chown")
        .args(["-R", &format!("{}:{}", GAME_USER, GAME_USER), &base_dir])
        .output()
        .await;

    if let Err(e) = chown_result {
        update_status(
            &registry,
            &server_id,
            ProvisioningStatus::Error,
            &format!("Failed to chown directory: {}", e),
        )
        .await;
        return;
    }

    // Step 1b: Download and set up LinuxGSM
    update_status(
        &registry,
        &server_id,
        ProvisioningStatus::Installing,
        "Downloading LinuxGSM...",
    )
    .await;

    let lgsm_cmd = format!(
        "cd '{}' && curl -Lo linuxgsm.sh https://linuxgsm.sh && chmod +x linuxgsm.sh && bash linuxgsm.sh rustserver",
        base_dir
    );

    let download_result = run_as_user(&lgsm_cmd).await;

    match download_result {
        Ok(ref output) if output.status.success() => {
            update_status(
                &registry,
                &server_id,
                ProvisioningStatus::Installing,
                "LinuxGSM installed",
            )
            .await;
        }
        Ok(ref output) => {
            update_status(
                &registry,
                &server_id,
                ProvisioningStatus::Error,
                &format!("LinuxGSM install failed\n{}", format_output(output)),
            )
            .await;
            return;
        }
        Err(e) => {
            update_status(
                &registry,
                &server_id,
                ProvisioningStatus::Error,
                &format!("Failed to run LinuxGSM setup: {}", e),
            )
            .await;
            return;
        }
    }

    // Step 2: Install the game server
    update_status(
        &registry,
        &server_id,
        ProvisioningStatus::Downloading,
        "Downloading Rust server files (this may take a while)...",
    )
    .await;

    let install_cmd = format!("cd '{}' && ./rustserver auto-install", base_dir);
    let install_result = run_as_user(&install_cmd).await;

    match install_result {
        Ok(ref output) if output.status.success() => {
            update_status(
                &registry,
                &server_id,
                ProvisioningStatus::Downloading,
                "Game server files installed",
            )
            .await;
        }
        Ok(ref output) => {
            update_status(
                &registry,
                &server_id,
                ProvisioningStatus::Error,
                &format!("Server install failed\n{}", format_output(output)),
            )
            .await;
            return;
        }
        Err(e) => {
            update_status(
                &registry,
                &server_id,
                ProvisioningStatus::Error,
                &format!("Failed to run server install: {}", e),
            )
            .await;
            return;
        }
    }

    // Step 3: Install Oxide (if modded)
    if def.server_type == ServerType::Modded {
        update_status(
            &registry,
            &server_id,
            ProvisioningStatus::InstallingOxide,
            "Installing Oxide/uMod framework...",
        )
        .await;

        let oxide_cmd = format!(
            "cd '{}/serverfiles' && curl -Lo Oxide.Rust.zip https://umod.org/games/rust/download && unzip -o Oxide.Rust.zip && rm -f Oxide.Rust.zip",
            base_dir
        );
        let oxide_result = run_as_user(&oxide_cmd).await;

        match oxide_result {
            Ok(ref output) if output.status.success() => {
                update_status(
                    &registry,
                    &server_id,
                    ProvisioningStatus::InstallingOxide,
                    "Oxide installed",
                )
                .await;
            }
            Ok(_) | Err(_) => {
                update_status(
                    &registry,
                    &server_id,
                    ProvisioningStatus::InstallingOxide,
                    "Oxide install failed (non-fatal, continuing...)",
                )
                .await;
            }
        }
    }

    // Step 4: Configure server.cfg
    update_status(
        &registry,
        &server_id,
        ProvisioningStatus::Configuring,
        "Writing server configuration...",
    )
    .await;

    let cfg_dir = format!("{}/serverfiles/server/rustserver/cfg", base_dir);
    let _ = std::fs::create_dir_all(&cfg_dir);

    let server_cfg = format!(
        r#"server.hostname "{hostname}"
server.seed "{seed}"
server.worldsize "{worldsize}"
server.maxplayers "{maxplayers}"
rcon.ip 0.0.0.0
rcon.port {rcon_port}
rcon.password "{rcon_password}"
rcon.web 1
server.queryport {query_port}
server.port {game_port}
"#,
        hostname = def.hostname,
        seed = def.seed,
        worldsize = def.world_size,
        maxplayers = def.max_players,
        rcon_port = def.rcon_port,
        rcon_password = def.rcon_password,
        query_port = def.query_port,
        game_port = def.game_port,
    );

    let cfg_path = format!("{}/server.cfg", cfg_dir);
    if let Err(e) = std::fs::write(&cfg_path, server_cfg) {
        update_status(
            &registry,
            &server_id,
            ProvisioningStatus::Error,
            &format!("Failed to write server.cfg: {}", e),
        )
        .await;
        return;
    }

    // chown cfg to game user
    let _ = tokio::process::Command::new("chown")
        .args(["-R", &format!("{}:{}", GAME_USER, GAME_USER), &cfg_dir])
        .output()
        .await;

    // Step 5: Mark as Ready and initialize runtime
    update_status(
        &registry,
        &server_id,
        ProvisioningStatus::Ready,
        "Server provisioning complete!",
    )
    .await;

    // Initialize runtime
    let game_server_config = def.to_game_server_config();
    let rcon_client = Arc::new(RconClient::new(game_server_config.rcon.clone()));
    let game_monitor = Arc::new(GameMonitor::new(config.monitor.history_size));
    let lgsm_lock = Arc::new(LgsmLock::new());

    let collector_handle = crate::monitor::spawn_game_collector(
        game_monitor.clone(),
        rcon_client.clone(),
        config.monitor.clone(),
        server_id.clone(),
    );

    let runtime = ServerRuntime {
        rcon: rcon_client,
        game_monitor,
        lgsm_lock,
        collector_handle: Some(collector_handle),
    };

    registry
        .runtimes
        .write()
        .await
        .insert(server_id.clone(), runtime);

    // Save updated definitions
    {
        let defs = registry.definitions.read().await;
        let dynamic: Vec<_> = defs
            .iter()
            .filter(|d| d.source == ServerSource::Dynamic)
            .cloned()
            .collect();
        if let Err(e) = crate::persistence::save_servers(&dynamic) {
            tracing::error!("Failed to save servers after provisioning: {}", e);
        }
    }

    tracing::info!("Server '{}' provisioning complete!", server_id);
}

async fn update_status(
    registry: &ServerRegistry,
    server_id: &str,
    status: ProvisioningStatus,
    message: &str,
) {
    tracing::info!("Provisioning '{}': {:?} - {}", server_id, status, message);
    let mut defs = registry.definitions.write().await;
    if let Some(def) = defs.iter_mut().find(|d| d.id == server_id) {
        def.provisioning_status = status;
        def.provisioning_log.push(message.to_string());
    }
}
