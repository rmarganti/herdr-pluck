---
# herdr-pluck-vhdh
title: Implement Herdr layout-tab launcher and executor
status: todo
type: task
priority: high
tags:
- pluck
- herdr
- layout-tab
- refactor
created_at: 2026-06-19T03:16:14.467335Z
updated_at: 2026-06-21T13:14:58.970104Z
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
- Replace the production action entrypoint with `herdr-pluck open`; keep any overlay entrypoint debug-only if retained at all.
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
