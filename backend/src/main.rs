mod auth;
mod config;
mod filemanager;
mod lgsm;
mod logs;
mod monitor;
mod plugins;
mod rcon;
mod scheduler;
mod websocket;

use actix_cors::Cors;
use actix_files::Files;
use actix_web::{web, App, HttpServer};
use std::sync::Arc;

use crate::config::AppConfig;
use crate::lgsm::LgsmLock;
use crate::monitor::{GameMonitor, SystemMonitor};
use crate::rcon::RconClient;
use crate::scheduler::Scheduler;

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Load configuration
    let config = AppConfig::load()?;
    tracing::info!(
        "Starting server on {}:{}",
        config.server.host,
        config.server.port
    );

    // Create shared state
    let rcon_client = Arc::new(RconClient::new(config.rcon.clone()));
    let sys_monitor = Arc::new(SystemMonitor::new(config.monitor.history_size));
    let game_monitor = Arc::new(GameMonitor::new(config.monitor.history_size));
    let scheduler = Arc::new(Scheduler::new());
    let lgsm_lock = Arc::new(LgsmLock::new());

    // Try initial RCON connection (non-fatal if server is offline)
    {
        let rcon = rcon_client.clone();
        match rcon.connect().await {
            Ok(()) => tracing::info!("Initial RCON connection established"),
            Err(e) => tracing::warn!("Initial RCON connection failed (will retry on demand): {}", e),
        }
    }

    // Spawn background tasks
    let _sys_collector = monitor::spawn_system_collector(
        sys_monitor.clone(),
        config.monitor.clone(),
    );
    let _game_collector = monitor::spawn_game_collector(
        game_monitor.clone(),
        rcon_client.clone(),
        config.monitor.clone(),
    );
    let _scheduler_handle = scheduler::spawn_scheduler(
        scheduler.clone(),
        rcon_client.clone(),
        config.clone(),
        lgsm_lock.clone(),
    );

    // Capture bind address before moving config
    let bind_host = config.server.host.clone();
    let bind_port = config.server.port;

    // Start HTTP server
    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin("http://localhost:5173")
            .allowed_origin(&format!("http://{}:{}", config.server.host, config.server.port))
            .allowed_origin(&format!("https://{}:{}", config.server.host, config.server.port))
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
            .app_data(web::Data::new(rcon_client.clone()))
            .app_data(web::Data::new(sys_monitor.clone()))
            .app_data(web::Data::new(game_monitor.clone()))
            .app_data(web::Data::new(scheduler.clone()))
            .app_data(web::Data::new(lgsm_lock.clone()))
            // Auth routes
            .route("/api/auth/login", web::post().to(auth::login))
            .route("/api/auth/me", web::get().to(auth::me))
            // Server control routes (LGSM)
            .route("/api/server/start", web::post().to(lgsm::server_start))
            .route("/api/server/stop", web::post().to(lgsm::server_stop))
            .route("/api/server/restart", web::post().to(lgsm::server_restart))
            .route("/api/server/update", web::post().to(lgsm::server_update))
            .route("/api/server/backup", web::post().to(lgsm::server_backup))
            .route("/api/server/save", web::post().to(lgsm::server_save))
            .route("/api/server/wipe", web::post().to(lgsm::server_wipe))
            .route("/api/server/status", web::get().to(lgsm::server_status))
            // Monitor routes
            .route("/api/monitor/system", web::get().to(monitor::get_system_metrics))
            .route("/api/monitor/game", web::get().to(monitor::get_game_metrics))
            // File manager routes
            .route("/api/files/list", web::get().to(filemanager::list_files))
            .route("/api/files/read", web::get().to(filemanager::read_file))
            .route("/api/files/write", web::put().to(filemanager::write_file))
            .route("/api/files/upload", web::post().to(filemanager::upload_file))
            .route("/api/files/download", web::get().to(filemanager::download_file))
            .route("/api/files/mkdir", web::post().to(filemanager::mkdir))
            .route("/api/files/delete", web::delete().to(filemanager::delete_file))
            // Plugin routes
            .route("/api/plugins", web::get().to(plugins::list_plugins))
            .route("/api/plugins/upload", web::post().to(plugins::upload_plugin))
            .route("/api/plugins/umod/search", web::get().to(plugins::umod_search))
            .route("/api/plugins/umod/install", web::post().to(plugins::umod_install))
            .route("/api/plugins/{name}", web::delete().to(plugins::delete_plugin))
            .route("/api/plugins/{name}/config", web::get().to(plugins::get_plugin_config))
            .route("/api/plugins/{name}/config", web::put().to(plugins::save_plugin_config))
            .route("/api/plugins/{name}/reload", web::post().to(plugins::reload_plugin))
            // Scheduler routes
            .route("/api/schedule", web::get().to(scheduler::list_jobs))
            .route("/api/schedule", web::post().to(scheduler::create_job))
            .route("/api/schedule/{id}", web::put().to(scheduler::update_job))
            .route("/api/schedule/{id}", web::delete().to(scheduler::delete_job))
            .route("/api/schedule/{id}/toggle", web::post().to(scheduler::toggle_job))
            // Log routes
            .route("/api/logs/tail", web::get().to(logs::tail_log))
            // WebSocket routes
            .route("/ws/console", web::get().to(websocket::ws_console))
            .route("/ws/monitor", web::get().to(websocket::ws_monitor))
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
