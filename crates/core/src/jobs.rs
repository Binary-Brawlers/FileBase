use chrono::Utc;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::FileBaseResult;

const PENDING_QUEUE: &str = "filebase:jobs:pending";
const PROCESSING_QUEUE: &str = "filebase:jobs:processing";
const FAILED_QUEUE: &str = "filebase:jobs:failed";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum JobKind {
    ProcessImage,
    UploadToStorage,
    GenerateThumbnail,
    SendWebhook,
    DeleteFile,
    CleanupTempFile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobEnvelope {
    pub id: String,
    pub kind: JobKind,
    pub payload: Value,
    pub attempts: u32,
    pub max_attempts: u32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedJob {
    pub job: JobEnvelope,
    pub error: String,
    pub failed_at: String,
}

impl JobEnvelope {
    pub fn new(kind: JobKind, payload: Value) -> Self {
        let now = Utc::now().to_rfc3339();
        Self {
            id: format!("job_{}", Uuid::new_v4().simple()),
            kind,
            payload,
            attempts: 0,
            max_attempts: 3,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    pub fn with_max_attempts(mut self, max_attempts: u32) -> Self {
        self.max_attempts = max_attempts.max(1);
        self
    }
}

#[derive(Clone)]
pub struct JobQueue {
    client: redis::Client,
}

impl JobQueue {
    pub fn new(client: redis::Client) -> Self {
        Self { client }
    }

    pub async fn enqueue(&self, job: &JobEnvelope) -> FileBaseResult<()> {
        let mut connection = self.client.get_multiplexed_async_connection().await?;
        let raw =
            serde_json::to_string(job).map_err(|e| crate::FileBaseError::Queue(e.to_string()))?;
        let _: usize = connection.lpush(PENDING_QUEUE, raw).await?;
        Ok(())
    }

    pub async fn reserve(&self, timeout_seconds: usize) -> FileBaseResult<Option<ReservedJob>> {
        let mut connection = self.client.get_multiplexed_async_connection().await?;
        let raw: Option<String> = redis::cmd("BRPOPLPUSH")
            .arg(PENDING_QUEUE)
            .arg(PROCESSING_QUEUE)
            .arg(timeout_seconds)
            .query_async(&mut connection)
            .await?;
        let Some(raw) = raw else {
            return Ok(None);
        };
        let job =
            serde_json::from_str(&raw).map_err(|e| crate::FileBaseError::Queue(e.to_string()))?;
        Ok(Some(ReservedJob { raw, job }))
    }

    pub async fn complete(&self, reserved: &ReservedJob) -> FileBaseResult<()> {
        self.remove_processing(&reserved.raw).await
    }

    pub async fn retry(&self, reserved: ReservedJob) -> FileBaseResult<JobEnvelope> {
        let mut job = reserved.job;
        job.attempts += 1;
        job.updated_at = Utc::now().to_rfc3339();
        let raw =
            serde_json::to_string(&job).map_err(|e| crate::FileBaseError::Queue(e.to_string()))?;
        let mut connection = self.client.get_multiplexed_async_connection().await?;
        let _: i64 = redis::cmd("LREM")
            .arg(PROCESSING_QUEUE)
            .arg(1)
            .arg(reserved.raw)
            .query_async(&mut connection)
            .await?;
        let _: usize = connection.lpush(PENDING_QUEUE, raw).await?;
        Ok(job)
    }

    pub async fn fail(&self, reserved: ReservedJob, error: String) -> FileBaseResult<FailedJob> {
        let failed = FailedJob {
            job: reserved.job,
            error,
            failed_at: Utc::now().to_rfc3339(),
        };
        let raw_failed = serde_json::to_string(&failed)
            .map_err(|e| crate::FileBaseError::Queue(e.to_string()))?;
        let mut connection = self.client.get_multiplexed_async_connection().await?;
        let _: i64 = redis::cmd("LREM")
            .arg(PROCESSING_QUEUE)
            .arg(1)
            .arg(reserved.raw)
            .query_async(&mut connection)
            .await?;
        let _: usize = connection.lpush(FAILED_QUEUE, raw_failed).await?;
        Ok(failed)
    }

    async fn remove_processing(&self, raw: &str) -> FileBaseResult<()> {
        let mut connection = self.client.get_multiplexed_async_connection().await?;
        let _: i64 = redis::cmd("LREM")
            .arg(PROCESSING_QUEUE)
            .arg(1)
            .arg(raw)
            .query_async(&mut connection)
            .await?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ReservedJob {
    pub raw: String,
    pub job: JobEnvelope,
}
