# Upgrades

This guide covers upgrading a self-hosted FileBase installation safely and recovering if something goes wrong.

## Pre-flight checks

The API exposes an operator-only upgrade check:

```bash
curl http://localhost:8080/admin/upgrade-check \
  -H "Authorization: Bearer <admin-session-token>"
```

The response reports the current version, migration counts, and a list of checks:

| Check                     | What it verifies                                               |
| ------------------------- | -------------------------------------------------------------- |
| `database_migrations`     | Applied and pending migration counts.                          |
| `jwt_secret_strength`     | `JWT_SECRET` is at least 32 characters.                        |
| `encryption_key_strength` | `ENCRYPTION_KEY` is at least 16 characters.                    |
| `local_storage`           | The local upload path is writable and has at least 1 GiB free. |
| `redis`                   | The Redis job queue is reachable.                              |
| `ffmpeg`                  | Video handling binaries are available.                         |
| `transform_cache`         | The dynamic transform cache directory is writable.             |

`upgradeSafe` is `false` only when a check returns `error` (for example, the storage path is not writable). Review warnings before upgrading. The dashboard shows the same output under **Operations → Upgrade safety**.

## Upgrading with the script

The repository ships an upgrade script for installations created by `scripts/install.sh`:

```bash
sudo ./scripts/upgrade.sh --dir /opt/filebase --version 0.2.0
```

The script:

1. Confirms the API is running and its health endpoint responds.
2. Creates a pre-upgrade backup with `scripts/backup.sh`.
3. Pins `FILEBASE_VERSION` in `.env` and pulls the target images.
4. Recreates the stack with `docker compose up -d`.
5. Waits for `/health/ready` and prints rollback instructions.

Useful flags: `--skip-backup` when you already have a recent backup, and `--version latest` to track the newest release.

## Manual upgrade

```bash
cd /opt/filebase
sudo ./scripts/backup.sh --dir /opt/filebase
sed -i 's/^FILEBASE_VERSION=.*/FILEBASE_VERSION=0.2.0/' .env
docker compose pull
docker compose up -d
docker compose logs -f api
```

FileBase applies database migrations automatically at API startup. Pending migrations are safe to apply; the API exits if a migration fails, leaving the old container running.

## Rolling back

If the new version does not start or behaves incorrectly:

```bash
sudo ./scripts/restore.sh /opt/filebase/backups/filebase-backup-<timestamp>.tar.gz --dir /opt/filebase
```

Then pin the previous version and pull it:

```bash
sed -i 's/^FILEBASE_VERSION=.*/FILEBASE_VERSION=<previous>/' /opt/filebase/.env
docker compose -f /opt/filebase/docker-compose.yml pull
docker compose -f /opt/filebase/docker-compose.yml up -d
```

The backup contains the database and local uploads. Object-storage files are not included; back those up through your provider.

## Recommended routine

- Run `upgrade-check` or open **Operations → Upgrade safety** before every upgrade.
- Keep at least one off-host backup before changing versions.
- Upgrade during a quiet period; in-flight uploads are not part of the backup.
- After upgrading, check **Operations → Failed jobs** and retry transient failures.
