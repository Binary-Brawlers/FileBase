# FileBase — Full Project Plan

## 1. Project Summary

**FileBase** is an open-source, self-hosted file upload platform that gives developers a modern upload experience on top of traditional storage systems like FTP, SFTP, and local server folders.

The project is designed for developers, agencies, startups, and businesses that still use cPanel, shared hosting, VPS servers, or FTP/SFTP-based storage but want a better file upload experience similar to Cloudinary, ImageKit, or UploadThing.

Instead of allowing applications to upload files directly to FTP, FileBase sits in the middle.

It receives files securely from frontend or backend applications, validates them, prevents filename conflicts, detects duplicates, optimizes media, stores metadata, uploads the final file to the selected storage destination, and returns a clean public URL.

---

## 2. Core Problem

Traditional FTP is simple, but it is not developer-friendly for modern applications.

Common FTP problems include:

- Files with the same name can overwrite existing files.
- FTP credentials cannot be safely exposed in frontend apps.
- There is no duplicate file detection.
- There is no automatic image optimization.
- There is no modern upload SDK.
- There is no dashboard for managing uploaded files.
- There is no signed upload flow.
- There is no file metadata tracking.
- There is no upload history or logs.
- There is no easy way to manage uploads from React, React Native, Next.js, Vue, or mobile apps.
- Large uploads can fail without proper retry or queue handling.
- Developers have to manually organize folders and file URLs.

FileBase solves this by acting as a smart upload layer between applications and storage.

---

## 3. Product Vision

The long-term vision is to become:

```text
A self-hosted, open-source Cloudinary/ImageKit alternative for developers who want modern upload infrastructure without being locked into expensive cloud media platforms.
```

The project starts by supporting:

```text
- Local filesystem storage
- FTP storage
- SFTP storage
```

Later, it can support:

```text
- AWS S3
- Cloudflare R2
- DigitalOcean Spaces
- Backblaze B2
- Wasabi
- Bunny Storage
- Other object storage providers
```

The product should not be limited to FTP alone.

It should be built around a **storage adapter system** so developers can choose where files are stored.

---

## 4. Product Positioning

### Simple Positioning

```text
Modern file uploads for FTP, SFTP, and local server storage.
```

### Developer-Focused Positioning

```text
FileBase gives your existing FTP/SFTP server modern upload features like secure frontend uploads, unique filenames, duplicate detection, image optimization, signed upload URLs, SDKs, and a dashboard.
```

### Long-Term Positioning

```text
The open-source media upload infrastructure for developers who want Cloudinary-like features on their own storage.
```

---

## 5. Target Users

The project is useful for:

- Developers using cPanel hosting
- Agencies managing multiple client websites
- Startups that cannot afford expensive media platforms
- Developers using VPS servers
- Teams with existing FTP/SFTP infrastructure
- Web app developers
- Mobile app developers
- Developers who want self-hosted upload infrastructure
- Small businesses, churches, schools, media platforms, and organizations with legacy hosting

---

## 6. Main Deployment Question

Since the platform uploads to FTP/SFTP or local storage, it must be deployed as a middle-layer application.

The software can be deployed in three main ways.

---

## 7. Deployment Model 1 — Same VPS as the Website/App

In this model, FileBase is deployed on the same VPS as the website or application.

```text
User's VPS
├── Website / Web App
├── FileBase API
├── FileBase Worker
├── FileBase Dashboard
├── PostgreSQL
├── Redis
└── Upload folder
```

Upload flow:

```text
Frontend App
   ↓
FileBase
   ↓
Validate + Process File
   ↓
Write to Local Folder or Upload through Local FTP/SFTP
   ↓
Return Public URL
```

In this model, the best storage driver is usually:

```text
local filesystem
```

Example:

```env
STORAGE_DRIVER=local
LOCAL_STORAGE_PATH=/var/www/example.com/public/uploads
PUBLIC_BASE_URL=https://example.com/uploads
```

This is faster and more reliable than using FTP internally because the gateway can write directly to the server folder.

However, FTP/SFTP can still be supported if the user wants it.

---

## 8. Deployment Model 2 — Separate VPS Uploading to Remote FTP/SFTP

In this model, FileBase is deployed on a separate VPS, while the final files are stored on a remote FTP/SFTP server.

This is useful when the user’s actual website is hosted on shared hosting or cPanel.

```text
Gateway VPS
├── FileBase API
├── FileBase Worker
├── FileBase Dashboard
├── PostgreSQL
└── Redis

Remote cPanel/shared hosting server
└── FTP/SFTP destination folder
```

Upload flow:

```text
Frontend App
   ↓
FileBase VPS
   ↓
Validate + Process File
   ↓
Upload to Remote FTP/SFTP Server
   ↓
Return Public URL
```

Example configuration:

```env
STORAGE_DRIVER=ftp
FTP_HOST=ftp.example.com
FTP_PORT=21
FTP_USER=username
FTP_PASSWORD=password
FTP_BASE_PATH=/public_html/uploads
PUBLIC_BASE_URL=https://example.com/uploads
```

This model is realistic for many developers who have old hosting but can deploy the gateway on a small VPS.

---

## 9. Deployment Model 3 — Hosted SaaS Version

This can be added later.

In this model, the project owner provides a hosted version of FileBase.

```text
Your Cloud Infrastructure
├── FileBase API
├── FileBase Workers
├── FileBase Dashboard
├── PostgreSQL
└── Redis

User's FTP/SFTP Server
└── Final file storage
```

Upload flow:

```text
Developer's App
   ↓
Hosted FileBase
   ↓
Validate + Process File
   ↓
Upload to User's FTP/SFTP Server
   ↓
Return Public URL
```

This is easier for users because they do not have to self-host anything.

However, it comes with more responsibilities:

- You store users' encrypted FTP/SFTP credentials.
- You pay for upload bandwidth and processing.
- You manage uptime.
- You manage abuse prevention.
- You handle user data securely.
- You need billing and usage limits.

This should not be the MVP.

Recommended approach:

```text
Start with self-hosted.
Add hosted SaaS later.
```

---

## 10. Recommended MVP Deployment Model

The MVP should focus on self-hosted deployment using Docker Compose, wrapped by a simple one-command installer similar to Coolify.

The recommended public installation experience should be:

```bash
curl -fsSL https://get.filebase.dev/install.sh | sudo bash
```

The installer should prepare the server, generate secure defaults, write the deployment files, start FileBase, and print the URL where the user can finish setup.

Manual deployment should still be supported for developers who prefer to clone the repository, configure `.env`, and run:

```bash
docker compose up -d
```

The deployment should include:

```text
- Rust API
- Rust worker
- Next.js dashboard
- PostgreSQL
- Redis
```

Recommended service structure:

```text
docker-compose.yml
├── api
├── worker
├── dashboard
├── postgres
└── redis
```

### 10.1 First-Run Onboarding

After installation, FileBase should run on a known port and guide the user through setup in the browser.

Example:

```text
http://SERVER_IP:8080
```

On first visit, if no admin user exists, FileBase should show an onboarding flow instead of the normal login screen.

First-run setup should include:

```text
1. Create the first admin account.
2. Create the default project.
3. Choose the initial upload behavior.
4. Configure the first storage connection.
5. Create a default upload preset.
6. Show the API URL, dashboard URL, and SDK integration snippet.
```

Initial upload behavior options:

```text
- Local server folder
- FTP server
- SFTP server
```

The first choice should not lock the user in. FileBase should support multiple storage connections per project so users can add, test, switch, or use additional storage types later from the dashboard.

The onboarding state should be derived from the database, not a temporary file. If no users exist, onboarding is required. Once the first admin user exists, public registration should be disabled by default unless explicitly enabled.

### 10.2 Installer Responsibilities

The install script should be production-oriented and safe by default.

It should:

```text
- Check OS compatibility.
- Require root or sudo privileges.
- Install Docker if missing.
- Install Docker Compose if missing.
- Create /opt/filebase.
- Generate .env with secure JWT_SECRET and ENCRYPTION_KEY values.
- Write docker-compose.yml or download the release compose file.
- Pull the latest FileBase images.
- Start the services.
- Print the first-run setup URL.
```

Recommended server paths:

```text
/opt/filebase
/opt/filebase/.env
/opt/filebase/docker-compose.yml
/opt/filebase/data/postgres
/opt/filebase/data/uploads
```

The installer should not ask users to manually edit `.env` before first launch. Storage-specific configuration should happen in the onboarding UI and dashboard.

---

## 11. Recommended Tech Stack

### 11.1 Backend

```text
Language: Rust
Web Framework: Axum
Async Runtime: Tokio
Database: PostgreSQL
Database Library: SeaORM
Queue/Jobs: Redis
Authentication: JWT
Password Hashing: bcrypt
API Documentation: utoipa + Swagger UI
File Upload Handling: multipart streaming
Image Processing: image crate, fast_image_resize, or libvips binding
FTP Client: suppaftp
SFTP Client: ssh2 or russh-based client
Config: environment variables
Logging: tracing
Error Handling: thiserror + anyhow
```

### 11.2 Frontend Dashboard

```text
Framework: Next.js
Language: TypeScript
Styling: Tailwind CSS
UI Components: shadcn/ui
Data Fetching: TanStack Query
Forms: React Hook Form
Validation: Zod
Authentication: JWT/session-based auth
```

### 11.3 SDKs

```text
Language: TypeScript
Core SDK: Fetch API
React SDK: React hooks and components
React Native SDK: URI/file upload support
Next.js SDK: server route helpers and client helpers
Node SDK: backend admin client
```

### 11.4 Infrastructure

```text
Containerization: Docker
Deployment: Docker Compose
Database: PostgreSQL
Cache/Queue: Redis
Reverse Proxy: Caddy, Nginx, or Traefik
```

---

## 12. High-Level Architecture

```text
Frontend SDK / Backend SDK / Dashboard
                ↓
          Rust API Server
                ↓
        Validation + Auth Layer
                ↓
        Upload Processing Layer
                ↓
        Storage Adapter Layer
                ↓
    Local Filesystem / FTP / SFTP
                ↓
          Public File URL
```

For heavy work:

```text
Rust API
   ↓
Redis Queue
   ↓
Rust Worker
   ↓
Image Processing + FTP/SFTP Upload
```

---

## 13. Storage Adapter System

The platform should use a storage adapter interface.

This is important because the platform should not be tightly coupled to FTP.

Supported MVP adapters:

```text
- Local filesystem
- FTP
- SFTP
```

Future adapters:

```text
- S3
- Cloudflare R2
- DigitalOcean Spaces
- Backblaze B2
- Wasabi
- Bunny Storage
```

Example conceptual adapter interface:

```rust
pub trait StorageAdapter {
    async fn upload(&self, input: UploadInput) -> Result<UploadResult>;
    async fn delete(&self, path: &str) -> Result<()>;
    async fn exists(&self, path: &str) -> Result<bool>;
    async fn public_url(&self, path: &str) -> Result<String>;
}
```

---

## 14. Storage Configuration

### 14.1 Local Storage

```env
STORAGE_DRIVER=local
LOCAL_STORAGE_PATH=/var/www/example.com/public/uploads
PUBLIC_BASE_URL=https://example.com/uploads
```

### 14.2 FTP Storage

```env
STORAGE_DRIVER=ftp
FTP_HOST=ftp.example.com
FTP_PORT=21
FTP_USER=username
FTP_PASSWORD=password
FTP_BASE_PATH=/public_html/uploads
PUBLIC_BASE_URL=https://example.com/uploads
```

### 14.3 SFTP Storage

```env
STORAGE_DRIVER=sftp
SFTP_HOST=example.com
SFTP_PORT=22
SFTP_USER=username
SFTP_PASSWORD=password
SFTP_PRIVATE_KEY=
SFTP_BASE_PATH=/var/www/example.com/public/uploads
PUBLIC_BASE_URL=https://example.com/uploads
```

---

## 15. Upload Flow

### 15.1 Backend Upload Flow

```text
1. Developer sends file to FileBase API.
2. API validates API key or user session.
3. API validates file size, MIME type, and extension.
4. API temporarily stores or streams the file.
5. API generates file hash.
6. API checks if the file already exists.
7. If duplicate exists, return existing file URL.
8. If new file, generate safe unique filename.
9. Process image if required.
10. Upload final file through storage adapter.
11. Save file metadata in PostgreSQL.
12. Trigger webhook if configured.
13. Return public file URL.
```

### 15.2 Frontend Secure Upload Flow

Frontend apps must never receive FTP/SFTP credentials.

Instead, they use signed upload sessions.

```text
1. Frontend asks application backend for a signed upload session.
2. Application backend calls FileBase using a private API key.
3. FileBase creates a short-lived upload session.
4. Frontend receives upload URL and temporary token.
5. Frontend uploads file directly to FileBase.
6. Gateway validates token.
7. Gateway processes and stores file.
8. Gateway returns final file URL.
```

Example:

```text
React App
   ↓ request signed upload session
App Backend
   ↓ private API key
FileBase
   ↓ returns temporary upload token
React App
   ↓ uploads file with token
FileBase
   ↓ stores file
FTP/SFTP/Local Storage
```

---

## 16. Secure Upload Session

Signed upload sessions are essential for frontend uploads.

An upload session should include:

```text
id
projectId
presetId
tokenHash
allowedMimeTypes
maxFileSize
folder
expiresAt
usedAt
createdAt
```

Rules:

- Tokens should expire quickly.
- Tokens should be scoped to one project.
- Tokens should be scoped to one preset.
- Tokens should have file size limits.
- Tokens should have MIME type limits.
- Tokens should be single-use where possible.
- Tokens should be stored as hashes, not plain text.

Example response:

```json
{
  "uploadUrl": "https://uploads.example.com/v1/uploads/session_abc123",
  "token": "temporary_signed_upload_token",
  "expiresAt": "2026-05-23T12:30:00Z"
}
```

---

## 17. File Naming Strategy

The platform should prevent accidental overwrites.

Raw FTP may replace:

```text
/uploads/image.png
```

if another file with the same name is uploaded.

FileBase should save files like:

```text
/uploads/2026/05/image-a8f91c.png
```

or:

```text
/uploads/users/avatars/profile-picture-f3a91c.webp
```

Recommended filename strategy:

```text
slugified-original-name + short-random-id + final-extension
```

Example:

```text
Original: My Profile Picture.png
Saved: my-profile-picture-a82f91.webp
```

Other supported strategies:

```text
- UUID
- Hash-based filename
- Timestamp + random suffix
- Random ID only
- Original filename with suffix
```

---

## 18. Duplicate Detection

The system should generate a SHA-256 hash for each file.

Before uploading, it checks if the same hash already exists in the same project.

If duplicate exists, behavior should be configurable:

```text
return_existing
upload_new_copy
reject_duplicate
```

Recommended default:

```text
return_existing
```

Example response:

```json
{
  "fileId": "file_12345",
  "url": "https://example.com/uploads/logo-a82f91.png",
  "duplicate": true,
  "duplicateOfFileId": "file_98765"
}
```

---

## 19. Image Processing

The platform should support image optimization.

MVP image features:

```text
- Compress image
- Convert to WebP
- Resize image
- Generate thumbnail
- Preserve original optionally
```

Future image features:

```text
- AVIF conversion
- Watermarking
- Smart cropping
- Background removal
- Dynamic URL transformations
```

Example preset:

```json
{
  "name": "profile_images",
  "folder": "users/profiles",
  "allowedMimeTypes": ["image/jpeg", "image/png", "image/webp"],
  "maxFileSize": 5242880,
  "transformations": {
    "format": "webp",
    "quality": 80,
    "resize": {
      "width": 500,
      "height": 500,
      "fit": "cover"
    }
  }
}
```

---

## 20. Upload Presets

Upload presets allow developers to define reusable upload rules.

Examples:

```text
profile_images
product_images
blog_images
documents
videos
```

Preset fields:

```text
id
projectId
name
folder
allowedMimeTypes
allowedExtensions
maxFileSize
duplicateStrategy
filenameStrategy
transformations
createdAt
updatedAt
```

Example frontend usage:

```ts
await upload(file, {
  preset: "profile_images"
});
```

---

## 21. Frontend SDKs

The project should not only be backend-focused.

It should provide SDKs that make uploads easy from frontend and mobile apps.

Recommended packages:

```text
@binary-brawlers/filebase-client
@binary-brawlers/filebase-react
@binary-brawlers/filebase-react-native
@binary-brawlers/filebase-next
@binary-brawlers/filebase-vue
@binary-brawlers/filebase-node
```

---

## 22. Core JavaScript SDK

Package:

```text
@binary-brawlers/filebase-client
```

Responsibilities:

- Request signed upload session
- Upload file
- Track upload progress
- Handle errors
- Return final file response
- Work in browser environments

Example:

```ts
import { FileBaseClient } from "@binary-brawlers/filebase-client";

const client = new FileBaseClient({
  signEndpoint: "/api/upload/sign"
});

const result = await client.upload(file, {
  preset: "profile_images"
});

console.log(result.url);
```

---

## 23. React SDK

Package:

```text
@binary-brawlers/filebase-react
```

Features:

```text
- useUpload hook
- UploadButton component
- UploadDropzone component
- Progress state
- Error state
- Success callback
```

Example:

```tsx
import { UploadButton } from "@binary-brawlers/filebase-react";

export function ProfileUploader() {
  return (
    <UploadButton
      signEndpoint="/api/upload/sign"
      preset="profile_images"
      accept="image/*"
      maxSize={5 * 1024 * 1024}
      onUploadComplete={(file) => {
        console.log(file.url);
      }}
    />
  );
}
```

---

## 24. React Native SDK

Package:

```text
@binary-brawlers/filebase-react-native
```

Features:

```text
- Upload from file URI
- Upload from Expo ImagePicker
- Upload progress
- Mobile-friendly errors
- Works with React Native and Expo
```

Example:

```tsx
import { uploadFile } from "@binary-brawlers/filebase-react-native";

const result = await uploadFile({
  uri: image.uri,
  name: "profile.jpg",
  type: "image/jpeg",
  signEndpoint: "https://api.example.com/upload/sign",
  preset: "profile_images"
});

console.log(result.url);
```

---

## 25. Next.js SDK

Package:

```text
@binary-brawlers/filebase-next
```

Features:

```text
- App Router helper
- Pages Router helper
- Server-side upload session creation
- Client upload helper
```

Example App Router signing route:

```ts
import { createFileBaseRoute } from "@binary-brawlers/filebase-next";

export const POST = createFileBaseRoute({
  apiKey: process.env.FILEBASE_API_KEY,
  gatewayUrl: process.env.FILEBASE_GATEWAY_URL
});
```

---

## 26. Node.js SDK

Package:

```text
@binary-brawlers/filebase-node
```

Features:

```text
- Server-side file upload
- Create upload sessions
- Delete files
- List files
- Manage presets
- Admin API wrapper
```

Example:

```ts
import { FileBase } from "@binary-brawlers/filebase-node";

const fileBase = new FileBase({
  apiKey: process.env.FILEBASE_API_KEY,
  gatewayUrl: process.env.FILEBASE_GATEWAY_URL
});

const session = await fileBase.createUploadSession({
  preset: "profile_images"
});
```

---

## 27. Dashboard Features

The Next.js dashboard should allow users to manage the platform.

MVP dashboard features:

```text
- First-run onboarding
- Login
- Project setup
- Storage connection setup
- Test storage connection
- View uploaded files
- Search files
- Filter files by type
- Filter files by date
- Copy file URL
- Delete file
- View file details
- View upload logs
- Manage upload presets
- Manage API keys
```

Future dashboard features:

```text
- Usage analytics
- Team accounts
- Role-based permissions
- Billing for hosted version
- Webhook logs
- Audit logs
- Storage usage charts
```

---

## 28. API Design

### 28.1 Auth

```http
GET /setup/status
POST /setup/initialize
POST /auth/register
POST /auth/login
GET /auth/me
POST /auth/logout
```

### 28.2 Projects

```http
POST /projects
GET /projects
GET /projects/:id
PATCH /projects/:id
DELETE /projects/:id
```

### 28.3 Storage Connections

```http
POST /storage-connections
GET /storage-connections
GET /storage-connections/:id
PATCH /storage-connections/:id
DELETE /storage-connections/:id
POST /storage-connections/:id/test
```

### 28.4 Upload Presets

```http
POST /upload-presets
GET /upload-presets
GET /upload-presets/:id
PATCH /upload-presets/:id
DELETE /upload-presets/:id
```

### 28.5 Upload Sessions

```http
POST /uploads/sign
GET /uploads/sessions/:id
```

### 28.6 Uploads

```http
POST /uploads
POST /uploads/:sessionId
GET /files
GET /files/:id
DELETE /files/:id
```

### 28.7 Webhooks

```http
POST /webhooks
GET /webhooks
PATCH /webhooks/:id
DELETE /webhooks/:id
POST /webhooks/:id/test
```

---

## 29. Database Models

### 29.1 User

```text
id
name
email
passwordHash
createdAt
updatedAt
```

### 29.2 Project

```text
id
userId
name
slug
createdAt
updatedAt
```

### 29.3 ApiKey

```text
id
projectId
name
keyHash
prefix
lastUsedAt
createdAt
revokedAt
```

### 29.4 StorageConnection

```text
id
projectId
type
host
port
username
encryptedPassword
encryptedPrivateKey
basePath
publicBaseUrl
createdAt
updatedAt
```

### 29.5 UploadPreset

```text
id
projectId
name
folder
allowedMimeTypes
allowedExtensions
maxFileSize
duplicateStrategy
filenameStrategy
transformationsJson
createdAt
updatedAt
```

### 29.6 UploadSession

```text
id
projectId
presetId
tokenHash
folder
allowedMimeTypes
maxFileSize
expiresAt
usedAt
createdAt
```

### 29.7 File

```text
id
projectId
storageConnectionId
originalName
savedName
mimeType
extension
size
hash
folder
path
url
status
duplicateOfFileId
metadataJson
createdAt
updatedAt
```

### 29.8 Webhook

```text
id
projectId
url
secret
events
isActive
createdAt
updatedAt
```

### 29.9 UploadLog

```text
id
projectId
fileId
event
status
message
metadataJson
createdAt
```

---

## 30. Security Design

### 30.1 Never Expose Storage Credentials

FTP/SFTP credentials should only exist on the FileBase server.

Frontend apps should never receive:

```text
FTP_HOST
FTP_USER
FTP_PASSWORD
SFTP_PRIVATE_KEY
```

Frontend apps should only receive:

```text
upload URL
temporary token
expiration time
allowed upload rules
```

### 30.2 Encrypt Credentials

Storage credentials must be encrypted before saving to the database.

Recommended approach:

```text
AES-256-GCM
```

Required environment variable:

```env
ENCRYPTION_KEY=
```

### 30.3 Hash API Keys

API keys should not be stored directly.

Store only:

```text
key hash
key prefix
last used date
```

Example API key format:

```text
fb_live_xxxxxxxxxxxxxxxxxxxxx
fb_test_xxxxxxxxxxxxxxxxxxxxx
```

### 30.4 Rate Limiting

Apply rate limits to:

```text
/auth/*
/uploads/sign
/uploads/*
```

### 30.5 File Type Validation

Do not trust only file extensions.

Validate with:

```text
- MIME type
- extension
- magic bytes where possible
```

### 30.6 Upload Token Rules

Upload tokens should be:

```text
- short-lived
- scoped to a project
- scoped to a preset
- single-use where possible
- stored as hashes
```

---

## 31. Background Jobs

Redis-backed jobs should be used for expensive work.

Queue jobs:

```text
process-image
upload-to-storage
generate-thumbnail
send-webhook
delete-file
cleanup-temp-file
```

This prevents large upload processing from blocking the API server.

---

## 32. Webhooks

The platform should notify external systems when upload events happen.

Supported events:

```text
file.uploaded
file.deleted
file.duplicate_detected
file.optimized
file.failed
```

Example webhook payload:

```json
{
  "event": "file.uploaded",
  "file": {
    "id": "file_12345",
    "url": "https://example.com/uploads/image-a82f91.webp",
    "originalName": "image.png",
    "size": 243000,
    "mimeType": "image/webp"
  }
}
```

Webhook security:

```text
- Sign webhook payloads with HMAC
- Allow retry on failure
- Store webhook delivery logs
```

---

## 33. Monorepo Structure

Recommended repository structure:

```text
filebase/
  apps/
    api/                 # Rust Axum API
    worker/              # Rust background worker
    dashboard/           # Next.js dashboard
    docs/                # Documentation site
  packages/
    client/              # Core TypeScript SDK
    react/               # React SDK
    react-native/        # React Native SDK
    next/                # Next.js helpers
    vue/                 # Vue SDK
    node/                # Node.js SDK
    shared/              # Shared TypeScript types
  crates/
    core/                # Rust domain logic
    storage/             # Storage adapter traits
    storage-local/       # Local filesystem adapter
    storage-ftp/         # FTP adapter
    storage-sftp/        # SFTP adapter
    image-processing/    # Image processing logic
  examples/
    nextjs-example/
    react-example/
    react-native-example/
  docker/
  docker-compose.yml
  README.md
```

---

## 34. MVP Scope

The MVP should be focused and practical.

### 34.1 MVP Features

```text
- One-command self-hosted installer
- Rust API using Axum
- Rust worker
- Next.js dashboard
- PostgreSQL database
- Redis queue
- First-run admin registration
- Guided storage onboarding
- User authentication
- Project creation
- API key creation
- Local storage adapter
- FTP adapter
- SFTP adapter
- Upload API
- Signed upload sessions
- Unique file naming
- Duplicate detection
- Basic image compression
- File metadata storage
- File listing dashboard
- File deletion
- Upload presets
- React SDK
- Core JavaScript SDK
- Docker Compose deployment
- Basic documentation
```

### 34.2 Not Needed in MVP

```text
- Video transcoding
- AI compression
- Dynamic image transformations by URL
- Team accounts
- Billing
- Hosted SaaS
- Advanced analytics
- CDN integration
- Watermarking
- Background removal
```

---

## 35. Roadmap

### Version 1 — MVP

Goal:

```text
Make FTP/SFTP/local uploads safer and developer-friendly.
```

Features:

```text
- One-command installer
- Self-hosted Docker deployment
- First-run onboarding
- Rust API
- Next.js dashboard
- Local/FTP/SFTP storage
- Secure upload sessions
- Duplicate detection
- Unique filenames
- Image compression
- React SDK
- JavaScript SDK
```

### Version 2 — Better Developer Experience

Goal:

```text
Make it easier to integrate in real apps.
```

Features:

```text
- React Native SDK
- Next.js SDK
- Node.js SDK
- Vue SDK
- Webhooks
- Better upload presets
- Folder management
- Upload logs
- Improved docs
```

### Version 3 — More Storage Providers

Goal:

```text
Move beyond FTP/SFTP.
```

Features:

```text
- S3 adapter
- Cloudflare R2 adapter
- DigitalOcean Spaces adapter
- Backblaze B2 adapter
- Wasabi adapter
```

### Version 4 — Advanced Media Features

Goal:

```text
Become a serious media management platform.
```

Features:

```text
- Video upload support
- Video thumbnail generation
- Image transformations by URL
- CDN integration
- Watermarking
- AVIF conversion
- Advanced thumbnails
```

### Version 5 — Hosted SaaS Mode

Goal:

```text
Allow users to use FileBase without self-hosting.
```

Features:

```text
- Hosted dashboard
- Hosted gateway
- Team accounts
- Billing
- Usage limits
- Organization management
- Audit logs
- Subscription plans
```

---

## 36. Example Self-Hosted .env

```env
APP_URL=https://uploads.example.com
DASHBOARD_URL=https://uploads.example.com/dashboard

DATABASE_URL=postgres://filebase:password@postgres:5432/filebase
REDIS_URL=redis://redis:6379

JWT_SECRET=change_me
ENCRYPTION_KEY=change_me_32_byte_key

STORAGE_DRIVER=ftp

FTP_HOST=ftp.example.com
FTP_PORT=21
FTP_USER=username
FTP_PASSWORD=password
FTP_BASE_PATH=/public_html/uploads
PUBLIC_BASE_URL=https://example.com/uploads

MAX_UPLOAD_SIZE=10485760
DEFAULT_DUPLICATE_STRATEGY=return_existing
```

---

## 37. Example Docker Compose Services

```yaml
services:
  api:
    image: filebase/api:latest
    env_file: .env
    depends_on:
      - postgres
      - redis

  worker:
    image: filebase/worker:latest
    env_file: .env
    depends_on:
      - postgres
      - redis

  dashboard:
    image: filebase/dashboard:latest
    env_file: .env
    depends_on:
      - api

  postgres:
    image: postgres:16
    environment:
      POSTGRES_USER: filebase
      POSTGRES_PASSWORD: password
      POSTGRES_DB: filebase
    volumes:
      - postgres_data:/var/lib/postgresql/data

  redis:
    image: redis:7

volumes:
  postgres_data:
```

---

## 38. Developer Experience Goal

The ideal developer experience should be:

```text
1. Install FileBase with one command.
2. Open the setup URL in the browser.
3. Register the first admin account.
4. Choose local, FTP, or SFTP as the initial upload behavior.
5. Add storage credentials through the onboarding UI.
6. Create or accept the default upload preset.
7. Install SDK.
8. Upload files securely from frontend.
9. Receive final public URL.
```

Example:

```bash
pnpm add @binary-brawlers/filebase-react
```

```tsx
<UploadButton
  signEndpoint="/api/upload/sign"
  preset="profile_images"
  onUploadComplete={(file) => console.log(file.url)}
/>
```

---

## 39. Competitive Advantage

FileBase is different because it starts from a problem many tools ignore:

```text
Many developers still use FTP, SFTP, cPanel, and shared hosting.
```

Most modern upload platforms assume developers already use S3-like object storage.

FileBase helps developers modernize their upload flow without immediately changing their storage infrastructure.

This gives it a clear niche:

```text
A modern upload platform for traditional hosting.
```

---

## 40. Project Name

Project name:

```text
FileBase
```

It is broad enough to support more than FTP in the future.

---

## 41. Success Metrics

The project is successful if developers can:

- Install it with one command on a server.
- Complete first-run setup in the browser.
- Deploy it with Docker Compose manually if preferred.
- Connect FTP/SFTP/local storage easily.
- Add multiple storage connections later from the dashboard.
- Upload files without overwriting existing files.
- Upload securely from React and React Native.
- Get file URLs automatically.
- Manage files from a dashboard.
- Reduce image sizes automatically.
- Avoid exposing FTP credentials.
- Use it in real projects within minutes.

---

## 42. Final Recommendation

Build the project as:

```text
A self-hosted upload gateway, not just an FTP uploader.
```

Use:

```text
Rust + Axum for the backend
Next.js for the dashboard
PostgreSQL for metadata
Redis for queues
Docker Compose for deployment
TypeScript SDKs for frontend/backend integrations
```

Support these storage drivers first:

```text
local
ftp
sftp
```

Then expand to object storage later.

This gives the project a strong technical foundation, clear market positioning, and enough flexibility to grow from a simple FTP improvement tool into a full open-source media infrastructure platform.
