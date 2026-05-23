# FileBase

FileBase is an open-source, self-hosted upload gateway for modern file uploads on top of local filesystem, FTP, and SFTP storage.

## Repository Status

This repository is being scaffolded as a monorepo for the FileBase API, worker, dashboard, SDKs, storage adapters, documentation, and examples.

## Structure

```text
apps/
  api/                 Rust Axum API service
  worker/              Rust background worker
  dashboard/           Next.js dashboard
  docs/                Documentation site
packages/
  client/              Core TypeScript SDK
  react/               React SDK
  react-native/        React Native SDK
  next/                Next.js helpers
  vue/                 Vue SDK
  node/                Node.js SDK
  shared/              Shared TypeScript types
crates/
  core/                Rust domain logic
  storage/             Storage adapter traits
  storage-local/       Local filesystem adapter
  storage-ftp/         FTP adapter
  storage-sftp/        SFTP adapter
  image-processing/    Image processing logic
examples/              Integration examples
docker/                Container build files
```

## Development

Prerequisites:

- Rust stable
- Node.js 20+
- pnpm 9+
- Docker and Docker Compose

Install JavaScript dependencies:

```bash
pnpm install
```

Run Rust checks:

```bash
cargo check --workspace
```

Start local infrastructure:

```bash
docker compose up -d postgres redis
```

## License

MIT
