use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;

use crate::config::AppConfig;

#[derive(Debug, Deserialize)]
pub struct TailQuery {
    /// Which log file: "console", "error", or a recognized alias.
    pub file: Option<String>,
    /// Number of lines to return from the end.
    pub lines: Option<usize>,
}

#[derive(Debug, Serialize)]
struct LogResponse {
    file: String,
    lines: Vec<String>,
    total_lines: usize,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: String,
}

/// Build the whitelist of allowed log file paths.
/// Only console and error logs are accessible.
fn allowed_log_files(config: &AppConfig) -> HashMap<String, PathBuf> {
    let mut map = HashMap::new();

    // Primary console log
    map.insert(
        "console".to_string(),
        PathBuf::from(&config.paths.server_log),
    );

    // Oxide log (if it exists)
    let oxide_log = PathBuf::from(&config.paths.server_files)
        .join("oxide/logs/oxide_log.txt");
    map.insert("oxide".to_string(), oxide_log);

    // LGSM script log
    let lgsm_log = PathBuf::from("/home/rustserver/log/script/rustserver-script.log");
    map.insert("script".to_string(), lgsm_log);

    map
}

/// Read the last N lines of a file efficiently.
fn tail_file(path: &PathBuf, n: usize) -> anyhow::Result<Vec<String>> {
    let file = std::fs::File::open(path)?;
    let file_size = file.metadata()?.len();

    if file_size == 0 {
        return Ok(Vec::new());
    }

    // For small files, just read the whole thing
    if file_size < 1_048_576 {
        // < 1 MB
        let reader = BufReader::new(file);
        let all_lines: Vec<String> = reader.lines().filter_map(|l| l.ok()).collect();
        let start = if all_lines.len() > n {
            all_lines.len() - n
        } else {
            0
        };
        return Ok(all_lines[start..].to_vec());
    }

    // For large files, seek from the end
    let mut reader = BufReader::new(file);
    let mut lines = Vec::new();
    let chunk_size: u64 = 65536; // 64 KB chunks
    let mut pos = file_size;

    loop {
        let seek_to = if pos > chunk_size {
            pos - chunk_size
        } else {
            0
        };

        reader.seek(SeekFrom::Start(seek_to))?;

        // If we didn't seek to the beginning, skip the first partial line
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

            // Stop if we've gone past our original position
            if reader.stream_position().unwrap_or(0) >= pos {
                break;
            }
        }

        // Prepend these lines
        chunk_lines.append(&mut lines);
        lines = chunk_lines;

        if lines.len() >= n || seek_to == 0 {
            break;
        }
        pos = seek_to;
    }

    // Take only the last N
    let start = if lines.len() > n {
        lines.len() - n
    } else {
        0
    };
    Ok(lines[start..].to_vec())
}

/// GET /api/logs/tail?file=console&lines=100
pub async fn tail_log(
    query: web::Query<TailQuery>,
    config: web::Data<AppConfig>,
) -> HttpResponse {
    let file_alias = query.file.as_deref().unwrap_or("console");
    let num_lines = query.lines.unwrap_or(100).min(5000); // Cap at 5000 lines

    let allowed = allowed_log_files(&config);

    let log_path = match allowed.get(file_alias) {
        Some(p) => p,
        None => {
            let available: Vec<&str> = allowed.keys().map(|k| k.as_str()).collect();
            return HttpResponse::BadRequest().json(ErrorBody {
                error: format!(
                    "Unknown log file '{}'. Available: {}",
                    file_alias,
                    available.join(", ")
                ),
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
