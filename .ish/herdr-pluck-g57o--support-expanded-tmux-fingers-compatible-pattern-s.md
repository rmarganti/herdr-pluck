---
# herdr-pluck-g57o
title: Support expanded tmux-fingers-compatible pattern set
status: todo
type: task
priority: low
tags:
- pluck
- matching
- deferred
created_at: 2026-06-19T18:34:14.319819Z
updated_at: 2026-06-19T18:34:14.319819Z
parent: herdr-pluck-t3sf
blocked_by:
- herdr-pluck-obnm
---

## Context
Herdr Pluck v1 intentionally ships with the PRD's six built-in pattern classes: URLs, file paths, UUIDs, Git SHAs, IPv4 addresses, and long numeric identifiers. After v1, consider expanding built-ins toward tmux-fingers parity and other commonly useful terminal tokens.

Use tmux-fingers as prior art for additional built-ins, but do not blindly add patterns without checking overlap, false-positive behavior, and hint usability in Herdr Pluck's copy-only workflow.

## Work
- Review tmux-fingers built-ins beyond the six v1 classes: `hex`, `kubernetes`, `kubernetes-pod`, `git-status`, `git-status-branch`, and `diff`.
- Identify any additional Herdr Pluck-specific patterns worth supporting.
- Decide whether expanded built-ins are always enabled or configurable.
- Add tests for each accepted pattern, named `match` captures, overlap resolution with existing higher-priority patterns, and false-positive boundaries.
- Document supported expanded patterns and any intentionally omitted tmux-fingers patterns.

## Verification
- Pattern tests cover every newly accepted class.
- Existing v1 pattern behavior remains unchanged unless deliberately documented.
- Project verification passes.
