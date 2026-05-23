use std::time::Duration;

use filebase_core::jobs::{JobEnvelope, JobKind, JobQueue, ReservedJob};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let redis_url = std::env::var("REDIS_URL")?;
    let queue = JobQueue::new(redis::Client::open(redis_url.as_str())?);
    tracing::info!("starting FileBase worker");

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("shutdown signal received");
                break;
            }
            result = queue.reserve(5) => {
                match result {
                    Ok(Some(reserved)) => handle_reserved(&queue, reserved).await,
                    Ok(None) => {}
                    Err(error) => {
                        tracing::error!(error = %error, "failed to reserve job");
                        tokio::time::sleep(Duration::from_secs(2)).await;
                    }
                }
            }
        }
    }

    Ok(())
}

async fn handle_reserved(queue: &JobQueue, reserved: ReservedJob) {
    let job = reserved.job.clone();
    tracing::info!(job_id = %job.id, job_kind = ?job.kind, attempt = job.attempts + 1, "processing job");

    match process_job(&job).await {
        Ok(()) => match queue.complete(&reserved).await {
            Ok(()) => tracing::info!(job_id = %job.id, "job completed"),
            Err(error) => {
                tracing::error!(job_id = %job.id, error = %error, "failed to acknowledge job")
            }
        },
        Err(error) => {
            let message = error.to_string();
            if job.attempts + 1 < job.max_attempts {
                match queue.retry(reserved).await {
                    Ok(requeued) => {
                        tracing::warn!(job_id = %requeued.id, attempts = requeued.attempts, error = %message, "job requeued")
                    }
                    Err(queue_error) => {
                        tracing::error!(job_id = %job.id, error = %queue_error, "failed to requeue job")
                    }
                }
            } else {
                match queue.fail(reserved, message.clone()).await {
                    Ok(failed) => {
                        tracing::error!(job_id = %failed.job.id, error = %message, "job moved to failed queue")
                    }
                    Err(queue_error) => {
                        tracing::error!(job_id = %job.id, error = %queue_error, "failed to move job to failed queue")
                    }
                }
            }
        }
    }
}

async fn process_job(job: &JobEnvelope) -> anyhow::Result<()> {
    match job.kind {
        JobKind::ProcessImage => process_image(job).await,
        JobKind::UploadToStorage => upload_to_storage(job).await,
        JobKind::GenerateThumbnail => generate_thumbnail(job).await,
        JobKind::SendWebhook => send_webhook(job).await,
        JobKind::DeleteFile => delete_file(job).await,
        JobKind::CleanupTempFile => cleanup_temp_file(job).await,
    }
}

async fn process_image(job: &JobEnvelope) -> anyhow::Result<()> {
    tracing::info!(job_id = %job.id, payload = %job.payload, "process-image job accepted");
    Ok(())
}

async fn upload_to_storage(job: &JobEnvelope) -> anyhow::Result<()> {
    tracing::info!(job_id = %job.id, payload = %job.payload, "upload-to-storage job accepted");
    Ok(())
}

async fn generate_thumbnail(job: &JobEnvelope) -> anyhow::Result<()> {
    tracing::info!(job_id = %job.id, payload = %job.payload, "generate-thumbnail job accepted");
    Ok(())
}

async fn send_webhook(job: &JobEnvelope) -> anyhow::Result<()> {
    tracing::info!(job_id = %job.id, payload = %job.payload, "send-webhook job accepted");
    Ok(())
}

async fn delete_file(job: &JobEnvelope) -> anyhow::Result<()> {
    tracing::info!(job_id = %job.id, payload = %job.payload, "delete-file job accepted");
    Ok(())
}

async fn cleanup_temp_file(job: &JobEnvelope) -> anyhow::Result<()> {
    tracing::info!(job_id = %job.id, payload = %job.payload, "cleanup-temp-file job accepted");
    Ok(())
}

fn init_tracing() {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .init();
}
