# Spec: `cloche setup` onboarding and verification

Date: 2026-06-17. Status: approved.

## Purpose

Let a new Linux user go from a fresh `cargo install cloche` to a working
one-press screenshot hotkey *and* a working agent integration with a single
guided command that proves, at the end, that both actually work. Today every
piece exists but is manual: install the grab script, edit desktop keyboard
settings, hand-move the native screenshot binding, then paste an MCP JSON
snippet into each agent's config by reading the README. Nothing confirms the
result.

The audience is sharp: Codex desktop Appshots does not exist on Linux and
Claude Code has no Appshots at all, so Linux users of Claude Code, OpenClaw,
and Codex CLI have no built-in capture path. `cloche setup` is that path.

## Non-Goals

- No Windows Print-key binding (Windows already has Win+Shift+S). On Windows,
  `setup` runs the agent-registration and verification steps and prints
  hotkey guidance instead of binding a key.
- No new capture, polish, or reels behavior. This is wiring and verification
  around existing commands.
- No GUI. Terminal only, stable JSON preserved.

## Design

### Command surface

A new `setup` subcommand group, dispatched from `cli.rs` into a new `setup`
module so `cli.rs` only declares args.

- `cloche setup` — guided full run: detect → print plan → confirm `[y/N]` →
  apply → verify → summary. `onboard` is an alias.
- `cloche setup hotkey` — only the Print-key piece.
- `cloche setup agent [--client <id>]` — only MCP registration. With no
  `--client`, auto-detects every installed client and configures each.
  `<id>` ∈ `claude-code | openclaw | codex | print`.
- `cloche setup verify` — re-run the confirmation checks standalone. This is
  exactly what `setup` runs at its end.

Global flags on every `setup` command:

- `--yes` — apply without the confirmation prompt (for scripts/CI).
- `--print` — dry-run: detect and print every command/edit it *would* make,
  change nothing. Implies no prompt.
- `--format <text|json>` — `text` is the human default for `setup`; `json`
  emits a stable machine object (see Contract). All other cloche commands
  default to `json`; `setup` defaults to `text` because its primary caller is
  a human, but `--format json` is always available.

Safety rules for all mutations:

- **Confirm-first** unless `--yes`. The plan printed before the prompt lists
  every file and setting that will change.
- **Backup-before-edit**: any config file edited in place is first copied to
  `<file>.cloche.bak` (overwritten on re-run so backups do not accumulate).
- **Idempotent**: re-running detects existing cloche entries/bindings and
  updates them in place or skips them; it never duplicates.

### 1. Hotkey automation

Detect the desktop session by extending the existing `session_info`
(`XDG_CURRENT_DESKTOP`, `XDG_SESSION_TYPE`, `WAYLAND_DISPLAY`/`DISPLAY`) into a
`Desktop` enum: `Gnome | Kde | Sway | I3 | Other`.

Common to all desktops:

- Install `scripts/cloche-grab.sh` to `~/.local/bin/cloche-grab` (`0755`). The
  script content is embedded in the binary via `include_str!` so an installed
  cloche with no source checkout can still lay it down. If `~/.local/bin` is
  not on `PATH`, emit a warning with the line to add to the shell profile.

Per-desktop binding:

- **GNOME** — auto-bind via `gsettings`. Append one entry to
  `org.gnome.settings-daemon.plugins.media-keys custom-keybindings`, then set
  that entry's `name=Cloche Grab`, `command=cloche-grab`, `binding=Print`.
  Idempotent: if an entry whose `command` is `cloche-grab` already exists,
  reuse it instead of adding a second. Because `Print` is owned by the native
  screenshot UI, also offer (separate y/N) to move it:
  `gsettings set org.gnome.shell.keybindings show-screenshot-ui "['<Shift>Print']"`.
- **KDE / sway / i3 / Other** — auto-binding is fragile (KDE writes
  `kglobalshortcutsrc` + needs a daemon reload; WMs need a config-file edit
  the user must reload). For these, **print** the exact copy-pasteable steps
  (the README's existing per-desktop instructions, parameterized with the
  resolved `cloche-grab` path) and mark the hotkey step as `manual` in the
  result. Honest reporting: the summary says which desktops were automated
  versus printed.

### 2. Agent / MCP registration

The MCP server is unchanged: `cloche mcp` already speaks stdio JSON-RPC and
exposes `capture`, `polish`, `list_windows`, `doctor`, `latest`, `gallery`.
Registration just writes the standard `{ "command": "cloche", "args":
["mcp"] }` entry into each client's config, keyed as `cloche`.

Detection: a client is "present" if its config dir/binary exists.

- **Claude Code** — if `claude` is on `PATH`, prefer the official CLI:
  `claude mcp add cloche -s user -- cloche mcp`. Otherwise edit
  `~/.claude.json` and add `cloche` under `mcpServers`.
- **Codex CLI** — edit `~/.codex/config.toml`, adding:
  ```toml
  [mcp_servers.cloche]
  command = "cloche"
  args = ["mcp"]
  ```
- **OpenClaw** — add an MCP entry to `~/.openclaw/openclaw.json`. The exact
  schema (MCP server block shape and key) is confirmed against an installed
  OpenClaw during implementation; if the schema cannot be confirmed safely,
  OpenClaw falls back to **print-only** rather than risk corrupting the
  gateway config.
- **Generic / `--client print`** — for any other client, or as the universal
  fallback, print the JSON snippet and the absolute path/command the user
  should add. Edits nothing.

Every in-place edit is backed up and idempotent per the safety rules. A
malformed existing config (unparseable JSON/TOML) is never overwritten: the
client is reported as `error` with a message and the print fallback is shown.

### 3. Verification — the "confirm it works" part

Three independent checks, each `pass | fail | skip`, each with a one-line
remediation hint on failure. `setup verify` runs all three; `setup` runs them
after applying.

1. **Capture pipeline** — run a real, non-interactive `capture --target
   screen --presentation both` into a temp dir. Pass iff `shot.png` and
   `shot-card.png` exist and are non-empty; then delete the temp dir. This
   exercises the full capture → polish chain on the user's actual machine, not
   just helper presence. Skipped (not failed) when no GUI session is present,
   with a clear reason.
2. **Hotkey** — `cloche-grab` resolves on `PATH`; on GNOME, a custom
   keybinding whose command is `cloche-grab` is registered. On non-GNOME
   desktops this reports `skip` ("binding is manual on <desktop>") because we
   cannot reliably read every WM's config.
3. **Agent / MCP** — spawn `cloche mcp` as a subprocess and perform a real
   JSON-RPC handshake: `initialize`, then `tools/list`. Pass iff the response
   lists `capture` and `polish`. Additionally, for each client that `setup`
   configured, confirm its config file now contains the `cloche` entry. This
   is the actual proof an LLM can call cloche.

### Module layout

- `src/setup.rs` — `SetupArgs`/dispatch, the guided flow, plan printing,
  confirmation prompt, summary, JSON contract.
- `src/setup/hotkey.rs` — desktop detection, grab-script install, GNOME
  gsettings binding, per-desktop print instructions.
- `src/setup/agents.rs` — per-client detection and registration, backups,
  idempotency, print fallback.
- `src/setup/verify.rs` — the three checks, reused by `setup` and `setup
  verify`.

`cli.rs` gains the `Setup(SetupArgs)` command variant and a thin dispatch
arm. The embedded grab script is shared with the existing repo script via a
single source of truth (the repo script is the canonical copy;
`include_str!` pulls it in at build time).

### JSON contract

`--format json` on any `setup` command emits:

```json
{
  "ok": true,
  "mode": "setup",
  "applied": ["hotkey:gnome-binding", "agent:claude-code"],
  "skipped": ["agent:codex"],
  "printed": ["hotkey:kde-instructions"],
  "backups": ["/home/u/.claude.json.cloche.bak"],
  "checks": [
    { "name": "capture-pipeline", "status": "pass", "detail": "..." },
    { "name": "hotkey", "status": "pass", "detail": "..." },
    { "name": "agent-mcp", "status": "pass", "detail": "..." }
  ],
  "warnings": [],
  "errors": []
}
```

`ok` is `true` iff no `errors` and no check is `fail` (a `skip` does not fail
the run). `--print` runs report what *would* happen with an `applied` list
describing planned actions and no `backups`.

## Testing

- Desktop detection maps `XDG_CURRENT_DESKTOP`/session-type combinations to
  the right `Desktop` variant (unit, env-injected).
- Agent registration is idempotent: registering twice yields one `cloche`
  entry; a backup is written; a pre-existing non-cloche `mcpServers`/
  `mcp_servers` block is preserved. Tested against temp JSON and TOML fixtures.
- Malformed existing config is reported as `error` and never overwritten.
- `--print` mutates nothing (assert config files and gsettings unchanged).
- The MCP self-test handshake parses a real `cloche mcp` `tools/list`
  response and finds `capture`/`polish` (integration test, gated on the
  binary building).
- The capture-pipeline check `skip`s cleanly with no GUI and does not error.
- GNOME gsettings binding is exercised behind a feature/env gate so CI without
  a session does not require dbus; the path-construction and idempotency
  logic is unit-tested without invoking `gsettings`.

## Rollout

- `cloche setup` is additive; no existing command changes behavior.
- README "Hotkey Workflow" and "MCP Server" sections gain a top line: "The
  fastest path is `cloche setup`." The manual steps stay as the explanation of
  what setup automates and the fallback for unsupported desktops.
- CHANGELOG `Unreleased` entry. No release cut unless requested.
