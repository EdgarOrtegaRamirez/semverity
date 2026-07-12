# Security Policy

## Supported Versions

| Version | Supported          |
|---------|--------------------|
| 1.0.x   | ✅ Active          |

## Reporting a Vulnerability

If you discover a security vulnerability in SemVerity, please report it privately by opening a security advisory on GitHub:

https://github.com/EdgarOrtegaRamirez/semverity/security/advisories/new

Please do **not** disclose vulnerabilities publicly until they have been addressed.

## Security Considerations

### Input Validation
- All version strings are validated against the SemVer 2.0.0 specification before processing
- Package file paths are processed only when explicitly provided by the user
- Range strings are parsed with strict validation — malformed inputs return clear errors

### Path Traversal
The `analyze` command reads files from the filesystem. Users should only analyze trusted package files. The tool does not follow symlinks or resolve paths beyond what the OS provides.

### No Network Access
SemVerity is a fully offline CLI tool. It does not make network requests, phone home, or collect telemetry.

### Dependencies
- `clap` — CLI argument parsing (well-audited)
- `serde` / `serde_json` — JSON serialization (industry standard)
- All dependencies are pinned to specific versions in `Cargo.lock`

## Build Reproducibility

`Cargo.lock` is committed to ensure reproducible builds. Verify checksums against the release page before using in sensitive environments.