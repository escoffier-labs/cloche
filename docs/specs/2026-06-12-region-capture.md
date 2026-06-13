# Spec: region capture target and clipboard flag

Date: 2026-06-12. Status: approved.

## Purpose

Let a human press a hotkey, drag a region of the screen, and get the polished
Cloche card on their clipboard, using nothing but cloche plus their desktop's
hotkey settings. Today `capture` only knows whole surfaces (active, window,
screen); region selection requires external glue scripts.

## Design

### `capture --target region`

`region` becomes the fourth `CaptureTarget` variant. It reuses the entire
existing capture contract: `shot.png`, `shot-card.png`, `metadata.json`,
optional `text.txt`, stable JSON on stdout, and automatic pickup by
`gallery`/`latest`/`preview`. `window` metadata is absent (a region is not a
window); frame extents do not apply.

Linux selector chain, mirroring the existing helper-discovery pattern:

1. **Flameshot** when available: `flameshot gui --accept-on-select --path
   <shot.png>`. The shot is taken the moment the drag is released. Critical
   gotcha encoded here: when a Flameshot daemon is already running, the
   capture is delegated and `--raw` emits on the daemon's stdout, so a file
   path is the only reliable transport. The file may land a beat after the
   client exits, so poll briefly for it.
2. **ImageMagick `import`** (X11 only) as fallback: `import <shot.png>` with
   no window argument is an interactive drag-select. `import` is already a
   required Linux helper for cloche.
3. Neither available: error instructing the user to install Flameshot (or
   ImageMagick on X11).

An aborted selection (Esc, no file produced) is an error in the JSON contract
(`ok: false`, `errors: ["region selection aborted or produced no image"]`),
so hotkey scripts can exit quietly on it.

Windows: explicit unsupported error for v1, pointing at Win+Shift+S followed
by `cloche polish <file>`. No untestable platform code ships.

Region capture requires a human to drag; the MCP schema description must say
so, since agents calling `capture` headlessly would otherwise hang the
selector until its own abort path fires.

### `capture --clipboard`

Opt-in flag. After a successful capture it copies the presentation card (or
the raw shot when `--presentation raw`) to the system clipboard as
`image/png`, shelling out per the no-new-dependencies rule:

- Wayland session (`WAYLAND_DISPLAY` set and `wl-copy` available): `wl-copy
  --type image/png < file`
- Otherwise `xclip -selection clipboard -t image/png -i <file>`
- No helper or copy failure: a warning in the JSON, never an error; the
  capture itself succeeded. Windows: warning that clipboard copy is not
  supported yet.

### MCP

The `capture` tool schema gains `"region"` in the target enum and a
`clipboard` boolean that maps to `--clipboard`.

### Docs

README gains a "Hotkey workflow" section: bind `cloche capture --target
region --presentation both --clipboard --out-dir ...` to a key (GNOME custom
shortcut, KDE shortcut, etc.). The hotkey binding itself stays user-side; a
CLI cannot portably own a global hotkey.

## Testing

- Unit: `region` serializes to the wire as `"region"`; MCP arg mapping for
  `target: region` and `clipboard: true`; clipboard command selection logic.
  No test may require a display (repo rule).
- Live (maintainer machine): `capture --target region --clipboard` driven by
  an xdotool synthetic drag; verify card file, clipboard contents, and JSON.
- Abort path: selector dismissed with Esc yields `ok: false` and exit 1.

## Out of scope

- Windows region selection (needs an interactive-session selector story).
- `polish --clipboard` (symmetric, trivial to add later if wanted).
- Owning global hotkeys from the CLI.
