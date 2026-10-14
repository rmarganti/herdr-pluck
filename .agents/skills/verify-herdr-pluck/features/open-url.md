# Open a visible URL

The user invokes **Open visible URL**. A `Herdr Pluck: Open URL` tab hints only `http://`, `https://`, and `file://` URLs. Typing a hint opens that URL with the system default handler (`open` on macOS, `xdg-open` on Linux), closes the tab, and leaves the clipboard untouched.

## Sub-features

- `url-only` hints URLs and nothing else, even when other tokens are visible.
- `url-open` passes the selected URL to the launcher as a single argument.
- `url-no-clipboard` leaves the clipboard unchanged.
- `url-empty` shows `Herdr Pluck: no openable URLs found` when no URL is visible.

## How to get to it (user POV)

- A keybinding bound to `rmarganti.herdr-pluck.open-url` (for example `prefix+o`).
- `herdr plugin action invoke rmarganti.herdr-pluck.open-url`.

## Driving it with pluck-verify

Preconditions:

- Baseline from the [index](./README.md). The source pane is focused.

- **Only URLs are hinted.** Run `$V seed tokens.txt` and `$V invoke open-url`. In `picker.txt` the `url`, `file`, and `trailing` rows have hints (`a`, `s`, `d`). The path, SHA, UUID, IP, hex, k8s, and number rows are unchanged.
- **Open a `file://` URL.** Run `$V select s`. `opened: file:///tmp/herdr-pluck/report.html` and `clipboard: <untouched>`.
- **Open an `https://` URL.** Run `$V invoke open-url` and `$V select a`. `opened: https://example.com/docs/getting-started`.
- **Empty state.** Run `$V seed no-urls.txt` and `$V invoke open-url`. The picker reads `Herdr Pluck: no openable URLs found` / `Press any non-Enter key to close`. Run `$V select x`. Nothing is opened.

## Gotchas

- The tab label is `Herdr Pluck: Open URL`, not `Herdr Pluck`.
- Custom patterns never apply to this action, by design. Don't use `custom-patterns.txt` here.
- `opened-urls` is written by the `open`/`xdg-open` shim. A real browser never launches during verification, so whether the OS handler accepts the URL is out of scope.
