---
# herdr-pluck-z4cv
title: Implement inline overlay renderer
status: todo
type: task
priority: high
tags:
- pluck
- renderer
created_at: 2026-06-19T03:15:58.194704Z
updated_at: 2026-06-19T03:21:41.655210Z
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
