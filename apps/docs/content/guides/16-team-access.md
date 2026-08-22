# Team Access and Project Roles

FileBase projects are the boundary for team access. Every project has one owner and can include admins, editors, and viewers without giving those users access to unrelated projects.

## Roles

| Role   | Access                                                                                               |
| ------ | ---------------------------------------------------------------------------------------------------- |
| Owner  | Full project control, including member management and project deletion.                              |
| Admin  | Viewer and editor access, plus project settings, storage connections, API keys, and team management. |
| Editor | Read access plus file deletion, upload preset management, and webhook management.                    |
| Viewer | Read-only access to project files, folders, logs, analytics, and configuration.                      |

Project ownership cannot be assigned through the role selector. This prevents an admin from replacing or removing the owner accidentally.

## Invite a teammate

1. Open **Dashboard → Team access**.
2. Select a project where you are an owner or admin.
3. Choose **Invite teammate**.
4. Enter the teammate's email and select `viewer`, `editor`, or `admin`.
5. Copy the generated invite link and send it through a trusted channel.

FileBase shows the invite link once. The token is placed in the URL fragment so it is not sent to the dashboard server, and FileBase stores only its SHA-256 hash. The invitation expires after seven days. Creating another active invitation for the same email and project is blocked; revoke the existing invitation first if you need a replacement.

FileBase does not send invitation email itself. This keeps the self-hosted installation independent of an email provider.

## Accept an invitation

The link opens the invitation acceptance page:

- A new user enters their name and creates a password of at least eight characters.
- A user whose email already exists enters their current FileBase password to confirm ownership of that account.

After acceptance, FileBase creates the project membership, marks the invitation as used, signs the user in, and opens the dashboard. An invitation cannot be reused.

## Manage access

Owners and admins can change non-owner members between admin, editor, and viewer roles or remove them from a project. Changes apply immediately to API authorization and dashboard controls.

Relevant API endpoints:

- `GET /projects/:id/members`
- `PATCH /projects/:id/members/:user_id`
- `DELETE /projects/:id/members/:user_id`
- `GET /projects/:id/invitations`
- `POST /projects/:id/invitations`
- `DELETE /projects/:id/invitations/:invitation_id`
- `POST /team-invitations/preview`
- `POST /team-invitations/accept`

The preview and acceptance endpoints accept invitation tokens in JSON request bodies so reverse-proxy access logs do not receive the token as part of the URL.

## Existing installations

The database migration automatically adds an `owner` membership for the current owner of every existing project. No manual backfill is required.
