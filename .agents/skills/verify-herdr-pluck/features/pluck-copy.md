# Pluck a visible token

With a pane focused, the user invokes **Pluck visible token**. A temporary `Herdr Pluck` tab mirrors the pane and overlays short hints on every copyable token. Typing a hint copies that token to the system clipboard, closes the tab, and returns focus to the original pane. Escape or Ctrl-C cancels without copying.

## Sub-features

- `pluck-builtins` hints URLs, `file://` URLs, paths, SHAs, UUIDs, IPv4 addresses, hex literals, Kubernetes references, and long numbers.
- `pluck-copy` copies exactly the selected token, with no trailing newline.
- `pluck-dup` gives identical text the same hint everywhere it appears.
- `pluck-trim` drops sentence punctuation that trails a URL or path.
- `pluck-two-letter` switches to two-letter hints when there are more than 26 unique tokens.
- `pluck-cancel` closes on Escape or Ctrl-C without copying.

## How to get to it (user POV)

- A keybinding bound to `rmarganti.herdr-pluck.pluck` (for example `prefix+q`).
- `herdr plugin action invoke rmarganti.herdr-pluck.pluck`.

## Driving it with pluck-verify

Preconditions:

- Baseline from the [index](./README.md). The source pane is focused.

- **Hint every built-in.** Run `$V seed tokens.txt` and `$V invoke pluck`. In `picker.txt`, every value column has a hint over its first characters: `attps://…`, `sile:///…`, `drc/picker/session.rs`, `fa63023f`, `g23e4567-…`, `h92.168.40.12`, `jxdeadBEEF`, `keployment.apps/frontend`, `l876543210123`, and `qttps://example.com/trailing.`.
- **Duplicates share a hint.** In the same screen, both `src/picker/session.rs` rows show the same hint (`d`).
- **Copy.** Run `$V select d`. `clipboard: src/picker/session.rs`, and `focused:` is the source pane.
- **Trailing punctuation.** Run `$V invoke pluck` and `$V select q`. `clipboard: https://example.com/trailing`, without the period.
- **Two-letter hints.** Run `$V seed many.txt` and `$V invoke pluck`. The 30 IPs show `aa`, `as`, … `sf`. Run `$V select s d`. `clipboard: 10.0.0.29`.
- **Cancel.** Run `$V invoke pluck` and `$V select esc`, then `$V invoke pluck` and `$V select ctrl+c`. Both report `clipboard: <untouched>` and refocus the source pane.

## Gotchas

- The shell prompt is part of the screen, so the prompt's cwd path also gets a hint. Ignore it.
- Hint order is top-to-bottom, left-to-right over **unique** texts. Adding a line to a fixture shifts every later hint.
- More than 26 unique tokens switches *every* hint to two letters, including ones that used to be single letters.
- A user's global config can override built-ins. For example, a `version` pattern like `\d+\.\d+\.\d+` at priority 25 beats IPv4 (priority 50) and swallows `10.0.0.1` as `10.0.0`. The private Herdr home exists to keep the user's config out of the run. Don't verify against the real one.
- The copy empty state (`Herdr Pluck: no copyable matches found`) can't be reached from a shell pane, because the prompt's cwd is always a path match. It is covered by unit tests in `src/picker/render.rs`. Exercise empty states through [open-url](./open-url.md) instead.
