# Herdr Pluck

Herdr Pluck is a Herdr plugin for quickly copying visible terminal tokens with short keyboard hints, inspired by `tmux-fingers`.

Invoke the plugin while a pane is focused, type the displayed hint for the token you want, and the selected text is copied to your system clipboard. Escape or Ctrl-C cancels.

![Herdr Pluck picker overlay](docs/images/pluck-picker.png)

## Requirements

- Herdr 0.7.0 or newer
- For release installs, a download tool:
    - `curl` or `wget`
- Rust/Cargo only when forcing a source build or when no matching prebuilt binary is available
- A system clipboard command:
    - macOS: `pbcopy`
    - Linux Wayland: `wl-copy`
    - Linux X11: `xclip` or `xsel`

## Install

From the remote repository:

```bash
herdr plugin install rmarganti/herdr-pluck
```

Published releases provide prebuilt binaries for these targets:

- macOS Apple Silicon: `aarch64-apple-darwin`
- Linux x86_64: `x86_64-unknown-linux-musl`

To install a specific branch, tag, or commit, pass `--ref`:

```bash
herdr plugin install rmarganti/herdr-pluck --ref main
```

Install first downloads the GitHub Release asset matching the version in `herdr-plugin.toml`. If that asset is unavailable, it falls back to a local Cargo build when Rust is available.

From this checkout:

```bash
herdr plugin link .
```

By default, linking also installs the prebuilt binary matching `herdr-plugin.toml`. To build the checked-out source instead:

```bash
HERDR_PLUCK_BUILD_FROM_SOURCE=1 herdr plugin link .
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

Custom global patterns can be added in the plugin config directory:

```bash
CONFIG_DIR="$(herdr plugin config-dir rmarganti.herdr-pluck)"
$EDITOR "$CONFIG_DIR/config.toml"
```

Example:

```toml
[[patterns]]
name = "jira"
regex = "\\b[A-Z][A-Z0-9]+-[0-9]+\\b"
priority = 25
```

Project-local patterns are also enabled by default. Herdr Pluck looks for `.herdr-pluck.toml` from the focused pane's working directory up to the Git root. Disable or customize this in the global config:

```toml
[project]
patterns = true
pattern_files = [".herdr-pluck.toml"]
```

Project-local config files use the same `[[patterns]]` shape as global config. Pattern precedence for equal-priority overlaps is project-local, then global, then built-ins.

`regex` uses Rust regular expression syntax. If a named capture called `match` is present, only that capture is copied; otherwise the whole regex match is copied:

```toml
[[patterns]]
name = "trace-id"
regex = "trace_id=(?<match>[A-Za-z0-9_-]+)"
priority = 25
```

For `trace_id=abc123`, this pattern highlights and copies only `abc123`.

Lower `priority` values win overlapping matches. If omitted, custom pattern priority defaults to `25`.

When identical text appears more than once, every visible occurrence shows the same hint and copies the same text.

## Releasing binaries

Tag releases as `vX.Y.Z`. GitHub Actions validates the crate, builds release archives, and uploads platform binaries to the matching GitHub Release.

## Troubleshooting

If invoking the action does nothing useful, check that the plugin is linked and the installed binary exists:

```bash
herdr plugin link .
ls -l ./bin/herdr-pluck
herdr plugin action list --plugin rmarganti.herdr-pluck
```

If no release asset matches the plugin version, make sure Rust/Cargo is available for the local fallback build. Set `HERDR_PLUCK_BUILD_FROM_SOURCE=1` to skip the release download and build the checked-out source explicitly.

If copying fails, install one of the supported clipboard tools for your platform and try again.
