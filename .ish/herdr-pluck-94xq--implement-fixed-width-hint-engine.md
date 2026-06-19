---
# herdr-pluck-94xq
title: Implement fixed-width hint engine
status: todo
type: task
priority: high
tags:
- pluck
- hints
created_at: 2026-06-19T03:15:50.019325Z
updated_at: 2026-06-19T03:15:50.019325Z
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
