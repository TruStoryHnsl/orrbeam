# Contributing to Orrbeam

## Local Development Setup

1. Install Rust 1.80+, Node.js 20+, and the Tauri v2 system dependencies for your OS.
2. Clone the repo and install frontend packages:

   ```bash
   git clone git@github.com:TruStoryHnsl/orrbeam.git
   cd orrbeam
   cd frontend && npm install && cd ..
   ```

3. Run the desktop app in development mode:

   ```bash
   cargo tauri dev
   ```

4. For frontend-only iteration, run:

   ```bash
   cd frontend && npm run dev
   ```

## Workspace Layout

- `crates/orrbeam-core`: shared types, config, identity, peers, TLS, wire helpers
- `crates/orrbeam-net`: discovery plus control-plane client/server code
- `crates/orrbeam-platform`: Linux, macOS, and Windows platform adapters
- `src-tauri`: Tauri shell, IPC command handlers, tray wiring, app state
- `frontend`: React UI, Zustand stores, Tauri API wrapper, shared components

## Branch Naming

Use a short prefix that matches the kind of work:

- `feat/<slug>`
- `fix/<slug>`
- `refactor/<slug>`
- `chore/<slug>`

Keep one topic per branch. Do not mix unrelated fixes into the same branch.

## Commit Format

Use Conventional Commits for every commit:

- `feat: add persistent node registry`
- `fix: reject malformed peer addresses`
- `docs: add architecture overview`
- `chore: align frontend package metadata`

If the change is breaking, use `feat!:` or add a `BREAKING CHANGE:` footer.

## Verification Before Commit

Run the expected checks before opening a pull request:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd frontend && npm run test
```

If your change touches the frontend build or TypeScript surface, also run:

```bash
cd frontend && npm run build
```

## Pull Request Process

- Open a focused branch with a conventional-commit-ready PR title
- Link the issue or plan item the PR addresses
- Include a short test plan with exact commands run
- Keep docs in sync with code changes
- Wait for CI to pass before requesting review
- Require at least one review before merge

## Releases and Changelog

- Releases use `/release orrbeam <bump>` from `main`, where `<bump>` is `patch`, `minor`, or `major`
- The changelog is generated from conventional commits and then curated into `CHANGELOG.md`
- Do not cut ad hoc tags from feature branches

## API Docs Publish Step

`OPT-015` will wire `cargo doc --workspace --no-deps` into the release workflow. Once that lands, publish the API docs by creating the release-tagged build from `main`; the GitHub Actions release job will generate the docs artifact and push it to GitHub Pages. Until `OPT-015` is implemented, there is no supported manual publish path.
