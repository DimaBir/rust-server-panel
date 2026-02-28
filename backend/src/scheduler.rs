use actix_web::{web, HttpResponse};
use chrono::{DateTime, NaiveTime, Utc, Weekday, Datelike};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use uuid::Uuid;

use crate::config::AppConfig;
use crate::lgsm::LgsmLock;
use crate::rcon::RconClient;

/// Scheduled job types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum JobType {
    Restart,
    Update,
    Backup,
    WipeMap,
    WipeFull,
    RconCommand,
    Announce,
}

/// Scheduled job definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledJob {
    pub id: String,
    pub name: String,
    pub job_type: JobType,
    pub enabled: bool,
    /// Cron-like schedule: "HH:MM" for daily, or "Day HH:MM" for weekly.
    pub schedule: String,
    /// For RconCommand type: the command to execute.
    /// For Announce type: the message to broadcast.
    pub payload: Option<String>,
    pub last_run: Option<DateTime<Utc>>,
    pub next_run: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateJobRequest {
    pub name: String,
    pub job_type: JobType,
    pub schedule: String,
    pub payload: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateJobRequest {
    pub name: Option<String>,
    pub job_type: Option<JobType>,
    pub schedule: Option<String>,
    pub payload: Option<String>,
    pub enabled: Option<bool>,
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

const SCHEDULES_FILE: &str = "schedules.json";

/// Shared scheduler state.
pub struct Scheduler {
    pub jobs: RwLock<Vec<ScheduledJob>>,
}

impl Scheduler {
    pub fn new() -> Self {
        let jobs = Self::load_from_disk().unwrap_or_default();
        Self {
            jobs: RwLock::new(jobs),
        }
    }

    fn load_from_disk() -> anyhow::Result<Vec<ScheduledJob>> {
        let path = Path::new(SCHEDULES_FILE);
        if !path.exists() {
            return Ok(Vec::new());
        }
        let content = std::fs::read_to_string(path)?;
        let jobs: Vec<ScheduledJob> = serde_json::from_str(&content)?;
        Ok(jobs)
    }

    async fn save_to_disk(&self) -> anyhow::Result<()> {
        let jobs = self.jobs.read().await;
        let content = serde_json::to_string_pretty(&*jobs)?;
        std::fs::write(SCHEDULES_FILE, content)?;
        Ok(())
    }
}

/// Parse a schedule string to determine the next run time.
/// Formats:
///   "HH:MM" - daily at that time (UTC)
///   "Mon HH:MM", "Tue HH:MM", etc. - weekly on that day
fn compute_next_run(schedule: &str) -> Option<DateTime<Utc>> {
    let now = Utc::now();
    let parts: Vec<&str> = schedule.trim().split_whitespace().collect();

    match parts.len() {
        // "HH:MM" -> daily
        1 => {
            let time = NaiveTime::parse_from_str(parts[0], "%H:%M").ok()?;
            let today = now.date_naive().and_time(time);
            let today_utc = today.and_utc();
            if today_utc > now {
                Some(today_utc)
            } else {
                // Tomorrow
                let tomorrow = now.date_naive().succ_opt()?.and_time(time);
                Some(tomorrow.and_utc())
            }
        }
        // "Day HH:MM" -> weekly
        2 => {
            let target_day = parse_weekday(parts[0])?;
            let time = NaiveTime::parse_from_str(parts[1], "%H:%M").ok()?;

            let current_day = now.weekday();
            let mut days_ahead = (target_day.num_days_from_monday() as i64)
                - (current_day.num_days_from_monday() as i64);

            if days_ahead < 0 {
                days_ahead += 7;
            }

            let target_date = now.date_naive() + chrono::Duration::days(days_ahead);
            let target_dt = target_date.and_time(time).and_utc();

            if target_dt <= now {
                // Next week
                let next_week = target_date + chrono::Duration::days(7);
                Some(next_week.and_time(time).and_utc())
            } else {
                Some(target_dt)
            }
        }
        _ => None,
    }
}

fn parse_weekday(s: &str) -> Option<Weekday> {
    match s.to_lowercase().as_str() {
        "mon" | "monday" => Some(Weekday::Mon),
        "tue" | "tuesday" => Some(Weekday::Tue),
        "wed" | "wednesday" => Some(Weekday::Wed),
        "thu" | "thursday" => Some(Weekday::Thu),
        "fri" | "friday" => Some(Weekday::Fri),
        "sat" | "saturday" => Some(Weekday::Sat),
        "sun" | "sunday" => Some(Weekday::Sun),
        _ => None,
    }
}

/// Background task: check scheduled jobs every 30 seconds and execute due ones.
pub fn spawn_scheduler(
    scheduler: Arc<Scheduler>,
    rcon: Arc<RconClient>,
    config: AppConfig,
    lgsm_lock: Arc<LgsmLock>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut tick = interval(Duration::from_secs(30));

        loop {
            tick.tick().await;

            let now = Utc::now();
            let mut jobs = scheduler.jobs.write().await;

            for job in jobs.iter_mut() {
                if !job.enabled {
                    continue;
                }

                // Compute next_run if not set
                if job.next_run.is_none() {
                    job.next_run = compute_next_run(&job.schedule);
                }

                if let Some(next) = job.next_run {
                    if now >= next {
                        tracing::info!("Executing scheduled job: {} ({})", job.name, job.id);

                        // Execute the job
                        execute_job(job, &rcon, &config, &lgsm_lock).await;

                        job.last_run = Some(now);
                        job.next_run = compute_next_run(&job.schedule);
                    }
                }
            }

            drop(jobs);

            // Persist
            if let Err(e) = scheduler.save_to_disk().await {
                tracing::error!("Failed to save schedules: {}", e);
            }
        }
    })
}

async fn execute_job(
    job: &ScheduledJob,
    rcon: &RconClient,
    config: &AppConfig,
    lgsm_lock: &LgsmLock,
) {
    let result = match job.job_type {
        JobType::Restart => {
            let _guard = lgsm_lock.lock.lock().await;
            run_lgsm(&config.paths.lgsm_script, "restart").await
        }
        JobType::Update => {
            let _guard = lgsm_lock.lock.lock().await;
            run_lgsm(&config.paths.lgsm_script, "update").await
        }
        JobType::Backup => {
            let _guard = lgsm_lock.lock.lock().await;
            run_lgsm(&config.paths.lgsm_script, "backup").await
        }
        JobType::WipeMap => {
            let _guard = lgsm_lock.lock.lock().await;
            let _ = run_lgsm(&config.paths.lgsm_script, "stop").await;
            delete_wipe_files(&config.paths.server_files, false);
            run_lgsm(&config.paths.lgsm_script, "start").await
        }
        JobType::WipeFull => {
            let _guard = lgsm_lock.lock.lock().await;
            let _ = run_lgsm(&config.paths.lgsm_script, "stop").await;
            delete_wipe_files(&config.paths.server_files, true);
            run_lgsm(&config.paths.lgsm_script, "start").await
        }
        JobType::RconCommand => {
            let cmd = job.payload.as_deref().unwrap_or("");
            rcon.execute(cmd).await.map_err(|e| e.to_string())
        }
        JobType::Announce => {
            let msg = job.payload.as_deref().unwrap_or("Server announcement");
            rcon.say(msg).await.map_err(|e| e.to_string())
        }
    };

    match result {
        Ok(output) => tracing::info!("Job '{}' completed: {}", job.name, output),
        Err(e) => tracing::error!("Job '{}' failed: {}", job.name, e),
    }
}

async fn run_lgsm(script: &str, action: &str) -> Result<String, String> {
    let output = tokio::process::Command::new(script)
        .arg(action)
        .output()
        .await
        .map_err(|e| e.to_string())?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn delete_wipe_files(server_files: &str, full: bool) {
    let server_dir = format!("{}/server/rustserver", server_files);
    if let Ok(entries) = std::fs::read_dir(&server_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                let should_delete = if full {
                    ext == "sav" || ext == "map" || ext == "db"
                } else {
                    ext == "sav" || ext == "map"
                };
                if should_delete {
                    let _ = std::fs::remove_file(&path);
                }
            }
        }
    }
}

// --- API Endpoints ---

/// GET /api/schedule - list all jobs.
pub async fn list_jobs(
    scheduler: web::Data<Arc<Scheduler>>,
) -> HttpResponse {
    let jobs = scheduler.jobs.read().await;
    HttpResponse::Ok().json(&*jobs)
}

/// POST /api/schedule - create a new job.
pub async fn create_job(
    body: web::Json<CreateJobRequest>,
    scheduler: web::Data<Arc<Scheduler>>,
) -> HttpResponse {
    let next_run = compute_next_run(&body.schedule);
    let job = ScheduledJob {
        id: Uuid::new_v4().to_string(),
        name: body.name.clone(),
        job_type: body.job_type.clone(),
        enabled: body.enabled.unwrap_or(true),
        schedule: body.schedule.clone(),
        payload: body.payload.clone(),
        last_run: None,
        next_run,
        created_at: Utc::now(),
    };

    {
        let mut jobs = scheduler.jobs.write().await;
        jobs.push(job.clone());
    }

    if let Err(e) = scheduler.save_to_disk().await {
        tracing::error!("Failed to save schedules: {}", e);
    }

    HttpResponse::Created().json(job)
}

/// PUT /api/schedule/{id} - update a job.
pub async fn update_job(
    id: web::Path<String>,
    body: web::Json<UpdateJobRequest>,
    scheduler: web::Data<Arc<Scheduler>>,
) -> HttpResponse {
    let mut jobs = scheduler.jobs.write().await;
    let job = match jobs.iter_mut().find(|j| j.id == *id) {
        Some(j) => j,
        None => {
            return HttpResponse::NotFound().json(ErrorBody {
                error: "Job not found".to_string(),
            });
        }
    };

    if let Some(ref name) = body.name {
        job.name = name.clone();
    }
    if let Some(ref job_type) = body.job_type {
        job.job_type = job_type.clone();
    }
    if let Some(ref schedule) = body.schedule {
        job.schedule = schedule.clone();
        job.next_run = compute_next_run(schedule);
    }
    if let Some(ref payload) = body.payload {
        job.payload = Some(payload.clone());
    }
    if let Some(enabled) = body.enabled {
        job.enabled = enabled;
    }

    let job = job.clone();
    drop(jobs);

    if let Err(e) = scheduler.save_to_disk().await {
        tracing::error!("Failed to save schedules: {}", e);
    }

    HttpResponse::Ok().json(job)
}

/// DELETE /api/schedule/{id} - delete a job.
pub async fn delete_job(
    id: web::Path<String>,
    scheduler: web::Data<Arc<Scheduler>>,
) -> HttpResponse {
    let mut jobs = scheduler.jobs.write().await;
    let original_len = jobs.len();
    jobs.retain(|j| j.id != *id);

    if jobs.len() == original_len {
        return HttpResponse::NotFound().json(ErrorBody {
            error: "Job not found".to_string(),
        });
    }

    drop(jobs);

    if let Err(e) = scheduler.save_to_disk().await {
        tracing::error!("Failed to save schedules: {}", e);
    }

    HttpResponse::Ok().json(SuccessBody {
        success: true,
        message: format!("Job {} deleted", id),
    })
}

/// POST /api/schedule/{id}/toggle - enable/disable a job.
pub async fn toggle_job(
    id: web::Path<String>,
    scheduler: web::Data<Arc<Scheduler>>,
) -> HttpResponse {
    let mut jobs = scheduler.jobs.write().await;
    let job = match jobs.iter_mut().find(|j| j.id == *id) {
        Some(j) => j,
        None => {
            return HttpResponse::NotFound().json(ErrorBody {
                error: "Job not found".to_string(),
            });
        }
    };

    job.enabled = !job.enabled;
    if job.enabled {
        job.next_run = compute_next_run(&job.schedule);
    }

    let job = job.clone();
    drop(jobs);

    if let Err(e) = scheduler.save_to_disk().await {
        tracing::error!("Failed to save schedules: {}", e);
    }

    HttpResponse::Ok().json(job)
}
