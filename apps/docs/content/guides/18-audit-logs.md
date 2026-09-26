# Audit Logs

FileBase persists an audit trail of administrative, authentication, API key, upload, and webhook activity. Audit entries are stored in the `audit_logs` table and survive application restarts.

## What is recorded

| Category       | Example actions                                                                                                                                                    |
| -------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Authentication | `auth.login_succeeded`, `auth.login_failed`, `auth.logout`                                                                                                         |
| Setup          | `setup.initialized`                                                                                                                                                |
| Projects/team  | `project.created`, `project.updated`, `project.deleted`, `project_member.updated`, `project_member.removed`, `project_invitation.*`, `project_invitation.accepted` |
| API keys       | `api_key.created`, `api_key.revoked`                                                                                                                               |
| Presets        | `upload_preset.created`, `upload_preset.updated`, `upload_preset.deleted`                                                                                          |
| Files          | `file.deleted`                                                                                                                                                     |
| Storage        | `storage_connection.created`, `storage_connection.updated`, `storage_connection.deleted`, `storage_connection.test_*`                                              |
| Uploads        | `upload_session.created`, `upload.direct_succeeded`, `upload.session_succeeded`, `upload.chunked_succeeded`, `upload.chunked_failed`                               |
| Webhooks       | `webhook.created`, `webhook.updated`, `webhook.deleted`                                                                                                            |
| Maintenance    | `maintenance.cleanup`                                                                                                                                              |

Each entry stores the actor (`user`, `api_key`, or `system`), actor id and email when known, project, action, resource type and id, status, IP address, user agent, metadata, and timestamp. Secrets, passwords, tokens, and signing keys are never written to audit metadata.

## Reading audit logs

Owners, admins, editors, and viewers can read audit entries for projects they can access, plus their own account activity such as sign-ins.

```bash
curl "http://localhost:8080/audit-logs?project_id=prj_123&limit=50" \
  -H "Authorization: Bearer <admin-session-token>"
```

| Query parameter | Description                                          |
| --------------- | ---------------------------------------------------- |
| `project_id`    | Scope to one accessible project.                     |
| `actor_id`      | Filter by actor id.                                  |
| `action`        | Exact action match, for example `auth.login_failed`. |
| `status`        | `success` or `failure`.                              |
| `from` / `to`   | RFC3339 timestamps.                                  |
| `limit`         | 1–200, defaults to 50.                               |
| `offset`        | Pagination offset.                                   |

The dashboard page **Audit logs** provides the same filters with metadata inspection.

## Retention and pruning

Audit rows are retained indefinitely by default. To prune old entries, use the maintenance control:

```bash
curl -X POST http://localhost:8080/admin/maintenance/cleanup \
  -H "Authorization: Bearer <admin-session-token>" \
  -H "Content-Type: application/json" \
  -d '{ "scope": "audit_logs", "older_than_days": 90 }'
```

The same action is available under **Dashboard → Operations → Audit log retention** (90 days). Maintenance requires owner or admin access on at least one project.
