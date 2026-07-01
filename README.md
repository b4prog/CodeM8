# CodeM8

CodeM8 is a Rust command-line application for deterministic source code reports.
It can report functions whose cognitive or cyclomatic complexity exceeds
configurable limits:

```bash
codem8 --report-complexity
```

CodeM8 can also detect duplicated line-based code blocks in a repository:

```bash
codem8 --report-duplicate
```

The duplicate report is designed for both human developers and coding agents. It
trims source lines, ignores empty lines, hashes normalized lines with XXH3
128-bit, classifies syntax-only lines as block-only, groups repeated blocks, and
prints a stable plain-text report sorted by duplicate weight.

## Installation

Install `codem8` from the GitHub source with Cargo:

```bash
cargo install --locked --git https://github.com/b4prog/CodeM8 codem8
```

Build from a local checkout with Cargo:

```bash
cargo build --release
```

Install from a local checkout:

```bash
cargo install --locked --path .
```

Run from the local checkout without installing:

```bash
cargo run -- --report-complexity
```

## Usage

Analyze function complexity for languages supported by `rust-code-analysis`:

```bash
codem8 --report-complexity
```

Analyze supported source files from the current directory for duplicate code:

```bash
codem8 --report-duplicate
```

Restrict analysis to specific extensions:

```bash
codem8 --report-duplicate -file-extension=ts,tsx,js,jsx
```

Analyze an explicit list of files instead of recursively discovering files:

```bash
codem8 --report-duplicate -file-extension=ts,js -files="src/a.ts,src/b.js"
```

Quoting `-files` values is recommended in PowerShell when paths contain file
extensions.

Analyze lines changed on the current local Git branch compared to the origin
base branch:

```bash
codem8 --report-complexity -git-branch
```

The complexity and duplicate reports are mutually exclusive; run one report per
command.

Reports exit with a non-zero status when they detect issues: functions above the
configured limits for `--report-complexity`, or duplicate blocks for
`--report-duplicate`.

Include analyzed files, report metrics, and timing information:

```bash
codem8 --report-complexity -verbose
```

Set complexity thresholds:

```bash
codem8 --report-complexity -max-cognitive-complexity=15 -max-cyclomatic-complexity=10
```

## Complexity Report

The complexity report uses `rust-code-analysis` and only applies to file
extensions supported by that crate. It reports `SpaceKind::Function` entries
whose cognitive complexity exceeds the configured cognitive limit or whose
cyclomatic complexity exceeds the configured cyclomatic limit.

The default maximum cognitive complexity is 15, and the default maximum
cyclomatic complexity is 10. Use `-max-cognitive-complexity=<value>` and
`-max-cyclomatic-complexity=<value>` to adjust them.

Use `-git-branch` to analyze complexity only in supported lines changed on the
current local branch. The same origin branch resolution and `-files` exclusion
rules used by the duplicate report apply.

Use `-verbose` to list analyzed files and timing information.

## Duplicate Report

By default, CodeM8 analyzes all registered source file extensions. Recursive
discovery respects Git ignore rules, works outside Git repositories, and skips
common irrelevant directories such as `.git`, `node_modules`, `target`, `dist`,
`build`, `coverage`, `.next`, `.nuxt`, `.svelte-kit`, `.idea`, and `.vscode`.
Symbolic links are not followed.

Every non-empty line is normalized with Rust string trimming, so leading and
trailing Unicode whitespace are removed before hashing and comparison. Empty
trimmed lines are ignored. CodeM8 currently expects UTF-8 source files; invalid
UTF-8 produces a clear error rather than lossy output.

Use `-git-branch` to search duplicate code only in lines changed on the current
local branch. CodeM8 resolves that branch set from `origin/HEAD` with
`origin/main` and `origin/master` fallbacks. The option requires a Git
repository and cannot be combined with `-files`.

Duplicate block weight is calculated as:

```text
(occurrences - 1) * duplicated_line_count * cumulative_normalized_character_count
```

Reports are sorted deterministically by descending weight, then by line count,
character count, first location, and normalized block text.

By default, each duplicate block prints only the duplicate locations. Use
`-verbose` to also show analyzed files, the duplicated code, weight, line count,
occurrence count, and timings for discovery, file processing, and duplicate
detection. Character counts are used internally for scoring and sorting, but are
not printed.

## Development

Run the full local verification set:

```bash
cargo test
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings -W clippy::too_many_lines -W clippy::too_many_arguments -W clippy::type_complexity -W clippy::excessive_nesting -W clippy::cognitive_complexity -W clippy::pedantic -W clippy::nursery -W clippy::cargo
cargo build --locked --all-targets
```

The repository includes GitHub Actions workflows for Rust CI and a CodeRabbit
review gate. CI verifies formatting, build success, and tests on pushes and pull
requests. The CodeRabbit gate runs when CodeRabbit submits or edits a pull
request review and fails if CodeRabbit requests changes on the current PR head.
