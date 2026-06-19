---
# herdr-pluck-5dl8
title: Add local Herdr Pluck keybinding
status: completed
type: task
priority: normal
tags:
- pluck
- herdr
- keybinding
created_at: 2026-06-19T03:55:45.702479Z
updated_at: 2026-06-19T03:55:54.692081Z
parent: herdr-pluck-t3sf
blocked_by:
- herdr-pluck-c6k7
---

## Context
Add a local Herdr config keybinding for the linked Herdr Pluck plugin so manual testing can be triggered from the keyboard.

## Work
- Add a `[[keys.command]]` plugin action binding for `rmarganti.herdr-pluck.pluck`.
- Reload Herdr config.
- Record the chosen key and verification.

## Verification
- `herdr server reload-config` succeeds.



## Implementation Notes
- Added `prefix+q` as a `plugin_action` keybinding in `~/.config/herdr/config.toml`.
- Bound command: `rmarganti.herdr-pluck.pluck`.
- Description: `pluck visible token`.
- Chose `prefix+q` because this config uses `prefix+d` for detach and `prefix+q` was free.

## Verification Results
- `herdr server reload-config` succeeded.
- `herdr status` confirmed the server remains running and compatible.
