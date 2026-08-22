# Analytics

FileBase analytics provide an operational view of current storage and historical upload activity. Open **Dashboard → Analytics** to review all projects or select one project.

## Metrics

The analytics workspace includes:

- Current file count and storage used.
- Project and logical folder totals.
- Upload count, uploaded bytes, average file size, duplicate events, and failure events for the selected window.
- Daily upload volume for 7, 30, or 90 days.
- MIME type, storage adapter, and folder breakdowns.
- Upload and webhook log outcomes.

Current totals reflect files that still exist in FileBase. Upload trends are derived from upload logs, so deleting a file does not remove its historical upload event.

## API

The authenticated analytics endpoint returns all metrics in one response:

```http
GET /analytics/summary?project_id=prj_example&from=2026-08-01T00:00:00Z&to=2026-08-31T23:59:59Z
Authorization: Bearer <dashboard-token>
```

All query parameters are optional. Without dates, the endpoint uses the latest 30 days. Ranges are inclusive and limited to 366 days. Dates must be RFC 3339 timestamps.

The response contains:

- `totals` for current files, bytes, projects, and folder paths.
- `period` for upload and outcome metrics in the requested window.
- `trend` with one entry per day, including zero-activity days.
- `mime_types`, `storage_types`, and `folders` ranked by upload count.
- `outcomes` aggregated from upload and webhook logs.

New upload logs include file size, MIME type, folder, and storage connection metadata. Older retained logs fall back to the current file record for byte totals when that file still exists.
