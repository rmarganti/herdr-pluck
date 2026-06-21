---
# herdr-pluck-k71j
title: Establish deep Herdr layout-tab foundation and snapshot transport contract
status: completed
type: task
priority: high
tags:
- pluck
- herdr
- layout-tab
- refactor
created_at: 2026-06-21T13:06:26.558253Z
updated_at: 2026-06-21T13:15:08.332030Z
parent: herdr-pluck-t3sf
blocking:
- herdr-pluck-vhdh
blocked_by:
- herdr-pluck-jyye
---

## Context
This is the first production layout-tab refactor task. It exists to prevent the refactor from drifting into shallow top-level utilities or premature snapshot-transport complexity.

Read the full production plan before implementation: `.local/plans/1781838388-herdr-pluck-production-layout-tab.md`.

Current code is still overlay-first (`open-overlay`, overlay pane manifest entry, `SourceGeometrySnapshot`, overlay composition helpers). The production direction is layout-tab-first: create a temporary tab that recreates the source tab's split topology, run the picker in the temp pane corresponding to the original target pane, render pane-locally at `(0,0)`, and cleanup by explicit temp tab / return pane ids.

Review feedback incorporated into the plan:

- Keep Herdr layout functionality inside a deep `herdr` module/interface. Do not scatter Herdr-specific layout planning into unrelated top-level modules. If one file becomes too large, use a multi-file `src/herdr/` module with a narrow public API.
- Do not assume temp-file snapshot transport before proving it is necessary. Define a snapshot transport abstraction and choose env/argv/stdin/temp-file based on Herdr `pane run` reliability, quoting safety, and payload size.

## Scope
This task establishes the architecture contracts and first implementation slice that downstream layout-tab tasks will build on. It should be completed before broad executor/picker wiring in `herdr-pluck-vhdh`.

## Work
- Re-read `.local/plans/1781838388-herdr-pluck-production-layout-tab.md` and keep implementation aligned with it.
- Design the public Herdr module interface for layout-tab launching, including target capture, source snapshot capture, temp-tab creation/replay, picker launch, and cleanup/session ids.
- Decide whether to keep `src/herdr.rs` as one deep module or convert it to `src/herdr/mod.rs` plus private submodules such as `layout`, `commands`, and `snapshot`; either way, keep callers behind the Herdr interface.
- Add/refine domain types needed by the Herdr interface: source pane snapshot, source pane geometry, split direction, layout node/recreation plan, temp tab session, capture mode, and snapshot transport choice.
- Implement or stub the snapshot transport abstraction with tests documenting the chosen initial transport and the fallback conditions for temp files.
- Ensure pure Herdr layout planning code is callable in tests without running Herdr commands.
- Update existing overlay-oriented names/comments only where they block the layout-tab architecture; full overlay removal/default-manifest switch can remain in downstream tasks.
- Update `herdr-pluck-vhdh` if this task changes dependencies, transport shape, or public interface assumptions.

## Out of Scope
- Full split replay into a live Herdr temp tab.
- Full picker input/copy flow.
- README/manifest finalization.
- Manual verification of the entire geometry matrix.

## Verification
- Unit tests cover the new Herdr interface/domain serialization boundaries and snapshot transport decision behavior.
- Any pure layout-planning scaffolding added here is tested without a running Herdr server.
- `ish check` passes.
- Required project validation passes:
  - `cargo fmt --all -- --check`
  - `cargo test --all-features`
  - `cargo clippy --all-targets --all`

## Success Criteria
- Downstream agents can implement `herdr-pluck-vhdh` without guessing where Herdr layout code belongs.
- The crate has an explicit Herdr-module boundary for layout-tab orchestration.
- Snapshot transport is an intentional, tested abstraction rather than an unreviewed temp-file assumption.
- This ish and related downstream ishes reference `.local/plans/1781838388-herdr-pluck-production-layout-tab.md` for total-task context.



## Implementation Notes
- Kept the layout-tab foundation inside the deep Herdr module boundary in `src/herdr.rs`; no top-level layout utility module was added.
- Added production layout-tab domain types in `src/model.rs` for pane snapshots, pane geometry, split directions, layout nodes, recreation plans, temp-tab sessions, and capture modes.
- Added `SnapshotTransportConstraints` and `SnapshotTransport` with tests. The initial contract prefers direct env JSON only for small payloads with direct env support and no shell involvement; otherwise it falls back to temp files.
- Added pure `derive_layout_recreation_plan` scaffolding in `src/herdr.rs` with tests for single-pane and right-split layouts. Full nested/2x2/inconsistent coverage remains for `herdr-pluck-vhdh`.
- Updated `herdr-pluck-vhdh` with downstream interface and transport notes.

## Verification
- `ish check` passed with existing archive-state warnings only.
- `cargo fmt --all -- --check` passed.
- `cargo test --all-features` passed.
- `cargo clippy --all-targets --all` passed.
