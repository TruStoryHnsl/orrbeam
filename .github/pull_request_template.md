## Summary

<!-- One paragraph: what does this PR do and why? -->

## Linked issue

Closes #

## Changes

<!-- Bullet list of what changed. Be specific about crates and files. -->

-

## Test plan

- [ ] `cargo build --workspace`
- [ ] `cargo test --workspace`
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] `cargo deny check` (if dependencies changed)
- [ ] `cd frontend && npm run build` passes
- [ ] `cd frontend && npm run test` (if frontend changed)
- [ ] Manual verification on at least one of: orrion (Linux), orrpheus (macOS), Windows, iOS, Android

Describe what you actually exercised — not just "compiles". What user-visible
behavior did you observe?

## Breaking changes

- [ ] None
- [ ] Yes — described below

If yes: what breaks, what migrates, and is the commit marked `feat!:` /
`fix!:` (or does it carry a `BREAKING CHANGE:` footer)?

## Checklist

- [ ] Branch name follows `feat/…`, `fix/…`, `refactor/…`, `chore/…`, etc.
- [ ] Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/) (`feat:`, `fix:`, `chore:`, etc.)
- [ ] No copyleft (GPL/LGPL/AGPL) dependencies introduced
- [ ] CHANGELOG.md updated under `[Unreleased]`
- [ ] Docs updated where behavior changed (or n/a)
- [ ] Tests added or updated where applicable (see CLAUDE.md test-authoring rules)
