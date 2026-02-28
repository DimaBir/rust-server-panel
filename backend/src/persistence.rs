use std::path::Path;

use crate::registry::ServerDefinition;

const SERVERS_FILE: &str = "servers.json";

/// Load dynamically created servers from servers.json.
pub fn load_servers() -> Vec<ServerDefinition> {
    let path = Path::new(SERVERS_FILE);
    if !path.exists() {
        return Vec::new();
    }
    match std::fs::read_to_string(path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|e| {
            tracing::warn!("Failed to parse {}: {}", SERVERS_FILE, e);
            Vec::new()
        }),
        Err(e) => {
            tracing::warn!("Failed to read {}: {}", SERVERS_FILE, e);
            Vec::new()
        }
    }
}

/// Save dynamically created servers to servers.json.
pub fn save_servers(defs: &[ServerDefinition]) -> anyhow::Result<()> {
    let content = serde_json::to_string_pretty(defs)?;
    std::fs::write(SERVERS_FILE, content)?;
    Ok(())
}
