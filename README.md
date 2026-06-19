# Herdr Pluck

Rust Herdr plugin scaffold for tmux-fingers-style inline hints over visible pane text.

## Development

```bash
cargo build
cargo test
cargo fmt --check
```

## Herdr plugin shape

The plugin manifest is `herdr-plugin.toml`.

- Action: `rmarganti.herdr-pluck.pluck`
- Overlay pane entrypoint: `picker`
- Binary: `herdr-pluck`

During local development:

```bash
cargo build --release
herdr plugin link .
herdr plugin action invoke rmarganti.herdr-pluck.pluck
```

Suggested keybinding:

```toml
[[keys.command]]
key = "prefix+q"
type = "plugin_action"
command = "rmarganti.herdr-pluck.pluck"
description = "pluck visible token"
```

## Entrypoints

```bash
herdr-pluck open-overlay [--target-pane PANE_ID]
herdr-pluck pick [--target-pane PANE_ID]
```

The action entrypoint captures the originally focused pane from Herdr context
and opens the picker overlay with `HERDR_PLUCK_TARGET_PANE_ID` set. The picker
entrypoint is currently a scaffold placeholder; matching, hinting, rendering,
input, clipboard, and Herdr API behavior are split into isolated modules for
follow-up ishes.
