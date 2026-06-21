---
# herdr-pluck-guwf
title: Wire picker input loop and copy flow
status: completed
type: task
priority: high
tags:
- pluck
- picker
- input
created_at: 2026-06-19T03:16:25.652609Z
updated_at: 2026-06-21T19:37:44.663361Z
parent: herdr-pluck-t3sf
blocked_by:
- herdr-pluck-obnm
- herdr-pluck-94xq
- herdr-pluck-z4cv
- herdr-pluck-gz33
- herdr-pluck-vhdh
- herdr-pluck-1iof
---

## Context
This is the end-to-end picker mode for Herdr Pluck v1. It runs inside the overlay pane, reads the explicit target pane id from the environment, reads the live bottom viewport, matches tokens, assigns hints, renders the inline view, accepts keyboard hints, copies selected text, and exits.

The interaction is copy-only and keyboard-only. Exact fixed-width hint entry copies immediately and closes. Escape and Ctrl-C cancel. Enter does nothing. Invalid fixed-width hints clear the input buffer and keep the picker active. Mouse support and non-copy actions are out of scope.

## Dependencies
- Blocked by `herdr-pluck-obnm` for pattern matching and deduplication.
- Blocked by `herdr-pluck-94xq` for fixed-width hint assignment and lookup.
- Blocked by `herdr-pluck-z4cv` for abstract overlay rendering output.
- Blocked by `herdr-pluck-1iof` for terminal smoke output before full picker input/copy wiring.
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



## Implementation Notes
- Replaced the readonly picker entrypoint used by snapshot mode with the production input/copy flow.
- Deepened `src/picker/` into focused modules for rendering, input event conversion, input sources/raw-mode, fixed-width input state, clipboard copy mapping, and session orchestration.
- Added `PickerView` so the renderer path returns both terminal render lines and hint assignments for copy lookup.
- Removed the old top-level `input` module from the public crate surface; picker input internals are now private to `picker`.
- Implemented fixed-width keyboard behavior: exact hints copy immediately, invalid full-width hints clear the buffer, Enter/other events are ignored, and Escape/Ctrl-C cancel.
- Wired exact hint lookup to copy the matched text through the clipboard adapter, not the typed hint string, and render/report clipboard failures instead of treating them as success.
- Kept the picker flow testable with fake input sources, fake clipboard implementations, and in-memory terminal output.

## Verification Results
- `cargo fmt --all -- --check` passed.
- `cargo test --all-features` passed.
- `cargo clippy --all-targets --all` passed.
- `cargo build --release` passed.
- `ish check` passed.
- Manual Herdr smoke: created a temporary source tab containing `https://example.com/pluck-smoke-1782070597`, launched `./target/release/herdr-pluck open --target-pane wE:p8D`, typed hint `a` in the focused Herdr Pluck temp pane, verified `pbpaste` contained the selected URL, and confirmed the temp tab closed/focus returned before cleaning up the smoke source tab.
