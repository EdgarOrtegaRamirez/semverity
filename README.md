# SemVerity — Semantic Version Intelligence Toolkit

[![CI](https://github.com/EdgarOrtegaRamirez/semverity/actions/workflows/ci.yml/badge.svg)](https://github.com/EdgarOrtegaRamirez/semverity/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.74%2B-blue.svg)](https://www.rust-lang.org)

Parse, compare, resolve ranges, check constraints, detect conflicts, bump versions, and analyze package files across ecosystems.

SemVerity is a comprehensive CLI tool for working with [Semantic Versioning 2.0.0](https://semver.org/) — from basic parsing and comparison to multi-ecosystem range resolution and conventional commit-based version bumping.

## Features

- **Parse** — Validate and display semantic versions with full pre-release and build metadata support
- **Compare** — Compare two versions per SemVer 2.0.0 precedence rules
- **Sort** — Sort versions in ascending or descending order
- **Range Resolution** — `^`, `~`, `>=`, `>`, `<=`, `<`, `=`, `||`, `*` operators
- **Multi-Ecosystem** — NPM, Cargo/Rust, pip/Python, and Gem/Ruby range syntax
- **Intersect** — Find the common range between multiple constraints; detect conflicts
- **Check** — Test if a version satisfies a range constraint
- **Bump** — Bump to major, minor, patch, or pre-release versions
- **Bump from Commits** — Automatic version bumping based on conventional commit messages
- **Diff** — Show the delta between two versions with change type detection
- **Analyze** — Parse package files (`package.json`, `Cargo.toml`, `pyproject.toml`, `go.mod`) for dependency constraints and conflicts
- **Latest/Lowest** — Find the highest or lowest version in a list

## Installation

### From source

```bash
git clone https://github.com/EdgarOrtegaRamirez/semverity.git
cd semverity
cargo build --release
```

The binary will be at `./target/release/semverity`. Add it to your `PATH` or use `cargo install --path .`.

### Via Cargo

```bash
cargo install --git https://github.com/EdgarOrtegaRamirez/semverity.git
```

## Quick Start

```bash
# Parse a version
semverity parse 1.2.3-alpha.1+build.123

# Compare versions
semverity compare 1.0.0 2.0.0

# Sort versions
semverity sort 2.0.0 1.0.0-alpha 1.5.0

# Check if a version satisfies a range
semverity check 1.5.0 "^1.0.0"

# Check with different ecosystem syntax
semverity check 1.5.0 ">=1.0.0, <2.0.0" --ecosystem cargo

# Resolve a range
semverity resolve "^1.2.3"

# Intersect multiple ranges
semverity intersect ">=1.0.0" "<3.0.0"

# Bump a version
semverity bump 1.2.3 major
semverity bump 1.0.0 prerelease --label beta

# Bump from conventional commits
semverity bump-from-commits 1.0.0 "fix: bug fix" "feat: new feature"

# Diff two versions
semverity diff 1.0.0 2.0.0

# Analyze a package file
semverity analyze path/to/package.json
semverity analyze path/to/Cargo.toml --format json

# Find latest/lowest
semverity latest 1.0.0 2.0.0 1.5.0
semverity lowest 3.0.0 1.0.0 2.0.0
```

## Commands

| Command | Description |
|---------|-------------|
| `parse` | Parse and display a semantic version |
| `compare` | Compare two versions |
| `sort` | Sort a list of versions |
| `check` | Check if a version satisfies a range |
| `resolve` | Resolve and display a version range |
| `intersect` | Intersect multiple ranges |
| `bump` | Bump a version (major, minor, patch, prerelease) |
| `bump-from-commits` | Bump based on conventional commit messages |
| `diff` | Diff two versions |
| `analyze` | Analyze a package file for version constraints |
| `latest` | Find the latest version from a list |
| `lowest` | Find the lowest version from a list |

## Ecosystem Support

| Ecosystem | Range Syntax | Examples |
|-----------|-------------|---------|
| **NPM** (`npm`) | `^`, `~`, `>=`, `>`, `<=`, `<`, `=`, `\|\|`, `*`, space (AND) | `^1.2.3`, `>=1.0.0 <2.0.0 \|\| >=3.0.0` |
| **Cargo** (`cargo`) | Same as NPM, comma-separated AND | `^1.2.3`, `>=1.0.0, <2.0.0` |
| **pip** (`pip`) | `>=`, `>`, `<=`, `<`, `==`, `!=`, `~=`, comma-separated AND | `~=1.2.3`, `>=1.0.0, <2.0.0`, `!=1.2.3` |
| **Gem** (`gem`) | `~>` (pessimistic), `>=`, `>`, `<=`, `<`, `=`, comma-separated AND | `~>1.2.3`, `>=1.0.0, <2.0.0` |

## Conventional Commit Bumping

The `bump-from-commits` command follows the [Conventional Commits](https://www.conventionalcommits.org/) specification:

| Commit Type | Bump |
|-------------|------|
| `feat:` or `feat(scope):` | Minor |
| `fix:` | Patch |
| `BREAKING CHANGE:` (footer) | Major |
| `feat!:` or `fix!:`, etc. (with `!`) | Major |
| `docs:`, `chore:`, `refactor:`, `test:`, etc. | Patch |

## Security

See [SECURITY.md](SECURITY.md) for security policy.

## License

This project is licensed under the MIT License — see the [LICENSE](LICENSE) file for details.

## Related Projects

- [semver](https://crates.io/crates/semver) — The standard Rust semver library
- [node-semver](https://github.com/npm/node-semver) — The npm semver implementation