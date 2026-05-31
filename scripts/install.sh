#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cd "$repo_root"
cargo install --path . --force

cat <<'MSG'
Installed appshots.

Try:
  appshots doctor --format json
  appshots capture --target active --presentation both --format json
  appshots latest
  appshots preview
MSG
