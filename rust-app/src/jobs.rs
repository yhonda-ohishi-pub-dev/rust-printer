use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Job status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Cancelled,
}

/// Print job information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintJob {
    pub id: String,
    pub status: JobStatus,
    pub message: Option<String>,
    pub filename: Option<String>,
    pub printer: Option<String>,
    pub printer_ip: Option<String>,
    pub printer_name: Option<String>,
    pub print_mode: Option<String>,
    pub file_size: Option<usize>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl PrintJob {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            status: JobStatus::Pending,
            message: None,
            filename: None,
            printer: None,
            printer_ip: None,
            printer_name: None,
            print_mode: None,
            file_size: None,
            created_at: chrono::Utc::now(),
            completed_at: None,
        }
    }

    pub fn with_filename(mut self, filename: String) -> Self {
        self.filename = Some(filename);
        self
    }

    pub fn with_printer(mut self, printer: String) -> Self {
        self.printer = Some(printer);
        self
    }

    pub fn with_file_size(mut self, size: usize) -> Self {
        self.file_size = Some(size);
        self
    }
}

/// Job store for tracking async print jobs
#[derive(Debug, Clone)]
pub struct JobStore {
    jobs: Arc<RwLock<HashMap<String, PrintJob>>>,
}

impl Default for JobStore {
    fn default() -> Self {
        Self::new()
    }
}

impl JobStore {
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new job and return it
    pub async fn create_job(&self) -> PrintJob {
        let job = PrintJob::new();
        let mut jobs = self.jobs.write().await;
        jobs.insert(job.id.clone(), job.clone());
        job
    }

    /// Get a job by ID
    pub async fn get_job(&self, id: &str) -> Option<PrintJob> {
        let jobs = self.jobs.read().await;
        jobs.get(id).cloned()
    }

    /// Update job status to processing
    pub async fn set_processing(&self, id: &str) {
        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.get_mut(id) {
            job.status = JobStatus::Processing;
        }
    }

    /// Update job as completed
    pub async fn set_completed(&self, id: &str, message: &str) {
        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.get_mut(id) {
            job.status = JobStatus::Completed;
            job.message = Some(message.to_string());
            job.completed_at = Some(chrono::Utc::now());
        }
    }

    /// Update job as failed
    pub async fn set_failed(&self, id: &str, error: &str) {
        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.get_mut(id) {
            job.status = JobStatus::Failed;
            job.message = Some(error.to_string());
            job.completed_at = Some(chrono::Utc::now());
        }
    }

    /// Update job metadata
    pub async fn update_metadata(
        &self,
        id: &str,
        filename: Option<String>,
        printer_ip: Option<String>,
        printer_name: Option<String>,
        print_mode: Option<String>,
        file_size: Option<usize>,
    ) {
        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.get_mut(id) {
            if let Some(f) = filename {
                job.filename = Some(f);
            }
            if let Some(ip) = &printer_ip {
                job.printer_ip = Some(ip.clone());
            }
            if let Some(name) = &printer_name {
                job.printer_name = Some(name.clone());
            }
            if let Some(mode) = &print_mode {
                job.print_mode = Some(mode.clone());
            }
            if let Some(s) = file_size {
                job.file_size = Some(s);
            }
            // Also update legacy printer field for compatibility
            if printer_ip.is_some() || printer_name.is_some() {
                let ip_part = job.printer_ip.as_deref().unwrap_or("unknown");
                let name_part = job.printer_name.as_deref().unwrap_or("");
                let mode_part = job.print_mode.as_deref().unwrap_or("");
                if name_part.is_empty() {
                    job.printer = Some(format!("{} ({})", ip_part, mode_part));
                } else {
                    job.printer = Some(format!("{} - {} ({})", ip_part, name_part, mode_part));
                }
            }
        }
    }

    /// Clean up old jobs (older than 1 hour)
    pub async fn cleanup_old_jobs(&self) {
        let mut jobs = self.jobs.write().await;
        let now = chrono::Utc::now();
        let one_hour_ago = now - chrono::Duration::hours(1);
        jobs.retain(|_, job| job.created_at > one_hour_ago);
    }

    /// Get all jobs
    pub async fn get_all_jobs(&self) -> Vec<PrintJob> {
        let jobs = self.jobs.read().await;
        let mut job_list: Vec<PrintJob> = jobs.values().cloned().collect();
        job_list.sort_by(|a, b| b.created_at.cmp(&a.created_at)); // newest first
        job_list
    }

    /// Cancel a job (mark as cancelled)
    /// Returns true if the job was found and cancelled, false if not found
    pub async fn cancel_job(&self, id: &str) -> Option<PrintJob> {
        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.get_mut(id) {
            // Can only cancel pending or processing jobs
            if job.status == JobStatus::Pending || job.status == JobStatus::Processing {
                job.status = JobStatus::Cancelled;
                job.message = Some("Job cancelled by user".to_string());
                job.completed_at = Some(chrono::Utc::now());
                return Some(job.clone());
            }
        }
        None
    }

    /// Check if a job is cancelled
    pub async fn is_cancelled(&self, id: &str) -> bool {
        let jobs = self.jobs.read().await;
        jobs.get(id).map(|j| j.status == JobStatus::Cancelled).unwrap_or(false)
    }
}
