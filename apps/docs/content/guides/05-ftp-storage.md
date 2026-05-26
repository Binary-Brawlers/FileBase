# FTP Storage

FTP is the original use case for FileBase: you have an existing FTP server (often shared hosting or cPanel) and want a modern upload flow in front of it.

## When to use it

- Your final files must live on a remote FTP server you do not fully control.
- You are running FileBase on a separate VPS or container host.
- You cannot or do not want to give frontend apps FTP credentials directly.

## Configuration fields

Create an FTP connection from the dashboard or `POST /storage-connections`:

| Field | Description |
| --- | --- |
| `type` | `ftp` |
| `host` | FTP hostname, e.g. `ftp.example.com`. |
| `port` | Usually `21`. |
| `username` | FTP user. |
| `password` | FTP password. Encrypted at rest with AES-256-GCM using `ENCRYPTION_KEY`. |
| `basePath` | Remote directory where files are written, e.g. `/public_html/uploads`. |
| `publicBaseUrl` | Public URL prefix that maps to `basePath`, e.g. `https://example.com/uploads`. |

The password is never returned by API responses after creation. To rotate it, edit the connection and supply a new password.

## Permissions

The FTP user needs:

- `LIST` and `CWD` on every directory in `basePath`.
- `STOR` on the destination directory.
- `MKD` if FileBase needs to create subfolders for date-based or preset-based paths.
- `DELE` for file deletion.

## Test the connection

`POST /storage-connections/:id/test` connects, writes a small temporary file, reads it back, and deletes it. The dashboard wires this to the **Test connection** button.

Common errors:

| Error | Likely cause |
| --- | --- |
| `530 Login incorrect` | Wrong username or password. |
| `550 Permission denied` | The FTP user cannot write to `basePath`. |
| Connection timeout | Firewall or passive-mode port range blocked on the FTP host. |

## Passive mode and firewalls

FileBase uses passive-mode FTP. The FTP server must allow inbound connections on the passive port range it advertises. If FileBase runs in Docker, no special host-side configuration is needed for outbound connections, but the FTP server must accept connections from the FileBase host IP.

## Security note

Plain FTP transfers credentials and data in cleartext. Prefer SFTP whenever the destination host supports it. See [SFTP Storage](./06-sftp-storage.md) and [Security Recommendations](./12-security.md).
