---
# herdr-pluck-jyye
title: Scaffold Rust Herdr Pluck plugin project
status: completed
type: task
priority: high
tags:
- pluck
- rust
- scaffold
created_at: 2026-06-19T03:15:33.208549Z
updated_at: 2026-06-19T03:36:15.751044Z
parent: herdr-pluck-t3sf
---

## Context
The repository currently contains the PRD and Ish workspace but no implementation. Start the v1 codebase for `.local/prds/1781838387-herdr-pluck-inline-hints.md` as a Rust-based Herdr plugin that builds to a single binary.

The architecture must anticipate separate modules for Herdr integration, pattern matching, hint assignment, rendering, input handling, clipboard copying, and CLI entrypoints.

## Dependencies
None. This is the foundation for implementation work.

## Work
- Create a Cargo project/library layout appropriate for a shippable plugin binary.
- Add dependencies needed for regex matching, CLI parsing if useful, terminal input/rendering, serialization, and tests; keep choices simple and maintainable.
- Define initial module boundaries and shared data types for pane text, dimensions, matches, hints, styled render spans, picker outcomes, and copy results.
- Add a CLI entrypoint with at least two placeholder modes matching the PRD: one mode to open the overlay for the current target pane, and one mode to run the picker inside the overlay.
- Add a minimal Herdr plugin manifest/config artifact or documented expected manifest shape if the exact install location is not yet known.

## Verification
- `cargo fmt --check` passes.
- `cargo test` passes with at least smoke tests for argument parsing/module construction where practical.
- The binary can be built locally with `cargo build`.


## Reference Code
Before choosing project shape and manifest/action assumptions, inspect Herdr's plugin and CLI docs in the cached reference checkout:

- `/Users/rmarganti/.cache/checkouts/github.com/ogulcancelik/herdr/website/src/content/docs/plugins.mdx`
- `/Users/rmarganti/.cache/checkouts/github.com/ogulcancelik/herdr/website/src/content/docs/cli-reference.mdx`
- `/Users/rmarganti/.cache/checkouts/github.com/ogulcancelik/herdr/website/src/content/docs/configuration.mdx`



## Implementation Notes
- Initialized a Rust Cargo application/library crate named `herdr-pluck`.
- Added module boundaries for CLI, Herdr integration, pattern matching, hint assignment, rendering, input handling, clipboard copying, and shared models.
- Added CLI modes `open-overlay` and `pick` matching the action and overlay picker entrypoints.
- Added `herdr-plugin.toml` with plugin id `rmarganti.herdr-pluck`, action `pluck`, overlay pane `picker`, and release build command.
- Added README setup/keybinding guidance.

## Verification
- `cargo fmt --check` passed.
- `cargo test` passed.
- `cargo build` passed.
- `ish check` passed.



## Follow-up Fix
- Updated `open-overlay` target-pane discovery to use `HERDR_PANE_ID` before falling back to `HERDR_PLUGIN_CONTEXT_JSON`.
- Added a unit test covering the direct Herdr pane id environment fallback.

## Follow-up Verification
- `cargo fmt --check` passed.
- `cargo test` passed.
- `cargo build` passed.



## Environment Fallback Audit
- Grepped scaffold code for Herdr/env/context assumptions.
- Added `HERDR_ACTIVE_PANE_ID` as a harmless fallback for non-plugin/custom-command style launches.
- Added flat context keys (`focused_pane_id`, `pane_id`, `target_pane_id`) in addition to nested context objects.
- Fixed context path lookup so a missing earlier candidate path does not prevent checking later candidate paths.

## Environment Fallback Audit Verification
- `cargo fmt --check` passed.
- `cargo test` passed.
- `cargo build` passed.
- `ish check` passed.
