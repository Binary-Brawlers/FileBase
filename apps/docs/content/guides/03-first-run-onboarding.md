# First-Run Onboarding

The first time you open the FileBase dashboard, it detects that no admin user exists and runs you through a guided setup. This guide describes each step so you know what to prepare.

## Detect setup state

The dashboard calls `GET /setup/status` on load. If `setup_required` is `true`, you are routed into the onboarding flow instead of the login screen. Once an admin user exists this endpoint returns `false` and the setup endpoints refuse to run again.

You can check from the command line:

```bash
curl http://SERVER_IP:8080/setup/status
```

## Step 1 — Create the admin account

Provide a name, email, and password. The password is hashed with bcrypt and stored as a `passwordHash`. Public registration is disabled by default after this step.

## Step 2 — Choose the initial upload behavior

Pick one of:

- **Local server folder** — files are written to a directory on the FileBase server.
- **FTP server** — files are uploaded over plain FTP/FTPS.
- **SFTP server** — files are uploaded over SSH.

The choice is not permanent. You can add, edit, or switch storage connections later. See [Storage Connections](./04-local-storage.md).

## Step 3 — Configure the first storage connection

Depending on your choice in step 2 you will fill out one of:

- [Local storage form](./04-local-storage.md)
- [FTP storage form](./05-ftp-storage.md)
- [SFTP storage form](./06-sftp-storage.md)

The dashboard calls `POST /storage-connections/:id/test` before saving to confirm credentials work.

## Step 4 — Create the default upload preset

A preset is a reusable upload rule (allowed MIME types, max size, folder, duplicate strategy, transformations). The default preset is created so your first SDK upload works without extra configuration. You can edit or add more presets later. See [Upload Presets](./08-upload-presets.md).

## Step 5 — Completion screen

The final screen shows:

- The API URL your apps should call
- The dashboard URL for ongoing administration
- A copy-pasteable SDK snippet to start uploading immediately

After this you land on the authenticated dashboard.

## Re-running setup

You cannot re-run setup against a live database. To reset onboarding, drop the `users` table or restore an empty database, then refresh the dashboard.
