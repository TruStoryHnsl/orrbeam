# Contributing to orrbeam

Thanks for being here. orrbeam is a small project with a narrow scope; the bar for contributions is "does it make the bidirectional mesh experience better on a real machine you actually use?"

## Filing an issue

- **Bug reports** and **feature requests** go through the issue forms — pick the right template. Blank issues are disabled on purpose; the templates exist so we can act on the report without three rounds of clarification.
- **Setup help** and **open-ended ideas** belong in [Discussions](https://github.com/TruStoryHnsl/orrbeam/discussions).
- **Security reports** go through the private advisory form linked in the issue picker — please do not file vulnerabilities as public issues.

## Opening a PR

1. **Branch off `main`.** Use a conventionally-named branch:

   ```
   feat/<short-slug>
   fix/<short-slug>
   refactor/<short-slug>
   chore/<short-slug>
   ```

   If multiple people might be working on similar areas in parallel, append a short suffix to disambiguate (e.g. `feat/peer-discovery-a3f9`). One session = one branch; don't append to someone else's WIP.

2. **Use Conventional Commits.** Every commit message follows:

   ```
   <type>[optional scope]: <description>
   ```

   Types we use: `feat`, `fix`, `docs`, `refactor`, `perf`, `test`, `chore`, `ci`, `build`. Breaking changes are `feat!:` / `fix!:` or carry a `BREAKING CHANGE:` footer.

3. **Make it pass locally before pushing.** The CI workflow runs the same set:

   ```bash
   cargo fmt --all -- --check
   cargo clippy --all-targets --all-features -- -D warnings
   cargo build --workspace
   cargo test --workspace
   cargo deny check
   cd frontend && npm install && npm run test
   ```

4. **Open the PR.** Fill in the template — particularly the test plan section. "Compiles" is not a test plan; describe what user-visible behavior you exercised, on which OS.

5. **One concern per PR.** Mixing a refactor and a feature in the same PR is the fastest way to slow review down. Split them.

## License rules (enforced in CI)

orrbeam is MIT-licensed and stays MIT-compatible. Every dependency we pull in must be permissive — no GPL, LGPL, or AGPL.

**Permitted Rust crate licenses:** MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, CC0-1.0.

**Permitted npm package licenses:** MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Unlicense, CC0-1.0.

**Not permitted:** GPL-2.0, GPL-3.0, AGPL-3.0, LGPL-2.0, LGPL-2.1, LGPL-3.0.

If `cargo deny check licenses` rejects a new dep, either pick a permissive alternative or add a `[[licenses.exceptions]]` entry in `deny.toml` with a written justification — and call that out in the PR description so it gets a real look.

## Style + scope

- New backend code in Rust. The frontend is React 19 + TypeScript; keep it that way unless there's a strong reason to introduce another runtime.
- Format and lint are non-negotiable: `cargo fmt`, `cargo clippy -D warnings`. Frontend uses the configured ESLint/Prettier.
- Tests aren't optional for behavior changes. Unit tests in the relevant crate are the floor; e2e tests under `tests/e2e/` for control-plane changes.
- Don't expand scope without discussion. orrbeam is "Sunshine + Moonlight in one app on a self-hosted mesh." Cloud features, account systems, and remote relay services are out of scope unless specifically discussed in an issue first.

## Code of conduct

Be civil. See [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md).
