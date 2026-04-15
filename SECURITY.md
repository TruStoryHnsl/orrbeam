# Security Policy

## Supported versions

Only the latest `0.x` release receives security fixes. Pre-release (`-alpha`, `-beta`, `-rc`) versions are not supported.

| Version | Supported |
|---------|-----------|
| 0.x (latest) | Yes |
| older 0.x | No — upgrade to latest |

## Reporting a vulnerability

**Do not file a public GitHub issue for security vulnerabilities.**

### Preferred channel

Use [GitHub Security Advisories](https://github.com/TruStoryHnsl/orrbeam/security/advisories/new) to open a private report. This keeps details confidential until a fix is ready.

### Fallback channel

If you cannot use GitHub Security Advisories, email **colton.j.orr@gmail.com** with subject line `[SECURITY] orrbeam — <brief description>`. Encrypt your message with the maintainer's GPG key if the details are sensitive.

## Triage SLA

| Milestone | Target |
|-----------|--------|
| Acknowledgement | 72 hours |
| Initial severity assessment | 14 days |
| Fix or documented mitigation | Depends on severity (see below) |

Severity targets:

- **Critical** (CVSS ≥ 9.0): patch within 7 days of confirmed severity
- **High** (CVSS 7.0–8.9): patch within 30 days
- **Medium / Low**: patched in next scheduled release

## Scope

### In scope

- `crates/orrbeam-core` — identity, TLS, wire protocol, peer trust store
- `crates/orrbeam-net` — control plane server (Axum HTTPS, Ed25519 auth), discovery (mDNS, Headscale API)
- `crates/orrbeam-platform` — Sunshine/Moonlight process management
- `src-tauri` — Tauri IPC command surface, AppState
- Frontend (`frontend/`) — IPC wrappers, Zustand stores

### Out of scope

- Vulnerabilities in **Sunshine** or **Moonlight** upstream — report those to their respective projects
- Vulnerabilities in **Headscale** — report to the Headscale project
- Issues requiring physical access to the machine
- Theoretical attacks with no realistic exploit path

## Disclosure policy

We follow [coordinated disclosure](https://cheatsheetseries.owasp.org/cheatsheets/Vulnerability_Disclosure_Cheat_Sheet.html). Once a fix is released, a GitHub Security Advisory will be published crediting the reporter (unless anonymity is requested).
