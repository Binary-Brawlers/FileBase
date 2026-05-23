# ADR 0001: Monorepo Structure

## Status

Accepted

## Context

FileBase contains Rust services, Rust reusable crates, TypeScript SDK packages, a dashboard, documentation, examples, and deployment assets.

## Decision

Use a monorepo with `apps/`, `packages/`, `crates/`, `examples/`, and `docker/` directories. Rust workspace management is handled by Cargo. JavaScript workspace management is handled by pnpm and Turborepo.

## Consequences

The repository can share code and CI across all project parts while keeping deployable applications and reusable libraries clearly separated.
