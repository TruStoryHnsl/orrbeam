# Security Policy

## Supported versions

Orrbeam is currently a pre-`1.0` project. Only the latest `0.x` release receives security fixes. Pre-release (`-alpha`, `-beta`, `-rc`) versions are not supported.

| Version | Supported |
|---------|-----------|
| 0.x (latest) | Yes |
| older 0.x | No — upgrade to latest |
| Unreleased feature branches, local-only patches, and archived prototype code such as `v1/` | No |

## Reporting a vulnerability

**Do not file a public GitHub issue for security vulnerabilities.**

### Preferred channel

Use [GitHub Security Advisories](https://github.com/TruStoryHnsl/orrbeam/security/advisories/new) to open a private report. This keeps details confidential until a fix is ready.

### Fallback channel

If you cannot use GitHub Security Advisories, email **colton.j.orr@gmail.com** with subject line `[SECURITY] orrbeam — <brief description>`. Encrypt your message with the maintainer's GPG key if the details are sensitive. (Or use `TruStoryHnsl@users.noreply.github.com`.)

### Please include

- A clear description of the issue and affected component
- Reproduction steps or a proof of concept
- Impact assessment, including required privileges or network position
- Version, platform, and relevant configuration details

## Triage SLA

| Milestone | Target |
|-----------|--------|
| Acknowledgement | 72 hours |
| Initial severity assessment | 14 days |
| Fix or documented mitigation | Depends on severity (see below) |

Initial assessment means confirming whether the report appears valid, whether it is in scope, and what the next handling step will be.

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
- Security issues involving credential handling, identity material, configuration parsing, or privilege boundaries implemented by Orrbeam itself

### Out of scope

- Vulnerabilities in **Sunshine** or **Moonlight** upstream — report those to their respective projects
- Vulnerabilities in **Headscale** — report to the Headscale project
- Third-party package vulnerabilities that are not reachable through Orrbeam's shipped behavior
- Findings that require prior local administrator or root compromise on the target machine
- Issues requiring physical access to the machine
- General hardening suggestions, missing best-practice headers, or purely theoretical concerns without a demonstrable security impact in Orrbeam

If a report spans both Orrbeam and an upstream dependency, report it here if Orrbeam's integration meaningfully contributes to the impact. Pure upstream defects should be reported to the upstream project.

## Disclosure policy

We follow [coordinated disclosure](https://cheatsheetseries.owasp.org/cheatsheets/Vulnerability_Disclosure_Cheat_Sheet.html). Once a fix is released, a GitHub Security Advisory will be published crediting the reporter (unless anonymity is requested).
