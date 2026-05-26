use std::time::Duration;

use filebase_core::jobs::{JobEnvelope, JobKind, JobQueue, ReservedJob};
use hmac::{Hmac, Mac};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, Statement};
use serde_json::{json, Value};
use sha2::Sha256;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let redis_url = std::env::var("REDIS_URL")?;
    let database_url = std::env::var("DATABASE_URL")?;
    let db = Database::connect(&database_url).await?;
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
                    Ok(Some(reserved)) => handle_reserved(&queue, &db, reserved).await,
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

async fn handle_reserved(queue: &JobQueue, db: &DatabaseConnection, reserved: ReservedJob) {
    let job = reserved.job.clone();
    tracing::info!(job_id = %job.id, job_kind = ?job.kind, attempt = job.attempts + 1, "processing job");

    match process_job(&job, db).await {
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

async fn process_job(job: &JobEnvelope, db: &DatabaseConnection) -> anyhow::Result<()> {
    match job.kind {
        JobKind::ProcessImage => process_image(job).await,
        JobKind::UploadToStorage => upload_to_storage(job).await,
        JobKind::GenerateThumbnail => generate_thumbnail(job).await,
        JobKind::SendWebhook => send_webhook(job, db).await,
        JobKind::DeleteFile => delete_file(job).await,
        JobKind::CleanupTempFile => cleanup_temp_file(job).await,
    }
}

async fn process_image(job: &JobEnvelope) -> anyhow::Result<()> {
    tracing::info!(job_id = %job.id, "process-image job accepted");
    Ok(())
}

async fn upload_to_storage(job: &JobEnvelope) -> anyhow::Result<()> {
    tracing::info!(job_id = %job.id, "upload-to-storage job accepted");
    Ok(())
}

async fn generate_thumbnail(job: &JobEnvelope) -> anyhow::Result<()> {
    tracing::info!(job_id = %job.id, "generate-thumbnail job accepted");
    Ok(())
}

async fn send_webhook(job: &JobEnvelope, db: &DatabaseConnection) -> anyhow::Result<()> {
    let url = required_payload_str(job, "url")?;
    let secret = required_payload_str(job, "secret")?;
    let event = required_payload_str(job, "event")?;
    let webhook_id = required_payload_str(job, "webhookId")?;
    let project_id = required_payload_str(job, "projectId")?;
    let file_id = job
        .payload
        .get("fileId")
        .and_then(|value| value.as_str())
        .map(str::to_string);
    let payload = webhook_payload(job);
    let body = serde_json::to_vec(&payload)?;
    let signature = sign_payload(&secret, &body)?;
    let client = reqwest::Client::new();
    let result = client
        .post(&url)
        .header("content-type", "application/json")
        .header("user-agent", "FileBase-Webhook/0.1")
        .header("x-filebase-event", &event)
        .header("x-filebase-delivery", &job.id)
        .header("x-filebase-signature", format!("sha256={signature}"))
        .body(body)
        .send()
        .await;

    let response = match result {
        Ok(response) => response,
        Err(error) => {
            record_delivery(
                db,
                DeliveryRecord {
                    job,
                    webhook_id: &webhook_id,
                    project_id: &project_id,
                    file_id: file_id.as_deref(),
                    event: &event,
                    url: &url,
                    status: "failed",
                    status_code: None,
                    error: Some(error.to_string()),
                    response_body: None,
                },
            )
            .await?;
            return Err(error.into());
        }
    };

    let status = response.status();
    let status_code = i32::from(status.as_u16());
    let response_body = response.text().await.ok();
    if !status.is_success() {
        record_delivery(
            db,
            DeliveryRecord {
                job,
                webhook_id: &webhook_id,
                project_id: &project_id,
                file_id: file_id.as_deref(),
                event: &event,
                url: &url,
                status: "failed",
                status_code: Some(status_code),
                error: Some(format!("webhook delivery failed with status {status}")),
                response_body,
            },
        )
        .await?;
        anyhow::bail!("webhook delivery failed with status {status}");
    }

    record_delivery(
        db,
        DeliveryRecord {
            job,
            webhook_id: &webhook_id,
            project_id: &project_id,
            file_id: file_id.as_deref(),
            event: &event,
            url: &url,
            status: "delivered",
            status_code: Some(status_code),
            error: None,
            response_body,
        },
    )
    .await?;

    tracing::info!(job_id = %job.id, url = %url, status = %status, "webhook delivered");
    Ok(())
}

struct DeliveryRecord<'a> {
    job: &'a JobEnvelope,
    webhook_id: &'a str,
    project_id: &'a str,
    file_id: Option<&'a str>,
    event: &'a str,
    url: &'a str,
    status: &'a str,
    status_code: Option<i32>,
    error: Option<String>,
    response_body: Option<String>,
}

async fn record_delivery(
    db: &DatabaseConnection,
    record: DeliveryRecord<'_>,
) -> anyhow::Result<()> {
    let request_json = json!({
        "jobId": record.job.id,
        "url": record.url,
        "payload": webhook_payload(record.job),
    });
    let response_json = json!({ "body": record.response_body });
    let file_id = record.file_id.map(str::to_string);
    let stmt = Statement::from_sql_and_values(
        db.get_database_backend(),
        r#"
        INSERT INTO webhook_delivery_logs
          (id, webhook_id, project_id, file_id, event, status, attempt, status_code, error, request_json, response_json, created_at)
        VALUES
          ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        "#,
        [
            format!("whdl_{}", Uuid::new_v4().simple()).into(),
            record.webhook_id.into(),
            record.project_id.into(),
            file_id.into(),
            record.event.into(),
            record.status.into(),
            i32::try_from(record.job.attempts + 1)
                .unwrap_or(i32::MAX)
                .into(),
            record.status_code.into(),
            record.error.into(),
            request_json.into(),
            response_json.into(),
            chrono::Utc::now().into(),
        ],
    );
    db.execute(stmt).await?;
    Ok(())
}

fn required_payload_str(job: &JobEnvelope, key: &str) -> anyhow::Result<String> {
    job.payload
        .get(key)
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .ok_or_else(|| anyhow::anyhow!("send-webhook job missing payload field: {key}"))
}

fn sign_payload(secret: &str, body: &[u8]) -> anyhow::Result<String> {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())?;
    mac.update(body);
    Ok(mac
        .finalize()
        .into_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

async fn delete_file(job: &JobEnvelope) -> anyhow::Result<()> {
    tracing::info!(job_id = %job.id, "delete-file job accepted");
    Ok(())
}

async fn cleanup_temp_file(job: &JobEnvelope) -> anyhow::Result<()> {
    tracing::info!(job_id = %job.id, "cleanup-temp-file job accepted");
    Ok(())
}

fn webhook_payload(job: &JobEnvelope) -> Value {
    let mut payload = job.payload.clone();
    if let Some(object) = payload.as_object_mut() {
        object.remove("secret");
    }
    payload
}

fn init_tracing() {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .init();
}
