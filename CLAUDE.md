# CLAUDE.md - Claude Code Rules

## Project rules

- The `@AGENTS.md` import above loads the cross-harness rules into Claude Code.

## Memory handoff

The canonical memory owner on this repo is **claude**. Claude Code may keep local session context, but durable knowledge must be written as a Memory Handoff in `.claude/memory-handoffs/`. Full contract in `AGENTS.md`.

At the end of any substantial task, check whether the session produced durable knowledge. If yes, write a handoff using `.claude/memory-handoffs/TEMPLATE.md`. Do not wait to be reminded.

## Brigade work loop (Mandatory)

This repo is Brigade-wired. Route real work through Brigade so its outcome ledger fills. Invoke the `brigade-work` skill: `brigade work brief` at the start; run verifications via `brigade work verify run --target . --command "<test>" --capture brigade-work` when the result should count; handoff at the end. Atomic `--capture` records pass and failure receipts without a second command. If `brigade outcome rank` says "ranking: none", work is not flowing through Brigade. Full contract in `AGENTS.md`.

For large scoping work, invoke `ultra-work-scout` first. It is installed with the built-in Brigade skills and keeps Scout delegation tied to verified Brigade work.

## Closeout

- Report the exact verification command you ran.
- If verification could not run, state the blocker.
- If a Memory Handoff was warranted, confirm where it landed.

## Tool use

- Say it = call it. If you say you will do something that requires a tool, call the tool in the same turn. Silent intent is a lie.
- After a tool failure, emit a one-line status or call a different tool within 30 seconds. Do not silently reason for minutes.

## Git

- Do not add `Co-Authored-By` or AI-attribution trailers to commits, PR bodies, or public docs.
- Use conventional commits.
- Never bypass pre-push hooks (`--no-verify`) unless the user has explicitly accepted the risk.
- Never push to `main` directly on shared repos. Feature branch + PR.

## When in doubt

- Default to reading more before writing more.
- Ask one specific question rather than guess.
- Surface tradeoffs rather than presenting decisions as facts.

<!-- brigade:claude-work-loop:start -->
@AGENTS.md

## Brigade work loop (enforced)

Brigade hooks enforce this loop in every configured repository:

- Read the injected `brigade work brief` before real work.
- Run checks through `brigade work verify run --target . --command "<check>" --capture brigade-work`.
- Do not run tests, linters, type checks, builds, or verification scripts raw.
- After code or configuration changes, write a Memory Handoff before finishing.
- The same requirements apply inside subagents and after changing directories.
<!-- brigade:claude-work-loop:end -->
