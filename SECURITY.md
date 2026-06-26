# Security Policy

## Supported versions

Cloche is pre-1.0 and moving fast. Only the latest release on crates.io and the `master` branch receive security fixes. Pin to a released version if you need a known-good build.

## Reporting a vulnerability

Please **do not** open a public GitHub issue for security problems. Email **me@solomonneas.dev** with: <!-- content-guard: allow pii/email -->

- A short description of the issue.
- Steps to reproduce (or a minimal proof of concept).
- The version or commit you tested against (`cloche --version`).
- Whether you would like to be credited in the release notes.

You should get an acknowledgment within 72 hours. If you do not, please follow up - the mail may have been filtered.

## In scope

- Code execution or path-traversal flaws in `cloche capture`, `polish`, `reels render`, `gallery`, or `setup`.
- Output that leaks credentials, tokens, or unexpected on-screen content into a written artifact (`metadata.json`, `text.txt`, a shot card, or an exported HTML gallery).
- `cloche setup` writing outside the files it documents, or corrupting an agent config it edits without a backup.
- The `cloche mcp` server mishandling a malformed JSON-RPC request in a way that executes unintended commands.

## Capture is only as private as your screen

Cloche captures whatever is on your display: the active window, a selected window, the full screen, or an interactive region. A capture can include tokens, private messages, or other sensitive content that happened to be visible. Treat shot cards, raw images, extracted `text.txt`, and exported galleries as you would any screenshot. Review what is on screen before you share an artifact, especially `--target screen` and gallery exports that embed every image inline.

`cloche setup` edits local agent config files (Claude Code, OpenClaw, Codex CLI). It backs up any file it touches and skips clients already configured, but the edits run on your machine with your permissions. Run `cloche setup --print` first to preview every change.

## Out of scope

- Issues that require an attacker to already have write access to your machine, your desktop session, or your agent config.
- Behavior of the desktop tools Cloche shells out to (`grim`, `flameshot`, ImageMagick `import`, `xdotool`, `wmctrl`, `wl-copy`, `xclip`). Report those to their respective projects.
- Bugs in Remotion, HyperFrames, or Node tooling used by `cloche reels render`. Report those upstream.
- Content you captured and chose to share yourself. Cloche frames what is on your screen; it does not redact it.

## Disclosure

We aim to ship a fix within 14 days of confirming a valid report. A coordinated disclosure timeline can be negotiated for issues that need longer.
