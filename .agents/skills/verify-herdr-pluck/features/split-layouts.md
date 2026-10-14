# Split layouts

When the focused pane is one of several splits, the picker tab recreates the same split layout. The target pane's position shows the hinted copy, and every other position is a blank placeholder. After a selection or cancel, the picker tab closes and focus returns to the pane the user started from, not to the tab's first pane.

## Sub-features

- `split-mirror` makes the picker tab match the source tab's split structure.
- `split-target-only` hints only the focused pane. Sibling panes are blank placeholders.
- `split-refocus` returns focus to the originating split.

## How to get to it (user POV)

- Split a tab, focus any pane, and invoke **Pluck visible token** or **Open visible URL**.

## Driving it with pluck-verify

Preconditions:

- Baseline from the [index](./README.md). The source pane already shows a fixture (`$V seed tokens.txt`), so the left split has visible tokens that must *not* be hinted.

- **Split and seed.** Run `$V split custom-patterns.txt`. It prints the new right-hand pane id, which becomes the source pane and is focused.
- **Mirror the layout.** Run `$V invoke pluck`. The picker tab has two panes. `picker.txt` (the focused right pane) shows the custom-pattern hints from [custom-patterns](./custom-patterns.md).
- **Siblings are blank.** Run `$V panes`. The `"Herdr Pluck"` tab lists two panes, and the unfocused one is the left placeholder. Run `$V screen NN-pluck.left <that pane>`. The saved screen is entirely blank, with no `tokens.txt` content.
- **Focus returns.** Run `$V select s`. `clipboard: abc123`, and `focused:` is the right-hand split, not the left one.

## Gotchas

- Splitting halves the width. Long fixture lines wrap, and wrapped tokens may not match. Keep split fixtures short (`custom-patterns.txt` fits).
- `$V split` replaces the recorded source pane. Later drives target the new split until `down`.
- Don't call `herdr` directly to inspect the run. Inside a Herdr pane, the inherited `HERDR_SOCKET_PATH` points at the user's real server. Use `$V panes` and `$V screen`, which always target the private home.
