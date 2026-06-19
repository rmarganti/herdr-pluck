---
# herdr-pluck-1iof
title: Add renderer terminal smoke output path
status: todo
type: task
priority: high
tags:
- pluck
- renderer
- smoke
created_at: 2026-06-19T19:16:36.267406Z
updated_at: 2026-06-19T19:16:36.267406Z
parent: herdr-pluck-t3sf
blocking:
- herdr-pluck-guwf
blocked_by:
- herdr-pluck-z4cv
---

## Context
After the abstract renderer is implemented, add a small terminal-visible smoke path before the full picker input loop. This should let local Herdr overlay testing show real matched inline hints using the renderer without yet implementing hint typing/copy behavior.

This is intentionally separate from the input-loop/copy-flow task so rendering can be manually validated sooner.

## Dependencies
- Blocked by `herdr-pluck-z4cv` for abstract renderer output and style semantics.

## Work
- Add a CLI or picker-placeholder path that reads target pane text and geometry when available, runs matching + hint assignment + rendering, and displays the rendered hint view in the overlay.
- Convert abstract render styles to terminal output with simple v1 styling: dim/black non-matches, white matches, cyan hints.
- Keep behavior non-interactive or minimally interactive: show output and wait for a key to close; do not implement hint entry or clipboard copy here.
- Preserve the existing frozen source-geometry behavior and compose rendered source output into the overlay viewport where applicable.
- Document exact smoke-test steps for invoking the linked Herdr plugin and visually confirming inline hints.

## Verification
- Unit tests cover style-to-terminal/emission helpers where practical without requiring a real terminal.
- `cargo fmt --all -- --check` passes.
- `cargo test --all-features` passes.
- `cargo clippy --all-targets --all` passes.
- Manual Herdr smoke test displays rendered inline hints and closes on keypress.
