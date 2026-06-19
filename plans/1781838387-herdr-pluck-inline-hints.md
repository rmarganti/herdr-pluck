## Problem Statement

Users migrating from tmux to Herdr lose the fast “visible terminal pluck” workflow provided by tmux-fingers: press a key, see short inline key hints over interesting strings in the current terminal viewport, type the hint, and have that string copied immediately. Herdr has normal mouse selection and keyboard copy mode, but those workflows are slower when the target is a URL, file path, SHA, UUID, IP address, or other recognizable token visible somewhere in a pane.

The user wants a Herdr-native plugin that restores the core tmux-fingers copy workflow without attempting to clone tmux-fingers completely. The initial scope is copy-only: show inline hints for matched patterns in the live bottom viewport, accept a keyboard hint, copy the selected pattern via available system clipboard tools, and close immediately.

## Solution

Build a Rust-based Herdr plugin, tentatively named Herdr Pluck, that provides an interactive overlay picker. A Herdr keybinding invokes a plugin action. That action captures the originally focused pane id and opens a Herdr plugin overlay pane, explicitly passing the original pane id into the picker process. The picker reads the live bottom viewport as unwrapped recent text, obtains the target pane dimensions from Herdr layout APIs, scans hardcoded built-in regex patterns, assigns fixed-width keyboard hints, manually wraps the rendered logical lines to the target pane width, and displays a monochrome inline hint view.

The overlay renders non-matched text dim/black, full matched text white, and destructive inline hint characters cyan. Users type the displayed fixed-width hint to copy the corresponding matched text. Successful copy exits the overlay immediately. Escape and Ctrl-C cancel. Enter does nothing. Invalid fixed-width hints clear the input buffer.

The plugin uses a system-available clipboard fallback chain for v1. OSC52, custom regex configuration, richer actions, mouse support, and exact support for non-bottom scrolled viewports are deferred.

## User Stories

1. As a Herdr user migrating from tmux, I want to press a key to show copy hints over visible terminal strings, so that I can preserve my tmux-fingers workflow.
2. As a Herdr user, I want the picker to work inside a Herdr pane, so that I do not need tmux to copy visible patterns quickly.
3. As a Herdr user, I want the plugin to inspect the pane that was focused before the overlay opened, so that the picker targets the correct pane.
4. As a Herdr user, I want the picker to appear as an overlay, so that I keep visual context while selecting a string.
5. As a Herdr user, I want matched strings to be shown inline in a pane-like rendering, so that I can select items spatially rather than from a separate list.
6. As a Herdr user, I want the overlay to use a monochrome copy of the pane, so that the hint UI is clear and implementation complexity remains low.
7. As a Herdr user, I want non-matched text dimmed or black, so that matched strings stand out.
8. As a Herdr user, I want matched strings rendered white, so that I can quickly see every selectable target.
9. As a Herdr user, I want hint characters rendered cyan, so that the keys I need to type are visually obvious.
10. As a Herdr user, I want hints to destructively replace the beginning of the matched text, so that the pane geometry remains stable.
11. As a Herdr user, I want the plugin to find URLs, so that I can quickly copy visible links.
12. As a Herdr user, I want the plugin to find file paths, so that I can quickly copy paths from compiler output, test failures, git output, and logs.
13. As a Herdr user, I want the plugin to find Git SHAs, so that I can quickly copy commit hashes.
14. As a Herdr user, I want the plugin to find UUIDs, so that I can quickly copy identifiers from logs.
15. As a Herdr user, I want the plugin to find IPv4 addresses, so that I can quickly copy visible network addresses.
16. As a Herdr user, I want the plugin to find long numeric identifiers, so that I can quickly copy issue numbers, ports, IDs, and other numeric tokens.
17. As a Herdr user, I want pattern conflicts to resolve predictably, so that the full URL is selected instead of a smaller SHA or path inside it.
18. As a Herdr user, I want higher-priority patterns to win over lower-priority overlapping patterns, so that useful larger tokens are preferred.
19. As a Herdr user, I want longer matches to win within a priority level, so that more complete strings are copied.
20. As a Herdr user, I want leftmost/topmost matches to win when all else is equal, so that hint assignment is deterministic.
21. As a Herdr user, I want duplicate matched text to share the same hint, so that repeated identical tokens do not waste hint capacity.
22. As a Herdr user, I want the same hint rendered at every duplicate occurrence, so that any visible occurrence communicates the same copy action.
23. As a Herdr user, I want copied text to be based on the selected match text, not its screen position, so that duplicates behave naturally in copy-only mode.
24. As a Herdr user, I want hints assigned top-to-bottom and left-to-right, so that hint placement is predictable.
25. As a Herdr user, I want ergonomic home-row-ish hint letters, so that common selections are fast to type.
26. As a Herdr user, I want all lowercase letters available as hints, so that the hint space is maximized.
27. As a Herdr user, I want fixed-width hints, so that the picker never has ambiguous prefixes.
28. As a Herdr user, I want one-character hints when there are few matches, so that selection is fast.
29. As a Herdr user, I want two-character hints when there are more matches, so that the picker can cover a full busy viewport.
30. As a Herdr user, I want v1 to cap at two-character hints, so that the interaction remains usable.
31. As a Herdr user, I want excessive matches silently capped, so that v1 stays simple and avoids noisy warnings.
32. As a Herdr user, I want matches shorter than the hint width ignored, so that hints do not overwrite unrelated text.
33. As a Herdr user, I want the plugin to handle terminal soft wrapping, so that long URLs and paths can be copied even when they wrap on screen.
34. As a Herdr user, I want matching to operate on unwrapped logical lines, so that wrapped strings are treated as one copyable target.
35. As a Herdr user, I want rendering to wrap manually to the target pane width, so that the overlay lines align reasonably with the original pane.
36. As a Herdr user, I want the plugin to focus on the live bottom viewport, so that it matches my normal “copy what I just saw” workflow.
37. As a Herdr user, I want scrolled-back viewport support to be out of v1 scope, so that the initial plugin can be implemented quickly.
38. As a Herdr user, I want the plugin to read enough recent unwrapped lines to reconstruct the live bottom viewport, so that soft-wrapped matches work.
39. As a Herdr user, I want the plugin to use Herdr pane layout dimensions, so that wrapping and cropping use the actual target pane size.
40. As a Herdr user, I want typing an exact hint to copy immediately, so that the workflow is fast.
41. As a Herdr user, I want the overlay to close immediately after a successful copy, so that I return to my work without extra confirmation.
42. As a Herdr user, I want Escape to cancel, so that I can leave the picker quickly.
43. As a Herdr user, I want Ctrl-C to cancel, so that common terminal cancellation muscle memory works.
44. As a Herdr user, I want Enter to be ignored, so that fixed-width hints remain self-submitting and simple.
45. As a Herdr user, I want invalid fixed-width hints to clear the input buffer, so that I can recover from mistypes without restarting the picker.
46. As a Herdr user, I want no mouse support in v1, so that the plugin remains focused on key-hint copying.
47. As a Herdr user on macOS, I want copying to use available platform tools, so that selected text reaches my system clipboard.
48. As a Herdr user on Linux Wayland, I want copying to use available Wayland clipboard tools when present, so that selected text reaches my clipboard.
49. As a Herdr user on Linux X11, I want copying to use common X11 clipboard tools when present, so that selected text reaches my clipboard.
50. As a Herdr user, I want a clear failure if no clipboard tool is available, so that I know why the copy did not complete.
51. As a Herdr user, I want OSC52 deferred, so that v1 avoids terminal-specific clipboard complexity.
52. As a plugin developer, I want the Herdr integration isolated behind an adapter, so that matching and rendering can be tested without a running Herdr instance.
53. As a plugin developer, I want the pattern engine isolated, so that overlap resolution, named captures, and deduplication are easy to test.
54. As a plugin developer, I want the hint engine isolated, so that hint width, cap behavior, and duplicate mapping are easy to test.
55. As a plugin developer, I want the renderer isolated, so that wrapping and style segmentation can be tested independently of terminal input.
56. As a plugin developer, I want the clipboard adapter isolated, so that fallback selection can be tested without writing to the actual clipboard.
57. As a plugin developer, I want the CLI entrypoint separated from core logic, so that action-opening and picker behavior remain understandable.
58. As a future user of custom regexes, I want the matcher model to support a named capture called match, so that custom patterns can copy only the useful substring.
59. As a future user of custom regexes, I want v1 architecture to anticipate user-defined patterns, so that config support can follow without a rewrite.
60. As a maintainer, I want Rust to produce a single shippable binary, so that plugin installation and distribution stay simple.

## Implementation Decisions

- The plugin will be implemented in Rust.
- The plugin will use Herdr's plugin system with a keybound plugin action and a declared plugin overlay pane.
- The keybinding invokes a plugin action rather than launching the picker directly, because the action can capture the originally focused pane before the overlay changes focus.
- The action will pass the original target pane id to the overlay picker through an explicit environment variable.
- The picker will inspect the target pane id from that explicit environment variable, not from the current focused pane.
- The plugin will use Herdr CLI calls through the Herdr-provided binary path rather than binding to private Herdr internals.
- The picker will obtain target pane dimensions from Herdr layout information.
- The picker will read live bottom viewport content using recent unwrapped terminal text with a line count derived from target pane height.
- The plugin will target the live bottom viewport only in v1.
- Exact scrolled viewport support is out of scope for v1.
- Matching will operate on unwrapped logical lines to support soft-wrapped URLs and paths.
- Rendering will manually wrap styled spans to the target pane width rather than relying on terminal auto-wrap.
- The overlay will render a monochrome pane-like view rather than preserving original ANSI styling.
- Non-matches will render black or dim.
- Matched text will render white.
- Hint characters will render cyan.
- Hints will destructively replace the beginning of the copied/highlighted match region.
- Matches shorter than the fixed hint width will be ignored.
- The initial pattern set will be hardcoded built-ins.
- User-defined regex configuration will be anticipated architecturally but not included in v1.
- Pattern priority will be applied before match length and position.
- Overlapping matches will be rejected after accepting higher-priority, longer, earlier matches.
- The initial priority order will prefer URLs, then file paths, then UUIDs, then Git SHAs, then IPv4 addresses, then long numeric identifiers.
- A named capture called match will define the copied/highlighted substring when present; otherwise the full regex match will be used.
- Duplicate copied text will share one hint.
- The same hint will be rendered at every visible duplicate occurrence.
- Hint assignment will follow the first visible occurrence of each unique copied text in top-to-bottom, left-to-right order.
- The hint alphabet will be home-row-ish: home row letters first, then top row, then bottom row, with no excluded letters.
- Hints will be fixed-width.
- The plugin will use one-character hints when possible and two-character hints otherwise.
- The plugin will cap v1 hinting at two-character hints, yielding a maximum of 676 unique copied texts.
- Matches beyond the v1 hint cap will be silently omitted.
- Input will be keyboard-only.
- Exact hint entry will copy and exit immediately.
- Invalid fixed-width hint entry will clear the input buffer and continue.
- Escape and Ctrl-C will cancel the picker.
- Enter will be ignored.
- A successful copy will close the overlay immediately.
- Clipboard behavior will use a system-available tool fallback chain.
- OSC52 clipboard support is deferred.
- A Herdr adapter module will encapsulate plugin context parsing, pane layout retrieval, pane reading, and overlay launching.
- A pattern engine module will encapsulate hardcoded pattern definitions, priority handling, overlap rejection, named capture handling, and deduplication.
- A hint engine module will encapsulate alphabet ordering, fixed-width selection, cap behavior, and hint mapping.
- A renderer module will encapsulate monochrome styling, destructive hints, manual wrapping, and viewport cropping.
- An input loop module will encapsulate raw terminal mode, hint collection, cancellation, invalid input behavior, and exit behavior.
- A clipboard adapter module will encapsulate platform tool detection and copy execution.
- A CLI entrypoint module will provide at least two modes: one to open the overlay for the current target pane, and one to run the picker inside the overlay.

## Testing Decisions

- Tests should focus on external behavior and stable module contracts, not private implementation details.
- The pattern engine should have unit tests for built-in matching, priority order, longest-match selection, leftmost tie-breaking, overlap rejection, named capture behavior, duplicate text handling, and match cap preparation.
- The hint engine should have unit tests for fixed-width selection, one-character and two-character hint generation, maximum capacity behavior, deterministic ordering, and duplicate text mapping.
- The renderer should have unit tests for destructive hint replacement, match/non-match/hint style segmentation, manual wrapping to pane width, viewport height cropping, and behavior when matches are shorter than hint width.
- The clipboard adapter should have tests for fallback command selection and error reporting using faked command availability; tests should not require writing to the real system clipboard.
- The Herdr adapter should be designed so tests can use faked command responses rather than requiring a live Herdr server.
- The input loop should be tested at the behavior level where practical: exact hint triggers copy action, invalid hint clears input, Escape cancels, Ctrl-C cancels, and Enter is ignored.
- End-to-end tests against a live Herdr instance are not required for the first implementation, but the architecture should not preclude adding them later.
- Good tests should validate observable inputs and outputs: given pane text, dimensions, patterns, and user keystrokes, the plugin should produce expected rendered state and copy decisions.
- Existing prior art includes Herdr's own CLI/socket API shape for pane layout and pane read behavior, and tmux-fingers' behavior for joined wrapped lines and destructive inline hints.

## Out of Scope

- Non-copy actions such as open, paste, or jump mode.
- Mouse support.
- Preserving original pane ANSI colors and styles.
- Exact current scrolled viewport support.
- A native Herdr cell-overlay API.
- A Herdr-native clipboard or buffer API.
- OSC52 clipboard support.
- User-defined regex configuration in v1.
- Runtime plugin action registration.
- Native non-terminal plugin UI.
- Multi-select mode.
- Fuzzy finding or list-based picking.
- Configurable hint alphabet in v1.
- Configurable pattern priority in v1.
- Three-character or variable-length hints in v1.
- Warnings for omitted matches beyond the 676-match cap.
- Windows support unless it falls out naturally from the chosen Rust and command abstractions.

## Further Notes

The feature intentionally focuses on the highest-value tmux-fingers behavior: copy a visible pattern quickly by typing a short hint. The design borrows tmux-fingers' core trick for wrapped lines: match on unwrapped logical text, then render back into a same-width view so wrapping appears naturally. In Herdr, the closest v1 approximation is reading recent unwrapped bottom text using the target pane height, then manually wrapping in the plugin overlay.

The architecture should keep the matching, hinting, rendering, input, clipboard, and Herdr integration concerns separate. Those deep modules should make v1 easier to test and should make the near-term addition of user-defined regexes straightforward.

### Technical References

These references preserve important implementation context discovered while researching Herdr and tmux-fingers. They are not product requirements, but they should be useful during implementation.

- Herdr repository: https://github.com/ogulcancelik/herdr
- tmux-fingers repository: https://github.com/Morantron/tmux-fingers
- Herdr plugin authoring model: [`website/src/content/docs/plugins.mdx`](https://github.com/ogulcancelik/herdr/blob/main/website/src/content/docs/plugins.mdx). Key points: plugins are manifest-declared executable commands; there is no separate SDK; plugins call back through the Herdr CLI/socket API; plugin panes support `overlay` placement; plugin commands receive Herdr/plugin context environment.
- Herdr CLI reference: [`website/src/content/docs/cli-reference.mdx`](https://github.com/ogulcancelik/herdr/blob/main/website/src/content/docs/cli-reference.mdx). Key points: `pane read` supports `visible`, `recent`, `recent-unwrapped`, and `detection`; `plugin pane open` supports `overlay`; custom keybindings can invoke `plugin_action`.
- Herdr socket/API overview: [`website/src/content/docs/socket-api.mdx`](https://github.com/ogulcancelik/herdr/blob/main/website/src/content/docs/socket-api.mdx). Key points: pane APIs include `pane.layout`, `pane.read`, and plugin pane methods; `recent-unwrapped` is documented as the source that ignores soft wrapping.
- Herdr keybinding docs: [`website/src/content/docs/configuration.mdx`](https://github.com/ogulcancelik/herdr/blob/main/website/src/content/docs/configuration.mdx). Key point: `[[keys.command]]` with `type = "plugin_action"` is the intended keybinding path for installed plugin actions.
- Herdr pane schema: [`src/api/schema/panes.rs`](https://github.com/ogulcancelik/herdr/blob/main/src/api/schema/panes.rs). Key point: `PaneLayoutSnapshot` contains `PaneLayoutPane.rect`, and `PaneLayoutRect` includes `width` and `height`, which the picker can use for read line count and manual wrapping.
- Herdr terminal read implementation: [`src/pane/terminal.rs`](https://github.com/ogulcancelik/herdr/blob/main/src/pane/terminal.rs). Key points: `visible_text` is built from rendered row cells; `recent_unwrapped` uses Ghostty `read_text_screen(..., unwrap)` behavior, making it the closest public equivalent to tmux `capture-pane -J` for bottom-viewport use.
- tmux-fingers pane capture flow: [`src/fingers/commands/start.cr`](https://github.com/Morantron/tmux-fingers/blob/master/src/fingers/commands/start.cr) and [`src/tmux.cr`](https://github.com/Morantron/tmux-fingers/blob/master/src/tmux.cr). Key point: normal copy mode calls `capture-pane -J`, joining wrapped lines before matching; jump mode does not join.
- tmux-fingers matching/hinting: [`src/fingers/hinter.cr`](https://github.com/Morantron/tmux-fingers/blob/master/src/fingers/hinter.cr). Key points: matching runs against joined logical lines; duplicate text can reuse hints; named capture `match` is used as the copied/highlighted substring; hints longer than the highlighted text are skipped.
- tmux-fingers destructive rendering: [`src/fingers/match_formatter.cr`](https://github.com/Morantron/tmux-fingers/blob/master/src/fingers/match_formatter.cr). Key point: the hint replaces/chops part of the highlighted text rather than inserting extra columns, preserving geometry.
- tmux-fingers clipboard behavior: [`src/fingers/action_runner.cr`](https://github.com/Morantron/tmux-fingers/blob/master/src/fingers/action_runner.cr) and [`src/tmux.cr`](https://github.com/Morantron/tmux-fingers/blob/master/src/tmux.cr). Key points: tmux-fingers writes to the tmux buffer, optionally uses tmux `load-buffer -w`, and for system copy shells out through available tools such as `pbcopy`, `wl-copy`, `xclip`, and `xsel`.
