---
# herdr-pluck-vhdh
title: Implement Herdr adapter and overlay action
status: todo
type: task
priority: high
tags:
- pluck
- herdr
- integration
created_at: 2026-06-19T03:16:14.467335Z
updated_at: 2026-06-19T03:21:41.642627Z
parent: herdr-pluck-t3sf
blocked_by:
- herdr-pluck-jyye
---

## Context
Herdr Pluck v1 must be invoked by a Herdr plugin action rather than launching the picker directly. The action captures the originally focused pane id before the overlay changes focus, then opens a plugin overlay pane and explicitly passes that target pane id to the picker process.

Use Herdr's documented plugin/CLI/socket APIs, not private internals. Relevant PRD references: plugin authoring docs, CLI `pane read recent-unwrapped`, `plugin pane open --overlay`, layout APIs with pane rect width/height, and plugin context environment.

## Dependencies
- Blocked by `herdr-pluck-jyye` for CLI mode scaffolding and shared adapter interfaces.

## Work
- Parse Herdr/plugin context environment, including the Herdr-provided binary/CLI path where available.
- Implement the action mode that determines the originally focused pane id and opens the picker as an overlay plugin pane.
- Pass the target pane id to picker mode via an explicit environment variable; picker mode must not infer target from the newly focused overlay pane.
- Implement pane layout retrieval and extract target pane width/height.
- Implement live bottom viewport reading through recent unwrapped text, with line count derived from target pane height enough to reconstruct manual wrapping.
- Keep Herdr command execution abstracted so tests can use faked command responses.
- Document any exact manifest/action/keybinding assumptions discovered during implementation.

## Verification
- Unit tests or adapter-level tests cover context parsing, target pane id propagation, layout parsing, recent-unwrapped read command construction, and error handling with faked Herdr CLI responses.
- A manual smoke-test command or documented steps can show the action opening an overlay against the prior target pane in a running Herdr instance.
- `cargo test` passes.


## Reference Code
Use the online Herdr repository as the source for exact plugin/CLI/API behavior: https://github.com/ogulcancelik/herdr

- Plugin model/manifest/action/overlay docs: [`website/src/content/docs/plugins.mdx`](https://github.com/ogulcancelik/herdr/blob/main/website/src/content/docs/plugins.mdx)
- CLI commands including `pane read` and `plugin pane open`: [`website/src/content/docs/cli-reference.mdx`](https://github.com/ogulcancelik/herdr/blob/main/website/src/content/docs/cli-reference.mdx)
- Socket/API overview including pane APIs: [`website/src/content/docs/socket-api.mdx`](https://github.com/ogulcancelik/herdr/blob/main/website/src/content/docs/socket-api.mdx)
- Keybinding `plugin_action` docs: [`website/src/content/docs/configuration.mdx`](https://github.com/ogulcancelik/herdr/blob/main/website/src/content/docs/configuration.mdx)
- Pane layout schema: [`src/api/schema/panes.rs`](https://github.com/ogulcancelik/herdr/blob/main/src/api/schema/panes.rs)
- Terminal read implementation and `recent_unwrapped` behavior: [`src/pane/terminal.rs`](https://github.com/ogulcancelik/herdr/blob/main/src/pane/terminal.rs)
