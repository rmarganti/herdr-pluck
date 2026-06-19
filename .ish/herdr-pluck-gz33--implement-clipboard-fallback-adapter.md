---
# herdr-pluck-gz33
title: Implement clipboard fallback adapter
status: todo
type: task
priority: normal
tags:
- pluck
- clipboard
created_at: 2026-06-19T03:16:05.347232Z
updated_at: 2026-06-19T03:21:41.660778Z
parent: herdr-pluck-t3sf
blocked_by:
- herdr-pluck-jyye
---

## Context
Herdr Pluck v1 copies selected text through system-available clipboard tools. OSC52 and any Herdr-native clipboard/buffer API are out of scope. Clipboard behavior must be isolated so tests can fake command availability and command execution without writing to the real system clipboard.

Target fallback coverage from the PRD: macOS `pbcopy`; Linux Wayland tools such as `wl-copy`; Linux X11 tools such as `xclip` and `xsel`; clear failure when no usable tool is available.

## Dependencies
- Blocked by `herdr-pluck-jyye` for the Rust project scaffold and shared error/result types.

## Work
- Define a clipboard adapter API that accepts selected text and returns success or a useful user-facing error.
- Detect available commands in priority order appropriate to the platform/session.
- Implement copy execution by piping the selected text to the chosen command.
- Keep command lookup/execution injectable or abstracted for tests.
- Ensure failures are surfaced clearly to the picker rather than silently exiting as if copy succeeded.

## Verification
- Unit tests cover fallback command selection for macOS, Wayland, X11, and no-tool scenarios using fakes.
- Unit tests cover command execution success and failure without touching the real system clipboard.
- `cargo test` passes.


## Reference Code
Use tmux-fingers clipboard behavior as prior art for shelling out to available platform tools, while implementing Herdr Pluck's isolated/testable Rust adapter:

- `/Users/rmarganti/.cache/checkouts/github.com/Morantron/tmux-fingers/src/fingers/action_runner.cr`
- `/Users/rmarganti/.cache/checkouts/github.com/Morantron/tmux-fingers/src/tmux.cr`
