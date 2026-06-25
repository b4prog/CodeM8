# CodeM8

CodeM8 is a Rust command-line application for deterministic source code reports.
The initial report detects duplicated line-based code blocks in a repository:

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
cargo install --git https://github.com/b4prog/CodeM8 codem8
```

Build from a local checkout with Cargo:

```bash
cargo build --release
```

Install from a local checkout:

```bash
cargo install --path .
```

Run from the local checkout without installing:

```bash
cargo run -- --report-duplicate
```

## Usage

Analyze TypeScript files from the current directory:

```bash
codem8 --report-duplicate
```

Analyze multiple extensions:

```bash
codem8 --report-duplicate -file-extension=ts,tsx,js,jsx
```

Analyze an explicit list of files instead of recursively discovering files:

```bash
codem8 --report-duplicate -file-extension=ts,js -files=src/a.ts,src/b.js
```

## Duplicate Report

By default, CodeM8 analyzes `.ts` files. Recursive discovery skips common
irrelevant directories such as `.git`, `node_modules`, `target`, `dist`,
`build`, `coverage`, `.next`, `.nuxt`, `.svelte-kit`, `.idea`, and `.vscode`.
Symbolic links are not followed.

Every non-empty line is normalized with Rust string trimming, so leading and
trailing Unicode whitespace are removed before hashing and comparison. Empty
trimmed lines are ignored. CodeM8 currently expects UTF-8 source files; invalid
UTF-8 produces a clear error rather than lossy output.

Duplicate block weight is calculated as:

```text
(occurrences - 1) * duplicated_line_count * cumulative_normalized_character_count
```

Reports are sorted deterministically by descending weight, then by line count,
character count, first location, and normalized block text.

## Language Heuristics

CodeM8 includes a hard-coded registry of block-only line patterns for common
languages and markup formats:

- TypeScript / JavaScript
- Rust
- C / C++ / Objective-C
- C#
- Java / Kotlin / Scala
- Go
- Python
- Ruby
- PHP
- Swift
- Shell
- PowerShell
- HTML / XML
- CSS / SCSS / Sass / Less
- SQL
- YAML / JSON / TOML

Block-only lines, such as braces or closing tags, cannot start a duplicate by
themselves. They can still be included inside a larger duplicated block when
surrounding comparison lines match.

## Development

Run the full local verification set:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings -W clippy::too_many_lines -W clippy::too_many_arguments -W clippy::type_complexity -W clippy::excessive_nesting -W clippy::cognitive_complexity
rtk cargo build --locked --all-targets
cargo test --all-targets
```

The repository includes GitHub Actions workflows for Rust CI and a CodeRabbit
review gate. CI verifies formatting, build success, and tests on pushes and pull
requests. The CodeRabbit gate runs when CodeRabbit submits or edits a pull
request review and fails if CodeRabbit requests changes on the current PR head.

## Dependency Policy

CodeM8 avoids external packages for functionality that is simple to implement
and maintain directly. The first implementation uses one runtime dependency,
`xxhash-rust`, for the required XXH3 128-bit hash implementation. The crate is
widely used and permissively licensed under MIT or Apache-2.0.
