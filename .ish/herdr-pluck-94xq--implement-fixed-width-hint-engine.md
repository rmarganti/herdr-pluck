---
# herdr-pluck-94xq
title: Implement fixed-width hint engine
status: completed
type: task
priority: high
tags:
- pluck
- hints
created_at: 2026-06-19T03:15:50.019325Z
updated_at: 2026-06-19T18:13:14.397020Z
parent: herdr-pluck-t3sf
blocked_by:
- herdr-pluck-jyye
---

## Context
Build the isolated hint engine for the v1 Herdr Pluck picker. Hints are keyboard-only, fixed-width, and assigned to unique copied texts in the order of their first visible occurrence.

The v1 alphabet uses all lowercase letters in home-row-ish order: home row first, then top row, then bottom row. Hints are one character when possible and two characters otherwise; v1 caps at two-character hints, or 676 unique copied texts.

## Dependencies
- Blocked by `herdr-pluck-jyye` for the Rust project scaffold and shared types.

## Work
- Define the canonical hint alphabet with all 26 lowercase letters in ergonomic order.
- Given ordered unique copied texts, select fixed width: 1 when count <= 26, otherwise 2.
- Generate deterministic hints in alphabet order for the selected width.
- Silently omit unique matches beyond the 676 hint capacity.
- Ensure duplicate copied texts share one hint and retain enough occurrence mapping for renderer use.
- Expose simple APIs that the input loop can use to validate exact fixed-width input and the renderer can use to place hints.

## Verification
- Unit tests cover one-character and two-character hint generation.
- Unit tests cover fixed-width selection, max capacity truncation, deterministic ordering, duplicate mapping, and exact/invalid hint lookup behavior.
- `cargo test` passes.



## Implementation Notes
- Replaced lexical `BTreeMap` assignment with caller-order-preserving assignment using an ordered builder list and text-to-index lookup.
- Added `HintAssignments` wrapper with immutable assignment access, width reporting, exact hint-to-copied-text lookup, valid hint iteration, and owned conversion for downstream renderer/input integration.
- Added `HINT_ALPHABET`, `MAX_HINT_CAPACITY`, zero-match/no-width behavior, one- and two-character deterministic hint generation, and silent cap at 676 unique copied texts.
- Duplicate copied texts now share one hint and retain all occurrences; duplicates of already-assigned texts are retained even after the unique-text cap is reached.
- Added `unicode-width 0.2.2` after checking the current stable crate version and exposed a `display_width` helper for future Unicode-aware renderer/input work.
- Left `src/input.rs` untouched; future picker/input work can consume `HintAssignments::width`, `valid_hints`, and `copied_text_for_hint`.

## Verification Results
- `cargo fmt --all -- --check` passed.
- `cargo test --all-features` passed.
- `cargo clippy --all-targets --all` passed.



## Review Follow-up
- Derived `MAX_HINT_CAPACITY` from `HINT_ALPHABET.len()` to prevent drift if the alphabet changes.
- Documented that `HintAssignments::len()` counts unique assigned copied texts, not total match occurrences.
- Documented that `HintAssignments::width()` is fixed typed hint character width, not Unicode display column width.
- Fixed the `AGENTS.md` typo: `documentaiton` -> `documentation`.

## Review Follow-up Verification
- `cargo fmt --all -- --check` passed.
- `cargo test --all-features` passed.
- `cargo clippy --all-targets --all` passed.
