# Herdr Pluck

Herdr Pluck is a Herdr plugin for quickly copying visible terminal tokens with short keyboard hints, inspired by `tmux-fingers`.

Invoke the plugin while a pane is focused, type the displayed hint for the token you want, and the selected text is copied to your system clipboard. Escape or Ctrl-C cancels.

## Requirements

- Herdr 0.7.0 or newer
- Rust/Cargo to build from source
- A system clipboard command:
  - macOS: `pbcopy`
  - Linux Wayland: `wl-copy`
  - Linux X11: `xclip` or `xsel`

## Install

From this checkout:

```bash
cargo build --release
herdr plugin link .
```

Verify Herdr can see the action:

```bash
herdr plugin action list --plugin rmarganti.herdr-pluck
```

The action id is:

```text
rmarganti.herdr-pluck.pluck
```

## Keybinding

Add a Herdr `plugin_action` binding to your Herdr config, choosing any free key you prefer:

```toml
[[keys.command]]
key = "prefix+q"
type = "plugin_action"
command = "rmarganti.herdr-pluck.pluck"
description = "pluck visible token"
```

Reload Herdr config after editing:

```bash
herdr server reload-config
```

## Usage

1. Focus a Herdr pane containing a URL, path, commit SHA, UUID, IP address, long numeric identifier, hex literal, Kubernetes reference, Git status path, branch, or diff path.
2. Invoke `rmarganti.herdr-pluck.pluck` through your keybinding or Herdr's plugin action command.
3. Herdr Pluck opens a temporary picker tab that mirrors the source layout and shows hints over copyable text in the target pane.
4. Type the shown one- or two-letter hint to copy that token and close the picker.
5. Press Escape or Ctrl-C to cancel without copying.

You can also invoke the action from the CLI:

```bash
herdr plugin action invoke rmarganti.herdr-pluck.pluck
```

## What gets matched

Herdr Pluck recognizes these built-in token types, in priority order:

1. URLs
2. Git status paths, Git upstream branch names, and diff paths
3. Kubernetes resource references such as `pod/nginx` or `deployment.apps/frontend`
4. File paths
5. UUIDs
6. Deployment-managed Kubernetes pod names
7. Git SHAs
8. Hex literals such as `0xdeadBEEF`
9. IPv4 addresses
10. Long numeric identifiers

The expanded pattern set is always enabled for v1.1-style compatibility with `tmux-fingers`; user-configurable regex sets remain out of scope for v1.

When identical text appears more than once, every visible occurrence shows the same hint and copies the same text.

## Behavior and limits

- Hints are keyboard-only and fixed-width: one character for small match sets, two characters for larger match sets.
- Hint characters replace the beginning of matched text on screen so pane geometry stays aligned.
- Matching is based on visible pane content and handles soft-wrapped tokens such as long URLs.
- The picker renders a simplified view instead of preserving the source pane's original colors.
- v1 supports up to 676 unique copied texts in one picker view.
- Enter is ignored while the picker is active.
- Invalid full-width hints clear the typed hint buffer so you can try again.

## Not in v1

- Mouse selection
- OSC52 clipboard copying
- Custom regex configuration
- Non-copy actions such as opening URLs or jumping to text
- Multi-select
- Preserving original ANSI colors/styles
- Windows support

## Troubleshooting

If invoking the action does nothing useful, check that the plugin is linked and the release binary exists:

```bash
cargo build --release
herdr plugin link .
herdr plugin action list --plugin rmarganti.herdr-pluck
```

If copying fails, install one of the supported clipboard tools for your platform and try again.
