# Contributing

Thanks for your interest in improving `brownnoise`. Bug reports, fixes, documentation, and feature
proposals are all welcome.

By participating you agree to follow the [Code of Conduct](CODE_OF_CONDUCT.md). Report security issues
privately as described in [SECURITY.md](SECURITY.md), not in public issues.

## Getting started

Requirements:

- Rust 1.85 or later (the `rust-version` in `Cargo.toml`), installed with [rustup](https://rustup.rs/)
- On Linux, the ALSA development files and `pkg-config`:

  | Distribution | Command |
  | ------------ | ------- |
  | Debian, Ubuntu | `sudo apt-get install libasound2-dev pkg-config` |
  | Fedora | `sudo dnf install alsa-lib-devel pkgconf-pkg-config` |
  | Arch Linux | `sudo pacman -S alsa-lib pkgconf` |

- Optional: [cargo-deny](https://github.com/EmbarkStudios/cargo-deny) (`cargo install --locked cargo-deny`
  or `brew install cargo-deny`) to check dependencies the same way CI does

macOS and Windows need nothing beyond the Rust toolchain.

| Command | Purpose |
| ------- | ------- |
| `cargo run -- --color pink` | Build and run a debug binary |
| `cargo build --release --locked` | Build the optimized binary into `target/release/` |
| `cargo test --locked` | Run the unit tests |
| `cargo fmt --all` | Format the code |
| `cargo clippy --all-targets --locked -- -D warnings` | Lint; CI treats every warning as an error |
| `cargo deny check` | Check advisories, licenses, duplicate crates and sources against `deny.toml` |
| `cargo +1.85 check --locked` | Check that the code still builds with the minimum supported Rust version |

## Making changes

1. Fork the repository and create a branch from `main`.
2. Make your change, with tests for behavior changes.
3. Add a line to `CHANGELOG.md` under `## [Unreleased]` for user-facing changes.
4. Run `cargo fmt --all`, `cargo clippy --all-targets --locked -- -D warnings`, and `cargo test --locked`,
   then open a pull request.

CI runs formatting, clippy and tests on Linux, macOS and Windows, an MSRV check, `cargo deny`,
[zizmor](https://docs.zizmor.sh/) on the workflows, and CodeQL.

### Dependencies

Keep the dependency tree small. A new dependency must pass `cargo deny check`: its license has to be on the
allowlist in `deny.toml`, and it must come from crates.io. Commit the updated `Cargo.lock`.

### Workflows

Pin every action to a full commit SHA with a version comment (`uses: owner/action@<sha> # vX.Y.Z`),
give each job only the permissions it needs, and pass untrusted values to `run:` steps through `env:`
rather than `${{ }}` interpolation. The `zizmor` CI job flags violations.

### Pull request titles

Use [Conventional Commits](https://www.conventionalcommits.org/) for pull request titles, for example
`feat: add --fade flag`, `fix(audio): reopen stream after device change`, or `docs: clarify Linux setup`.
Pull requests are squash-merged.

### Sign-off

Signing off your commits under the [Developer Certificate of Origin](https://developercertificate.org/)
is encouraged:

```sh
git commit -s -m "fix: keep volume when switching color"
```

## Release process

Releases are cut by the maintainer from `main`:

1. Open a pull request that:
   - sets `version` in `Cargo.toml` (for example `0.2.0`) and runs `cargo check` so `Cargo.lock` picks it up;
   - moves the `## [Unreleased]` entries in `CHANGELOG.md` into a new `## [0.2.0] - YYYY-MM-DD` section
     and updates the link references at the bottom of the file.
2. Merge the pull request once CI passes.
3. Tag the merge commit and push the tag:

   ```sh
   git tag -s v0.2.0 -m "v0.2.0"
   git push origin v0.2.0
   ```

The `Release` workflow does the rest. It fails early unless the tag is a semantic version (`vX.Y.Z` or
`vX.Y.Z-pre`), matches the `Cargo.toml` version, and has a `## [X.Y.Z]` section in `CHANGELOG.md`. It then:

- builds `brownnoise-vX.Y.Z-<target>.tar.gz` (or `.zip` on Windows) for macOS (Apple Silicon, Intel),
  Linux (x86_64, aarch64) and Windows (x86_64), without any build cache;
- writes `SHA256SUMS`, records SLSA build provenance for every archive (`brownnoise-vX.Y.Z.intoto.jsonl`),
  signs `SHA256SUMS` with keyless cosign (`SHA256SUMS.sigstore.json`), and generates an SPDX SBOM
  (`brownnoise-vX.Y.Z.spdx.json`);
- publishes the GitHub release with those files and the matching `CHANGELOG.md` section as release notes.
  Tags with a pre-release suffix (`v0.2.0-rc.1`) are published as pre-releases;
- for final releases, updates the formula in [krax1337/homebrew-tap](https://github.com/krax1337/homebrew-tap)
  from `packaging/homebrew/` when the `HOMEBREW_TAP_TOKEN` secret is set, and skips that step otherwise.

See [SECURITY.md](SECURITY.md#verifying-release-artifacts) for how users verify these artifacts.
