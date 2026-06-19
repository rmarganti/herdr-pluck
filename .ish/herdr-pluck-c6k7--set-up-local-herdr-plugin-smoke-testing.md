---
# herdr-pluck-c6k7
title: Set up local Herdr plugin smoke testing
status: completed
type: task
priority: high
tags:
- pluck
- herdr
- smoke
created_at: 2026-06-19T03:51:29.979003Z
updated_at: 2026-06-19T03:52:20.819002Z
parent: herdr-pluck-t3sf
blocked_by:
- herdr-pluck-jyye
---

## Context
Link the scaffolded Herdr Pluck plugin into the local Herdr instance so follow-up implementation ishes can be manually tested through the real plugin/action/overlay loop.

## Work
- Build the release binary used by the plugin manifest.
- Link this checkout as a local Herdr plugin.
- Verify Herdr can list the plugin action.
- Invoke the action or pane enough to confirm the manifest and entrypoints are wired.
- Capture any CLI/manifest mismatches as implementation notes.

## Verification
- `cargo build --release` passes.
- `herdr plugin link .` succeeds.
- `herdr plugin action list --plugin rmarganti.herdr-pluck` shows `pluck`.
- Manual smoke-test command result/logs are recorded.



## Implementation Notes
- Confirmed Herdr 0.7.0 server is running and linked this checkout with `herdr plugin link .`.
- Verified `herdr plugin action list --plugin rmarganti.herdr-pluck` returns the `pluck` action.
- Initial action invocation exposed a scaffold bug: `herdr plugin pane open` rejects `--target-pane` for overlay panes because overlays target the active pane.
- Fixed `HerdrAdapter::open_picker_overlay` to omit `--target-pane` while still forwarding `HERDR_PLUCK_TARGET_PANE_ID` to the picker pane.
- Rebuilt release binary and re-invoked `rmarganti.herdr-pluck.pluck`; log `plugin-log-2` succeeded and opened overlay pane `picker` for pane `w3:p1`.

## Verification Results
- `cargo fmt --check` passed.
- `cargo test` passed.
- `cargo build --release` passed.
- `herdr plugin link .` succeeded.
- `herdr plugin action list --plugin rmarganti.herdr-pluck` showed `pluck`.
- `herdr plugin action invoke rmarganti.herdr-pluck.pluck` succeeded after the overlay launch fix.
