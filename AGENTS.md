# Agent Instructions

These instructions apply to code agents working in this repository, including Codex.

## Before finishing a change

Run the repository verification commands from the workspace root and fix any issues before handing work back:

```bash
cargo fmt --all -- --check
cargo test
cargo clippy --workspace --all-targets --all-features -- -D warnings -W clippy::too_many_lines -W clippy::too_many_arguments -W clippy::type_complexity -W clippy::excessive_nesting -W clippy::cognitive_complexity -W clippy::pedantic -W clippy::nursery -W clippy::cargo
cargo build --locked --all-targets
```

## Notes

- Treat Clippy warnings as errors for generated or edited code.
- Prefer changes that satisfy the repository `clippy.toml` configuration without adding `#[allow(...)]` attributes unless a maintainer explicitly asks for them.
- If a command cannot be run in the current environment, call that out clearly in the handoff.
