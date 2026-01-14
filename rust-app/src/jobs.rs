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
}

/// Print job information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintJob {
    pub id: String,
    pub status: JobStatus,
    pub message: Option<String>,
    pub filename: Option<String>,
    pub printer: Option<String>,
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
    pub async fn update_metadata(&self, id: &str, filename: Option<String>, printer: Option<String>, file_size: Option<usize>) {
        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.get_mut(id) {
            if let Some(f) = filename {
                job.filename = Some(f);
            }
            if let Some(p) = printer {
                job.printer = Some(p);
            }
            if let Some(s) = file_size {
                job.file_size = Some(s);
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
}
