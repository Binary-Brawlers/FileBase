# FileBase Implementation Checklist

This checklist turns the full project plan into an ordered implementation path. The goal is to build FileBase from the self-hosted foundation upward, so every later feature has the required installation, database, authentication, and dashboard base.

## Current Next Step

- [x] Implement the backend foundation for first-run setup.

This means the next code milestone should add a real Axum API server, configuration loading, PostgreSQL connection, migrations, and the first setup endpoints:

```http
GET /setup/status
POST /setup/initialize
```

This comes before uploads, SDKs, image optimization, FTP, or SFTP because FileBase needs a reliable install and onboarding path first.

## Phase 1: Self-Hosted Foundation

- [x] Replace the placeholder API binary with a real Axum server.
- [x] Add structured API configuration loading from environment variables.
- [x] Add health endpoints.
- [x] Add PostgreSQL database connection pooling.
- [x] Add Redis connection configuration.
- [x] Add database migration tooling.
- [x] Create initial migrations for users, projects, storage connections, upload presets, API keys, upload sessions, files, webhooks, and upload logs.
- [x] Add consistent API response and error handling types.
- [x] Add tracing/logging setup.
- [x] Add request ID or correlation ID middleware.
- [x] Add CORS configuration for dashboard and SDK usage.
- [x] Update Docker files once the API runs as a real service.

## Phase 2: First-Run Setup And Admin Auth

- [x] Implement `GET /setup/status`.
- [x] Return whether setup is required based on whether any admin user exists.
- [x] Implement `POST /setup/initialize`.
- [x] Create the first admin user.
- [x] Create the default project.
- [x] Create the first storage connection from setup input.
- [x] Create a default upload preset.
- [x] Hash admin passwords securely.
- [x] Prevent setup initialization after the first admin exists.
- [x] Implement login.
- [x] Implement logout/session invalidation behavior.
- [x] Implement `GET /auth/me`.
- [x] Add JWT or secure session handling.
- [x] Add auth middleware for protected routes.
- [x] Disable public registration by default after first setup.

## Phase 3: Dashboard Onboarding UI

- [ ] Replace the placeholder dashboard page with an app shell.
- [ ] Add API client helpers for dashboard-to-API communication.
- [ ] Add first-run setup detection.
- [ ] Show onboarding when setup is required.
- [ ] Build admin registration step.
- [ ] Build initial upload behavior selection step.
- [ ] Build local storage configuration form.
- [ ] Build FTP storage configuration form.
- [ ] Build SFTP storage configuration form.
- [ ] Build default upload preset step.
- [ ] Build setup completion screen with API URL, dashboard URL, and SDK snippet.
- [ ] Add login screen for already configured installs.
- [ ] Add authenticated dashboard layout.

## Phase 4: Storage Connections

- [ ] Finalize storage adapter trait and shared types.
- [ ] Implement local filesystem storage adapter.
- [ ] Implement FTP storage adapter.
- [ ] Implement SFTP storage adapter.
- [ ] Encrypt stored FTP/SFTP credentials with `ENCRYPTION_KEY`.
- [ ] Add storage connection CRUD endpoints.
- [ ] Add `POST /storage-connections/:id/test`.
- [ ] Add dashboard pages for managing storage connections.
- [ ] Allow multiple storage connections per project.
- [ ] Allow users to switch or select storage connections for presets.

## Phase 5: Projects, Presets, And API Keys

- [ ] Implement project CRUD endpoints.
- [ ] Implement upload preset CRUD endpoints.
- [ ] Add preset validation for MIME types, extensions, max file size, duplicate strategy, filename strategy, folder, and transformations.
- [ ] Implement API key creation.
- [ ] Generate API keys with `fb_live_` and `fb_test_` prefixes.
- [ ] Store only API key hashes and prefixes.
- [ ] Add API key revocation.
- [ ] Add dashboard pages for projects.
- [ ] Add dashboard pages for upload presets.
- [ ] Add dashboard pages for API keys.

## Phase 6: Upload Core

- [ ] Implement signed upload session creation.
- [ ] Store upload session tokens as hashes.
- [ ] Enforce token expiration.
- [ ] Enforce project and preset scoping.
- [ ] Implement direct backend upload endpoint.
- [ ] Implement session-based frontend upload endpoint.
- [ ] Add multipart streaming upload handling.
- [ ] Validate file size.
- [ ] Validate MIME type and extension.
- [ ] Add magic byte validation where practical.
- [ ] Generate SHA-256 file hashes.
- [ ] Implement duplicate detection.
- [ ] Implement duplicate strategies: `return_existing`, `upload_new_copy`, and `reject_duplicate`.
- [ ] Implement safe filename generation.
- [ ] Upload files through the selected storage adapter.
- [ ] Save file metadata.
- [ ] Add upload logs.

## Phase 7: Worker And Background Jobs

- [ ] Replace the placeholder worker with a real worker process.
- [ ] Add Redis-backed job queue abstraction.
- [ ] Add `process-image` job.
- [ ] Add `upload-to-storage` job if uploads are processed asynchronously.
- [ ] Add `generate-thumbnail` job.
- [ ] Add `send-webhook` job.
- [ ] Add `delete-file` job.
- [ ] Add `cleanup-temp-file` job.
- [ ] Add retry behavior for failed jobs.
- [ ] Add job logging and failure visibility.

## Phase 8: Image Processing

- [ ] Choose MVP image processing implementation.
- [ ] Add image compression.
- [ ] Add WebP conversion.
- [ ] Add resizing.
- [ ] Add thumbnail generation.
- [ ] Add option to preserve originals.
- [ ] Save transformation metadata.
- [ ] Expose image transformation controls through upload presets.

## Phase 9: Files Dashboard

- [ ] Add files list endpoint.
- [ ] Add file detail endpoint.
- [ ] Add file delete endpoint.
- [ ] Delete files from storage adapter and database safely.
- [ ] Add file listing dashboard page.
- [ ] Add search by filename/path.
- [ ] Add filters by MIME type and date.
- [ ] Add copy URL action.
- [ ] Add file detail drawer or page.
- [ ] Add upload logs view.

## Phase 10: Webhooks

- [ ] Add webhook CRUD endpoints.
- [ ] Add webhook event model.
- [ ] Send `file.uploaded` event.
- [ ] Send `file.deleted` event.
- [ ] Send `file.duplicate_detected` event.
- [ ] Send `file.optimized` event.
- [ ] Send `file.failed` event.
- [ ] Sign webhook payloads with HMAC.
- [ ] Add webhook retry behavior.
- [ ] Add webhook delivery logs.
- [ ] Add dashboard webhook management.

## Phase 11: SDKs

- [ ] Implement `@filebase/client` upload flow.
- [ ] Add signed upload session request support.
- [ ] Add file upload support.
- [ ] Add upload progress support where browser APIs allow it.
- [ ] Add structured SDK errors.
- [ ] Implement `@filebase/react` `useUpload` hook.
- [ ] Implement `@filebase/react` `UploadButton`.
- [ ] Implement `@filebase/react` `UploadDropzone`.
- [ ] Implement `@filebase/next` route helpers.
- [ ] Implement `@filebase/node` admin client.
- [ ] Add React Native SDK in the post-MVP developer-experience phase.
- [ ] Add Vue SDK in the post-MVP developer-experience phase.

## Phase 12: One-Command Installer

- [ ] Create `scripts/install.sh`.
- [ ] Check Linux distribution compatibility.
- [ ] Require root or sudo privileges.
- [ ] Install Docker if missing.
- [ ] Install Docker Compose if missing.
- [ ] Create `/opt/filebase`.
- [ ] Create `/opt/filebase/data/postgres`.
- [ ] Create `/opt/filebase/data/uploads`.
- [ ] Generate secure `JWT_SECRET`.
- [ ] Generate secure `ENCRYPTION_KEY`.
- [ ] Write `/opt/filebase/.env`.
- [ ] Write or download `/opt/filebase/docker-compose.yml`.
- [ ] Pull FileBase images.
- [ ] Start services.
- [ ] Detect server IP.
- [ ] Print first-run setup URL.
- [ ] Add uninstall or upgrade notes.

## Phase 13: Docker And Release Infrastructure

- [ ] Finalize production Dockerfiles.
- [ ] Ensure dashboard can reach API correctly in Docker Compose.
- [ ] Add persistent volumes for PostgreSQL and local uploads.
- [ ] Add image build CI.
- [ ] Publish API image.
- [ ] Publish worker image.
- [ ] Publish dashboard image.
- [ ] Publish release compose file.
- [ ] Add version tags.
- [ ] Add release notes automation.

## Phase 14: Documentation

- [ ] Write quick-start install guide.
- [ ] Write manual Docker Compose guide.
- [ ] Write first-run onboarding guide.
- [ ] Write local storage guide.
- [ ] Write FTP storage guide.
- [ ] Write SFTP storage guide.
- [ ] Write API key guide.
- [ ] Write upload preset guide.
- [ ] Write signed upload guide.
- [ ] Write SDK guides.
- [ ] Write troubleshooting guide.
- [ ] Write security recommendations.

## Phase 15: Hardening Before Public MVP

- [ ] Add rate limiting for auth and upload endpoints.
- [ ] Add request body size limits.
- [ ] Add secure default headers.
- [ ] Add audit-worthy logging for sensitive actions.
- [ ] Ensure credentials are never returned by API responses.
- [ ] Ensure secrets are not logged.
- [ ] Add integration tests for setup flow.
- [ ] Add integration tests for auth flow.
- [ ] Add integration tests for local uploads.
- [ ] Add integration tests for duplicate detection.
- [ ] Add basic load testing for upload endpoints.
- [ ] Review Docker defaults for production safety.

## Phase 16: Post-MVP Expansion

- [ ] Add React Native SDK.
- [ ] Add Vue SDK.
- [ ] Add folder management.
- [ ] Add improved upload logs.
- [ ] Add S3 adapter.
- [ ] Add Cloudflare R2 adapter.
- [ ] Add DigitalOcean Spaces adapter.
- [ ] Add Backblaze B2 adapter.
- [ ] Add Wasabi adapter.
- [ ] Add advanced analytics.
- [ ] Add team accounts.
- [ ] Add role-based permissions.
- [ ] Add hosted SaaS mode later.

## Implementation Rule

Build in dependency order. Do not start SDKs, advanced dashboard screens, or image processing until setup, authentication, storage connections, upload presets, and the basic upload path are working end to end.
