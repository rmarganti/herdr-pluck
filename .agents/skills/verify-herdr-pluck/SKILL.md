---
name: verify-herdr-pluck
description: Live, end-to-end verification of the Herdr Pluck plugin. Builds this checkout, starts a private headless Herdr with only this plugin linked, invokes the real `pluck` and `open-url` plugin actions, types hint keys, and captures clipboard/browser side effects as evidence. Use after changing picker, pattern, hint, layout, config, clipboard, or URL-opening behavior, or whenever a task needs proof that the plugin works inside Herdr and not just in `cargo test`.
---

# Verify Herdr Pluck

Herdr Pluck is a Herdr plugin. The user focuses a pane and invokes an action, usually through a keybinding. The plugin then opens a temporary picker tab that mirrors the source layout and overlays one- or two-letter hints on visible tokens. Typing a hint copies the token (`pluck`) or opens the URL (`open-url`), closes the tab, and returns focus to the source pane.

This skill drives that exact path through `herdr plugin action invoke`, the same entry point a keybinding uses. It runs against a private, throwaway Herdr, so the user's real Herdr, plugin config, clipboard, and browser are never touched.

Run everything from the root of **the checkout you changed** through one helper:

```bash
V=.agents/skills/verify-herdr-pluck/scripts/pluck-verify
$V --help
```

Requirements: `herdr` 0.7.4 or newer, `jq`, `cargo`, macOS or Linux. Any checkout or git worktree works. The helper tests the checkout that contains your current directory, and doctor's `plugin root is this checkout` line confirms which one.

This complements the static checks AGENTS.md requires; it doesn't replace them: `cargo fmt --all -- --check`, `cargo test --all-features`, `cargo clippy --all-targets --all`.

## Launch

```bash
$V up            # build + install bin/herdr-pluck, start a private Herdr, link, doctor
$V up --no-build # reuse the current bin/herdr-pluck
```

`up` does the following:

1. Runs the plugin's real build hook in source mode: `HERDR_PLUCK_BUILD_FROM_SOURCE=1 ./scripts/install-binary.sh`.
2. Creates `target/verify-herdr-pluck/<run-id>/` and points `target/verify-herdr-pluck/current` at it. Every later subcommand reads `current`, so separate shell calls share one run. Only one run per checkout can be active.
3. Starts a headless `herdr server` with `XDG_CONFIG_HOME` set to a fresh `/tmp/hp.XXXXXX` directory. That gives the run its own plugin registry, plugin config dir, sockets, and sessions. The path stays short because Unix socket paths have a length limit.
4. Puts shims first on that server's `PATH` for `pbcopy`, `wl-copy`, `xclip`, `xsel`, `open`, and `xdg-open`. Plugin actions inherit the server's environment, so the plugin's real command runners call the shims, which record what they receive.
5. Runs `herdr plugin link <checkout>` in the private registry, then installs `fixtures/global-config.toml` as the private global plugin config.
6. Creates workspace `verify` with cwd `fixture/project`, under a fake Git root (`fixture/.git`) whose `.herdr-pluck.toml` comes from `fixtures/project-config.toml`. The root pane becomes the **source pane**.

The run is ready when `up` ends with doctor printing only `ok` lines. Teardown is `$V down`; see [Cleanup](#cleanup).

The helper unsets every inherited `HERDR_*` variable. This matters when the agent itself runs inside a Herdr pane, because `HERDR_SOCKET_PATH` would otherwise route commands to the user's real server. For the same reason, don't call `herdr` directly to inspect a run. Use `$V panes` and `$V screen`.

## Doctor

```bash
$V doctor
```

Doctor is read-only. Run it first whenever anything looks off, and again after any failed drive. Don't drive while any line says `FAIL`. Doctor checks that:

- The server PID started by this run is alive, and its socket is inside this run's private home.
- The server's `PATH` starts with this run's shims. Without that, a drive would write to the real clipboard or open a real browser.
- `rmarganti.herdr-pluck` is the only enabled plugin, and its `plugin_root` is this checkout.
- The private global plugin config is exactly `fixtures/global-config.toml`.
- `bin/herdr-pluck --version` matches `herdr-plugin.toml`, and the binary is byte-identical to `target/release/herdr-pluck`, so the tested binary is this source.
- The source pane is focused, and no `Herdr Pluck*` picker tab is open.

## Drive

One drive is `invoke` followed by `select`:

```bash
$V seed tokens.txt             # clear the source pane and print a fixture
$V invoke pluck                # or: open-url. Prints the drive id, the picker pane, and the hinted screen
$V select d                    # type the hint shown on screen; waits for the picker tab to close
$V select s d                  # two-letter hint
$V select esc                  # cancel (ctrl+c also cancels)
$V split custom-patterns.txt   # split the source pane right, seed it, make it the new source
$V panes                       # list panes with their tab label and focus
$V screen <name> [pane]        # save any pane's screen as extra evidence
```

- **Fixtures** live in `fixtures/` and each one ends with an `end-of-fixture` line. `seed` waits for that line because it appears only in the output, never in the typed command. Your own fixture file must end with the same line.
- **Hints overwrite the leading characters of each token**: `https://…` with hint `a` renders as `attps://…`. Read the hint from the screen `invoke` prints.
- **Focus decides the target.** `invoke` stops if Herdr reports a focused pane other than the recorded source pane.
- Picker tabs are labelled `Herdr Pluck` (pluck) and `Herdr Pluck: Open URL` (open-url).

The feature map in [`features/README.md`](features/README.md) lists every journey, its exact commands, and the observable result that proves it. **Before driving, read its "Choosing journeys" table.** It says which existing journeys your change must re-run, beyond a fixture for the new case, and how to update the map when user-facing behavior changes.

## Evidence

Everything lands in `target/verify-herdr-pluck/<run-id>/evidence/` and survives `down`. Each drive has an id like `03-pluck`:

| File | Proves |
| --- | --- |
| `NN-<action>.invoke.json` | The real action ran, with Herdr's invocation context (`focused_pane_id`, `focused_pane_cwd`) |
| `NN-<action>.picker.txt` / `.ansi` | The hinted screen the user saw (`.ansi` keeps the hint styling) |
| `NN-<action>.keys.txt` | The keys typed |
| `NN-<action>.clipboard` | The exact bytes sent to the clipboard tool, with no trailing newline |
| `NN-<action>.opened-urls` | The arguments passed to the URL launcher, one per line |
| `NN-<action>.result.txt` | Summary: clipboard, opened URLs, and the focused pane afterwards |
| `plugin-link.json` | The private registry linked this checkout |
| `plugin-log.json`, `herdr-server.log`, `server.out` | Herdr's action log and server output, saved by `down` |

Proof standards:

- Drive through `herdr plugin action invoke` only, which is what `$V invoke` does. Running `bin/herdr-pluck pick …` directly skips layout, context, config discovery, and cleanup.
- Capture both the hinted picker screen and the resulting side effect. A hint on screen doesn't prove it copies the right text, and a clipboard value doesn't prove the user could see the hint.
- A missing `.clipboard` file means nothing was copied. That's the proof for cancel and for open-url.
- Assert that focus returned (`focused:` in `result.txt`) and that no picker tab is left (`doctor`).
- `plugin-log.json` covers only the short-lived `open` launcher, which exits 0 as soon as the picker tab exists. It says nothing about the selection. The captures are the proof.
- The shims replace the system tools at a process boundary the plugin already isolates (`src/clipboard/system.rs`, `src/url_opener/system.rs`). The arguments each platform tool receives (for example `wl-copy --type text/plain;charset=utf-8`) are covered by unit tests, not by this skill.

## Cleanup

```bash
$V down
```

`down` saves the plugin log and server logs, stops the private server, signals only the PID this run started, deletes the private `/tmp/hp.*` home, and removes the run's shims, captures, and fixture project. It keeps `evidence/` and lists it. Run `down` after a failed attempt too, so broken runs don't strand servers. If `up` refuses because a run is still active, `down` that run first.

Never run `herdr server stop` or `herdr session stop` yourself, never kill Herdr by process name, and never touch the user's real `~/.config/herdr`. Don't run `herdr plugin link` or edit `herdr plugin config-dir` outside the helper. Those operate on the user's real registry and config.

## Helpers

- `scripts/pluck-verify` is the only helper. It is executable. Run `$V --help` for the subcommand list.
- `fixtures/tokens.txt` holds every built-in token type, a duplicate path, and a URL with a trailing period.
- `fixtures/many.txt` holds 30 unique IPs, which forces two-letter hints.
- `fixtures/no-urls.txt` holds tokens but no URLs, for the open-url empty state.
- `fixtures/custom-patterns.txt` holds text matched only by the custom patterns.
- `fixtures/global-config.toml` is the private global config: `global-ticket`, `global-prec`, and `build-id`.
- `fixtures/project-config.toml` is the project-local config: `ticket`, `trace-id` (with a `match` capture), and `project-prec`.
