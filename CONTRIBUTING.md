# Contributing to Braincode Backend

Thank you for your interest in contributing to `rust-dual-db-backend`.

## Development Prerequisites

- Rust toolchain `1.82+` (`rustup`, `cargo`, `rustfmt`, `clippy`)
- Docker and Docker Compose (optional, for running PostgreSQL + ClickHouse locally)
- `pre-commit` (recommended for local formatting and secret hygiene hooks)

## Local Setup

```bash
cp .env.example .env
pre-commit install
make test
```

## Code Quality Standards

All contributions must satisfy the following checks before opening a pull request:

1. **Formatting**: `make fmt-check` (`cargo fmt --all -- --check`) passes cleanly.
2. **Linting**: `make lint` (`cargo clippy --all-targets -- -D warnings`) produces zero warnings.
3. **Testing**: `make test` (`cargo test --all-targets`) passes 100% offline without requiring live databases for unit/integration suites.
4. **Error Handling**: Return typed `AppError` or `anyhow::Result` values; avoid `.unwrap()` and `.expect()` on fallible runtime paths outside tests.
5. **Security**: Never commit `.env` files, real credentials, or raw tokens. Redact DSNs before logging.

## Commit Convention

Follow [Conventional Commits](https://www.conventionalcommits.org/):

- `feat(scope): add new capability`
- `fix(scope): resolve defect`
- `test(scope): add or update tests`
- `ci(scope): update pipeline or tooling`
- `ops(scope): update container or runbook`
- `docs(scope): update documentation`

## Pull Request Process

1. Create a feature branch from `main`.
2. Ensure all CI checks (`lint`, `test`, `build-and-security`) pass.
3. Update `CHANGELOG.md` and `runbook.md` when introducing behavioral or operational changes.
