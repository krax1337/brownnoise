# Security Policy

## Supported versions

Security fixes are released for the latest minor version only. Upgrade to the latest release to receive fixes.

| Version | Supported |
| ------- | --------- |
| 0.1.x   | Yes       |

## Reporting a vulnerability

Please do not open public issues for security problems.

Report vulnerabilities through GitHub private vulnerability reporting: open the repository's
**Security** tab and choose **Report a vulnerability**
(<https://github.com/krax1337/brownnoise/security/advisories/new>).
If that is not possible, email [asanali@bulatov.dev](mailto:asanali@bulatov.dev).

Include the affected version, your operating system, a description of the issue and its impact,
and steps to reproduce.

This project is maintained by volunteers, so response times are best effort. The targets are:

- acknowledgement within 5 business days;
- an initial assessment within 10 business days;
- a fix or mitigation for confirmed issues as soon as practical, coordinated with the reporter before public disclosure.

## Verifying release artifacts

Releases are built from a tag by the [release workflow](.github/workflows/release.yml) on
GitHub-hosted runners. Each release attaches, next to the archives and `SHA256SUMS`:

| Asset | Contents |
| ----- | -------- |
| `SHA256SUMS.sigstore.json` | Keyless [cosign](https://github.com/sigstore/cosign) signature bundle for `SHA256SUMS` |
| `brownnoise-vX.Y.Z.intoto.jsonl` | SLSA build provenance for every archive, as a Sigstore bundle |
| `brownnoise-vX.Y.Z.spdx.json` | SPDX SBOM of the crate dependencies in `Cargo.lock` |

Signatures, provenance and SBOMs start with the first release after v0.1.0; v0.1.0 itself only ships
`SHA256SUMS`. The examples below use `v0.2.0` and the `aarch64-apple-darwin` archive; substitute the
version and archive you downloaded. They need the GitHub CLI 2.49 or later, or cosign v3 or later.

### With the GitHub CLI

Verify an archive against the provenance stored by GitHub:

```sh
gh attestation verify brownnoise-v0.2.0-aarch64-apple-darwin.tar.gz \
  --repo krax1337/brownnoise
```

The same check works without querying the attestation API, using the bundle attached to the release:

```sh
gh attestation verify brownnoise-v0.2.0-aarch64-apple-darwin.tar.gz \
  --repo krax1337/brownnoise \
  --bundle brownnoise-v0.2.0.intoto.jsonl
```

### With cosign

Verify the signature on `SHA256SUMS`, then check the archives you downloaded against it. This needs only
the files on the release page, not the GitHub API:

```sh
cosign verify-blob SHA256SUMS --bundle SHA256SUMS.sigstore.json \
  --certificate-identity-regexp '^https://github.com/krax1337/brownnoise/.github/workflows/release.yml@refs/tags/v' \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com
sha256sum --ignore-missing -c SHA256SUMS
```

On macOS without GNU coreutils, use `shasum -a 256 --ignore-missing -c SHA256SUMS` for the second command.
