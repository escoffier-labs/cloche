#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

version="$(grep '^version =' Cargo.toml | head -n 1 | cut -d '"' -f 2)"
kernel="$(uname -s | tr '[:upper:]' '[:lower:]')"
arch="$(uname -m)"

case "$kernel:$arch" in
  linux:x86_64) target="x86_64-unknown-linux-gnu" ;;
  linux:aarch64 | linux:arm64) target="aarch64-unknown-linux-gnu" ;;
  *)
    echo "unsupported release target: ${kernel}/${arch}" >&2
    exit 1
    ;;
esac

cargo build --release --bin cloche --bin appshots

stage="dist/cloche-${version}-${target}"
archive="${stage}.tar.gz"
rm -rf "$stage" "$archive"
mkdir -p "$stage"

cp target/release/cloche "$stage/cloche"
cp target/release/appshots "$stage/appshots"
cp LICENSE README.md ROADMAP.md "$stage/"

tar -C dist -czf "$archive" "$(basename "$stage")"
echo "$archive"
