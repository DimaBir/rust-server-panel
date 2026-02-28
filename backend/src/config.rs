use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_panel_config", alias = "server")]
    pub panel: PanelConfig,
    #[serde(default = "default_auth_config")]
    pub auth: AuthConfig,
    #[serde(default = "default_monitor_config")]
    pub monitor: MonitorConfig,
    #[serde(default)]
    pub provisioning: ProvisioningConfig,
    /// Multi-server list. If absent, falls back to legacy top-level rcon/paths.
    #[serde(default)]
    pub servers: Vec<GameServerConfig>,

    // Legacy single-server fields (used for backward compat)
    #[serde(default)]
    rcon: Option<RconConfig>,
    #[serde(default)]
    paths: Option<PathsConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PanelConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GameServerConfig {
    #[serde(default = "default_server_id")]
    pub id: String,
    #[serde(default = "default_server_name")]
    pub name: String,
    #[serde(default = "default_rcon_config")]
    pub rcon: RconConfig,
    #[serde(default = "default_paths_config")]
    pub paths: PathsConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
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

#[derive(Debug, Clone, Deserialize, Serialize)]
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

#[derive(Debug, Clone, Deserialize)]
pub struct ProvisioningConfig {
    #[serde(default = "default_provisioning_base_path")]
    pub base_path: String,
    #[serde(default = "default_port_range_start")]
    pub port_range_start: u16,
    #[serde(default = "default_port_offset")]
    pub port_offset: u16,
    #[serde(default = "default_max_servers")]
    pub max_servers: usize,
}

impl Default for ProvisioningConfig {
    fn default() -> Self {
        Self {
            base_path: default_provisioning_base_path(),
            port_range_start: default_port_range_start(),
            port_offset: default_port_offset(),
            max_servers: default_max_servers(),
        }
    }
}

// Default value functions
fn default_panel_config() -> PanelConfig {
    PanelConfig {
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
fn default_server_id() -> String {
    "main".to_string()
}
fn default_server_name() -> String {
    "Main Server".to_string()
}

fn default_provisioning_base_path() -> String {
    "/home".to_string()
}
fn default_port_range_start() -> u16 {
    28015
}
fn default_port_offset() -> u16 {
    10
}
fn default_max_servers() -> usize {
    10
}

impl AppConfig {
    pub fn load() -> anyhow::Result<Self> {
        let config_path = Path::new("config.yaml");
        let mut config = if config_path.exists() {
            let contents = std::fs::read_to_string(config_path)?;
            let config: AppConfig = serde_yaml::from_str(&contents)?;
            config
        } else {
            tracing::warn!("config.yaml not found, using defaults");
            AppConfig {
                panel: default_panel_config(),
                auth: default_auth_config(),
                monitor: default_monitor_config(),
                servers: Vec::new(),
                rcon: None,
                paths: None,
                provisioning: ProvisioningConfig::default(),
            }
        };

        // Backward compatibility: if no servers defined but legacy rcon/paths exist,
        // wrap them into a single server entry.
        if config.servers.is_empty() {
            let rcon = config.rcon.take().unwrap_or_else(default_rcon_config);
            let paths = config.paths.take().unwrap_or_else(default_paths_config);
            config.servers.push(GameServerConfig {
                id: default_server_id(),
                name: default_server_name(),
                rcon,
                paths,
            });
            tracing::info!("Migrated legacy config to single-server format");
        }

        Ok(config)
    }
}
