---
# herdr-pluck-gz33
title: Implement clipboard fallback adapter
status: completed
type: task
priority: normal
tags:
- pluck
- clipboard
created_at: 2026-06-19T03:16:05.347232Z
updated_at: 2026-06-21T19:26:58.286482Z
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
Use tmux-fingers clipboard behavior as prior art for shelling out to available platform tools, while implementing Herdr Pluck's isolated/testable Rust adapter. Online repo: https://github.com/Morantron/tmux-fingers

- [`src/fingers/action_runner.cr`](https://github.com/Morantron/tmux-fingers/blob/master/src/fingers/action_runner.cr)
- [`src/tmux.cr`](https://github.com/Morantron/tmux-fingers/blob/master/src/tmux.cr)


## Implementation Notes
- Replaced the flat clipboard module with a deep `src/clipboard/` module tree and a small public interface: `Clipboard`, `SystemClipboard`, `CopySuccess`, `ClipboardError`, and `copy_to_system_clipboard`.
- Added platform/session-aware fallback ordering for `pbcopy`, `wl-copy`, `xclip -selection clipboard`, and `xsel --clipboard --input`, while still trying supported fallbacks when session hints are absent.
- Added an injectable command-runner boundary so command discovery and copy execution can be tested without touching the real system clipboard.
- Implemented system execution by piping selected text to the chosen command's stdin and surfacing spawn, write, wait, command-status, and no-tool failures as user-facing `ClipboardError`s.
- Removed the clipboard-specific `CopyResult` from global `model.rs`; clipboard result/error types now live in the clipboard domain.

## Verification Results
- `cargo fmt --all -- --check` passed.
- `cargo test --all-features` passed.
- `cargo clippy --all-targets --all` passed.
- `ish check` passed.
