# Security Policy

## Supported Versions

Orrbeam is currently a pre-`1.0` project. Security fixes are provided for supported `0.x` releases only.

| Version | Supported |
| --- | --- |
| `0.x` | Yes |
| Unreleased feature branches, local-only patches, and archived prototype code such as `v1/` | No |

## Reporting a Vulnerability

Please do not open public GitHub issues for suspected security vulnerabilities.

Preferred reporting channel:

- GitHub Security Advisories for `TruStoryHnsl/orrbeam`: <https://github.com/TruStoryHnsl/orrbeam/security/advisories/new>

Fallback if GitHub private reporting is unavailable:

- `TruStoryHnsl@users.noreply.github.com`

Please include:

- A clear description of the issue and affected component
- Reproduction steps or a proof of concept
- Impact assessment, including required privileges or network position
- Version, platform, and relevant configuration details

## Response Targets

- Acknowledgement within 72 hours
- Initial assessment within 14 days
- Follow-up updates after triage if more time is required for reproduction, coordination, or a fix

Initial assessment means confirming whether the report appears valid, whether it is in scope, and what the next handling step will be.

## In Scope

- Vulnerabilities in the Rust workspace crates under `crates/`, including `orrbeam-core`, `orrbeam-net`, and `orrbeam-platform`
- Tauri IPC command handlers and backend integration under `src-tauri/`
- Network discovery, peer registration, and mesh-related behavior, including LAN discovery and orrtellite-backed discovery paths
- Security issues involving credential handling, identity material, configuration parsing, or privilege boundaries implemented by Orrbeam itself

## Out of Scope

- Upstream bugs or vulnerabilities in Sunshine itself
- Upstream bugs or vulnerabilities in Moonlight itself
- Third-party package vulnerabilities that are not reachable through Orrbeam's shipped behavior
- Findings that require prior local administrator or root compromise on the target machine
- General hardening suggestions, missing best-practice headers, or purely theoretical concerns without a demonstrable security impact in Orrbeam

If a report spans both Orrbeam and an upstream dependency, report it here if Orrbeam's integration meaningfully contributes to the impact. Pure upstream defects should be reported to the upstream project.
