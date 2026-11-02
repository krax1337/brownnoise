#!/bin/sh
# Render the Homebrew formula for a release and print it to stdout.
#
#   sh packaging/homebrew/render.sh <version> <SHA256SUMS path>
#
# <version> may be given with or without a leading "v" (0.1.0 or v0.1.0).
# <SHA256SUMS path> is the SHA256SUMS file attached to the GitHub Release.
set -eu

usage() {
    echo "usage: $0 <version> <SHA256SUMS path>" >&2
    exit 2
}

[ "$#" -eq 2 ] || usage

version=${1#v}
sums=$2
template="$(dirname "$0")/brownnoise.rb.tmpl"

case $version in
    '' | *[!0-9A-Za-z.+-]*)
        echo "error: invalid version '$1'" >&2
        exit 1
        ;;
esac
[ -r "$sums" ] || { echo "error: cannot read '$sums'" >&2; exit 1; }
[ -r "$template" ] || { echo "error: cannot read '$template'" >&2; exit 1; }

# Print the sha256 recorded for the archive of the given target.
sha_for() {
    archive="brownnoise-v$version-$1.tar.gz"
    sha=$(awk -v f="$archive" '{ name = $2; sub(/^\*/, "", name) } name == f { print $1; exit }' "$sums")
    case $sha in
        '' | *[!0-9a-f]*)
            echo "error: no valid sha256 for $archive in $sums" >&2
            exit 1
            ;;
    esac
    [ "${#sha}" -eq 64 ] || { echo "error: malformed sha256 for $archive" >&2; exit 1; }
    printf '%s\n' "$sha"
}

macos_arm=$(sha_for aarch64-apple-darwin)
macos_intel=$(sha_for x86_64-apple-darwin)
linux_arm=$(sha_for aarch64-unknown-linux-gnu)
linux_intel=$(sha_for x86_64-unknown-linux-gnu)

sed \
    -e "s|@VERSION@|$version|g" \
    -e "s|@SHA256_MACOS_ARM@|$macos_arm|g" \
    -e "s|@SHA256_MACOS_INTEL@|$macos_intel|g" \
    -e "s|@SHA256_LINUX_ARM@|$linux_arm|g" \
    -e "s|@SHA256_LINUX_INTEL@|$linux_intel|g" \
    "$template"
