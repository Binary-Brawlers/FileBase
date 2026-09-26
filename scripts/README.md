# Scripts

Repository automation scripts belong here.

## install.sh

One-command FileBase installer for Linux servers. Installs Docker if missing,
creates `/opt/filebase`, generates a `.env` with secure secrets, writes a
production `docker-compose.yml`, pulls images, and starts the stack.

```bash
curl -fsSL https://get.filebase.dev/install.sh | sudo bash
```

Flags: `--dir`, `--version`, `--http-port`, `--dashboard-port`, `--compose-url`.
See the header of [`install.sh`](install.sh) for the full reference.

## backup.sh

Creates a compressed archive of the PostgreSQL database and local uploads for a
Docker Compose installation.

```bash
sudo ./scripts/backup.sh --dir /opt/filebase --output /opt/filebase/backups
```

## restore.sh

Restores a backup archive, stopping the API, worker, and dashboard while the
database and uploads are replaced.

```bash
sudo ./scripts/restore.sh /opt/filebase/backups/filebase-backup-<timestamp>.tar.gz --dir /opt/filebase
```

## upgrade.sh

Upgrades a Docker Compose installation with a pre-upgrade backup, image pull,
and health check.

```bash
sudo ./scripts/upgrade.sh --dir /opt/filebase --version 0.2.0
```

## integration-phase15.sh

End-to-end smoke test covering setup, auth, API keys, and duplicate detection.

## load-upload.sh

Concurrent upload smoke test against a running installation.
