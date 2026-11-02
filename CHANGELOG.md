# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1] - 2026-11-02

### Changed

- README rewritten, with a demo of the terminal UI.

### Fixed

- The status line no longer switches to a narrower layout (dropping the
  "deep/bright" labels) when the quit countdown or timer appears.

### Security

- Releases attach `SHA256SUMS.sigstore.json`, a keyless cosign signature of
  `SHA256SUMS`, so archives can be verified from the release page alone.
- Releases carry SLSA build provenance for every archive, stored by GitHub and
  attached as `brownnoise-vX.Y.Z.intoto.jsonl` (`gh attestation verify`).
- Releases attach an SPDX SBOM of the crate dependencies,
  `brownnoise-vX.Y.Z.spdx.json`.
- CI runs CodeQL (Rust and GitHub Actions), OpenSSF Scorecard, `cargo deny`
  (advisories, licenses, duplicate crates, sources) and zizmor, plus an MSRV
  (1.85) check.
- Every GitHub Action is pinned to a commit SHA, workflows default to no token
  permissions, and release builds no longer use a dependency cache.
- Release notes come from this changelog, and the release fails unless the tag
  is a semantic version with a matching changelog section.

## [0.1.0] - 2026-11-02

### Added

- Endless, gapless white, pink and brown noise synthesized sample by sample on
  the audio thread — no audio files and no loop points.
- Independent left and right channels for a wide stereo image.
- Automatic switch to the new default output device (e.g. headphones or
  AirPods) when it changes.
- One-line terminal UI with live controls: volume (`↑`/`↓`, `+`/`-`), tone
  (`←`/`→`, `r` resets to neutral), color (`w`/`p`/`b`, `c` to cycle), pause
  (`space`) and quit (`q`, `Esc`, `Ctrl+C`) with a short fade-out shown as a
  countdown. Volume and tone sliders move one cell per step and show their
  exact value.
- `--color`, `--volume` and `--tone` options to set the starting sound.
- `--timer` option to stop after a duration (`45m`, `1h30m`, `90s`, or a bare
  number of minutes), fading out over the last 10 seconds.
- Color, volume and tone are remembered between runs in `config.toml` under the
  platform config directory.
- Headless mode when not attached to a terminal: plays until `SIGINT`/`SIGTERM`
  or the timer ends.
- Prebuilt binaries for macOS (Apple Silicon, Intel), Linux (x86_64, aarch64)
  and Windows (x86_64), plus a Homebrew tap.

[Unreleased]: https://github.com/krax1337/brownnoise/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/krax1337/brownnoise/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/krax1337/brownnoise/releases/tag/v0.1.0
