# Security Policy

## Supported Versions

The `main` branch is the supported security baseline.

## Reporting a Vulnerability

Please **do not** open public GitHub issues for security findings.

Report privately by emailing the maintainers with:

- Vulnerability description
- Reproduction steps / PoC
- Impact assessment
- Suggested remediation (if known)

We will:

1. Acknowledge receipt within 72 hours.
2. Triage severity and impact.
3. Provide remediation status updates.
4. Coordinate disclosure once a fix is available.

## Security Hardening Notes

Recent hardening includes:

- Environment-only auth credentials
- Signed HttpOnly session cookies (protected routes are cookie-only)
- Restricted CORS + localhost default bind
- WebSocket auth without URL credentials
- Request/input limits and rate limiting
- Hardened production systemd units (`ProtectHome=true`, minimal `ReadWritePaths`)
- HSTS operationalized at the Cloudflare edge (with verification/rollback runbook)
- CI secret/dependency scanning
