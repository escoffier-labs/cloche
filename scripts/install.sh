#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cd "$repo_root"
cargo install --path . --force --bins

cat <<'MSG'
Installed Cloche.

Try:
  cloche doctor --format json
  cloche capture --target active --presentation both --format json
  cloche latest
  cloche preview
MSG
