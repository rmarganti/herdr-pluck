---
# herdr-pluck-clt9
title: Finalize plugin packaging and v1 verification
status: completed
type: task
priority: normal
tags:
- pluck
- packaging
- docs
created_at: 2026-06-19T03:16:34.072605Z
updated_at: 2026-06-21T19:42:24.727901Z
parent: herdr-pluck-t3sf
blocked_by:
- herdr-pluck-guwf
---

## Context
After the implementation is wired, finish Herdr Pluck v1 so a future user/maintainer can build, install, configure a keybinding, and verify the copy workflow described in `.local/prds/1781838387-herdr-pluck-inline-hints.md`.

This task should not add deferred scope such as OSC52, custom regex configuration, mouse support, non-copy actions, or exact scrolled viewport support.

## Dependencies
- Blocked by `herdr-pluck-guwf`, which integrates the picker flow and all core modules.

## Work
- Ensure the plugin manifest/action names and overlay pane invocation are documented and checked into the repo.
- Document build/install steps and a sample Herdr keybinding using `plugin_action`.
- Add a concise README covering v1 behavior, supported built-in patterns, clipboard tool requirements, and out-of-scope/deferred features.
- Run formatting, tests, and build in release or debug mode as appropriate.
- Perform or document a manual smoke test in Herdr: focus a pane with URLs/paths/SHAs/etc., invoke action, confirm hints appear inline, type a hint, verify system clipboard content, and verify Escape/Ctrl-C cancellation.
- Review the implementation against the PRD user stories and testing decisions, recording any known limitations.

## Verification
- `cargo fmt --check`, `cargo test`, and `cargo build` pass.
- README/install docs are present and mention keybinding, overlay action, supported patterns, clipboard fallbacks, and v1 limitations.
- Manual smoke-test results or exact reproducible smoke-test steps are recorded.


## Reference Code
When finalizing docs and install instructions, verify the current Herdr plugin/keybinding shape against the online Herdr repository: https://github.com/ogulcancelik/herdr

- [`website/src/content/docs/plugins.mdx`](https://github.com/ogulcancelik/herdr/blob/main/website/src/content/docs/plugins.mdx)
- [`website/src/content/docs/configuration.mdx`](https://github.com/ogulcancelik/herdr/blob/main/website/src/content/docs/configuration.mdx)
- [`website/src/content/docs/cli-reference.mdx`](https://github.com/ogulcancelik/herdr/blob/main/website/src/content/docs/cli-reference.mdx)


## Implementation Notes
- Rewrote `README.md` for plugin users/maintainers rather than implementation history.
- Documented Herdr requirements, build/link/install steps, the manifest action id, a sample `plugin_action` keybinding, normal usage, supported built-in patterns, clipboard tool requirements, v1 behavior, and v1 limitations.
- Verified `herdr-plugin.toml` remains checked in with plugin id `rmarganti.herdr-pluck` and action `pluck` invoking `./target/release/herdr-pluck open`.
- Reviewed the delivered v1 behavior against the PRD user workflow: focus pane, invoke action, see inline hints for built-in patterns, type fixed-width hint to copy, cancel with Escape/Ctrl-C, and surface clipboard-tool requirements.

## Verification Results
- `cargo fmt --all -- --check` passed.
- `cargo test --all-features` passed.
- `cargo clippy --all-targets --all` passed.
- `cargo build --release` passed.
- `ish check` passed.
- `herdr plugin link .` succeeded.
- `herdr plugin action list --plugin rmarganti.herdr-pluck` showed action `pluck`.
- Manual Herdr copy smoke: created a temporary source tab containing `https://example.com/herdr-pluck-final-1782073400`, `/tmp/herdr-pluck-final`, `abcdef1234567890`, `10.20.30.40`, and `9876543210`; invoked `herdr plugin action invoke rmarganti.herdr-pluck.pluck`; confirmed the temporary picker tab rendered inline hints; typed the visible URL hint; verified `pbpaste` contained `https://example.com/herdr-pluck-final-1782073400`; confirmed the picker tab closed and focus returned to the source tab.
- Manual Herdr Escape smoke: invoked the plugin action again, sent Escape to the picker pane, and confirmed the temporary picker tab closed with focus restored.
- Manual Herdr Ctrl-C smoke: invoked the plugin action again, sent Ctrl-C as the control character to the picker pane, and confirmed the temporary picker tab closed with focus restored.
- Cleaned up the temporary smoke-test tab after verification.
