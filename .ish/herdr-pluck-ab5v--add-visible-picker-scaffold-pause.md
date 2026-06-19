---
# herdr-pluck-ab5v
title: Add visible picker scaffold pause
status: completed
type: task
priority: normal
tags:
- pluck
- herdr
- smoke
created_at: 2026-06-19T03:57:52.350986Z
updated_at: 2026-06-19T03:58:23.443288Z
parent: herdr-pluck-t3sf
blocked_by:
- herdr-pluck-c6k7
---

## Context
The temporary picker scaffold exits immediately, making the configured keybinding look like only a UI flash. Keep the scaffold visible until the user presses a key so manual invocation testing is obvious.

## Work
- Update the picker placeholder to display a clear scaffold message.
- Wait for any key before exiting.
- Keep behavior temporary and isolated from future picker implementation.

## Verification
- `cargo fmt --check` passes.
- `cargo test` passes.
- `cargo build --release` passes.



## Implementation Notes
- Updated the temporary picker placeholder to print a clear scaffold screen with the target pane id.
- Added raw-mode key reading so the overlay remains visible until any key is pressed.
- The pause is intentionally isolated to `run_picker_placeholder` and should be removed/replaced by the real picker loop in follow-up work.

## Verification Results
- `cargo fmt --check` passed.
- `cargo test` passed.
- `cargo build --release` passed.
- `ish check` passed.
