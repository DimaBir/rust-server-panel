use actix_multipart::Multipart;
use actix_web::{web, HttpResponse};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::config::AppConfig;
use crate::rcon::RconClient;

#[derive(Debug, Serialize)]
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

/// Derive plugin name from .cs filename (strip extension).
fn plugin_name_from_file(filename: &str) -> String {
    Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(filename)
        .to_string()
}

/// GET /api/plugins - list all installed Oxide plugins (.cs files).
pub async fn list_plugins(
    config: web::Data<AppConfig>,
) -> HttpResponse {
    let plugins_dir = Path::new(&config.paths.oxide_plugins);
    let config_dir = Path::new(&config.paths.oxide_config);

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

                    // Check if config file exists
                    let config_file = config_dir.join(format!("{}.json", name));
                    let has_config = config_file.exists();

                    plugins.push(PluginInfo {
                        name,
                        filename,
                        size,
                        modified,
                        has_config,
                    });
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

/// GET /api/plugins/{name}/config - read plugin config JSON.
pub async fn get_plugin_config(
    name: web::Path<String>,
    config: web::Data<AppConfig>,
) -> HttpResponse {
    let config_path = PathBuf::from(&config.paths.oxide_config)
        .join(format!("{}.json", name.as_ref()));

    if !config_path.exists() {
        return HttpResponse::NotFound().json(ErrorBody {
            error: format!("Config file not found for plugin '{}'", name),
        });
    }

    match std::fs::read_to_string(&config_path) {
        Ok(content) => {
            // Try to parse as JSON for validation
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(json) => HttpResponse::Ok().json(serde_json::json!({
                    "plugin": name.as_ref(),
                    "config": json,
                })),
                Err(_) => {
                    // Return raw content if not valid JSON
                    HttpResponse::Ok().json(serde_json::json!({
                        "plugin": name.as_ref(),
                        "raw_config": content,
                    }))
                }
            }
        }
        Err(e) => HttpResponse::InternalServerError().json(ErrorBody {
            error: format!("Failed to read config: {}", e),
        }),
    }
}

/// PUT /api/plugins/{name}/config - save plugin config and reload.
pub async fn save_plugin_config(
    name: web::Path<String>,
    body: web::Json<serde_json::Value>,
    config: web::Data<AppConfig>,
    rcon: web::Data<Arc<RconClient>>,
) -> HttpResponse {
    let config_path = PathBuf::from(&config.paths.oxide_config)
        .join(format!("{}.json", name.as_ref()));

    // Ensure config directory exists
    if let Some(parent) = config_path.parent() {
        if !parent.exists() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return HttpResponse::InternalServerError().json(ErrorBody {
                    error: format!("Failed to create config directory: {}", e),
                });
            }
        }
    }

    // Write the config
    let json_str = match serde_json::to_string_pretty(&body.into_inner()) {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::BadRequest().json(ErrorBody {
                error: format!("Invalid JSON: {}", e),
            });
        }
    };

    // Backup existing config
    if config_path.exists() {
        let backup = format!("{}.bak", config_path.display());
        let _ = std::fs::copy(&config_path, &backup);
    }

    if let Err(e) = std::fs::write(&config_path, &json_str) {
        return HttpResponse::InternalServerError().json(ErrorBody {
            error: format!("Failed to write config: {}", e),
        });
    }

    // Reload plugin via RCON
    let reload_result = match rcon.oxide_reload(name.as_ref()).await {
        Ok(msg) => msg,
        Err(e) => format!("Reload failed (server may be offline): {}", e),
    };

    HttpResponse::Ok().json(SuccessBody {
        success: true,
        message: format!("Config saved for '{}'. Reload: {}", name, reload_result),
    })
}

/// POST /api/plugins/upload - upload a .cs plugin file.
pub async fn upload_plugin(
    mut payload: Multipart,
    config: web::Data<AppConfig>,
    rcon: web::Data<Arc<RconClient>>,
) -> HttpResponse {
    let plugins_dir = PathBuf::from(&config.paths.oxide_plugins);

    // Ensure plugins directory exists
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
            Err(e) => {
                return HttpResponse::BadRequest().json(ErrorBody {
                    error: format!("Multipart error: {}", e),
                });
            }
        };

        let filename = field
            .content_disposition()
            .and_then(|cd| cd.get_filename().map(|f| f.to_string()))
            .unwrap_or_else(|| "plugin.cs".to_string());

        if !filename.ends_with(".cs") {
            return HttpResponse::BadRequest().json(ErrorBody {
                error: "Only .cs plugin files are allowed".to_string(),
            });
        }

        let target_path = plugins_dir.join(&filename);

        let mut file_data = Vec::new();
        while let Some(chunk) = field.next().await {
            if let Ok(bytes) = chunk {
                file_data.extend_from_slice(&bytes);
            }
        }

        if let Err(e) = std::fs::write(&target_path, &file_data) {
            return HttpResponse::InternalServerError().json(ErrorBody {
                error: format!("Failed to write plugin: {}", e),
            });
        }

        let plugin_name = plugin_name_from_file(&filename);

        // Try to load the plugin via RCON
        let load_result = match rcon.oxide_load(&plugin_name).await {
            Ok(msg) => msg,
            Err(e) => format!("Load failed (server may be offline): {}", e),
        };

        return HttpResponse::Ok().json(SuccessBody {
            success: true,
            message: format!("Plugin '{}' uploaded. Load: {}", plugin_name, load_result),
        });
    }

    HttpResponse::BadRequest().json(ErrorBody {
        error: "No file provided".to_string(),
    })
}

/// DELETE /api/plugins/{name} - unload and delete a plugin.
pub async fn delete_plugin(
    name: web::Path<String>,
    config: web::Data<AppConfig>,
    rcon: web::Data<Arc<RconClient>>,
) -> HttpResponse {
    let plugin_file = PathBuf::from(&config.paths.oxide_plugins)
        .join(format!("{}.cs", name.as_ref()));

    if !plugin_file.exists() {
        return HttpResponse::NotFound().json(ErrorBody {
            error: format!("Plugin '{}' not found", name),
        });
    }

    // Unload via RCON first
    let unload_result = match rcon.oxide_unload(name.as_ref()).await {
        Ok(msg) => msg,
        Err(e) => format!("Unload failed (server may be offline): {}", e),
    };

    // Delete the file
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

/// POST /api/plugins/{name}/reload - reload a plugin via RCON.
pub async fn reload_plugin(
    name: web::Path<String>,
    rcon: web::Data<Arc<RconClient>>,
) -> HttpResponse {
    match rcon.oxide_reload(name.as_ref()).await {
        Ok(msg) => HttpResponse::Ok().json(SuccessBody {
            success: true,
            message: format!("Plugin '{}' reloaded: {}", name, msg),
        }),
        Err(e) => HttpResponse::InternalServerError().json(ErrorBody {
            error: format!("Failed to reload plugin '{}': {}", name, e),
        }),
    }
}

/// uMod search result.
#[derive(Debug, Serialize, Deserialize)]
struct UmodSearchResult {
    #[serde(default)]
    data: Vec<UmodPlugin>,
}

#[derive(Debug, Serialize, Deserialize)]
struct UmodPlugin {
    #[serde(default)]
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    author: String,
    #[serde(default)]
    version: String,
    #[serde(default)]
    download_url: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    icon_url: String,
    #[serde(default)]
    latest_release_at: String,
    #[serde(default)]
    downloads: u64,
}

/// GET /api/plugins/umod/search?q= - search the uMod marketplace.
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

/// POST /api/plugins/umod/install - download a plugin from uMod.
pub async fn umod_install(
    body: web::Json<UmodInstallBody>,
    config: web::Data<AppConfig>,
    rcon: web::Data<Arc<RconClient>>,
) -> HttpResponse {
    let plugins_dir = PathBuf::from(&config.paths.oxide_plugins);

    if !plugins_dir.exists() {
        if let Err(e) = std::fs::create_dir_all(&plugins_dir) {
            return HttpResponse::InternalServerError().json(ErrorBody {
                error: format!("Failed to create plugins directory: {}", e),
            });
        }
    }

    // Validate filename
    if !body.filename.ends_with(".cs") {
        return HttpResponse::BadRequest().json(ErrorBody {
            error: "Filename must end with .cs".to_string(),
        });
    }

    // Download the plugin
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

                    // Load via RCON
                    let load_result = match rcon.oxide_load(&plugin_name).await {
                        Ok(msg) => msg,
                        Err(e) => format!("Load failed (server may be offline): {}", e),
                    };

                    HttpResponse::Ok().json(SuccessBody {
                        success: true,
                        message: format!(
                            "Plugin '{}' installed from uMod. Load: {}",
                            plugin_name, load_result
                        ),
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

/// Simple URL encoding for query parameters.
fn urlencoded(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            ' ' => "+".to_string(),
            _ => format!("%{:02X}", c as u8),
        })
        .collect()
}
