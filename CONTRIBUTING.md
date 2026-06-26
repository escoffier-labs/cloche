# Contributing to Cloche

Cloche is an open-source desktop capture CLI for people and agents: it captures the active app or window, writes polished artifacts plus metadata, and prints stable JSON. Patches are welcome. Before you start, please skim this file so we both spend our time on the right things.

## What kinds of changes land easily

- **Bug fixes** in capture backends, `polish`, `gallery`, `setup`, the MCP server, or the JSON contract.
- **Backend improvements**: better Wayland/X11/Windows window detection, more reliable text extraction, clearer `doctor` diagnostics.
- **Desktop coverage**: hotkey-bind instructions or `setup` support for a desktop environment that is not yet handled.
- **Docs and examples** that match the real CLI signatures.
- **Test coverage** for any of the above.

## What needs a conversation first

- **New top-level commands or breaking changes to the JSON contract** (`AppshotResult`, `PolishResult`, `ReelRenderResult`). Agents and scripts depend on stable stdout; open an issue first describing the user story.
- **New runtime dependencies.** Cloche keeps its dependency tree lean on purpose (every dependency builds with `default-features = false` and an explicit feature list). Adding one needs justification.
- **Renaming or removing the `appshots` compatibility alias.** It exists for users mid-migration.

## What does not land

- Personal details, hostnames, IPs, account IDs, or live auth profiles in code, tests, docs, or committed sample images. The whole point of a capture tool is that it is safe to publish its artifacts; keep that stuff out of the repo. The `content-guard` check will flag any.
- Code that phones home, uploads captures, or calls the network without an explicit, documented opt-in. Cloche runs locally and writes local files.
- AI-co-authorship trailers on commits (`Co-Authored-By: <model>`). Conventional commits only.

## Local dev

Cloche needs Rust 1.88 or newer (the `image` crate sets the MSRV). Distro `cargo` packages can lag; install a current toolchain with [rustup](https://rustup.rs) if needed.

```bash
git clone https://github.com/escoffier-labs/cloche.git
cd cloche
cargo build
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

Run the CLI straight from the checkout:

```bash
cargo run -- doctor --format json
cargo run -- polish path/to/shot.png --format json
```

`cloche doctor --format json` is the fastest way to see which capture backends are available on your machine. The reel engines (`cloche reels render`) need Node tooling: Remotion under `remotion/` (`cd remotion && npm install`) or `npx hyperframes` for the `--engine hyperframes` path.

## Architecture map

- `src/cli.rs` is the clap command surface and the dispatch for every subcommand.
- `src/capture/` holds the per-platform capture backends (`linux.rs`, `windows.rs`); `backends` picks one at runtime.
- `src/polish.rs` renders the presentation card (gradient backdrop, rounded window, shadow) and owns the palette/style-seed system shared by stills and reels.
- `src/contract.rs` defines the JSON output types. Change these carefully; they are the public contract.
- `src/setup/` is the guided `cloche setup` flow (hotkey, agent config, verify).
- `src/mcp.rs` is the stdio MCP server; it shells out to the same binary so its JSON matches the CLI exactly.

## Filing issues

Please use the templates under `.github/ISSUE_TEMPLATE/`. The most useful capture-backend report includes the output of:

```bash
cloche doctor --format json
```

Before posting output, remove tokens, private hostnames, private account names, and unredacted absolute paths. Good labels are `capture`, `backend`, `setup`, `mcp`, and `docs`.

## License

By contributing you agree that your contribution is licensed under the Apache License 2.0, the same license as the rest of the repo.
