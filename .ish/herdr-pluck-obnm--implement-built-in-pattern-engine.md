---
# herdr-pluck-obnm
title: Implement built-in pattern engine
status: todo
type: task
priority: high
tags:
- pluck
- matching
created_at: 2026-06-19T03:15:42.545423Z
updated_at: 2026-06-19T03:21:41.649243Z
parent: herdr-pluck-t3sf
blocked_by:
- herdr-pluck-jyye
---

## Context
Build the isolated pattern engine described in `.local/prds/1781838387-herdr-pluck-inline-hints.md`. Matching operates on recent unwrapped logical lines from the target pane so soft-wrapped URLs/paths remain a single copyable target.

Built-in v1 patterns are hardcoded. User-defined regex configuration is out of scope, but the model should already support a named capture group called `match` so future custom patterns can copy/highlight only the useful substring.

## Dependencies
- Blocked by `herdr-pluck-jyye` for the Rust project scaffold and shared types.

## Work
- Define built-in patterns with this priority order: URLs, file paths, UUIDs, Git SHAs, IPv4 addresses, long numeric identifiers.
- Match against unwrapped logical lines and return line/byte or line/column ranges suitable for renderer input.
- If a regex has a named capture `match`, use that capture as the copied/highlighted substring; otherwise use the full match.
- Resolve overlaps predictably: higher priority first, then longer match, then earlier top-to-bottom/left-to-right position.
- Deduplicate by copied text while preserving all visible occurrences for rendering.
- Prepare data in first-visible-occurrence order for hint assignment.
- Keep v1 cap handling compatible with the hint engine: only the first 676 unique copied texts can receive hints.

## Verification
- Unit tests cover all built-in token classes.
- Unit tests cover priority order, longest-match selection, leftmost/topmost tie-breaking, overlap rejection, named capture behavior, duplicate text handling, and deterministic ordering.
- `cargo test` passes.


## Reference Code
Use tmux-fingers as behavioral prior art, not as a requirement to clone every feature. Online repo: https://github.com/Morantron/tmux-fingers

- Capture/start flow and joined wrapped-line behavior: [`src/fingers/commands/start.cr`](https://github.com/Morantron/tmux-fingers/blob/master/src/fingers/commands/start.cr)
- tmux capture/copy helper behavior: [`src/tmux.cr`](https://github.com/Morantron/tmux-fingers/blob/master/src/tmux.cr)
- Matching, named `match` capture, duplicate hint reuse, and skip-shorter-than-hint behavior: [`src/fingers/hinter.cr`](https://github.com/Morantron/tmux-fingers/blob/master/src/fingers/hinter.cr)
