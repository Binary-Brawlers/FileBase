# FileBase Implementation Checklist

This checklist turns the full project plan into an ordered implementation path. The goal is to build FileBase from the self-hosted foundation upward, so every later feature has the required installation, database, authentication, and dashboard base.

## Current Next Step

- [x] Implement the backend foundation for first-run setup.
- [x] Implement the dashboard onboarding UI (Phase 3).
- [x] Implement storage adapters, connection management backend, and dashboard (Phase 4).
- [x] Implement MVP SDKs: `@binary-brawlers/filebase-client`, `@binary-brawlers/filebase-react`, `@binary-brawlers/filebase-next`, `@binary-brawlers/filebase-node` (Phase 11).
- [x] Add one-command installer `scripts/install.sh` (Phase 12).
- [x] Write Phase 14 documentation guides.

Next code milestone is Phase 16 post-MVP expansion.

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

- [x] Replace the placeholder dashboard page with an app shell.
- [x] Add API client helpers for dashboard-to-API communication.
- [x] Add first-run setup detection.
- [x] Show onboarding when setup is required.
- [x] Build admin registration step.
- [x] Build initial upload behavior selection step.
- [x] Build local storage configuration form.
- [x] Build FTP storage configuration form.
- [x] Build SFTP storage configuration form.
- [x] Build default upload preset step.
- [x] Build setup completion screen with API URL, dashboard URL, and SDK snippet.
- [x] Add login screen for already configured installs.
- [x] Add authenticated dashboard layout.

## Phase 4: Storage Connections

- [x] Finalize storage adapter trait and shared types.
- [x] Implement local filesystem storage adapter.
- [x] Implement FTP storage adapter.
- [x] Implement SFTP storage adapter.
- [x] Encrypt stored FTP/SFTP credentials with `ENCRYPTION_KEY`.
- [x] Add storage connection CRUD endpoints.
- [x] Add `POST /storage-connections/:id/test`.
- [x] Add dashboard pages for managing storage connections.
- [x] Allow multiple storage connections per project.
- [x] Allow users to switch or select storage connections for presets.

## Phase 5: Projects, Presets, And API Keys

- [x] Implement project CRUD endpoints.
- [x] Implement upload preset CRUD endpoints.
- [x] Add preset validation for MIME types, extensions, max file size, duplicate strategy, filename strategy, folder, and transformations.
- [x] Implement API key creation.
- [x] Generate API keys with `fb_live_` and `fb_test_` prefixes.
- [x] Store only API key hashes and prefixes.
- [x] Add API key revocation.
- [x] Add dashboard pages for projects.
- [x] Add dashboard pages for upload presets.
- [x] Add dashboard pages for API keys.

## Phase 6: Upload Core

- [x] Implement signed upload session creation.
- [x] Store upload session tokens as hashes.
- [x] Enforce token expiration.
- [x] Enforce project and preset scoping.
- [x] Implement direct backend upload endpoint.
- [x] Implement session-based frontend upload endpoint.
- [x] Add multipart streaming upload handling.
- [x] Validate file size.
- [x] Validate MIME type and extension.
- [x] Add magic byte validation where practical.
- [x] Generate SHA-256 file hashes.
- [x] Implement duplicate detection.
- [x] Implement duplicate strategies: `return_existing`, `upload_new_copy`, and `reject_duplicate`.
- [x] Implement safe filename generation.
- [x] Upload files through the selected storage adapter.
- [x] Save file metadata.
- [x] Add upload logs.

## Phase 7: Worker And Background Jobs

- [x] Replace the placeholder worker with a real worker process.
- [x] Add Redis-backed job queue abstraction.
- [x] Add `process-image` job.
- [x] Add `upload-to-storage` job if uploads are processed asynchronously.
- [x] Add `generate-thumbnail` job.
- [x] Add `send-webhook` job.
- [x] Add `delete-file` job.
- [x] Add `cleanup-temp-file` job.
- [x] Add retry behavior for failed jobs.
- [x] Add job logging and failure visibility.

## Phase 8: Image Processing

- [x] Choose MVP image processing implementation.
- [x] Add image compression.
- [x] Add WebP conversion.
- [x] Add resizing.
- [x] Add thumbnail generation.
- [x] Add option to preserve originals.
- [x] Save transformation metadata.
- [x] Expose image transformation controls through upload presets.

## Phase 9: Files Dashboard

- [x] Add files list endpoint.
- [x] Add file detail endpoint.
- [x] Add file delete endpoint.
- [x] Delete files from storage adapter and database safely.
- [x] Add file listing dashboard page.
- [x] Add search by filename/path.
- [x] Add filters by MIME type and date.
- [x] Add copy URL action.
- [x] Add file detail drawer or page.
- [x] Add upload logs view.

## Phase 10: Webhooks

- [x] Add webhook CRUD endpoints.
- [x] Add webhook event model.
- [x] Send `file.uploaded` event.
- [x] Send `file.deleted` event.
- [x] Send `file.duplicate_detected` event.
- [x] Send `file.optimized` event.
- [x] Send `file.failed` event.
- [x] Sign webhook payloads with HMAC.
- [x] Add webhook retry behavior.
- [x] Add webhook delivery logs.
- [x] Add dashboard webhook management.

## Phase 11: SDKs

- [x] Implement `@binary-brawlers/filebase-client` upload flow.
- [x] Add signed upload session request support.
- [x] Add file upload support.
- [x] Add upload progress support where browser APIs allow it.
- [x] Add structured SDK errors.
- [x] Implement `@binary-brawlers/filebase-react` `useUpload` hook.
- [x] Implement `@binary-brawlers/filebase-react` `UploadButton`.
- [x] Implement `@binary-brawlers/filebase-react` `UploadDropzone`.
- [x] Implement `@binary-brawlers/filebase-next` route helpers.
- [x] Implement `@binary-brawlers/filebase-node` admin client.
- [x] Add React Native SDK in the post-MVP developer-experience phase.
- [ ] Add Vue SDK in the post-MVP developer-experience phase.

## Phase 12: One-Command Installer

- [x] Create `scripts/install.sh`.
- [x] Check Linux distribution compatibility.
- [x] Require root or sudo privileges.
- [x] Install Docker if missing.
- [x] Install Docker Compose if missing.
- [x] Create `/opt/filebase`.
- [x] Create `/opt/filebase/data/postgres`.
- [x] Create `/opt/filebase/data/uploads`.
- [x] Generate secure `JWT_SECRET`.
- [x] Generate secure `ENCRYPTION_KEY`.
- [x] Write `/opt/filebase/.env`.
- [x] Write or download `/opt/filebase/docker-compose.yml`.
- [x] Pull FileBase images.
- [x] Start services.
- [x] Detect server IP.
- [x] Print first-run setup URL.
- [x] Add uninstall or upgrade notes.

## Phase 13: Docker And Release Infrastructure

- [x] Finalize production Dockerfiles.
- [x] Ensure dashboard can reach API correctly in Docker Compose.
- [x] Add persistent volumes for PostgreSQL and local uploads.
- [x] Add image build CI.
- [x] Publish API image.
- [x] Publish worker image.
- [x] Publish dashboard image.
- [x] Publish release compose file.
- [x] Add version tags.
- [x] Add release notes automation.

## Phase 14: Documentation

- [x] Write quick-start install guide.
- [x] Write manual Docker Compose guide.
- [x] Write first-run onboarding guide.
- [x] Write local storage guide.
- [x] Write FTP storage guide.
- [x] Write SFTP storage guide.
- [x] Write API key guide.
- [x] Write upload preset guide.
- [x] Write signed upload guide.
- [x] Write SDK guides.
- [x] Write troubleshooting guide.
- [x] Write security recommendations.

## Phase 15: Hardening Before Public MVP

- [x] Add rate limiting for auth and upload endpoints.
- [x] Add request body size limits.
- [x] Add secure default headers.
- [x] Add audit-worthy logging for sensitive actions.
- [x] Ensure credentials are never returned by API responses.
- [x] Ensure secrets are not logged.
- [x] Add integration tests for setup flow.
- [x] Add integration tests for auth flow.
- [x] Add integration tests for local uploads.
- [x] Add integration tests for duplicate detection.
- [x] Add basic load testing for upload endpoints.
- [x] Review Docker defaults for production safety.

## Phase 16: Post-MVP Expansion

- [x] Add React Native SDK.
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
