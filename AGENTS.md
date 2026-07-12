# SemVerity — AI Agent Guide

## Overview

SemVerity is a Rust CLI tool for semantic versioning operations across multiple package ecosystems. It parses, compares, resolves ranges, checks constraints, bumps versions, and analyzes package files.

## Building

```bash
cargo build --release
```

## Testing

```bash
cargo test
```

## Running

```bash
cargo run -- parse 1.2.3
cargo run -- compare 1.0.0 2.0.0
```

## Architecture

- `src/main.rs` — CLI entry point with clap commands
- `src/version.rs` — SemVer 2.0.0 parsing, comparison, and bumping
- `src/range.rs` — Version range intervals, intersection, union, containment
- `src/ecosystem.rs` — Multi-ecosystem range syntax parsers
- `src/differ.rs` — Version diffing and change type detection
- `src/bumper.rs` — Version bumping with conventional commit support
- `src/analyzer.rs` — Package file analysis (package.json, Cargo.toml, etc.)
- `tests/cli.rs` — Integration tests using `assert_cmd`

## Key Design Decisions

- Full SemVer 2.0.0 compliance including build metadata precedence rules
- Build metadata is ignored for version precedence (per spec) but tracked for diffing
- `PartialEq` on `Version` ignores build metadata; diff uses full field comparison
- Ecosystem parsers share a common `PartialVersion` type with ecosystem-specific upper bound logic
- Range resolution uses interval-based math (bounded, unbounded, exact, negated)

## Adding a New Ecosystem

1. Add a variant to the `Ecosystem` enum
2. Implement a parser function (e.g., `parse_python_range`)
3. Add ecosystem identifier aliases in `FromStr`
4. Handle in the top-level `parse_range` match
5. Add unit tests

## Security

Do not accept user-provided package files without validation. The `analyze` command reads files from disk — ensure proper path sanitization in agent-controlled workflows.

## Common Tasks for AI Agents

- **Update dependencies**: Update versions in `Cargo.toml` and run `cargo update`
- **Fix test regressions**: Run `cargo test`, inspect failures, adjust source or tests
- **Add CI**: The CI workflow runs `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo build`, and `cargo test`
- **Publish**: `cargo publish` (requires crate.io token)