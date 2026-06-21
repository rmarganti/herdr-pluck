---
# herdr-pluck-1iof
title: Render readonly picker view in terminal
status: completed
type: task
priority: high
tags:
- pluck
- renderer
created_at: 2026-06-19T19:16:36.267406Z
updated_at: 2026-06-21T19:17:02.367727Z
parent: herdr-pluck-t3sf
blocking:
- herdr-pluck-guwf
blocked_by:
- herdr-pluck-z4cv
---

## Context
After the abstract renderer is implemented, add the first production readonly picker rendering path before the full picker input loop. This path should run from the real layout-tab picker snapshot, show matched inline hints using the renderer, and provide production terminal emission that later input/copy work can reuse.

This is intentionally separate from the input-loop/copy-flow task: rendering is production code, but hint typing and clipboard copy remain for `herdr-pluck-guwf`.

## Dependencies
- Blocked by `herdr-pluck-z4cv` for abstract renderer output and style semantics.

## Work
- Add a picker rendering path that reads snapshot text and geometry, runs matching + hint assignment + rendering, and displays the rendered hint view in the temporary picker pane.
- Convert abstract render styles to terminal output with simple v1 styling: dim/dark non-matches, white matches, cyan hints.
- Keep behavior readonly: show output and wait for a non-Enter key to close; do not implement hint entry or clipboard copy here.
- Preserve the existing layout-tab snapshot/cleanup behavior.
- Verify the linked Herdr plugin action visually displays inline hints.

## Verification
- Unit tests cover style-to-terminal/emission helpers where practical without requiring a real terminal.
- Unit tests cover readonly picker view construction for matches and no-match state.
- `cargo fmt --all -- --check` passes.
- `cargo test --all-features` passes.
- `cargo clippy --all-targets --all` passes.
- Manual Herdr CLI/action verification displays rendered inline hints and closes on keypress.

## Implementation Notes
- Added a production `picker` module with `picker::render` for readonly picker orchestration.
- Added `renderer::terminal` to emit abstract render lines with crossterm styles: dark/dim unmatched text, white matches, and bold cyan hint characters.
- Replaced the layout-tab scaffold body with `run_readonly_picker`, keeping `herdr::executor` focused on Herdr/session orchestration.
- Added a no-match readonly view that fills the target viewport with a clear close prompt.

## Verification Results
- `cargo fmt --all -- --check` passed.
- `cargo test --all-features` passed.
- `cargo clippy --all-targets --all` passed.
- `cargo build --release` passed.
- Manual Herdr CLI verification: created a source tab with URL/path/SHA/IP/number text, ran `./target/release/herdr-pluck open --target-pane <pane>`, verified the temporary `Herdr Pluck` tab rendered inline hints with cyan hint characters and white matched text, sent `q`, and confirmed the temp tab closed and focus returned.
- Manual Herdr plugin action verification: `herdr plugin link .`, `herdr plugin action list --plugin rmarganti.herdr-pluck`, focused a source tab, invoked `herdr plugin action invoke rmarganti.herdr-pluck.pluck`, verified rendered inline hints in the temporary tab, sent `q`, and confirmed cleanup.



## Exact Visible Viewport Follow-up
- Switched production capture from recent-unwrapped bottom approximation to exact visible pane rows via `herdr pane read --source visible` before creating the temporary tab.
- Added `VisibleViewport` and `LogicalLineVisualSegment` models plus `viewport::map_visible_viewport` to reconstruct logical lines from soft-wrapped visible rows while preserving source row/column placement.
- Added `renderer::render_visible_inline_hints` so hints and match styling are applied directly onto exact visible rows instead of bottom-cropping/padding logical text.
- Preserved wrapped-token matching by matching against reconstructed logical lines and mapping accepted match byte ranges back to one or more visible row segments.

## Exact Visible Viewport Verification
- Added unit tests proving top-row content remains on row 0 and wrapped URL matches are highlighted across multiple visible rows.
- `cargo fmt --all -- --check` passed.
- `cargo test --all-features` passed.
- `cargo clippy --all-targets --all` passed.
- `ish check` passed.
- Manual Herdr top-row verification: a source pane containing `echo https://www.google.com` at the top rendered the hinted URL on row 1 of the picker, not at the bottom.
- Manual Herdr wrapped-token verification: a narrow source pane with a long URL rendered one hint at the URL start and continued white match styling across the wrapped second row.



## Palette Follow-up
- Updated terminal styling to improve hint/match contrast: matches now render yellow, and hint characters render bold black on cyan background.

## Palette Verification
- `cargo fmt --all -- --check` passed.
- `cargo test --all-features` passed.
- `cargo clippy --all-targets --all` passed.
- `cargo build --release` passed.
- Manual Herdr verification confirmed ANSI output uses black-on-cyan hints and yellow match text in the picker pane.



## Review Follow-up
- Addressed the UTF-8 column mapping review finding by converting match byte offsets into display-column offsets before styling exact visible rows.
- Updated visible row cell modeling to account for wide Unicode cells so matches after CJK/wide prefixes are not shifted.
- Removed a dead no-op assignment from `map_visible_viewport`.
- Kept `SourcePaneSnapshot.logical_lines` alongside `visible_viewport.logical_lines` intentionally for fallback/backward-compatible rendering when no exact viewport is available.
- Verified the `--lines` usage for visible reads in Herdr manual testing; it returns the requested visible viewport height used by the snapshot.

## Review Follow-up Verification
- Added unit tests for display-width segment columns and visible rendering after a wide UTF-8 prefix.
- `cargo fmt --all -- --check` passed.
- `cargo test --all-features` passed.
- `cargo clippy --all-targets --all` passed.
- `cargo build --release` passed.
- Manual Herdr Unicode verification: a source pane containing `界 https://example.com/unicode` rendered the hint after the wide prefix at the correct visible location.
