use actix_multipart::Multipart;
use actix_web::{web, HttpResponse};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::config::GameServerConfig;
use crate::rcon::RconClient;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginInfo {
    pub name: String,
    pub filename: String,
    pub size: u64,
    pub modified: Option<String>,
    pub has_config: bool,
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

#[derive(Debug, Deserialize)]
pub struct UmodSearchQuery {
    pub q: String,
}

#[derive(Debug, Deserialize)]
pub struct UmodInstallBody {
    pub url: String,
    pub filename: String,
}

fn plugin_name_from_file(filename: &str) -> String {
    Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(filename)
        .to_string()
}

fn get_server_paths(server_id: &str, server_configs: &[GameServerConfig]) -> Result<(String, String), HttpResponse> {
    let config = server_configs
        .iter()
        .find(|s| s.id == server_id)
        .ok_or_else(|| HttpResponse::NotFound().json(ErrorBody { error: "Server not found".to_string() }))?;
    Ok((config.paths.oxide_plugins.clone(), config.paths.oxide_config.clone()))
}

/// GET /api/servers/{server_id}/plugins
pub async fn list_plugins(
    server_id: web::Path<String>,
    server_configs: web::Data<Vec<GameServerConfig>>,
) -> HttpResponse {
    let (plugins_dir_str, config_dir_str) = match get_server_paths(&server_id, &server_configs) {
        Ok(p) => p,
        Err(e) => return e,
    };
    let plugins_dir = Path::new(&plugins_dir_str);
    let config_dir = Path::new(&config_dir_str);

    if !plugins_dir.exists() {
        return HttpResponse::Ok().json(Vec::<PluginInfo>::new());
    }

    let mut plugins = Vec::new();
    match std::fs::read_dir(plugins_dir) {
        Ok(entries) => {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("cs") {
                    let filename = entry.file_name().to_string_lossy().to_string();
                    let name = plugin_name_from_file(&filename);
                    let metadata = entry.metadata().ok();
                    let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
                    let modified = metadata
                        .as_ref()
                        .and_then(|m| m.modified().ok())
                        .map(|t| {
                            let dt: chrono::DateTime<chrono::Utc> = t.into();
                            dt.to_rfc3339()
                        });
                    let config_file = config_dir.join(format!("{}.json", name));
                    let has_config = config_file.exists();

                    plugins.push(PluginInfo { name, filename, size, modified, has_config });
                }
            }
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(ErrorBody {
                error: format!("Failed to read plugins directory: {}", e),
            });
        }
    }

    plugins.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    HttpResponse::Ok().json(plugins)
}

/// GET /api/servers/{server_id}/plugins/{name}/config
pub async fn get_plugin_config(
    path: web::Path<(String, String)>,
    server_configs: web::Data<Vec<GameServerConfig>>,
) -> HttpResponse {
    let (server_id, name) = path.into_inner();
    let (_, config_dir_str) = match get_server_paths(&server_id, &server_configs) {
        Ok(p) => p,
        Err(e) => return e,
    };
    let config_path = PathBuf::from(&config_dir_str).join(format!("{}.json", name));

    if !config_path.exists() {
        return HttpResponse::NotFound().json(ErrorBody {
            error: format!("Config file not found for plugin '{}'", name),
        });
    }

    match std::fs::read_to_string(&config_path) {
        Ok(content) => {
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(json) => HttpResponse::Ok().json(serde_json::json!({
                    "plugin": name,
                    "config": json,
                })),
                Err(_) => HttpResponse::Ok().json(serde_json::json!({
                    "plugin": name,
                    "raw_config": content,
                })),
            }
        }
        Err(e) => HttpResponse::InternalServerError().json(ErrorBody {
            error: format!("Failed to read config: {}", e),
        }),
    }
}

/// PUT /api/servers/{server_id}/plugins/{name}/config
pub async fn save_plugin_config(
    path: web::Path<(String, String)>,
    body: web::Json<serde_json::Value>,
    server_configs: web::Data<Vec<GameServerConfig>>,
    rcon_clients: web::Data<HashMap<String, Arc<RconClient>>>,
) -> HttpResponse {
    let (server_id, name) = path.into_inner();
    let (_, config_dir_str) = match get_server_paths(&server_id, &server_configs) {
        Ok(p) => p,
        Err(e) => return e,
    };
    let config_path = PathBuf::from(&config_dir_str).join(format!("{}.json", name));

    if let Some(parent) = config_path.parent() {
        if !parent.exists() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return HttpResponse::InternalServerError().json(ErrorBody {
                    error: format!("Failed to create config directory: {}", e),
                });
            }
        }
    }

    let json_str = match serde_json::to_string_pretty(&body.into_inner()) {
        Ok(s) => s,
        Err(e) => return HttpResponse::BadRequest().json(ErrorBody { error: format!("Invalid JSON: {}", e) }),
    };

    if config_path.exists() {
        let backup = format!("{}.bak", config_path.display());
        let _ = std::fs::copy(&config_path, &backup);
    }

    if let Err(e) = std::fs::write(&config_path, &json_str) {
        return HttpResponse::InternalServerError().json(ErrorBody {
            error: format!("Failed to write config: {}", e),
        });
    }

    let reload_result = if let Some(rcon) = rcon_clients.get(&server_id) {
        match rcon.oxide_reload(&name).await {
            Ok(msg) => msg,
            Err(e) => format!("Reload failed (server may be offline): {}", e),
        }
    } else {
        "RCON not available".to_string()
    };

    HttpResponse::Ok().json(SuccessBody {
        success: true,
        message: format!("Config saved for '{}'. Reload: {}", name, reload_result),
    })
}

/// POST /api/servers/{server_id}/plugins/upload
pub async fn upload_plugin(
    server_id: web::Path<String>,
    mut payload: Multipart,
    server_configs: web::Data<Vec<GameServerConfig>>,
    rcon_clients: web::Data<HashMap<String, Arc<RconClient>>>,
) -> HttpResponse {
    let (plugins_dir_str, _) = match get_server_paths(&server_id, &server_configs) {
        Ok(p) => p,
        Err(e) => return e,
    };
    let plugins_dir = PathBuf::from(&plugins_dir_str);

    if !plugins_dir.exists() {
        if let Err(e) = std::fs::create_dir_all(&plugins_dir) {
            return HttpResponse::InternalServerError().json(ErrorBody {
                error: format!("Failed to create plugins directory: {}", e),
            });
        }
    }

    while let Some(item) = payload.next().await {
        let mut field = match item {
            Ok(f) => f,
            Err(e) => return HttpResponse::BadRequest().json(ErrorBody { error: format!("Multipart error: {}", e) }),
        };

        let filename = field
            .content_disposition()
            .and_then(|cd| cd.get_filename().map(|f| f.to_string()))
            .unwrap_or_else(|| "plugin.cs".to_string());

        if !filename.ends_with(".cs") {
            return HttpResponse::BadRequest().json(ErrorBody { error: "Only .cs plugin files are allowed".to_string() });
        }

        let target_path = plugins_dir.join(&filename);

        let mut file_data = Vec::new();
        while let Some(chunk) = field.next().await {
            if let Ok(bytes) = chunk { file_data.extend_from_slice(&bytes); }
        }

        if let Err(e) = std::fs::write(&target_path, &file_data) {
            return HttpResponse::InternalServerError().json(ErrorBody {
                error: format!("Failed to write plugin: {}", e),
            });
        }

        let plugin_name = plugin_name_from_file(&filename);

        let load_result = if let Some(rcon) = rcon_clients.get(server_id.as_str()) {
            match rcon.oxide_load(&plugin_name).await {
                Ok(msg) => msg,
                Err(e) => format!("Load failed (server may be offline): {}", e),
            }
        } else {
            "RCON not available".to_string()
        };

        return HttpResponse::Ok().json(SuccessBody {
            success: true,
            message: format!("Plugin '{}' uploaded. Load: {}", plugin_name, load_result),
        });
    }

    HttpResponse::BadRequest().json(ErrorBody { error: "No file provided".to_string() })
}

/// DELETE /api/servers/{server_id}/plugins/{name}
pub async fn delete_plugin(
    path: web::Path<(String, String)>,
    server_configs: web::Data<Vec<GameServerConfig>>,
    rcon_clients: web::Data<HashMap<String, Arc<RconClient>>>,
) -> HttpResponse {
    let (server_id, name) = path.into_inner();
    let (plugins_dir_str, _) = match get_server_paths(&server_id, &server_configs) {
        Ok(p) => p,
        Err(e) => return e,
    };
    let plugin_file = PathBuf::from(&plugins_dir_str).join(format!("{}.cs", name));

    if !plugin_file.exists() {
        return HttpResponse::NotFound().json(ErrorBody { error: format!("Plugin '{}' not found", name) });
    }

    let unload_result = if let Some(rcon) = rcon_clients.get(&server_id) {
        match rcon.oxide_unload(&name).await {
            Ok(msg) => msg,
            Err(e) => format!("Unload failed (server may be offline): {}", e),
        }
    } else {
        "RCON not available".to_string()
    };

    if let Err(e) = std::fs::remove_file(&plugin_file) {
        return HttpResponse::InternalServerError().json(ErrorBody {
            error: format!("Failed to delete plugin file: {}", e),
        });
    }

    HttpResponse::Ok().json(SuccessBody {
        success: true,
        message: format!("Plugin '{}' deleted. Unload: {}", name, unload_result),
    })
}

/// POST /api/servers/{server_id}/plugins/{name}/reload
pub async fn reload_plugin(
    path: web::Path<(String, String)>,
    rcon_clients: web::Data<HashMap<String, Arc<RconClient>>>,
) -> HttpResponse {
    let (server_id, name) = path.into_inner();
    let rcon = match rcon_clients.get(&server_id) {
        Some(r) => r,
        None => return HttpResponse::NotFound().json(ErrorBody { error: "Server not found".to_string() }),
    };

    match rcon.oxide_reload(&name).await {
        Ok(msg) => HttpResponse::Ok().json(SuccessBody {
            success: true,
            message: format!("Plugin '{}' reloaded: {}", name, msg),
        }),
        Err(e) => HttpResponse::InternalServerError().json(ErrorBody {
            error: format!("Failed to reload plugin '{}': {}", name, e),
        }),
    }
}

/// GET /api/plugins/umod/search - global, not per-server
pub async fn umod_search(
    query: web::Query<UmodSearchQuery>,
) -> HttpResponse {
    let url = format!(
        "https://umod.org/plugins/search.json?query={}&page=1&sort=title&sortdir=asc&categories%5B%5D=rust",
        urlencoded(&query.q)
    );

    let client = reqwest::Client::new();
    match client.get(&url).send().await {
        Ok(response) => {
            match response.json::<serde_json::Value>().await {
                Ok(json) => HttpResponse::Ok().json(json),
                Err(e) => HttpResponse::InternalServerError().json(ErrorBody {
                    error: format!("Failed to parse uMod response: {}", e),
                }),
            }
        }
        Err(e) => HttpResponse::InternalServerError().json(ErrorBody {
            error: format!("Failed to search uMod: {}", e),
        }),
    }
}

/// POST /api/servers/{server_id}/plugins/umod/install
pub async fn umod_install(
    server_id: web::Path<String>,
    body: web::Json<UmodInstallBody>,
    server_configs: web::Data<Vec<GameServerConfig>>,
    rcon_clients: web::Data<HashMap<String, Arc<RconClient>>>,
) -> HttpResponse {
    let (plugins_dir_str, _) = match get_server_paths(&server_id, &server_configs) {
        Ok(p) => p,
        Err(e) => return e,
    };
    let plugins_dir = PathBuf::from(&plugins_dir_str);

    if !plugins_dir.exists() {
        if let Err(e) = std::fs::create_dir_all(&plugins_dir) {
            return HttpResponse::InternalServerError().json(ErrorBody {
                error: format!("Failed to create plugins directory: {}", e),
            });
        }
    }

    if !body.filename.ends_with(".cs") {
        return HttpResponse::BadRequest().json(ErrorBody { error: "Filename must end with .cs".to_string() });
    }

    let client = reqwest::Client::new();
    match client.get(&body.url).send().await {
        Ok(response) => {
            match response.bytes().await {
                Ok(bytes) => {
                    let target_path = plugins_dir.join(&body.filename);
                    if let Err(e) = std::fs::write(&target_path, &bytes) {
                        return HttpResponse::InternalServerError().json(ErrorBody {
                            error: format!("Failed to write plugin: {}", e),
                        });
                    }

                    let plugin_name = plugin_name_from_file(&body.filename);

                    let load_result = if let Some(rcon) = rcon_clients.get(server_id.as_str()) {
                        match rcon.oxide_load(&plugin_name).await {
                            Ok(msg) => msg,
                            Err(e) => format!("Load failed (server may be offline): {}", e),
                        }
                    } else {
                        "RCON not available".to_string()
                    };

                    HttpResponse::Ok().json(SuccessBody {
                        success: true,
                        message: format!("Plugin '{}' installed from uMod. Load: {}", plugin_name, load_result),
                    })
                }
                Err(e) => HttpResponse::InternalServerError().json(ErrorBody {
                    error: format!("Failed to download plugin: {}", e),
                }),
            }
        }
        Err(e) => HttpResponse::InternalServerError().json(ErrorBody {
            error: format!("Failed to fetch from uMod: {}", e),
        }),
    }
}

fn urlencoded(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            ' ' => "+".to_string(),
            _ => format!("%{:02X}", c as u8),
        })
        .collect()
}
