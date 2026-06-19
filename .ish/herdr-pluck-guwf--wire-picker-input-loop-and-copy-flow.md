---
# herdr-pluck-guwf
title: Wire picker input loop and copy flow
status: todo
type: task
priority: high
tags:
- pluck
- picker
- input
created_at: 2026-06-19T03:16:25.652609Z
updated_at: 2026-06-19T03:16:25.652609Z
parent: herdr-pluck-t3sf
blocked_by:
- herdr-pluck-obnm
- herdr-pluck-94xq
- herdr-pluck-z4cv
- herdr-pluck-gz33
- herdr-pluck-vhdh
---

## Context
This is the end-to-end picker mode for Herdr Pluck v1. It runs inside the overlay pane, reads the explicit target pane id from the environment, reads the live bottom viewport, matches tokens, assigns hints, renders the inline view, accepts keyboard hints, copies selected text, and exits.

The interaction is copy-only and keyboard-only. Exact fixed-width hint entry copies immediately and closes. Escape and Ctrl-C cancel. Enter does nothing. Invalid fixed-width hints clear the input buffer and keep the picker active. Mouse support and non-copy actions are out of scope.

## Dependencies
- Blocked by `herdr-pluck-obnm` for pattern matching and deduplication.
- Blocked by `herdr-pluck-94xq` for fixed-width hint assignment and lookup.
- Blocked by `herdr-pluck-z4cv` for overlay rendering output.
- Blocked by `herdr-pluck-gz33` for clipboard copy execution.
- Blocked by `herdr-pluck-vhdh` for target pane read/layout integration.

## Work
- Implement picker mode orchestration: obtain target pane id, layout dimensions, and recent unwrapped text via the Herdr adapter.
- Run pattern engine, hint engine, and renderer to draw the overlay view.
- Implement raw terminal keyboard input loop and buffer handling for fixed-width hints.
- On exact hint, copy the associated text through the clipboard adapter and exit immediately on success.
- On clipboard failure, display or report a clear failure rather than pretending success; choose a simple v1 behavior consistent with overlay UX.
- Handle Escape and Ctrl-C as cancellation, Enter as ignored, invalid full-width hint as buffer clear, and other character input predictably.
- Keep input behavior testable with faked input events and fake copy actions.

## Verification
- Behavior tests cover exact hint copying, duplicate hint copy target, invalid hint clearing, Escape cancel, Ctrl-C cancel, Enter ignored, and clipboard failure handling.
- Integration-style tests with fake Herdr adapter + fake clipboard validate the full picker decision flow from pane text to copied string.
- `cargo test` passes.
