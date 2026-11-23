# Security Policy

## Supported Versions

| Version | Supported |
| ------- | --------- |
| 1.0.x   | Yes       |
| < 1.0   | No        |

## Reporting a Vulnerability

If you discover a security vulnerability in this repository:

1. **Do NOT open a public GitHub issue** for undisclosed vulnerabilities.
2. Submit a private report via GitHub Security Advisories (`Security -> Advisories -> Report a vulnerability`).
3. Include affected file paths, line numbers, reproduction steps, and estimated severity.
4. Initial acknowledgement will be provided within **48 hours**.

## Security Controls

- **Secret Hygiene**: `.env` files are excluded via `.gitignore`; startup logs automatically redact PostgreSQL credentials (`redact_dsn`).
- **Endpoint & Input Hardening**: Explicit CORS origin allowlisting via `ALLOWED_ORIGINS`, centralized JSON fallback error handling (`ErrorHandler`), strict input validation, and parameterized SQLx / ClickHouse queries.
- **Automated Vulnerability Management**: Weekly Dependabot updates (`cargo`, `docker`, `github-actions` with `semver-major` ignored), Semgrep Rust SAST, and Trivy container image scanning in CI.
- **Container Hardening**: Multi-stage Rust release build into minimal `debian:bookworm-slim` running as non-root `appuser` (UID `10001`) with a built-in `/health` probe.
