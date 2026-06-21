---
# herdr-pluck-t3sf
title: Herdr Pluck inline hints v1
status: completed
type: milestone
priority: high
tags:
- pluck
- prd-1781838387
created_at: 2026-06-19T03:15:21.064212Z
updated_at: 2026-06-21T19:49:17.670771Z
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
Use the online repositories as the stable source for reference code and documentation:

- Herdr: https://github.com/ogulcancelik/herdr
- tmux-fingers: https://github.com/Morantron/tmux-fingers

Important PRD reference files to inspect before implementation details:

- Herdr plugin authoring: [`website/src/content/docs/plugins.mdx`](https://github.com/ogulcancelik/herdr/blob/main/website/src/content/docs/plugins.mdx)
- Herdr CLI reference: [`website/src/content/docs/cli-reference.mdx`](https://github.com/ogulcancelik/herdr/blob/main/website/src/content/docs/cli-reference.mdx)
- Herdr socket/API overview: [`website/src/content/docs/socket-api.mdx`](https://github.com/ogulcancelik/herdr/blob/main/website/src/content/docs/socket-api.mdx)
- Herdr config/keybindings: [`website/src/content/docs/configuration.mdx`](https://github.com/ogulcancelik/herdr/blob/main/website/src/content/docs/configuration.mdx)
- Herdr pane schema/layout: [`src/api/schema/panes.rs`](https://github.com/ogulcancelik/herdr/blob/main/src/api/schema/panes.rs)
- Herdr terminal read behavior: [`src/pane/terminal.rs`](https://github.com/ogulcancelik/herdr/blob/main/src/pane/terminal.rs)
- tmux-fingers capture flow: [`src/fingers/commands/start.cr`](https://github.com/Morantron/tmux-fingers/blob/master/src/fingers/commands/start.cr) and [`src/tmux.cr`](https://github.com/Morantron/tmux-fingers/blob/master/src/tmux.cr)
- tmux-fingers matching/hinting: [`src/fingers/hinter.cr`](https://github.com/Morantron/tmux-fingers/blob/master/src/fingers/hinter.cr)
- tmux-fingers destructive rendering: [`src/fingers/match_formatter.cr`](https://github.com/Morantron/tmux-fingers/blob/master/src/fingers/match_formatter.cr)
- tmux-fingers clipboard behavior: [`src/fingers/action_runner.cr`](https://github.com/Morantron/tmux-fingers/blob/master/src/fingers/action_runner.cr) and [`src/tmux.cr`](https://github.com/Morantron/tmux-fingers/blob/master/src/tmux.cr)


## Wrap-up Notes
- All active child implementation ishes are completed, including final packaging/docs and expanded tmux-fingers-compatible pattern support.
- Final verification passed for formatting, tests, clippy, and Ish graph validation.

## Final Verification Results
- `cargo fmt --all -- --check` passed.
- `cargo test --all-features` passed.
- `cargo clippy --all-targets --all` passed.
- `ish check` passed.
