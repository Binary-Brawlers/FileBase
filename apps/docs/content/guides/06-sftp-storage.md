# SFTP Storage

SFTP storage uploads files over SSH. It is the recommended remote storage option whenever the destination host supports SSH access.

## When to use it

- Your destination server runs `sshd` (most VPS hosts).
- You want encrypted transfers and key-based authentication.
- You need stronger guarantees than plain FTP can provide.

## Configuration fields

Create an SFTP connection from the dashboard or `POST /storage-connections`:

| Field | Description |
| --- | --- |
| `type` | `sftp` |
| `host` | SSH hostname, e.g. `example.com`. |
| `port` | Usually `22`. |
| `username` | SSH user. |
| `password` | Optional. Used if no private key is provided. Encrypted at rest. |
| `privateKey` | Optional PEM-encoded private key. Encrypted at rest. |
| `basePath` | Absolute remote path, e.g. `/var/www/example.com/public/uploads`. |
| `publicBaseUrl` | Public URL prefix, e.g. `https://example.com/uploads`. |

Provide either `password` or `privateKey`. Keys are preferred. Both fields are encrypted with AES-256-GCM using `ENCRYPTION_KEY` and are never returned by the API after creation.

## Generating an SFTP key for FileBase

On the FileBase host:

```bash
ssh-keygen -t ed25519 -f filebase_sftp -N ""
```

Copy `filebase_sftp.pub` to the destination server:

```bash
ssh-copy-id -i filebase_sftp.pub user@example.com
```

Paste the contents of the **private** key (`filebase_sftp`) into the dashboard form. Delete the local copy of the private key once it is saved in FileBase.

## Permissions

The SSH user needs read/write access to `basePath`. Files inherit the user's umask; if your web server runs as a different user, ensure it can read the directory:

```bash
sudo chmod 755 /var/www/example.com/public/uploads
```

## Test the connection

`POST /storage-connections/:id/test` opens an SFTP session, writes a temporary file, reads it back, and deletes it. Common failure causes:

| Error | Likely cause |
| --- | --- |
| `Permission denied (publickey,password)` | Wrong user, key, or password. Verify with `ssh user@host` from the FileBase host. |
| `Failed to open file for writing` | The user cannot write to `basePath`. |
| `Host key verification failed` | First connection — FileBase will record the host key. Re-test once allowed. |

## Host key trust

FileBase records the host key on first successful connection and rejects later connections if it changes. If the destination server's key is rotated intentionally, delete and recreate the storage connection.
