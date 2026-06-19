---
# herdr-pluck-50oj
title: Handle overlay/source pane geometry mismatch
status: completed
type: task
priority: high
tags:
- pluck
- herdr
- geometry
created_at: 2026-06-19T17:03:11.933956Z
updated_at: 2026-06-19T17:35:10.023481Z
parent: herdr-pluck-t3sf
blocking:
- herdr-pluck-vhdh
- herdr-pluck-z4cv
- herdr-pluck-guwf
blocked_by:
- herdr-pluck-jyye
---

## Context
Herdr overlay panes are implemented by splitting the active pane, focusing the plugin pane, and zooming the tab. Pane rendering frames terminals when the underlying layout has multiple panes, and Herdr reserves a stable one-column scrollbar gutter from terminal content when possible.

This means the picker cannot safely use post-overlay `pane.layout` dimensions for the source pane, and cannot assume the overlay terminal size equals the source pane's visible terminal size:

- A single source pane is unframed before launch, but the overlay is framed after launch, so the overlay content area is smaller and offset by the border.
- An unzoomed split source pane is framed inside its split rect, but the overlay is zoomed to the full tab, so the overlay pane is much larger than the source pane.
- A zoomed source pane in a multi-pane tab is framed before launch, so it most closely matches the overlay geometry.

## Work
- In action mode, before opening the overlay, capture a frozen pre-overlay geometry snapshot for the target pane.
- Derive effective source terminal content rect from Herdr layout semantics: use full terminal area when the tab is zoomed to the focused target, otherwise the target pane's layout rect; inset by one-cell borders only when the underlying pane count is greater than one; reserve Herdr's stable scrollbar gutter by subtracting one content column when width permits.
- Pass this frozen geometry to picker mode via an explicit environment variable, alongside the target pane id.
- Do not let picker mode recompute source geometry from post-overlay layout, because opening the overlay mutates the tab layout and zoom state.
- Have the renderer compose source-screen output into the actual overlay terminal viewport using captured source rect and predicted/actual overlay content rect. Pad or crop so hints appear over the original source pane region where possible.
- Keep matching/wrapping based on source terminal content width; keep final emission bounded by the overlay terminal size.
- Add adapter/renderer tests for single-pane unframed source, unzoomed split source, and zoomed multi-pane source.

## Verification
- Tests demonstrate that source geometry is captured before overlay launch and remains stable after launch.
- Tests cover derived effective content rects for single pane, split pane, and zoomed multi-pane cases.
- Renderer tests cover padding/cropping from source rect to overlay viewport.


## Implementation Notes
- Added serializable geometry primitives (`Rect`, `SourceGeometrySnapshot`) to distinguish terminal area, source outer rect, and effective source content rect.
- `open-overlay` now captures `herdr pane layout --pane <target>` before opening the overlay, derives frozen source geometry, and passes it to picker mode through `HERDR_PLUCK_SOURCE_GEOMETRY_JSON` alongside `HERDR_PLUCK_TARGET_PANE_ID`.
- Picker placeholder now parses and displays the frozen geometry so manual testing can verify pre-overlay capture behavior.
- Geometry derivation handles single-pane unframed sources, unzoomed split panes with border inset, zoomed multi-pane tabs using full tab area, and Herdr's one-column right scrollbar gutter.
- Added renderer composition helpers that place source-sized output into an overlay viewport using frozen source coordinates with padding/cropping.
- Manual smoke test invoked `rmarganti.herdr-pluck.pluck`; overlay pane `wE:pF` displayed the captured geometry for target pane `wE:pE`.

## Verification Results
- `cargo fmt --all -- --check` passed.
- `cargo test --all-features` passed.
- `cargo clippy --all-targets --all` passed.
- `cargo build --release` passed.
- `herdr plugin action invoke rmarganti.herdr-pluck.pluck` succeeded and the placeholder displayed frozen geometry.


## Coordinate-Space Follow-up
- Confirmed Herdr layout rect `x`/`y` values are Herdr-global UI coordinates that include sidebar and tab-bar offsets.
- Added `Rect::relative_to` plus `SourceGeometrySnapshot::{source_content_rect_in_terminal,source_outer_rect_in_terminal}` so downstream rendering can normalize Herdr-global rects before writing to overlay-local terminal coordinates.
- Added focused docblocks for new geometry models and geometry transport/derivation helpers.
- Updated picker placeholder to display both absolute source content geometry and source content normalized to Herdr's terminal area.
- Added tests proving sidebar/tab-bar offsets normalize away and renderer composition does not add sidebar padding when rects are normalized.

## Coordinate-Space Verification Results
- `cargo fmt --all -- --check` passed.
- `cargo test --all-features` passed.
- `cargo clippy --all-targets --all` passed.
- `cargo build --release` passed.
- Manual Herdr invocation displayed `source content in terminal`, e.g. absolute `x=140 y=2` normalized to terminal-local `x=114 y=1` for the right split pane.


## Review Follow-up
- Accepted review feedback that `source_geometry_from_env()` was redundant in `cli.rs` because clap already binds `HERDR_PLUCK_SOURCE_GEOMETRY_JSON` into `source_geometry_json`.
- Removed the fallback to avoid misleading dead code in the production CLI path.

## Review Follow-up Verification
- `cargo fmt --all -- --check` passed.
- `cargo test --all-features` passed.
- `cargo clippy --all-targets --all` passed.


## Docblock Follow-up
- Added succinct docblocks to the private Herdr `pane layout` response wrapper structs: `LayoutEnvelope` and `LayoutResult`.

## Docblock Follow-up Verification
- `cargo fmt --all -- --check` passed.
- `cargo test --all-features` passed.
- `cargo clippy --all-targets --all` passed.
