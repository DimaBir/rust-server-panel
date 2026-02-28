mod auth;
mod config;
mod filemanager;
mod lgsm;
mod logs;
mod map;
mod monitor;
mod persistence;
mod players;
mod plugins;
mod provisioner;
mod rcon;
mod registry;
mod scheduler;
mod servers;
mod websocket;

use actix_cors::Cors;
use actix_files::Files;
use actix_web::{web, App, HttpServer};
use std::collections::HashMap;
use std::sync::Arc;

use crate::config::AppConfig;
use crate::map::PositionStore;
use crate::monitor::SystemMonitor;
use crate::registry::{
    ServerDefinition, ServerRegistry, ServerRuntime, ServerSource, ProvisioningStatus,
};
use crate::scheduler::Scheduler;

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let config = AppConfig::load()?;
    tracing::info!(
        "Starting server on {}:{} with {} game server(s)",
        config.panel.host,
        config.panel.port,
        config.servers.len()
    );

    // Build server definitions from static config + dynamic persistence
    let mut definitions: Vec<ServerDefinition> = Vec::new();
    let mut static_configs: HashMap<String, config::GameServerConfig> = HashMap::new();

    for server in &config.servers {
        let def = ServerDefinition::from_static_config(server);
        definitions.push(def);
        static_configs.insert(server.id.clone(), server.clone());
    }

    // Load dynamically created servers
    let dynamic_servers = persistence::load_servers();
    for ds in dynamic_servers {
        if !definitions.iter().any(|d| d.id == ds.id) {
            definitions.push(ds);
        }
    }

    tracing::info!(
        "Loaded {} total server definitions ({} static, {} dynamic)",
        definitions.len(),
        static_configs.len(),
        definitions.iter().filter(|d| d.source == ServerSource::Dynamic).count()
    );

    // Create the shared registry
    let registry = Arc::new(ServerRegistry::new(definitions.clone(), static_configs));

    // Global system monitor
    let sys_monitor = Arc::new(SystemMonitor::new(config.monitor.history_size));

    // Initialize runtimes for all Ready servers
    for def in &definitions {
        if def.provisioning_status != ProvisioningStatus::Ready {
            tracing::info!("Skipping runtime init for '{}' (status: {:?})", def.id, def.provisioning_status);
            continue;
        }

        let server_config = registry.get_config(&def.id).await.unwrap();
        let rcon_client = Arc::new(rcon::RconClient::new(server_config.rcon.clone()));
        let game_monitor = Arc::new(monitor::GameMonitor::new(config.monitor.history_size));
        let lgsm_lock = Arc::new(lgsm::LgsmLock::new());

        // Try initial RCON connection (non-fatal)
        {
            let rcon = rcon_client.clone();
            match rcon.connect().await {
                Ok(()) => tracing::info!("RCON connected for server '{}'", def.id),
                Err(e) => tracing::warn!(
                    "RCON connection failed for '{}' (will retry on demand): {}",
                    def.id,
                    e
                ),
            }
        }

        // Spawn per-server game collector
        let collector_handle = monitor::spawn_game_collector(
            game_monitor.clone(),
            rcon_client.clone(),
            config.monitor.clone(),
            def.id.clone(),
        );

        let runtime = ServerRuntime {
            rcon: rcon_client,
            game_monitor,
            lgsm_lock,
            collector_handle: Some(collector_handle),
        };

        registry.runtimes.write().await.insert(def.id.clone(), runtime);
    }

    // Spawn global system collector
    let _sys_collector =
        monitor::spawn_system_collector(sys_monitor.clone(), config.monitor.clone());

    // Global scheduler
    let scheduler = Arc::new(Scheduler::new());
    let _scheduler_handle = scheduler::spawn_scheduler(
        scheduler.clone(),
        registry.clone(),
    );

    // Position store for live map
    let position_store = Arc::new(PositionStore::new());

    let bind_host = config.panel.host.clone();
    let bind_port = config.panel.port;

    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin("http://localhost:5173")
            .allowed_origin(&format!(
                "http://{}:{}",
                config.panel.host, config.panel.port
            ))
            .allowed_origin(&format!(
                "https://{}:{}",
                config.panel.host, config.panel.port
            ))
            .allow_any_origin()
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
            .allowed_headers(vec![
                actix_web::http::header::AUTHORIZATION,
                actix_web::http::header::CONTENT_TYPE,
                actix_web::http::header::ACCEPT,
            ])
            .max_age(3600);

        App::new()
            .wrap(cors)
            .wrap(auth::JwtAuth)
            // Shared state
            .app_data(web::Data::new(config.clone()))
            .app_data(web::Data::new(sys_monitor.clone()))
            .app_data(web::Data::new(scheduler.clone()))
            .app_data(web::Data::new(registry.clone()))
            .app_data(web::Data::new(position_store.clone()))
            // Auth routes (global)
            .route("/api/auth/login", web::post().to(auth::login))
            .route("/api/auth/me", web::get().to(auth::me))
            // Server list + CRUD (global)
            .route("/api/servers", web::get().to(servers::list_servers))
            .route("/api/servers", web::post().to(servers::create_server))
            // System monitor (global)
            .route(
                "/api/monitor/system",
                web::get().to(monitor::get_system_metrics),
            )
            // uMod search (global)
            .route(
                "/api/plugins/umod/search",
                web::get().to(plugins::umod_search),
            )
            // Scheduler routes (global scope, jobs have server_id field)
            .route("/api/schedule", web::get().to(scheduler::list_jobs))
            .route("/api/schedule", web::post().to(scheduler::create_job))
            .route(
                "/api/schedule/{id}",
                web::put().to(scheduler::update_job),
            )
            .route(
                "/api/schedule/{id}",
                web::delete().to(scheduler::delete_job),
            )
            .route(
                "/api/schedule/{id}/toggle",
                web::post().to(scheduler::toggle_job),
            )
            // Per-server routes
            .service(
                web::scope("/api/servers/{server_id}")
                    .route("/status", web::get().to(lgsm::server_status))
                    .route("/start", web::post().to(lgsm::server_start))
                    .route("/stop", web::post().to(lgsm::server_stop))
                    .route("/restart", web::post().to(lgsm::server_restart))
                    .route("/update", web::post().to(lgsm::server_update))
                    .route("/backup", web::post().to(lgsm::server_backup))
                    .route("/save", web::post().to(lgsm::server_save))
                    .route("/wipe", web::post().to(lgsm::server_wipe))
                    // Players
                    .route("/players", web::get().to(players::list_players))
                    .route("/players/kick", web::post().to(players::kick_player))
                    .route("/players/ban", web::post().to(players::ban_player))
                    .route("/players/unban", web::post().to(players::unban_player))
                    // Game monitor
                    .route(
                        "/monitor/game",
                        web::get().to(monitor::get_game_metrics),
                    )
                    // Files
                    .route("/files/list", web::get().to(filemanager::list_files))
                    .route("/files/read", web::get().to(filemanager::read_file))
                    .route("/files/write", web::put().to(filemanager::write_file))
                    .route("/files/upload", web::post().to(filemanager::upload_file))
                    .route(
                        "/files/download",
                        web::get().to(filemanager::download_file),
                    )
                    .route("/files/mkdir", web::post().to(filemanager::mkdir))
                    .route(
                        "/files/delete",
                        web::delete().to(filemanager::delete_file),
                    )
                    // Plugins
                    .route("/plugins", web::get().to(plugins::list_plugins))
                    .route(
                        "/plugins/upload",
                        web::post().to(plugins::upload_plugin),
                    )
                    .route(
                        "/plugins/umod/install",
                        web::post().to(plugins::umod_install),
                    )
                    .route(
                        "/plugins/{name}",
                        web::delete().to(plugins::delete_plugin),
                    )
                    .route(
                        "/plugins/{name}/config",
                        web::get().to(plugins::get_plugin_config),
                    )
                    .route(
                        "/plugins/{name}/config",
                        web::put().to(plugins::save_plugin_config),
                    )
                    .route(
                        "/plugins/{name}/reload",
                        web::post().to(plugins::reload_plugin),
                    )
                    // Logs
                    .route("/logs/tail", web::get().to(logs::tail_log))
                    // Map & Positions
                    .route("/map", web::get().to(map::get_map_info))
                    .route("/positions", web::get().to(map::get_positions))
                    .route("/positions", web::post().to(map::update_positions))
                    // Provisioning
                    .route(
                        "/provision-status",
                        web::get().to(servers::provision_status),
                    )
                    // Delete server
                    .route("", web::delete().to(servers::delete_server)),
            )
            // WebSocket routes (per-server)
            .route(
                "/ws/{server_id}/console",
                web::get().to(websocket::ws_console),
            )
            .route(
                "/ws/{server_id}/monitor",
                web::get().to(websocket::ws_monitor),
            )
            // Static files (Vue frontend) — must be last
            .service(
                Files::new("/", "./static")
                    .index_file("index.html")
                    .default_handler(actix_web::dev::fn_service(
                        |req: actix_web::dev::ServiceRequest| async {
                            let (req, _) = req.into_parts();
                            let file =
                                actix_files::NamedFile::open_async("./static/index.html").await?;
                            let res = file.into_response(&req);
                            Ok(actix_web::dev::ServiceResponse::new(req, res))
                        },
                    )),
            )
    })
    .bind(format!("{}:{}", bind_host, bind_port))?
    .shutdown_timeout(10)
    .run()
    .await?;

    tracing::info!("Server shutdown complete");
    Ok(())
}
