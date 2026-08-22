# Hosted Mode

FileBase supports two deployment modes. `self_hosted` is the default and keeps the one-time admin onboarding flow. `hosted` turns the installation into a multi-account gateway where users can register from the dashboard.

## Enable hosted registration

Set these variables on the API and dashboard deployment:

```env
DEPLOYMENT_MODE=hosted
PUBLIC_REGISTRATION_ENABLED=true
```

Restart the services after changing the environment. The public capability endpoint, `GET /setup/status`, reports the active deployment mode and whether registration is enabled. The dashboard uses that response to show or hide account creation.

When a user registers, FileBase creates the account, a starter project owned by that account, and an authenticated session in one transaction. The user is then sent to storage connection setup. Hosted users bring their own FTP, SFTP, S3, or S3-compatible storage credentials.

## Registration API

`POST /auth/register` accepts:

```json
{
  "name": "Ada Lovelace",
  "email": "ada@example.com",
  "password": "a-secure-password",
  "project_name": "Analytical Engine"
}
```

The response includes the session token, public user record, and starter project ID. Registration is covered by the authentication rate limit.

## Close registration

To stop new signups without taking existing accounts offline, set:

```env
PUBLIC_REGISTRATION_ENABLED=false
```

Login and existing hosted accounts continue to work. Both the signup page and registration API are disabled.

## Security boundary

Hosted mode disables `POST /setup/initialize`; it cannot be used to create a global first-run administrator. Projects remain the current tenant boundary, with all files, storage credentials, presets, keys, analytics, and team roles authorized by project membership.

Billing, organization-level tenancy, usage limits, email verification, and hosted abuse controls are separate follow-up milestones and must be added before operating FileBase as a public commercial service.
