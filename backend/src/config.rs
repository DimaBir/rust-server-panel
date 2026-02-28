use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_server_config")]
    pub server: ServerConfig,
    #[serde(default = "default_rcon_config")]
    pub rcon: RconConfig,
    #[serde(default = "default_auth_config")]
    pub auth: AuthConfig,
    #[serde(default = "default_paths_config")]
    pub paths: PathsConfig,
    #[serde(default = "default_monitor_config")]
    pub monitor: MonitorConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RconConfig {
    #[serde(default = "default_rcon_host")]
    pub host: String,
    #[serde(default = "default_rcon_port")]
    pub port: u16,
    #[serde(default = "default_rcon_password")]
    pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    #[serde(default = "default_admin_username")]
    pub admin_username: String,
    #[serde(default = "default_password_hash")]
    pub password_hash: String,
    #[serde(default = "default_jwt_secret")]
    pub jwt_secret: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PathsConfig {
    #[serde(default = "default_lgsm_script")]
    pub lgsm_script: String,
    #[serde(default = "default_server_files")]
    pub server_files: String,
    #[serde(default = "default_oxide_plugins")]
    pub oxide_plugins: String,
    #[serde(default = "default_oxide_config")]
    pub oxide_config: String,
    #[serde(default = "default_server_cfg")]
    pub server_cfg: String,
    #[serde(default = "default_server_log")]
    pub server_log: String,
    #[serde(default = "default_base_dir")]
    pub base_dir: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MonitorConfig {
    #[serde(default = "default_poll_interval")]
    pub poll_interval_secs: u64,
    #[serde(default = "default_history_size")]
    pub history_size: usize,
}

// Default value functions
fn default_server_config() -> ServerConfig {
    ServerConfig {
        host: default_host(),
        port: default_port(),
    }
}

fn default_rcon_config() -> RconConfig {
    RconConfig {
        host: default_rcon_host(),
        port: default_rcon_port(),
        password: default_rcon_password(),
    }
}

fn default_auth_config() -> AuthConfig {
    AuthConfig {
        admin_username: default_admin_username(),
        password_hash: default_password_hash(),
        jwt_secret: default_jwt_secret(),
    }
}

fn default_paths_config() -> PathsConfig {
    PathsConfig {
        lgsm_script: default_lgsm_script(),
        server_files: default_server_files(),
        oxide_plugins: default_oxide_plugins(),
        oxide_config: default_oxide_config(),
        server_cfg: default_server_cfg(),
        server_log: default_server_log(),
        base_dir: default_base_dir(),
    }
}

fn default_monitor_config() -> MonitorConfig {
    MonitorConfig {
        poll_interval_secs: default_poll_interval(),
        history_size: default_history_size(),
    }
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}
fn default_port() -> u16 {
    8443
}
fn default_rcon_host() -> String {
    "127.0.0.1".to_string()
}
fn default_rcon_port() -> u16 {
    28016
}
fn default_rcon_password() -> String {
    "changeme".to_string()
}
fn default_admin_username() -> String {
    "admin".to_string()
}
fn default_password_hash() -> String {
    // Default hash for "admin" - CHANGE IN PRODUCTION
    "$2b$12$LJ3m4ys3Lg2VhsMwKMriOe5VJxMWm9F0RPDOlAPsaGBVkle6sUNS6".to_string()
}
fn default_jwt_secret() -> String {
    "change-this-to-a-random-secret-string".to_string()
}
fn default_lgsm_script() -> String {
    "/home/rustserver/rustserver".to_string()
}
fn default_server_files() -> String {
    "/home/rustserver/serverfiles".to_string()
}
fn default_oxide_plugins() -> String {
    "/home/rustserver/serverfiles/oxide/plugins".to_string()
}
fn default_oxide_config() -> String {
    "/home/rustserver/serverfiles/oxide/config".to_string()
}
fn default_server_cfg() -> String {
    "/home/rustserver/serverfiles/server/rustserver/cfg/server.cfg".to_string()
}
fn default_server_log() -> String {
    "/home/rustserver/log/console/rustserver-console.log".to_string()
}
fn default_base_dir() -> String {
    "/home/rustserver".to_string()
}
fn default_poll_interval() -> u64 {
    5
}
fn default_history_size() -> usize {
    720
}

impl AppConfig {
    pub fn load() -> anyhow::Result<Self> {
        let config_path = Path::new("config.yaml");
        if config_path.exists() {
            let contents = std::fs::read_to_string(config_path)?;
            let config: AppConfig = serde_yaml::from_str(&contents)?;
            Ok(config)
        } else {
            tracing::warn!("config.yaml not found, using defaults");
            Ok(AppConfig {
                server: default_server_config(),
                rcon: default_rcon_config(),
                auth: default_auth_config(),
                paths: default_paths_config(),
                monitor: default_monitor_config(),
            })
        }
    }
}
