## Summary

<!-- One paragraph: what does this PR do and why? -->

## Linked issue

Closes #

## Changes

<!-- Bullet list of what changed. Be specific about crates and files. -->

- 

## Test plan

<!-- How did you verify this works? What should a reviewer check? -->

- [ ]
- [ ]

## Checklist

- [ ] Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/) (`feat:`, `fix:`, `chore:`, etc.)
- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes
- [ ] `cargo build -p orrbeam-core -p orrbeam-net -p orrbeam-platform` succeeds
- [ ] Frontend: `cd frontend && npm run build` passes
- [ ] Tests added or updated where applicable (see CLAUDE.md test-authoring rules)
- [ ] CHANGELOG.md updated under `[Unreleased]`
- [ ] Docs updated if public API changed
