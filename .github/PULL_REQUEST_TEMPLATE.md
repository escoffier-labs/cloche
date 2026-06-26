<!--
Thanks for sending a patch. Keep this short; delete sections that do not apply.
See CONTRIBUTING.md for what lands easily and what needs an issue first.
-->

## What and why

<!-- One or two sentences on the user-visible change and the problem it solves. -->

Closes #

## Type of change

- [ ] Bug fix
- [ ] Capture backend / desktop coverage
- [ ] Docs
- [ ] Refactor with no command-surface change
- [ ] Surface change (new command, flag, or change to the JSON output contract) - opened an issue first per CONTRIBUTING.md

## Checklist

- [ ] `cargo test` passes locally
- [ ] `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` pass
- [ ] Updated the `Unreleased` section of `CHANGELOG.md` for any user-visible effect (entries describe effects, not commit subjects)
- [ ] No personal details, hostnames, IPs, account names, tokens, or unredacted absolute paths in code, docs, tests, or committed sample images (the content-guard check will flag them)
- [ ] No new runtime dependencies without justification (Cloche keeps a lean dependency tree)
- [ ] Conventional commit messages, no AI co-authorship trailers
