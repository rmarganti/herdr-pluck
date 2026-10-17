# Clipboard backends

Pluck copies through one of two backends, chosen by `clipboard` in the global plugin config. The default, `osc52`, hands the text to Herdr, which writes it to the clipboard of the machine the user is viewing from, so copies reach the user's machine over `herdr --remote`. `system` runs `pbcopy`, `wl-copy`, `xclip`, or `xsel` on the machine running the Herdr server instead.

## Sub-features

- `clip-osc52-default` copies to the viewing client's clipboard when `clipboard` is unset, and never touches the server machine's clipboard.
- `clip-system` copies with the server machine's clipboard tools when `clipboard = "system"`, and the viewer's clipboard is untouched.
- `clip-invalid` treats an unknown `clipboard` value as a broken config: it logs `failed to load config, using defaults` and copies with `osc52`.

## How to get to it (user POV)

- Edit `$(herdr plugin config-dir rmarganti.herdr-pluck)/config.toml` and set `clipboard = "osc52"` or `clipboard = "system"`. It takes effect on the next invoke, with no reload.
- Then invoke `rmarganti.herdr-pluck.pluck` and pick a hint.

## Driving it with pluck-verify

Preconditions:

- Baseline from the [index](./README.md). The source pane is focused.

- **OSC 52 by default.** Run `$V seed tokens.txt`, `$V invoke pluck`, and `$V select d`. `clipboard: src/picker/session.rs` and `host clipboard: <untouched>`.
- **System backend.** Run `$V global-config global-config-system-clipboard.toml`, `$V invoke pluck`, and `$V select d`. `clipboard: <untouched>` and `host clipboard: src/picker/session.rs`.
- **Back to the default.** Run `$V global-config global-config.toml`, `$V invoke pluck`, and `$V select d`. The copy lands in `clipboard:` again.
- **Invalid value.** Write a scratch file containing `clipboard = "pbcopy"`, run `$V global-config <that file>`, `$V invoke pluck`, and `$V select d`. `clipboard: src/picker/session.rs` (the OSC 52 fallback). After `$V down`, `evidence/plugin-log.json` contains `failed to load config, using defaults`. Restore with `$V global-config global-config.toml`.

## Gotchas

- `global-config-system-clipboard.toml` has no custom patterns. Run [custom-patterns](./custom-patterns.md) journeys only with `global-config.toml` installed.
- Doctor compares the installed config against the last fixture passed to `global-config`, not always `global-config.toml`.
- An invalid config drops custom patterns too, not just the clipboard setting, because the whole file fails to parse.
