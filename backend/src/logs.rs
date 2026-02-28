use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;

use crate::config::GameServerConfig;

#[derive(Debug, Deserialize)]
pub struct TailQuery {
    pub file: Option<String>,
    pub lines: Option<usize>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LogResponse {
    file: String,
    lines: Vec<String>,
    total_lines: usize,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: String,
}

fn allowed_log_files(config: &GameServerConfig) -> HashMap<String, PathBuf> {
    let mut map = HashMap::new();
    map.insert("console".to_string(), PathBuf::from(&config.paths.server_log));

    let oxide_log = PathBuf::from(&config.paths.server_files).join("oxide/logs/oxide_log.txt");
    map.insert("oxide".to_string(), oxide_log);

    let lgsm_log = PathBuf::from("/home/rustserver/log/script/rustserver-script.log");
    map.insert("script".to_string(), lgsm_log);

    map
}

fn tail_file(path: &PathBuf, n: usize) -> anyhow::Result<Vec<String>> {
    let file = std::fs::File::open(path)?;
    let file_size = file.metadata()?.len();

    if file_size == 0 {
        return Ok(Vec::new());
    }

    if file_size < 1_048_576 {
        let reader = BufReader::new(file);
        let all_lines: Vec<String> = reader.lines().filter_map(|l| l.ok()).collect();
        let start = if all_lines.len() > n { all_lines.len() - n } else { 0 };
        return Ok(all_lines[start..].to_vec());
    }

    let mut reader = BufReader::new(file);
    let mut lines = Vec::new();
    let chunk_size: u64 = 65536;
    let mut pos = file_size;

    loop {
        let seek_to = if pos > chunk_size { pos - chunk_size } else { 0 };
        reader.seek(SeekFrom::Start(seek_to))?;

        if seek_to > 0 {
            let mut partial = String::new();
            let _ = reader.read_line(&mut partial);
        }

        let mut chunk_lines = Vec::new();
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    let trimmed = line.trim_end_matches('\n').trim_end_matches('\r').to_string();
                    chunk_lines.push(trimmed);
                }
                Err(_) => break,
            }
            if reader.stream_position().unwrap_or(0) >= pos {
                break;
            }
        }

        chunk_lines.append(&mut lines);
        lines = chunk_lines;

        if lines.len() >= n || seek_to == 0 {
            break;
        }
        pos = seek_to;
    }

    let start = if lines.len() > n { lines.len() - n } else { 0 };
    Ok(lines[start..].to_vec())
}

/// GET /api/servers/{server_id}/logs/tail
pub async fn tail_log(
    server_id: web::Path<String>,
    query: web::Query<TailQuery>,
    server_configs: web::Data<Vec<GameServerConfig>>,
) -> HttpResponse {
    let config = match server_configs.iter().find(|s| s.id == *server_id) {
        Some(c) => c,
        None => return HttpResponse::NotFound().json(ErrorBody { error: "Server not found".to_string() }),
    };

    let file_alias = query.file.as_deref().unwrap_or("console");
    let num_lines = query.lines.unwrap_or(100).min(5000);

    let allowed = allowed_log_files(config);

    let log_path = match allowed.get(file_alias) {
        Some(p) => p,
        None => {
            let available: Vec<&str> = allowed.keys().map(|k| k.as_str()).collect();
            return HttpResponse::BadRequest().json(ErrorBody {
                error: format!("Unknown log file '{}'. Available: {}", file_alias, available.join(", ")),
            });
        }
    };

    if !log_path.exists() {
        return HttpResponse::NotFound().json(ErrorBody {
            error: format!("Log file not found: {}", log_path.display()),
        });
    }

    match tail_file(log_path, num_lines) {
        Ok(lines) => {
            let total = lines.len();
            HttpResponse::Ok().json(LogResponse {
                file: file_alias.to_string(),
                lines,
                total_lines: total,
            })
        }
        Err(e) => HttpResponse::InternalServerError().json(ErrorBody {
            error: format!("Failed to read log: {}", e),
        }),
    }
}
