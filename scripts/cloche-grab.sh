#!/usr/bin/env bash
#
# cloche-grab: select a region of the screen, polish it into a Cloche card,
# and copy the card to the clipboard. Bind this to a key to get a one-press
# "screenshot -> share-ready card" flow. See the README "Hotkey Workflow".
#
# Requirements:
#   - cloche on PATH
#   - a region selector: flameshot (any session) or ImageMagick (X11)
#   - optional: a clipboard helper (wl-copy on Wayland, xclip on X11)
#   - optional: notify-send for the desktop notification
#
# Override the output location with CLOCHE_SHOTS_DIR.
set -euo pipefail

shots_dir="${CLOCHE_SHOTS_DIR:-$HOME/Pictures/ClocheShots}"
out_dir="$shots_dir/grab-$(date +%Y%m%d-%H%M%S)"

notify() {
  command -v notify-send >/dev/null 2>&1 || return 0
  notify-send -a Cloche "$@"
}

if cloche capture --target region --presentation both --clipboard \
    --out-dir "$out_dir" --format json >/dev/null 2>&1; then
  card="$out_dir/shot-card.png"
  [ -f "$card" ] && notify -i "$card" "Cloche card copied to clipboard" "$card"
else
  # Aborted selection (Esc) lands here too. Only complain if a raw shot was
  # actually written; otherwise clean up and stay silent.
  if [ -s "$out_dir/shot.png" ]; then
    notify "Cloche grab failed" "Raw screenshot kept in $out_dir"
    exit 1
  fi
  rm -rf "$out_dir" 2>/dev/null || true
fi
