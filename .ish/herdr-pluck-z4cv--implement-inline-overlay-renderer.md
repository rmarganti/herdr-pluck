---
# herdr-pluck-z4cv
title: Implement inline overlay renderer
status: completed
type: task
priority: high
tags:
- pluck
- renderer
created_at: 2026-06-19T03:15:58.194704Z
updated_at: 2026-06-19T23:29:24.191690Z
parent: herdr-pluck-t3sf
blocked_by:
- herdr-pluck-obnm
- herdr-pluck-94xq
---

## Context
Build the isolated renderer for the Herdr Pluck picker. The picker displays a monochrome pane-like copy of the target pane's live bottom viewport, with non-matched text dim/black, matched text white, and hint characters cyan.

Rendering must preserve geometry: hints destructively replace the beginning of the highlighted match text rather than inserting columns. Matching happens on unwrapped logical lines; rendering must manually wrap styled spans to the target pane width and crop to the target pane height for live bottom viewport behavior.

## Dependencies
- Blocked by `herdr-pluck-obnm` for match/occurrence data semantics.
- Blocked by `herdr-pluck-94xq` for hint width and hint mapping semantics.

## Work
- Convert logical lines, match occurrences, and assigned hints into styled render spans or terminal output records.
- Render non-matches dim/black, matched text white, and destructive inline hint characters cyan.
- Skip/omit rendered hinting for matches shorter than the fixed hint width, consistent with the PRD.
- Manually wrap output to the target pane width rather than relying on terminal auto-wrap.
- Crop to target pane height, focused on the bottom live viewport approximation.
- Keep rendering independent of Herdr and terminal input so it can be unit tested with plain inputs/outputs.

## Verification
- Unit tests cover destructive hint replacement, duplicate occurrence rendering, match/non-match/hint style segmentation, manual wrapping to pane width, viewport height cropping, and matches shorter than hint width.
- `cargo test` passes.


## Reference Code
Use tmux-fingers destructive inline rendering as prior art. Online repo: https://github.com/Morantron/tmux-fingers

- [`src/fingers/match_formatter.cr`](https://github.com/Morantron/tmux-fingers/blob/master/src/fingers/match_formatter.cr)
- Related hint/match data behavior: [`src/fingers/hinter.cr`](https://github.com/Morantron/tmux-fingers/blob/master/src/fingers/hinter.cr)



## Design Decisions
- Public renderer API should accept raw logical lines, `HintAssignments`, width, and height, returning abstract styled `Vec<RenderLine>`; terminal emission is deferred to `herdr-pluck-1iof`.
- Keep renderer-specific intermediate types private to `src/renderer.rs`; only promote types to `src/model.rs` when they are stable cross-module domain types.
- Rename `RenderStyle::Dim` to semantic `RenderStyle::Unmatched`; terminal smoke output can later map it to dim/black/gray presentation.
- Render all provided recent unwrapped logical lines, manually wrap styled content to pane width, then crop to the final `height` rows for bottom-live-viewport behavior.
- If wrapped content has fewer than `height` rows, top-pad with blank `Unmatched` rows. Return exactly `height` rows and pad each row to exactly `width` columns when dimensions are non-zero.
- If `width == 0` or `height == 0`, return an empty vector. Avoid renderer panics/errors.
- Hint replacement is destructive: the hint replaces the first `hint.chars().count()` characters of the matched substring; the remaining matched text stays `Match`. Renderer uses assignment hints exactly as provided; fixed-width semantics belong to `hints`.
- Built-in matches are effectively ASCII/single-column for v1. Use `MatchSpan` UTF-8 byte offsets; use display-width wrapping where practical, but do not build a full grapheme/cell engine.
- Trust upstream pattern/hint modules for overlap resolution, duplicate hint reuse, and fixed-width assignment. Renderer may sort occurrences by line/start for deterministic output but should not re-resolve overlaps.
- Silently ignore out-of-bounds occurrence lines, invalid byte offsets, and duplicate exact occurrences beyond the first deterministic render. Do not clamp invalid ranges.
- Treat matches shorter than hint width as a defensive edge case only; v1 built-ins should not normally produce them.
- Merge adjacent output spans with the same `RenderStyle`; tests should assert normalized `(style, text)` tuples per row rather than verbose struct literals.



## Implementation Notes
- Added `renderer::render_inline_hints(logical_lines, assignments, width, height) -> Vec<RenderLine>` as the abstract renderer API for v1 picker output.
- Renamed `RenderStyle::Dim` to semantic `RenderStyle::Unmatched`; terminal presentation remains deferred to the smoke-output follow-up.
- Implemented private renderer occurrence collection/validation, destructive hint replacement, styled span construction, manual styled wrapping, bottom viewport cropping, top padding, fixed-width row padding, and adjacent span merging.
- Renderer silently ignores out-of-bounds occurrence lines, invalid UTF-8 byte ranges, and duplicate exact occurrences after deterministic sorting.
- Short-match defensive handling renders the matched text as `Match` without a hint; v1 built-ins are not expected to hit this path.
- Existing overlay composition helpers were preserved and updated to use `Unmatched` style when flattening composed output.
- Terminal/crossterm emission and Herdr-visible smoke rendering remain deferred to `herdr-pluck-1iof`.

## Verification Results
- `cargo fmt --all -- --check` passed.
- `cargo test --all-features` passed.
- `cargo clippy --all-targets --all` passed.
- `ish check` passed.



## Review Follow-up
- Accepted PR review finding that `render_inline_hints` redundantly called `merge_render_line` after `wrap_styled_lines` had already normalized emitted rows.
- Removed the extra merge pass and returned the cropped/padded visible rows directly, avoiding an unnecessary allocation per visible row.
- Addressed the resulting clippy `let_and_return` cleanup.

## Review Follow-up Verification
- `cargo fmt --all -- --check` passed.
- `cargo test --all-features` passed.
- `cargo clippy --all-targets --all` passed.
- `ish check` passed.
