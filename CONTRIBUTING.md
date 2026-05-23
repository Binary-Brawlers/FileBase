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
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
