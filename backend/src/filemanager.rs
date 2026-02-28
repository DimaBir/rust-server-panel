use actix_multipart::Multipart;
use actix_web::{web, HttpResponse};
use chrono::{DateTime, Utc};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::config::GameServerConfig;

const MAX_FILE_SIZE: u64 = 1_048_576; // 1 MB for text reads

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<DateTime<Utc>>,
    pub is_text: bool,
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ReadQuery {
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct DownloadQuery {
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct WriteBody {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct MkdirBody {
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct DeleteQuery {
    pub path: String,
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

fn get_base_dir(server_id: &str, server_configs: &[GameServerConfig]) -> Result<String, HttpResponse> {
    server_configs
        .iter()
        .find(|s| s.id == server_id)
        .map(|s| s.paths.base_dir.clone())
        .ok_or_else(|| HttpResponse::NotFound().json(ErrorBody { error: "Server not found".to_string() }))
}

fn safe_resolve(base_dir: &str, relative_path: &str) -> Result<PathBuf, String> {
    let base = PathBuf::from(base_dir);
    let cleaned = relative_path.trim_start_matches('/');
    let requested = if cleaned.is_empty() {
        base.clone()
    } else {
        base.join(cleaned)
    };

    let canonical = if requested.exists() {
        requested.canonicalize().map_err(|e| format!("Failed to resolve path: {}", e))?
    } else {
        let parent = requested.parent().ok_or_else(|| "Invalid path: no parent".to_string())?;
        if !parent.exists() {
            return Err("Parent directory does not exist".to_string());
        }
        let canonical_parent = parent.canonicalize().map_err(|e| format!("Failed to resolve parent: {}", e))?;
        let file_name = requested.file_name().ok_or_else(|| "Invalid path: no filename".to_string())?;
        canonical_parent.join(file_name)
    };

    let canonical_base = base.canonicalize().unwrap_or_else(|_| base.clone());
    if !canonical.starts_with(&canonical_base) {
        return Err("Access denied: path is outside the allowed directory".to_string());
    }

    Ok(canonical)
}

fn is_text_file(path: &Path) -> bool {
    let text_extensions = [
        "txt", "cfg", "json", "yaml", "yml", "toml", "xml", "ini", "conf", "log",
        "cs", "lua", "py", "sh", "bash", "md", "html", "css", "js", "ts",
        "csv", "env", "properties", "config",
    ];
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| text_extensions.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// GET /api/servers/{server_id}/files/list
pub async fn list_files(
    server_id: web::Path<String>,
    query: web::Query<ListQuery>,
    server_configs: web::Data<Vec<GameServerConfig>>,
) -> HttpResponse {
    let base_dir = match get_base_dir(&server_id, &server_configs) {
        Ok(d) => d,
        Err(e) => return e,
    };
    let relative = query.path.as_deref().unwrap_or("");
    let dir_path = match safe_resolve(&base_dir, relative) {
        Ok(p) => p,
        Err(e) => return HttpResponse::Forbidden().json(ErrorBody { error: e }),
    };

    if !dir_path.is_dir() {
        return HttpResponse::BadRequest().json(ErrorBody { error: "Path is not a directory".to_string() });
    }

    let mut entries = Vec::new();
    match std::fs::read_dir(&dir_path) {
        Ok(read_dir) => {
            for entry in read_dir.flatten() {
                let metadata = entry.metadata().ok();
                let path = entry.path();
                let is_dir = path.is_dir();
                let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
                let modified = metadata
                    .as_ref()
                    .and_then(|m| m.modified().ok())
                    .map(|t| DateTime::<Utc>::from(t));

                let rel_path = path
                    .strip_prefix(&base_dir)
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| path.display().to_string());

                entries.push(FileEntry {
                    name: entry.file_name().to_string_lossy().to_string(),
                    path: rel_path,
                    is_dir,
                    size,
                    modified,
                    is_text: is_text_file(&path),
                });
            }
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(ErrorBody {
                error: format!("Failed to read directory: {}", e),
            });
        }
    }

    entries.sort_by(|a, b| {
        b.is_dir.cmp(&a.is_dir).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    HttpResponse::Ok().json(entries)
}

/// GET /api/servers/{server_id}/files/read
pub async fn read_file(
    server_id: web::Path<String>,
    query: web::Query<ReadQuery>,
    server_configs: web::Data<Vec<GameServerConfig>>,
) -> HttpResponse {
    let base_dir = match get_base_dir(&server_id, &server_configs) {
        Ok(d) => d,
        Err(e) => return e,
    };
    let file_path = match safe_resolve(&base_dir, &query.path) {
        Ok(p) => p,
        Err(e) => return HttpResponse::Forbidden().json(ErrorBody { error: e }),
    };

    if !file_path.is_file() {
        return HttpResponse::NotFound().json(ErrorBody { error: "File not found".to_string() });
    }

    if let Ok(metadata) = std::fs::metadata(&file_path) {
        if metadata.len() > MAX_FILE_SIZE {
            return HttpResponse::BadRequest().json(ErrorBody {
                error: format!("File too large ({} bytes, max {} bytes)", metadata.len(), MAX_FILE_SIZE),
            });
        }
    }

    match std::fs::read_to_string(&file_path) {
        Ok(content) => HttpResponse::Ok().json(serde_json::json!({
            "path": query.path,
            "content": content,
            "size": content.len(),
        })),
        Err(e) => HttpResponse::InternalServerError().json(ErrorBody {
            error: format!("Failed to read file: {}", e),
        }),
    }
}

/// PUT /api/servers/{server_id}/files/write
pub async fn write_file(
    server_id: web::Path<String>,
    body: web::Json<WriteBody>,
    server_configs: web::Data<Vec<GameServerConfig>>,
) -> HttpResponse {
    let base_dir = match get_base_dir(&server_id, &server_configs) {
        Ok(d) => d,
        Err(e) => return e,
    };
    let file_path = match safe_resolve(&base_dir, &body.path) {
        Ok(p) => p,
        Err(e) => return HttpResponse::Forbidden().json(ErrorBody { error: e }),
    };

    if file_path.exists() {
        let backup_path = format!("{}.bak", file_path.display());
        if let Err(e) = std::fs::copy(&file_path, &backup_path) {
            tracing::warn!("Failed to create backup: {}", e);
        }
    }

    match std::fs::write(&file_path, &body.content) {
        Ok(()) => HttpResponse::Ok().json(SuccessBody {
            success: true,
            message: format!("File written: {}", body.path),
        }),
        Err(e) => HttpResponse::InternalServerError().json(ErrorBody {
            error: format!("Failed to write file: {}", e),
        }),
    }
}

/// POST /api/servers/{server_id}/files/upload
pub async fn upload_file(
    server_id: web::Path<String>,
    mut payload: Multipart,
    server_configs: web::Data<Vec<GameServerConfig>>,
) -> HttpResponse {
    let base_dir = match get_base_dir(&server_id, &server_configs) {
        Ok(d) => d,
        Err(e) => return e,
    };

    let mut target_dir: Option<String> = None;
    let mut uploaded_files = Vec::new();

    while let Some(item) = payload.next().await {
        let mut field = match item {
            Ok(f) => f,
            Err(e) => return HttpResponse::BadRequest().json(ErrorBody { error: format!("Multipart error: {}", e) }),
        };

        let field_name = field.name().map(|n| n.to_string()).unwrap_or_default();

        if field_name == "path" {
            let mut data = Vec::new();
            while let Some(chunk) = field.next().await {
                if let Ok(bytes) = chunk { data.extend_from_slice(&bytes); }
            }
            target_dir = Some(String::from_utf8_lossy(&data).to_string());
            continue;
        }

        if field_name == "file" {
            let filename = field
                .content_disposition()
                .and_then(|cd| cd.get_filename().map(|f| f.to_string()))
                .unwrap_or_else(|| "uploaded_file".to_string());

            let dir = target_dir.as_deref().unwrap_or("");
            let target_path = match safe_resolve(&base_dir, &format!("{}/{}", dir, filename)) {
                Ok(p) => p,
                Err(e) => return HttpResponse::Forbidden().json(ErrorBody { error: e }),
            };

            let mut file_data = Vec::new();
            while let Some(chunk) = field.next().await {
                if let Ok(bytes) = chunk { file_data.extend_from_slice(&bytes); }
            }

            match std::fs::write(&target_path, &file_data) {
                Ok(()) => { uploaded_files.push(filename); }
                Err(e) => {
                    return HttpResponse::InternalServerError().json(ErrorBody {
                        error: format!("Failed to write uploaded file: {}", e),
                    });
                }
            }
        }
    }

    HttpResponse::Ok().json(SuccessBody {
        success: true,
        message: format!("Uploaded: {}", uploaded_files.join(", ")),
    })
}

/// GET /api/servers/{server_id}/files/download
pub async fn download_file(
    server_id: web::Path<String>,
    query: web::Query<DownloadQuery>,
    server_configs: web::Data<Vec<GameServerConfig>>,
) -> HttpResponse {
    let base_dir = match get_base_dir(&server_id, &server_configs) {
        Ok(d) => d,
        Err(e) => return e,
    };
    let file_path = match safe_resolve(&base_dir, &query.path) {
        Ok(p) => p,
        Err(e) => return HttpResponse::Forbidden().json(ErrorBody { error: e }),
    };

    if !file_path.is_file() {
        return HttpResponse::NotFound().json(ErrorBody { error: "File not found".to_string() });
    }

    let filename = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("download");

    match std::fs::read(&file_path) {
        Ok(data) => {
            let mime = mime_guess::from_path(&file_path).first_or_octet_stream().to_string();
            HttpResponse::Ok()
                .insert_header(("Content-Type", mime))
                .insert_header(("Content-Disposition", format!("attachment; filename=\"{}\"", filename)))
                .body(data)
        }
        Err(e) => HttpResponse::InternalServerError().json(ErrorBody {
            error: format!("Failed to read file: {}", e),
        }),
    }
}

/// POST /api/servers/{server_id}/files/mkdir
pub async fn mkdir(
    server_id: web::Path<String>,
    body: web::Json<MkdirBody>,
    server_configs: web::Data<Vec<GameServerConfig>>,
) -> HttpResponse {
    let base_dir = match get_base_dir(&server_id, &server_configs) {
        Ok(d) => d,
        Err(e) => return e,
    };
    let dir_path = match safe_resolve(&base_dir, &body.path) {
        Ok(p) => p,
        Err(e) => return HttpResponse::Forbidden().json(ErrorBody { error: e }),
    };

    match std::fs::create_dir_all(&dir_path) {
        Ok(()) => HttpResponse::Ok().json(SuccessBody {
            success: true,
            message: format!("Directory created: {}", body.path),
        }),
        Err(e) => HttpResponse::InternalServerError().json(ErrorBody {
            error: format!("Failed to create directory: {}", e),
        }),
    }
}

/// DELETE /api/servers/{server_id}/files/delete
pub async fn delete_file(
    server_id: web::Path<String>,
    query: web::Query<DeleteQuery>,
    server_configs: web::Data<Vec<GameServerConfig>>,
) -> HttpResponse {
    let base_dir = match get_base_dir(&server_id, &server_configs) {
        Ok(d) => d,
        Err(e) => return e,
    };
    let target_path = match safe_resolve(&base_dir, &query.path) {
        Ok(p) => p,
        Err(e) => return HttpResponse::Forbidden().json(ErrorBody { error: e }),
    };

    let canonical_base = PathBuf::from(&base_dir).canonicalize().unwrap_or_else(|_| PathBuf::from(&base_dir));
    if target_path == canonical_base {
        return HttpResponse::Forbidden().json(ErrorBody { error: "Cannot delete the base directory".to_string() });
    }

    let result = if target_path.is_dir() {
        std::fs::remove_dir_all(&target_path)
    } else {
        std::fs::remove_file(&target_path)
    };

    match result {
        Ok(()) => HttpResponse::Ok().json(SuccessBody {
            success: true,
            message: format!("Deleted: {}", query.path),
        }),
        Err(e) => HttpResponse::InternalServerError().json(ErrorBody {
            error: format!("Failed to delete: {}", e),
        }),
    }
}
