# Folders and upload logs

FileBase organizes files using the `folder` configured on an upload preset. The dashboard now turns those paths into a folder browser and provides a project-wide activity log for troubleshooting uploads.

## Browse folders

Open **Dashboard → Files** to see folders collected from upload presets and uploaded files. Parent folders include the file count and storage used by all of their descendants.

Selecting a folder filters the file list to that path and every nested path. For example, selecting `users` includes files in both `users` and `users/avatars`.

Folders are logical paths rather than separate storage records. An empty folder appears while at least one upload preset targets it. Configure or rename folder paths from **Dashboard → Upload presets**.

The same information is available from the authenticated API:

```http
GET /folders?project_id=prj_example
Authorization: Bearer <dashboard-token>
```

Filter the files endpoint to a folder subtree:

```http
GET /files?project_id=prj_example&folder=users
Authorization: Bearer <dashboard-token>
```

## Review upload activity

Open **Dashboard → Upload logs** to search activity across projects. Filters are available for project, event, status, date range, and free-text search.

Common events include:

- `file.uploaded`
- `file.deleted`
- `file.duplicate_detected`
- `webhook.file.uploaded`
- `webhook.file.deleted`
- `webhook.file.failed`

Each row includes its timestamp, associated file when available, message, and structured metadata. Deletion logs retain the original filename and path after the file record is removed.

Logs are also available through the authenticated API. Results are newest first, default to 200 rows, and accept a `limit` from 1 to 500.

```http
GET /upload-logs?project_id=prj_example&status=failed&limit=100
Authorization: Bearer <dashboard-token>
```

Supported query parameters are `project_id`, `file_id`, `event`, `status`, `search`, `from`, `to`, and `limit`. Date filters must be RFC 3339 timestamps.
