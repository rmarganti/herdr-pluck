# Herdr Pluck verification map

This directory is the maintained source for verifying Herdr Pluck's user-facing behavior. Read this index before driving, then use the matching feature file as the recipe. When the app gains, changes, or loses a user-facing behavior, update the map in the same change.

## Baseline preconditions

- `$V up` has completed and `$V doctor` prints only `ok` lines. `V=.agents/skills/verify-herdr-pluck/scripts/pluck-verify`.
- The private Herdr has exactly one plugin linked: this checkout as `rmarganti.herdr-pluck`.
- The private global plugin config is `fixtures/global-config.toml`. The source pane's cwd is `fixture/project`, under a fake Git root whose `.herdr-pluck.toml` is `fixtures/project-config.toml`.
- The source pane is focused and no picker tab is open.
- Never drive a Herdr home that this run's `up` did not create.

## Driving conventions

- Start every journey by seeding the source pane with a fixture: `$V seed <fixture>`.
- Invoke actions only through `$V invoke <pluck|open-url>`, which calls `herdr plugin action invoke` like a keybinding does.
- Read hints from the screen `invoke` prints. Hints overwrite the leading characters of each token, so `https://` with hint `a` reads `attps://`. Don't hard-code hint letters from these docs without checking the screen; the letters below are what the current fixtures produce.
- Type hints with `$V select <keys...>`. Multi-key hints go in one call (`$V select s d`).
- If a drive fails, run `$V doctor` before the next drive. If the picker is stuck, use `$V select esc`, or `$V down` and then `$V up --no-build`.

## Choosing journeys

A proof of a change is the new behavior **plus** the existing journeys that share its code. A one-off fixture that only shows the new case is not enough: re-run the mapped journeys below so regressions in neighbouring behavior show up. When unsure, run every journey. The full map is about 15 drives and takes a few minutes.

| If you changed… | Run at least |
| --- | --- |
| Built-in patterns, trimming, overlap rules (`src/patterns.rs`) | All of [pluck-copy](./pluck-copy.md), all of [custom-patterns](./custom-patterns.md), and "Only URLs are hinted" from [open-url](./open-url.md) |
| Custom pattern loading or discovery (`src/config.rs`) | All of [custom-patterns](./custom-patterns.md) |
| Hint assignment (`src/hints.rs`) | "Duplicates share a hint" and "Two-letter hints" from [pluck-copy](./pluck-copy.md) |
| Rendering (`src/renderer*`, `src/viewport.rs`, `src/picker/render.rs`) | "Hint every built-in", "Two-letter hints", the [open-url](./open-url.md) empty state, and "Siblings are blank" from [split-layouts](./split-layouts.md) |
| Key handling or picker lifecycle (`src/picker/session.rs`) | "Copy", "Two-letter hints", and "Cancel" from [pluck-copy](./pluck-copy.md), plus the [open-url](./open-url.md) empty state |
| Herdr integration: layout, launch, cleanup, focus (`src/herdr/*`) | All of [split-layouts](./split-layouts.md), plus "Copy" and "Cancel" from [pluck-copy](./pluck-copy.md) |
| Clipboard (`src/clipboard/*`) | "Copy" and "Cancel" from [pluck-copy](./pluck-copy.md) |
| URL opening (`src/url_opener/*`) | All of [open-url](./open-url.md) |
| Build or packaging (`scripts/install-binary.sh`, `herdr-plugin.toml`, `Cargo.toml`) | `$V up` with the build, then "Copy" from [pluck-copy](./pluck-copy.md) |

When the change adds or alters user-facing behavior, update the map in the same change:

- **New token type or pattern behavior:** add a line to `fixtures/tokens.txt`, or to `fixtures/custom-patterns.txt` and its config, and add a sub-feature and a driving step to the feature file. Adding fixture lines shifts later hint letters, so re-run the affected journeys and update every expected screen and letter in the docs from the new `picker.txt`.
- **Changed behavior:** rewrite the expected result in the feature file to match the new evidence.
- **New entry point or feature:** add a feature file following the contract below, and list it under [Features](#features).

## Proof and skip reporting

- Every drive leaves `evidence/NN-<action>.picker.txt`, which is what the user saw, and `evidence/NN-<action>.result.txt`, which records the clipboard, opened URLs, and the focused pane afterwards.
- Copy proof is the exact bytes in `NN-pluck.clipboard`. Open proof is `NN-open-url.opened-urls`. For cancel and open-url, the absence of a clipboard capture is the proof.
- Focus proof: `focused:` in `result.txt` equals the source pane, and doctor reports no leftover picker tab.
- Report the drive id with each claim, for example "03-pluck copied `src/picker/session.rs`".
- Report an unreachable path with the attempted command and the unmet precondition. Do not report a skipped entry point as verified through another one.

## Feature entry contract

Each feature file starts with an H1 title and one paragraph of user-visible behavior, followed by exactly these H2s, in order:

1. `Sub-features`: short IDs with one line each.
2. `How to get to it (user POV)`: every user entry point.
3. `Driving it with pluck-verify`: begins with `Preconditions:`, then labelled bullets pairing each user action with an exact command and the observable result.
4. `Gotchas`: traps that waste or invalidate a run.

Keep implementation details out unless they explain a gotcha.

## Features

- [Pluck a visible token](./pluck-copy.md) covers built-in token types, duplicate hints, trailing punctuation, two-letter hints, the empty state, and cancel.
- [Open a visible URL](./open-url.md) covers URL-only hinting, launching `http(s)://` and `file://` URLs, the clipboard staying untouched, and the empty state.
- [Custom patterns](./custom-patterns.md) covers global and project-local patterns, `match` captures, and priority over built-ins.
- [Split layouts](./split-layouts.md) covers mirroring the source layout, hinting only the target pane, and returning focus to the source pane.
