# Herdr Pluck

Rust Herdr plugin for tmux-fingers-style inline hints over visible pane text.

## Development

```bash
cargo build
cargo test
cargo fmt --all -- --check
```

## Herdr plugin shape

The plugin manifest is `herdr-plugin.toml`.

- Action: `rmarganti.herdr-pluck.pluck`
- Production entrypoint: `herdr-pluck open`
- Picker entrypoint: `herdr-pluck pick --snapshot PATH`
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
herdr-pluck open [--target-pane PANE_ID]
herdr-pluck pick --snapshot PATH
```

The action entrypoint captures the originally focused pane, creates a temporary
Herdr tab with the same split layout, launches picker mode in the corresponding
temporary pane, and closes the temporary tab when the picker exits. The current
picker is still a scaffold placeholder; full hint input and clipboard copy are
follow-up work.
