---
# herdr-pluck-vhdh
title: Implement Herdr layout-tab launcher and executor
status: completed
type: task
priority: high
tags:
- pluck
- herdr
- layout-tab
- refactor
created_at: 2026-06-19T03:16:14.467335Z
updated_at: 2026-06-21T18:49:23.786491Z
parent: herdr-pluck-t3sf
blocked_by:
- herdr-pluck-jyye
- herdr-pluck-k71j
---

## Context
This task now owns the production Herdr layout-tab launcher rather than the old overlay action.

On invocation, the canonical action must capture the source pane/tab/workspace/layout/text before creating a temporary tab. It then recreates the source tab's split topology in that temporary tab, launches inert blank commands in non-target temporary panes, runs the picker in the temporary pane corresponding to the original target pane, and leaves cleanup data for picker mode.

Use Herdr's documented plugin/CLI/socket APIs, not private internals. Prefer `HERDR_BIN_PATH` when available, falling back to `herdr`. Keep implementation aligned with `.local/plans/1781838388-herdr-pluck-production-layout-tab.md`.

## Dependencies
- Blocked by `herdr-pluck-jyye` for CLI mode scaffolding and shared adapter interfaces.
- Blocked by `herdr-pluck-k71j` for the deep Herdr-module interface and snapshot transport contract.
- The pure layout planning portion should be implemented/tested before relying on real Herdr side effects.

## Work
- Replace the production action entrypoint with `herdr-pluck open`; remove the old overlay entrypoint and manifest pane from the completed production path.
- Parse Herdr/plugin context environment to determine the originally focused target pane id.
- Capture Herdr `pane layout` data including area, panes, splits, tab id, workspace id, focused pane id, and zoom state.
- Add/refine domain types for source pane snapshots, per-pane geometry, split directions, layout nodes, temp-tab session data, and text capture mode.
- Implement pure layout planning from Herdr layout JSON into a binary `LayoutNode` tree and target pane mapping; do not hardcode one-pane/two-pane/2x2 layouts.
- Implement Herdr executor support for `tab create`, `pane split`, `pane focus`, `pane run`, and `tab close` through a fakeable command runner.
- Replay the planned split tree in the temp tab and return a source-pane-id to temp-pane-id mapping.
- Launch inert blank commands in non-target temporary panes where practical.
- Explicitly focus/run the picker in the mapped target temporary pane; do not rely solely on incidental split focus.
- Capture target pane text before temp tab creation, preferring exact visible unwrapped state if Herdr exposes it and otherwise recording a deliberate recent-unwrapped bottom approximation.
- Serialize/pass the full picker snapshot/session through the transport abstraction established by `herdr-pluck-k71j`; use direct env/argv/stdin only if safe, otherwise use a temp JSON file/path owned and cleaned up by the Herdr session abstraction.

## Verification
- Unit tests cover context parsing and Herdr layout JSON parsing.
- Pure layout planner tests cover single pane, horizontal/vertical uneven splits, 2x2 layouts, nested asymmetric splits, every target position, and inconsistent layout errors.
- Fake Herdr adapter tests cover temp tab creation, split replay order, focus target, pane run commands, inert non-target panes, chosen snapshot transport, and explicit cleanup ids.
- Manual Herdr smoke test creates a matching temp tab for one-pane/two-pane/2x2 layouts.
- Required validation passes:
  - `cargo fmt --all -- --check`
  - `cargo test --all-features`
  - `cargo clippy --all-targets --all`

## Reference Code
Use the online Herdr repository as the source for exact plugin/CLI/API behavior: https://github.com/ogulcancelik/herdr

- Plugin model/manifest/action docs: [`website/src/content/docs/plugins.mdx`](https://github.com/ogulcancelik/herdr/blob/main/website/src/content/docs/plugins.mdx)
- CLI commands including `pane read`, `pane layout`, `tab create`, `pane split`, `pane run`, and `tab close`: [`website/src/content/docs/cli-reference.mdx`](https://github.com/ogulcancelik/herdr/blob/main/website/src/content/docs/cli-reference.mdx)
- Socket/API overview including pane APIs: [`website/src/content/docs/socket-api.mdx`](https://github.com/ogulcancelik/herdr/blob/main/website/src/content/docs/socket-api.mdx)
- Keybinding `plugin_action` docs: [`website/src/content/docs/configuration.mdx`](https://github.com/ogulcancelik/herdr/blob/main/website/src/content/docs/configuration.mdx)
- Pane layout schema: [`src/api/schema/panes.rs`](https://github.com/ogulcancelik/herdr/blob/main/src/api/schema/panes.rs)
- Terminal read implementation and visible/recent unwrapped behavior: [`src/pane/terminal.rs`](https://github.com/ogulcancelik/herdr/blob/main/src/pane/terminal.rs)



## Notes from herdr-pluck-k71j
- The initial layout-tab foundation keeps Herdr-specific planning in `src/herdr.rs` rather than adding a top-level layout module.
- Production domain types now exist in `src/model.rs`: `SplitDirection`, `PaneTextCaptureMode`, `SourcePaneGeometry`, `SourcePaneSnapshot`, `TempTabSession`, `LayoutNode`, and `LayoutRecreationPlan`.
- `src/herdr.rs` exposes `derive_layout_recreation_plan` as pure planning scaffolding and `SnapshotTransportConstraints::choose_transport` for the initial env-json vs temp-file transport contract.
- Downstream work should expand this Herdr interface behind a fakeable command runner and replace the current simple split partitioning with full coverage for nested/2x2/inconsistent layout cases.



## Design Clarifications
- Do not keep `open-overlay` after this work is complete; once the layout-tab launcher is in place, remove the old overlay command and manifest pane rather than retaining a debug/deprecated alternate path.
- Completion should leave a semi-working implementation for manual testing: invoking `rmarganti.herdr-pluck.pluck` should create a temporary layout tab, replay one-pane/two-pane/2x2 source layouts, launch the picker placeholder or snapshot-aware smoke command in the corresponding target pane, keep non-target panes inert/quiet, and restore focus/close the temp tab on normal picker exit. Full hint input and clipboard copy remain blocked on downstream `herdr-pluck-guwf`; terminal styled hint smoke may be completed in `herdr-pluck-1iof` if not folded into this task.



## Implementation Notes
- Replaced the production `open-overlay` path with `herdr-pluck open` and removed the manifest overlay pane entry.
- Split the Herdr integration into `src/herdr/` modules: `context`, `layout`, `commands`, `executor`, and `snapshot`, with `mod.rs` exposing the narrow adapter API.
- Moved production layout-tab domain types into `src/model.rs`, including source snapshots, temp sessions, split directions, layout nodes, and picker snapshots.
- Added a fakeable Herdr command runner and command wrapper for `pane layout`, `pane read`, `tab create`, `pane split`, `pane run`, `tab focus`, and `tab close`.
- Implemented pure layout planning from Herdr layout snapshots into binary `LayoutNode` trees with tested single-pane, uneven split, 2x2/nested, missing-target, and inconsistent-boundary behavior.
- Implemented layout replay into a temporary tab, source-to-temp pane mapping, focus-aware split replay so the target pane is focused, inert non-target pane commands, snapshot temp-file transport, picker placeholder launch, and cleanup on picker exit.
- Updated README and `herdr-plugin.toml` to document/use the layout-tab entrypoint.

## Verification Results
- `cargo fmt --all -- --check` passed.
- `cargo test --all-features` passed.
- `cargo clippy --all-targets --all` passed.
- `cargo build --release` passed.
- `ish check` passed.
- `herdr plugin link .` succeeded and `herdr plugin action list --plugin rmarganti.herdr-pluck` showed `pluck` running `./target/release/herdr-pluck open`.
- Manual Herdr skill smoke: direct `./target/release/herdr-pluck open --target-pane <pane>` created a temporary `Herdr Pluck` tab, replayed the source tab pane count/layout, launched the picker scaffold in the mapped target temp pane, accepted `q`, closed the temp tab, and restored focus to the source tab.
- Manual Herdr skill 2x2 smoke: created a four-pane source tab, targeted the bottom-right pane containing `HERDR_PLUCK_TEST https://example.com/path /tmp/herdr-pluck-demo abcdef1234567890 10.20.30.40 9876543210`, verified the temporary tab recreated four panes with the picker focused in the bottom-right pane, verified captured text appeared in the picker scaffold, sent `q`, and confirmed temp/source smoke tabs were cleaned up.
- Manual Herdr skill action smoke: `herdr plugin action invoke rmarganti.herdr-pluck.pluck` launched the layout-tab picker through the manifest action, then `q` closed the temp tab and restored focus.



## Zoom Follow-up
- Preserved source zoom behavior in the layout-tab launcher: when the source layout reports `zoomed=true` and the target pane is focused, the mapped temporary target pane is explicitly zoomed with `herdr pane zoom <temp-pane> --on` before launching picker mode.
- Updated snapshot sizing/read-line capture to derive target dimensions from the effective visible source geometry, so zoomed source panes use the full zoomed content area instead of the unzoomed split rect.
- Added a fake-runner executor test proving the temp target pane is zoomed before pane run commands and that the picker snapshot records zoomed target dimensions.

## Zoom Follow-up Verification
- `cargo fmt --all -- --check` passed.
- `cargo test --all-features` passed.
- `cargo clippy --all-targets --all` passed.
- `ish check` passed.
- Manual Herdr skill zoom smoke: created a two-pane source tab, zoomed the right target pane, invoked `./target/release/herdr-pluck open --target-pane <zoomed-pane>`, confirmed the temporary `Herdr Pluck` tab reported `zoomed=true` with the mapped target pane focused, and confirmed picker scaffold target content used full zoomed dimensions (`350x61` in the smoke run). Cleanup restored focus and removed smoke tabs.



## Review Feedback Evaluation
- Medium snapshot leak: correct and relevant. `write_snapshot_file` happened before zoom/pane-run launch, and those failure paths only cleaned up Herdr state. Added failed-launch cleanup that removes the snapshot file as well as closing/focusing Herdr session ids.
- Low Enter prompt mismatch: correct. The picker intentionally ignores Enter to avoid immediately exiting from the `pane run` submission Enter, but the prompt said "any key". Updated the prompt to "any non-Enter key".
- Low disconnected snapshot transport: mostly correct. The abstraction was tested but production always wrote a temp file. Wired production through `choose_picker_snapshot_transport`; current `pane run` shell-command constraints deliberately select temp-file transport.
- Minor read-line clamp: correct enough to address. Replaced silent `height.max(1)` with a clear error when effective target content height is zero.

## Review Feedback Implementation Notes
- Added `choose_picker_snapshot_transport` and a production call site before writing picker snapshots.
- Added `cleanup_failed_launch` and tests covering session cleanup on zoom failure plus temp snapshot file removal.
- Added a zero-height geometry test that fails before pane text capture.
- Added fake command-runner failure support for executor error-path tests.

## Review Feedback Verification
- `cargo fmt --all -- --check` passed.
- `cargo test --all-features` passed.
- `cargo clippy --all-targets --all` passed.
- `cargo build --release` passed.
- `ish check` passed.
- Manual Herdr skill smoke: linked plugin, directly invoked `./target/release/herdr-pluck open --target-pane <focused-pane>`, confirmed temporary `Herdr Pluck` tab opened, picker scaffold displayed `Press any non-Enter key to close temporary tab...`, sent `q`, and confirmed focus returned to the source tab with the temp tab closed.
