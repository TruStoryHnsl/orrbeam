# Contributing to Orrbeam

## License Requirements

All dependencies (Rust crates and npm packages) must use OSS-compatible,
non-copyleft licenses. Permitted licenses:

**Rust (cargo-deny enforced):**
- MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, CC0-1.0

**npm (check-licenses.sh enforced):**
- MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Unlicense, CC0-1.0

**Not permitted:** GPL-2.0, GPL-3.0, AGPL-3.0, LGPL-2.0, LGPL-2.1, LGPL-3.0

## Running License Checks

```bash
# Install cargo-deny (once)
cargo install cargo-deny

# Check Rust dependency licenses
cargo deny check licenses

# Check all deny rules (licenses + advisories + bans + sources)
cargo deny check

# Check npm licenses (requires npm install in frontend/ first)
./scripts/check-licenses.sh
```

If `cargo deny check licenses` fails due to a new dependency, either:
1. Choose an alternative crate with a compatible license, or
2. Add an explicit `[[licenses.exceptions]]` entry to `deny.toml` with justification.

## Commit Style

All commits must follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(scope): add something new
fix(scope): correct a bug
docs: update readme
chore: update dependencies
```

## Development Setup

```bash
# Rust + Tauri backend
cargo build --workspace

# Frontend
cd frontend && npm install && npm run dev

# Full dev mode (hot-reload)
cargo tauri dev
```
