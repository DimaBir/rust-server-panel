use actix_web::{web, HttpResponse};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::Arc;
use sysinfo::System;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};

use crate::config::MonitorConfig;
use crate::rcon::RconClient;

/// A single system metrics snapshot.
#[derive(Debug, Clone, Serialize)]
pub struct SystemSnapshot {
    pub timestamp: DateTime<Utc>,
    pub cpu_percent: f32,
    pub mem_total: u64,
    pub mem_used: u64,
    pub mem_percent: f32,
    pub disk_total: u64,
    pub disk_used: u64,
    pub disk_percent: f32,
}

/// A single game server metrics snapshot.
#[derive(Debug, Clone, Serialize)]
pub struct GameSnapshot {
    pub timestamp: DateTime<Utc>,
    pub online: bool,
    pub players: u32,
    pub max_players: u32,
    pub queued: u32,
    pub fps: f64,
    pub entities: u64,
    pub uptime: u64,
    pub map_name: String,
    pub hostname: String,
}

/// Ring buffer for metric history.
#[derive(Debug)]
pub struct RingBuffer<T> {
    data: VecDeque<T>,
    capacity: usize,
}

impl<T: Clone> RingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            data: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, item: T) {
        if self.data.len() >= self.capacity {
            self.data.pop_front();
        }
        self.data.push_back(item);
    }

    pub fn latest(&self) -> Option<&T> {
        self.data.back()
    }

    pub fn to_vec(&self) -> Vec<T> {
        self.data.iter().cloned().collect()
    }
}

/// Shared state for system monitoring.
pub struct SystemMonitor {
    pub history: RwLock<RingBuffer<SystemSnapshot>>,
}

/// Shared state for game monitoring.
pub struct GameMonitor {
    pub history: RwLock<RingBuffer<GameSnapshot>>,
}

impl SystemMonitor {
    pub fn new(history_size: usize) -> Self {
        Self {
            history: RwLock::new(RingBuffer::new(history_size)),
        }
    }
}

impl GameMonitor {
    pub fn new(history_size: usize) -> Self {
        Self {
            history: RwLock::new(RingBuffer::new(history_size)),
        }
    }
}

/// Background task: poll system metrics at the configured interval.
pub fn spawn_system_collector(
    monitor: Arc<SystemMonitor>,
    config: MonitorConfig,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut sys = System::new_all();
        let mut tick = interval(Duration::from_secs(config.poll_interval_secs));

        loop {
            tick.tick().await;

            sys.refresh_all();

            let cpu_percent = sys.global_cpu_usage();

            let mem_total = sys.total_memory();
            let mem_used = sys.used_memory();
            let mem_percent = if mem_total > 0 {
                (mem_used as f32 / mem_total as f32) * 100.0
            } else {
                0.0
            };

            let disks = sysinfo::Disks::new_with_refreshed_list();
            let (disk_total, disk_used) = disks.list().iter().fold((0u64, 0u64), |(t, u), d| {
                (t + d.total_space(), u + (d.total_space() - d.available_space()))
            });
            let disk_percent = if disk_total > 0 {
                (disk_used as f32 / disk_total as f32) * 100.0
            } else {
                0.0
            };

            let snapshot = SystemSnapshot {
                timestamp: Utc::now(),
                cpu_percent,
                mem_total,
                mem_used,
                mem_percent,
                disk_total,
                disk_used,
                disk_percent,
            };

            let mut history = monitor.history.write().await;
            history.push(snapshot);
        }
    })
}

/// Background task: poll game server metrics via RCON at the configured interval.
pub fn spawn_game_collector(
    monitor: Arc<GameMonitor>,
    rcon: Arc<RconClient>,
    config: MonitorConfig,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut tick = interval(Duration::from_secs(config.poll_interval_secs));

        loop {
            tick.tick().await;

            let snapshot = match rcon.server_info().await {
                Ok(info) => GameSnapshot {
                    timestamp: Utc::now(),
                    online: true,
                    players: info.players,
                    max_players: info.max_players,
                    queued: info.queued,
                    fps: info.framerate,
                    entities: info.entity_count,
                    uptime: info.uptime,
                    map_name: info.map,
                    hostname: info.hostname,
                },
                Err(e) => {
                    tracing::debug!("Game server poll failed (server may be offline): {}", e);
                    GameSnapshot {
                        timestamp: Utc::now(),
                        online: false,
                        players: 0,
                        max_players: 0,
                        queued: 0,
                        fps: 0.0,
                        entities: 0,
                        uptime: 0,
                        map_name: String::new(),
                        hostname: String::new(),
                    }
                }
            };

            let mut history = monitor.history.write().await;
            history.push(snapshot);
        }
    })
}

/// API response for system monitoring.
#[derive(Serialize)]
struct SystemMonitorResponse {
    current: Option<SystemSnapshot>,
    history: Vec<SystemSnapshot>,
}

/// API response for game monitoring.
#[derive(Serialize)]
struct GameMonitorResponse {
    current: Option<GameSnapshot>,
    history: Vec<GameSnapshot>,
}

/// GET /api/monitor/system
pub async fn get_system_metrics(
    monitor: web::Data<Arc<SystemMonitor>>,
) -> HttpResponse {
    let history = monitor.history.read().await;
    let current = history.latest().cloned();
    let all = history.to_vec();

    HttpResponse::Ok().json(SystemMonitorResponse {
        current,
        history: all,
    })
}

/// GET /api/monitor/game
pub async fn get_game_metrics(
    monitor: web::Data<Arc<GameMonitor>>,
) -> HttpResponse {
    let history = monitor.history.read().await;
    let current = history.latest().cloned();
    let all = history.to_vec();

    HttpResponse::Ok().json(GameMonitorResponse {
        current,
        history: all,
    })
}
