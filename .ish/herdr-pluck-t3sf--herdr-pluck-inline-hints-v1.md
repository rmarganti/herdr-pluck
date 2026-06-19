---
# herdr-pluck-t3sf
title: Herdr Pluck inline hints v1
status: todo
type: milestone
priority: high
tags:
- pluck
- prd-1781838387
created_at: 2026-06-19T03:15:21.064212Z
updated_at: 2026-06-19T03:21:41.623915Z
---

## Context
Implement `.local/prds/1781838387-herdr-pluck-inline-hints.md`: a Rust Herdr plugin that opens an overlay picker, scans the originally focused pane's live bottom viewport for recognizable tokens, shows fixed-width inline hints, copies the selected match, and exits.

This milestone groups the v1 copy-only implementation. The PRD is the source of requirements; keep out-of-scope items deferred: mouse support, OSC52, user-defined regex config, exact scrolled viewport support, non-copy actions, original ANSI preservation, and >2-character hints.

## Dependencies
No external ish prerequisites. Child ishes and explicit `blocked_by` relationships define implementation order.

## Work
- Deliver a shippable Rust Herdr plugin binary and manifest/action/keybinding guidance.
- Preserve separation between Herdr integration, matching, hinting, rendering, input, clipboard, and CLI entrypoints.
- Provide unit/behavior tests for stable module contracts.

## Verification
- `ish check` passes for this work graph.
- Project tests pass.
- Manual smoke test can invoke the plugin action, see inline hints over a target pane, copy by typing a hint, and cancel with Escape/Ctrl-C.


## Reference Repositories
Remote reference code has been cached locally with the librarian workflow. Future agents should prefer these local paths for research and grep/read operations:

- Herdr: `https://github.com/ogulcancelik/herdr` cached at `/Users/rmarganti/.cache/checkouts/github.com/ogulcancelik/herdr`
- tmux-fingers: `https://github.com/Morantron/tmux-fingers` cached at `/Users/rmarganti/.cache/checkouts/github.com/Morantron/tmux-fingers`

Important PRD reference files to inspect before implementation details:

- Herdr plugin authoring: `/Users/rmarganti/.cache/checkouts/github.com/ogulcancelik/herdr/website/src/content/docs/plugins.mdx`
- Herdr CLI reference: `/Users/rmarganti/.cache/checkouts/github.com/ogulcancelik/herdr/website/src/content/docs/cli-reference.mdx`
- Herdr socket/API overview: `/Users/rmarganti/.cache/checkouts/github.com/ogulcancelik/herdr/website/src/content/docs/socket-api.mdx`
- Herdr config/keybindings: `/Users/rmarganti/.cache/checkouts/github.com/ogulcancelik/herdr/website/src/content/docs/configuration.mdx`
- Herdr pane schema/layout: `/Users/rmarganti/.cache/checkouts/github.com/ogulcancelik/herdr/src/api/schema/panes.rs`
- Herdr terminal read behavior: `/Users/rmarganti/.cache/checkouts/github.com/ogulcancelik/herdr/src/pane/terminal.rs`
- tmux-fingers capture flow: `/Users/rmarganti/.cache/checkouts/github.com/Morantron/tmux-fingers/src/fingers/commands/start.cr` and `/Users/rmarganti/.cache/checkouts/github.com/Morantron/tmux-fingers/src/tmux.cr`
- tmux-fingers matching/hinting: `/Users/rmarganti/.cache/checkouts/github.com/Morantron/tmux-fingers/src/fingers/hinter.cr`
- tmux-fingers destructive rendering: `/Users/rmarganti/.cache/checkouts/github.com/Morantron/tmux-fingers/src/fingers/match_formatter.cr`
- tmux-fingers clipboard behavior: `/Users/rmarganti/.cache/checkouts/github.com/Morantron/tmux-fingers/src/fingers/action_runner.cr` and `/Users/rmarganti/.cache/checkouts/github.com/Morantron/tmux-fingers/src/tmux.cr`
