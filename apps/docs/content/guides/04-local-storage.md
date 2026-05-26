# Local Storage

Local storage writes uploaded files directly to a directory on the FileBase server. It is the fastest and most reliable adapter, and is the recommended choice when FileBase runs on the same host that serves your website.

## When to use it

- FileBase and your website live on the same VPS.
- You can configure your web server (Nginx, Apache, Caddy) to serve files from a chosen directory.
- You want low latency and no extra network hop.

## Configuration fields

Create a Local storage connection from the dashboard or `POST /storage-connections`:

| Field | Description |
| --- | --- |
| `type` | `local` |
| `basePath` | Absolute path on the FileBase server where files are written, e.g. `/var/lib/filebase/uploads`. Inside Docker this is `/var/lib/filebase/uploads` by default. |
| `publicBaseUrl` | Public URL prefix where the same files are served, e.g. `https://example.com/uploads`. |

The Docker Compose stack mounts `uploads_data` at `/var/lib/filebase/uploads` for both the API and worker containers. If you change `basePath`, change the volume mount accordingly.

## Serving files

FileBase does **not** serve files itself by default. Point your web server or CDN at `basePath` and expose it at `publicBaseUrl`. Example with Nginx:

```nginx
location /uploads/ {
    alias /var/lib/filebase/uploads/;
    autoindex off;
    add_header Cache-Control "public, max-age=31536000, immutable";
}
```

## Permissions

The FileBase containers run as a non-root user. Make sure the host directory backing the volume is writable by that user. If you use a bind mount instead of the named volume:

```bash
sudo mkdir -p /var/lib/filebase/uploads
sudo chown -R 1000:1000 /var/lib/filebase/uploads
```

## Testing the connection

The dashboard runs a write/read/delete round-trip when you click **Test connection**, which calls `POST /storage-connections/:id/test`. If the test fails, check that:

- `basePath` exists and is writable.
- The container user has permission to traverse every parent directory.
- The mount is consistent between the `api` and `worker` services.
