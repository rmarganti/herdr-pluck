## Code documentation

- Add SUCCINCT docblocks to structs and methods.
- Give an added focus to Domain definitions.
- Do NOT document things that are painfully obvious. For example, a `Rect { w: int64, h: int64 }` has no need for documentation.
- If an implication is particularly complicated, document implementation sections.

## Verifying (MUST BE RUN BEFORE CONSIDERING A TASK COMPLETE)

- `cargo fmt --all -- --check`
- `cargo test --all-features`
- `cargo clippy --all-targets --all`
- For user-facing changes (picker, patterns, hints, layout, config, clipboard, URL opening), run the `verify-herdr-pluck` skill (`.agents/skills/verify-herdr-pluck/SKILL.md`, symlinked into `.claude/skills/`). It drives the real plugin actions in a private Herdr and captures evidence. Keep its `features/` map current when user-facing behavior changes.
