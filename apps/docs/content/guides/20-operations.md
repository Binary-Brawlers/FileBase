# Operations: Diagnostics, Maintenance, and Backups

This guide covers day-two operations for self-hosted FileBase installations: health diagnostics, maintenance controls, failed job recovery, and backup/restore tooling.

Access to diagnostics and maintenance requires an owner or admin role on at least one project. Every maintenance action is recorded in the audit log.

## Diagnostics

`GET /admin/diagnostics` returns a snapshot of the installation:

```bash
curl http://localhost:8080/admin/diagnostics \
  -H "Authorization: Bearer <admin-session-token>"
```

The response includes:

- **Database** — connectivity, database size, user/project/file counts, and total file bytes.
- **Redis** — connectivity and pending, processing, and failed job counts.
- **Storage** — local storage path, file and byte counts, temp file usage, chunk upload usage, and open upload sessions.
- **Media tooling** — whether `ffmpeg` and `ffprobe` are available for video processing.
- **Runtime** — API version, uptime, and configured limits.

The dashboard page **Operations** shows the same data with a 30-second refresh, plus maintenance and failed job controls.

## Maintenance controls

```bash
curl -X POST http://localhost:8080/admin/maintenance/cleanup \
  -H "Authorization: Bearer <admin-session-token>" \
  -H "Content-Type: application/json" \
  -d '{ "scope": "all", "older_than_hours": 1, "older_than_days": 90 }'
```

| Scope        | Action                                                                 |
| ------------ | ---------------------------------------------------------------------- |
| `temp`       | Removes uploaded temp files older than `older_than_hours` (default 1). |
| `sessions`   | Deletes expired, unused upload sessions and their chunks.              |
| `chunks`     | Removes orphaned chunk rows, files, and directories.                   |
| `audit_logs` | Deletes audit entries older than `older_than_days` (default 90).       |
| `all`        | Runs `temp`, `sessions`, and `chunks`.                                 |

## Failed jobs

Background jobs that exhaust their retry budget move to the failed queue.

| Endpoint                          | Description                           |
| --------------------------------- | ------------------------------------- |
| `GET /admin/jobs/failed?limit=50` | Lists failed jobs with error details. |
| `POST /admin/jobs/:job_id/retry`  | Resets attempts and requeues the job. |
| `DELETE /admin/jobs/:job_id`      | Deletes a failed job.                 |

The dashboard **Operations** page exposes retry and delete actions for each failed job.

## Backup and restore

The repository ships two scripts that operate on Docker Compose installations created by `scripts/install.sh`.

### Backup

```bash
sudo ./scripts/backup.sh --dir /opt/filebase --output /opt/filebase/backups
```

The script:

1. Runs `pg_dump --clean --if-exists` inside the Postgres container.
2. Archives `/opt/filebase/data/uploads`.
3. Writes a `manifest.json` with the creation time, hostname, directory, and version.
4. Produces `/opt/filebase/backups/filebase-backup-<timestamp>.tar.gz`.

For consistent backups, run the script during a quiet period or scale down the API and worker first. Uploads currently in flight may not be included.

### Restore

```bash
sudo ./scripts/restore.sh /opt/filebase/backups/filebase-backup-<timestamp>.tar.gz --dir /opt/filebase
```

The script stops the API, worker, and dashboard, restores the database dump and uploads, then starts the stack again. Redis is intentionally left untouched. Pass `--yes` for non-interactive restores.

### Manual verification after restore

```bash
docker compose -f /opt/filebase/docker-compose.yml ps
curl -fsS http://localhost:8080/health/ready
```

Sign in to the dashboard and confirm that recent files and settings are present.

## Recommended routine

- Run `backup.sh` on a schedule (for example, nightly cron) and copy archives off-host.
- Check **Operations → Failed jobs** after deploys and retry transient failures.
- Run the `temp` and `sessions` cleanup monthly to reclaim disk space.
- Set an audit retention window that matches your compliance requirements.
