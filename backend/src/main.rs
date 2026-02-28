mod auth;
mod config;
mod filemanager;
mod lgsm;
mod logs;
mod monitor;
mod players;
mod plugins;
mod rcon;
mod scheduler;
mod servers;
mod websocket;

use actix_cors::Cors;
use actix_files::Files;
use actix_web::{web, App, HttpServer};
use std::collections::HashMap;
use std::sync::Arc;

use crate::config::AppConfig;
use crate::lgsm::LgsmLock;
use crate::monitor::{GameMonitor, SystemMonitor};
use crate::rcon::RconClient;
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

    // Global system monitor
    let sys_monitor = Arc::new(SystemMonitor::new(config.monitor.history_size));

    // Per-server state maps
    let mut rcon_clients: HashMap<String, Arc<RconClient>> = HashMap::new();
    let mut game_monitors: HashMap<String, Arc<GameMonitor>> = HashMap::new();
    let mut lgsm_locks: HashMap<String, Arc<LgsmLock>> = HashMap::new();

    for server in &config.servers {
        let rcon_client = Arc::new(RconClient::new(server.rcon.clone()));
        let game_monitor = Arc::new(GameMonitor::new(config.monitor.history_size));
        let lgsm_lock = Arc::new(LgsmLock::new());

        // Try initial RCON connection (non-fatal)
        {
            let rcon = rcon_client.clone();
            match rcon.connect().await {
                Ok(()) => tracing::info!("RCON connected for server '{}'", server.id),
                Err(e) => tracing::warn!("RCON connection failed for '{}' (will retry on demand): {}", server.id, e),
            }
        }

        // Spawn per-server game collector
        let _game_collector = monitor::spawn_game_collector(
            game_monitor.clone(),
            rcon_client.clone(),
            config.monitor.clone(),
            server.id.clone(),
        );

        rcon_clients.insert(server.id.clone(), rcon_client);
        game_monitors.insert(server.id.clone(), game_monitor);
        lgsm_locks.insert(server.id.clone(), lgsm_lock);
    }

    // Spawn global system collector
    let _sys_collector = monitor::spawn_system_collector(
        sys_monitor.clone(),
        config.monitor.clone(),
    );

    // Global scheduler
    let scheduler = Arc::new(Scheduler::new());
    let _scheduler_handle = scheduler::spawn_scheduler(
        scheduler.clone(),
        rcon_clients.clone(),
        config.servers.clone(),
        lgsm_locks.clone(),
    );

    let bind_host = config.panel.host.clone();
    let bind_port = config.panel.port;

    // Clone maps for move into closure
    let server_configs = config.servers.clone();

    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin("http://localhost:5173")
            .allowed_origin(&format!("http://{}:{}", config.panel.host, config.panel.port))
            .allowed_origin(&format!("https://{}:{}", config.panel.host, config.panel.port))
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
            // Multi-server state maps
            .app_data(web::Data::new(rcon_clients.clone()))
            .app_data(web::Data::new(game_monitors.clone()))
            .app_data(web::Data::new(lgsm_locks.clone()))
            .app_data(web::Data::new(server_configs.clone()))
            // Auth routes (global)
            .route("/api/auth/login", web::post().to(auth::login))
            .route("/api/auth/me", web::get().to(auth::me))
            // Server list (global)
            .route("/api/servers", web::get().to(servers::list_servers))
            // System monitor (global)
            .route("/api/monitor/system", web::get().to(monitor::get_system_metrics))
            // uMod search (global)
            .route("/api/plugins/umod/search", web::get().to(plugins::umod_search))
            // Scheduler routes (global scope, jobs have server_id field)
            .route("/api/schedule", web::get().to(scheduler::list_jobs))
            .route("/api/schedule", web::post().to(scheduler::create_job))
            .route("/api/schedule/{id}", web::put().to(scheduler::update_job))
            .route("/api/schedule/{id}", web::delete().to(scheduler::delete_job))
            .route("/api/schedule/{id}/toggle", web::post().to(scheduler::toggle_job))
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
                    .route("/monitor/game", web::get().to(monitor::get_game_metrics))
                    // Files
                    .route("/files/list", web::get().to(filemanager::list_files))
                    .route("/files/read", web::get().to(filemanager::read_file))
                    .route("/files/write", web::put().to(filemanager::write_file))
                    .route("/files/upload", web::post().to(filemanager::upload_file))
                    .route("/files/download", web::get().to(filemanager::download_file))
                    .route("/files/mkdir", web::post().to(filemanager::mkdir))
                    .route("/files/delete", web::delete().to(filemanager::delete_file))
                    // Plugins
                    .route("/plugins", web::get().to(plugins::list_plugins))
                    .route("/plugins/upload", web::post().to(plugins::upload_plugin))
                    .route("/plugins/umod/install", web::post().to(plugins::umod_install))
                    .route("/plugins/{name}", web::delete().to(plugins::delete_plugin))
                    .route("/plugins/{name}/config", web::get().to(plugins::get_plugin_config))
                    .route("/plugins/{name}/config", web::put().to(plugins::save_plugin_config))
                    .route("/plugins/{name}/reload", web::post().to(plugins::reload_plugin))
                    // Logs
                    .route("/logs/tail", web::get().to(logs::tail_log))
            )
            // WebSocket routes (per-server)
            .route("/ws/{server_id}/console", web::get().to(websocket::ws_console))
            .route("/ws/{server_id}/monitor", web::get().to(websocket::ws_monitor))
            // Static files (Vue frontend) — must be last
            .service(
                Files::new("/", "./static")
                    .index_file("index.html")
                    .default_handler(
                        actix_web::dev::fn_service(|req: actix_web::dev::ServiceRequest| async {
                            let (req, _) = req.into_parts();
                            let file = actix_files::NamedFile::open_async("./static/index.html").await?;
                            let res = file.into_response(&req);
                            Ok(actix_web::dev::ServiceResponse::new(req, res))
                        }),
                    ),
            )
    })
    .bind(format!("{}:{}", bind_host, bind_port))?
    .shutdown_timeout(10)
    .run()
    .await?;

    tracing::info!("Server shutdown complete");
    Ok(())
}
