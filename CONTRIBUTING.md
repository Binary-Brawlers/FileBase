# Contributing

Thanks for your interest in contributing to FileBase.

## Development Workflow

1. Fork the repository and create a feature branch.
2. Keep changes focused and include tests when behavior changes.
3. Run relevant checks before opening a pull request.
4. Use clear commit messages and describe user-facing changes in the PR.

## Checks

```bash
pnpm lint
pnpm typecheck
pnpm test
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

### Tests that need external services

`cargo test --workspace` always runs the unit tests. The API integration tests
and the storage adapter contract tests skip themselves (printing `skipping: ...`)
unless their environment variables are set. CI sets all of them.

| Tests                                                 | Required variable                                                                                                                                                                       | Optional variables                                                                 |
| ----------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| API integration (`apps/api/src/integration_tests.rs`) | `FILEBASE_TEST_DATABASE_URL`, e.g. `postgres://postgres:postgres@localhost:5432/postgres`. The role must be allowed to `CREATE DATABASE`; each test creates and drops its own database. |                                                                                    |
| S3 adapter                                            | `FILEBASE_TEST_S3_ENDPOINT`, e.g. `http://localhost:9000` (MinIO). The bucket must exist.                                                                                               | `FILEBASE_TEST_S3_BUCKET`, `_REGION`, `_ACCESS_KEY`, `_SECRET_KEY`                 |
| FTP adapter                                           | `FILEBASE_TEST_FTP_HOST`                                                                                                                                                                | `FILEBASE_TEST_FTP_PORT`, `_USER`, `_PASSWORD`, `_BASE_PATH` (must exist)          |
| SFTP adapter                                          | `FILEBASE_TEST_SFTP_HOST`                                                                                                                                                               | `FILEBASE_TEST_SFTP_PORT`, `_USER`, `_PASSWORD`, `_PRIVATE_KEY_FILE`, `_BASE_PATH` |

See `.github/workflows/ci.yml` for the containers CI uses.
