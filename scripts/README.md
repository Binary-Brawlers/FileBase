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
