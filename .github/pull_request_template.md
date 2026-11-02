## Summary

<!-- What does this change and why? Link related issues (e.g. "Fixes #123"). -->

## Checklist

- [ ] PR title follows [Conventional Commits](https://www.conventionalcommits.org/) (`feat:`, `fix:`, `docs:`, `chore:`, ...)
- [ ] `cargo fmt --all --check` passes
- [ ] `cargo clippy --all-targets --locked -- -D warnings` passes
- [ ] `cargo test --locked` passes, with tests added or updated for behavior changes
- [ ] New or updated dependencies pass `cargo deny check`
- [ ] User-facing changes noted in `CHANGELOG.md` under `## [Unreleased]`
- [ ] Commits are signed off (`git commit -s`)
