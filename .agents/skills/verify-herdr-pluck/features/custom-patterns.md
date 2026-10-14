# Custom patterns

Users add `[[patterns]]` entries with a `name`, a `regex`, and an optional `priority` (default 25; lower wins) in two places: the global plugin config (`$(herdr plugin config-dir rmarganti.herdr-pluck)/config.toml`) and project-local `.herdr-pluck.toml` files found between the focused pane's cwd and its Git root. A named capture `match` narrows the copied text to that capture. Custom patterns apply to **pluck** only.

## Sub-features

- `custom-global` adds global config patterns to the picker.
- `custom-project` discovers `.herdr-pluck.toml` by walking up from the pane cwd to the Git root.
- `custom-capture` uses `(?<match>…)` to hint and copy only the capture.
- `custom-priority` lets a custom pattern with lower priority beat an overlapping built-in.
- `custom-overlap` resolves equal-priority overlaps; see Gotchas for the observed rule.

## How to get to it (user POV)

- Edit `config.toml` in `herdr plugin config-dir rmarganti.herdr-pluck`, or create `.herdr-pluck.toml` in a project, then invoke **Pluck visible token** in a pane under that project.

## Driving it with pluck-verify

Preconditions:

- Baseline from the [index](./README.md). `up` installed `fixtures/global-config.toml` as the private global config and `fixtures/project-config.toml` as `fixture/.herdr-pluck.toml`. The source pane's cwd is `fixture/project`, one level below the fake Git root.

- **Hint custom tokens.** Run `$V seed custom-patterns.txt` and `$V invoke pluck`. `picker.txt` shows `aLK-42`, `trace_id=sbc123`, `dLB-7`, `frec=alpha`, and `guild-1a2b3c4`.
- **Project pattern.** Run `$V select a`. `clipboard: PLK-42`.
- **Capture group.** Run `$V invoke pluck` and `$V select s`. `clipboard: abc123` (not `trace_id=abc123`). The hint sits on the capture, not on `trace_id=`.
- **Global pattern.** Run `$V invoke pluck` and `$V select d`. `clipboard: GLB-7`.
- **Beats a built-in.** Run `$V invoke pluck` and `$V select g`. `clipboard: build-1a2b3c4`, because global `build-id` (25) beats built-in `git-sha` (40) on the overlapping `1a2b3c4`.
- **Equal-priority overlap.** Run `$V invoke pluck` and `$V select f`. Observed: `clipboard: prec=alpha`. The longer global `global-prec` span wins over the shorter project `project-prec` capture `alpha`.

## Gotchas

- **Precedence differs from the README.** The README says equal-priority overlaps prefer project-local, then global, then built-ins. Overlap resolution (`resolve_overlaps` in `src/patterns.rs`) actually orders by priority, then **longer span**, then position, and only then by origin. Origin decides only between identical spans, and identical spans copy the same text, so the README rule can't be observed. The `prec` step pins the current behavior. If the product changes to match the README, expect `alpha` and update this step.
- Project discovery needs a Git root (a `.git` entry) above the cwd. Without one, project files are ignored silently.
- An invalid project config is ignored silently. A missing hint is the only symptom.
- Never edit the user's real global config. The private Herdr home's `plugin config-dir` is the only global config this skill touches.
