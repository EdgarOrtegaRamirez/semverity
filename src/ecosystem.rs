//! Multi-ecosystem version range syntax parsing.
//!
//! Supports range syntax from multiple ecosystems:
//! - npm/Node.js: ^, ~, >=, >, <=, <, =, ||, *
//! - Cargo/Rust: ^, ~, >=, >, <=, <, =, *, comma-separated (AND)
//! - pip/Python: >=, >, <=, <, ==, !=, ~=, comma-separated
//! - gem/Ruby: ~> (pessimistic), >=, >, <=, <, =, comma-separated

use crate::range::{Interval, VersionRange};
use crate::version::Version;
use std::fmt;

/// The ecosystem whose range syntax to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ecosystem {
    Npm,
    Cargo,
    Pip,
    Gem,
}

impl Ecosystem {
    /// All supported ecosystems.
    #[allow(dead_code)]
    pub fn all() -> Vec<Self> {
        vec![Self::Npm, Self::Cargo, Self::Pip, Self::Gem]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Npm => "npm",
            Self::Cargo => "cargo",
            Self::Pip => "pip",
            Self::Gem => "gem",
        }
    }
}

impl fmt::Display for Ecosystem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for Ecosystem {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "npm" | "node" | "nodejs" => Ok(Self::Npm),
            "cargo" | "rust" => Ok(Self::Cargo),
            "pip" | "python" | "pypi" => Ok(Self::Pip),
            "gem" | "ruby" | "bundler" => Ok(Self::Gem),
            _ => Err(format!("unknown ecosystem: {s}")),
        }
    }
}

/// Parse a version range string in the given ecosystem's syntax.
///
/// # Errors
/// Returns an error if the range string cannot be parsed.
pub fn parse_range(input: &str, ecosystem: Ecosystem) -> Result<VersionRange, RangeParseError> {
    let input = input.trim();
    if input.is_empty() || input == "*" {
        return Ok(VersionRange::all());
    }

    match ecosystem {
        Ecosystem::Npm => parse_npm_range(input),
        Ecosystem::Cargo => parse_cargo_range(input),
        Ecosystem::Pip => parse_pip_range(input),
        Ecosystem::Gem => parse_gem_range(input),
    }
}

/// Parse an npm-style range.
/// Supports: ^1.2.3, ~1.2.3, >=1.2.3, >1.2.3, <=1.2.3, <1.2.3, =1.2.3,
/// 1.2.3, 1.2.x, 1.x, *, || for union, space for intersection.
fn parse_npm_range(input: &str) -> Result<VersionRange, RangeParseError> {
    // Split on || for union
    let parts: Vec<&str> = input.split("||").collect();
    if parts.len() == 1 {
        return parse_npm_conjunction(input);
    }

    let mut range = VersionRange::none();
    for part in parts {
        let sub = parse_npm_conjunction(part.trim())?;
        range = range.union(&sub);
    }
    Ok(range)
}

/// Parse a space-separated conjunction of npm comparators.
fn parse_npm_conjunction(input: &str) -> Result<VersionRange, RangeParseError> {
    let comparators: Vec<&str> = input.split_whitespace().collect();
    if comparators.is_empty() {
        return Ok(VersionRange::all());
    }

    let mut range = VersionRange::all();
    for comp in comparators {
        let sub = parse_npm_comparator(comp)?;
        range = range.intersect(&sub);
    }
    Ok(range)
}

/// Parse a single npm comparator.
#[allow(clippy::needless_question_mark)]
fn parse_npm_comparator(input: &str) -> Result<VersionRange, RangeParseError> {
    let input = input.trim();

    if input == "*" || input.is_empty() {
        return Ok(VersionRange::all());
    }

    // Caret range: ^1.2.3 → >=1.2.3 <2.0.0 (or <1.3.0 if minor is 0, etc.)
    if let Some(rest) = input.strip_prefix('^') {
        return Ok(parse_caret_range(rest)?);
    }

    // Tilde range: ~1.2.3 → >=1.2.3 <1.3.0
    if let Some(rest) = input.strip_prefix('~') {
        return Ok(parse_tilde_range(rest)?);
    }

    // Comparison operators
    if let Some(rest) = input.strip_prefix(">=") {
        let v = parse_partial_version(rest.trim())?;
        return Ok(VersionRange::from_interval(Interval::lower_bound(v, true)));
    }
    if let Some(rest) = input.strip_prefix("<=") {
        let v = parse_partial_version(rest.trim())?;
        return Ok(VersionRange::from_interval(Interval::upper_bound(v, true)));
    }
    if let Some(rest) = input.strip_prefix('>') {
        let v = parse_partial_version(rest.trim())?;
        return Ok(VersionRange::from_interval(Interval::lower_bound(v, false)));
    }
    if let Some(rest) = input.strip_prefix('<') {
        let v = parse_partial_version(rest.trim())?;
        return Ok(VersionRange::from_interval(Interval::upper_bound(v, false)));
    }
    if let Some(rest) = input.strip_prefix('=') {
        let v = parse_partial_version(rest.trim())?;
        return Ok(VersionRange::exact(v));
    }

    // Bare version: exact match (or partial match)
    let v = parse_partial_version(input)?;
    Ok(VersionRange::exact(v))
}

/// Parse a caret (^) range.
/// ^1.2.3 → >=1.2.3 <2.0.0
/// ^0.2.3 → >=0.2.3 <0.3.0
/// ^0.0.3 → >=0.0.3 <0.0.4
fn parse_caret_range(input: &str) -> Result<VersionRange, RangeParseError> {
    let pv = parse_partial_version_detailed(input)?;
    let lower = pv.to_version();
    let upper = pv.caret_upper_bound();
    Ok(VersionRange::from_interval(Interval::bounded(
        lower, true, upper, false,
    )))
}

/// Parse a tilde (~) range.
/// ~1.2.3 → >=1.2.3 <1.3.0
/// ~1.2 → >=1.2.0 <1.3.0
/// ~1 → >=1.0.0 <2.0.0
fn parse_tilde_range(input: &str) -> Result<VersionRange, RangeParseError> {
    let pv = parse_partial_version_detailed(input)?;
    let lower = pv.to_version();
    let upper = pv.tilde_upper_bound();
    Ok(VersionRange::from_interval(Interval::bounded(
        lower, true, upper, false,
    )))
}

/// Parse a Cargo-style range (similar to npm but uses commas for AND).
fn parse_cargo_range(input: &str) -> Result<VersionRange, RangeParseError> {
    // Cargo uses commas for conjunction (AND)
    let comparators: Vec<&str> = input
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    if comparators.is_empty() {
        return Ok(VersionRange::all());
    }

    let mut range = VersionRange::all();
    for comp in comparators {
        let sub = parse_npm_comparator(comp)?; // Cargo syntax is similar to npm
        range = range.intersect(&sub);
    }
    Ok(range)
}

/// Parse a pip/Python-style range.
/// Supports: >=1.2.3, >1.2.3, <=1.2.3, <1.2.3, ==1.2.3, !=1.2.3, ~=1.2.3
/// Comma-separated for AND.
fn parse_pip_range(input: &str) -> Result<VersionRange, RangeParseError> {
    let comparators: Vec<&str> = input
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    if comparators.is_empty() {
        return Ok(VersionRange::all());
    }

    let mut range = VersionRange::all();
    for comp in comparators {
        let sub = parse_pip_comparator(comp)?;
        range = range.intersect(&sub);
    }
    Ok(range)
}

/// Parse a single pip comparator.
fn parse_pip_comparator(input: &str) -> Result<VersionRange, RangeParseError> {
    let input = input.trim();

    // Compatible release: ~=1.2.3 → >=1.2.3 <1.3.0; ~=1.2 → >=1.2 <2.0
    if let Some(rest) = input.strip_prefix("~=") {
        let pv = parse_partial_version_detailed(rest.trim())?;
        let lower = pv.to_version();
        let upper = pv.compat_upper_bound()?;
        return Ok(VersionRange::from_interval(Interval::bounded(
            lower, true, upper, false,
        )));
    }

    if let Some(rest) = input.strip_prefix(">=") {
        let v = parse_partial_version(rest.trim())?;
        return Ok(VersionRange::from_interval(Interval::lower_bound(v, true)));
    }
    if let Some(rest) = input.strip_prefix("<=") {
        let v = parse_partial_version(rest.trim())?;
        return Ok(VersionRange::from_interval(Interval::upper_bound(v, true)));
    }
    if let Some(rest) = input.strip_prefix("!=") {
        let v = parse_partial_version(rest.trim())?;
        let exact = VersionRange::exact(v);
        return Ok(exact.negate());
    }
    if let Some(rest) = input.strip_prefix("==") {
        let v = parse_partial_version(rest.trim())?;
        return Ok(VersionRange::exact(v));
    }
    if let Some(rest) = input.strip_prefix('>') {
        let v = parse_partial_version(rest.trim())?;
        return Ok(VersionRange::from_interval(Interval::lower_bound(v, false)));
    }
    if let Some(rest) = input.strip_prefix('<') {
        let v = parse_partial_version(rest.trim())?;
        return Ok(VersionRange::from_interval(Interval::upper_bound(v, false)));
    }

    // Bare version in pip is treated as exact match
    let v = parse_partial_version(input)?;
    Ok(VersionRange::exact(v))
}

/// Parse a gem/Ruby-style range.
/// Supports: ~>1.2.3 (pessimistic), >=1.2.3, >1.2.3, <=1.2.3, <1.2.3, =1.2.3
/// Comma-separated for AND.
fn parse_gem_range(input: &str) -> Result<VersionRange, RangeParseError> {
    let comparators: Vec<&str> = input
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    if comparators.is_empty() {
        return Ok(VersionRange::all());
    }

    let mut range = VersionRange::all();
    for comp in comparators {
        let sub = parse_gem_comparator(comp)?;
        range = range.intersect(&sub);
    }
    Ok(range)
}

/// Parse a single gem comparator.
fn parse_gem_comparator(input: &str) -> Result<VersionRange, RangeParseError> {
    let input = input.trim();

    // Pessimistic: ~>1.2.3 → >=1.2.3 <1.3.0; ~>1.2 → >=1.2 <2.0
    if let Some(rest) = input.strip_prefix("~>") {
        let pv = parse_partial_version_detailed(rest.trim())?;
        let lower = pv.to_version();
        let upper = pv.pessimistic_upper_bound();
        return Ok(VersionRange::from_interval(Interval::bounded(
            lower, true, upper, false,
        )));
    }

    if let Some(rest) = input.strip_prefix(">=") {
        let v = parse_partial_version(rest.trim())?;
        return Ok(VersionRange::from_interval(Interval::lower_bound(v, true)));
    }
    if let Some(rest) = input.strip_prefix("<=") {
        let v = parse_partial_version(rest.trim())?;
        return Ok(VersionRange::from_interval(Interval::upper_bound(v, true)));
    }
    if let Some(rest) = input.strip_prefix('>') {
        let v = parse_partial_version(rest.trim())?;
        return Ok(VersionRange::from_interval(Interval::lower_bound(v, false)));
    }
    if let Some(rest) = input.strip_prefix('<') {
        let v = parse_partial_version(rest.trim())?;
        return Ok(VersionRange::from_interval(Interval::upper_bound(v, false)));
    }
    if let Some(rest) = input.strip_prefix('=') {
        let v = parse_partial_version(rest.trim())?;
        return Ok(VersionRange::exact(v));
    }

    // Bare version
    let v = parse_partial_version(input)?;
    Ok(VersionRange::exact(v))
}

/// A partially-specified version (e.g., "1.2" means "1.2.x").
#[derive(Debug, Clone)]
struct PartialVersion {
    major: u64,
    minor: Option<u64>,
    patch: Option<u64>,
    pre: Vec<String>,
    build: Vec<String>,
}

impl PartialVersion {
    /// Convert to a full Version, filling missing parts with 0.
    fn to_version(&self) -> Version {
        Version::new(self.major, self.minor.unwrap_or(0), self.patch.unwrap_or(0))
            .with_pre(self.pre.clone())
            .with_build(self.build.clone())
    }

    /// Compute the upper bound for a caret (^) range.
    /// ^1.2.3 → <2.0.0, ^0.2.3 → <0.3.0, ^0.0.3 → <0.0.4
    fn caret_upper_bound(&self) -> Version {
        match (self.minor, self.patch) {
            (Some(0), Some(patch)) if self.major == 0 => {
                // ^0.0.z → <0.0.(z+1)
                Version::new(0, 0, patch + 1)
            }
            (Some(0), None) if self.major == 0 => {
                // ^0.0 → <0.1.0
                Version::new(0, 1, 0)
            }
            (Some(minor), _) if self.major == 0 => {
                // ^0.x.y with x > 0 → <0.(x+1).0
                Version::new(0, minor + 1, 0)
            }
            _ => {
                // ^x.y.z → <(x+1).0.0
                Version::new(self.major + 1, 0, 0)
            }
        }
    }

    /// Compute the upper bound for gem's pessimistic (~>) operator.
    /// ~>1.2.3 → <1.3.0 (three levels specified, bump minor)
    /// ~>1.2 → <2.0.0 (two levels specified, bump major)
    /// ~>1 → <2.0.0 (one level specified, bump major)
    fn pessimistic_upper_bound(&self) -> Version {
        match (self.minor, self.patch) {
            (Some(_minor), Some(_patch)) => {
                // ~>1.2.3 → <1.3.0
                Version::new(self.major, _minor + 1, 0)
            }
            _ => {
                // ~>1.2 or ~>1 → <(major+1).0.0
                Version::new(self.major + 1, 0, 0)
            }
        }
    }

    /// Compute the upper bound for a tilde (~) range.
    /// ~1.2.3 → <1.3.0, ~1.2 → <1.3.0, ~1 → <2.0.0
    fn tilde_upper_bound(&self) -> Version {
        match self.minor {
            Some(minor) => Version::new(self.major, minor + 1, 0),
            None => Version::new(self.major + 1, 0, 0),
        }
    }

    /// Compute the upper bound for a pip ~= compatible release.
    /// ~=1.2.3 → <1.3.0, ~=1.2 → <2.0.0
    fn compat_upper_bound(&self) -> Result<Version, RangeParseError> {
        match (self.minor, self.patch) {
            (Some(minor), Some(_)) => Ok(Version::new(self.major, minor + 1, 0)),
            (Some(_), None) => Ok(Version::new(self.major + 1, 0, 0)),
            (None, _) => Err(RangeParseError::InvalidFormat(
                "~= requires at least major.minor".to_string(),
            )),
        }
    }
}

/// Parse a partial version string (e.g., "1", "1.2", "1.2.3", "1.2.x", "1.x").
fn parse_partial_version(input: &str) -> Result<Version, RangeParseError> {
    let pv = parse_partial_version_detailed(input)?;
    Ok(pv.to_version())
}

/// Parse a partial version with detailed information.
fn parse_partial_version_detailed(input: &str) -> Result<PartialVersion, RangeParseError> {
    let input = input.trim();
    if input.is_empty() || input == "*" || input == "x" || input == "X" {
        // Wildcard: treat as 0.0.0
        return Ok(PartialVersion {
            major: 0,
            minor: None,
            patch: None,
            pre: Vec::new(),
            build: Vec::new(),
        });
    }

    // Split off build metadata
    let (version_pre, build) = match input.find('+') {
        Some(pos) => (&input[..pos], split_ids(&input[pos + 1..])),
        None => (input, Vec::new()),
    };

    // Split off pre-release
    let (version_core, pre) = match version_pre.find('-') {
        Some(pos) => (&version_pre[..pos], split_ids(&version_pre[pos + 1..])),
        None => (version_pre, Vec::new()),
    };

    let parts: Vec<&str> = version_core.split('.').collect();
    if parts.is_empty() {
        return Err(RangeParseError::InvalidFormat("empty version".to_string()));
    }

    fn parse_part(s: &str) -> Result<Option<u64>, RangeParseError> {
        let s = s.trim();
        if s.is_empty() || s == "*" || s == "x" || s == "X" {
            return Ok(None);
        }
        if s.len() > 1 && s.starts_with('0') {
            return Err(RangeParseError::LeadingZero);
        }
        s.parse::<u64>()
            .map(Some)
            .map_err(|_| RangeParseError::InvalidFormat(format!("not a number: {s}")))
    }

    let major = parse_part(parts[0])?
        .ok_or_else(|| RangeParseError::InvalidFormat("major is required".to_string()))?;
    let minor = if parts.len() > 1 {
        parse_part(parts[1])?
    } else {
        None
    };
    let patch = if parts.len() > 2 {
        parse_part(parts[2])?
    } else {
        None
    };

    Ok(PartialVersion {
        major,
        minor,
        patch,
        pre,
        build,
    })
}

fn split_ids(s: &str) -> Vec<String> {
    if s.is_empty() {
        Vec::new()
    } else {
        s.split('.').map(|p| p.to_string()).collect()
    }
}

/// Errors during range parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RangeParseError {
    InvalidFormat(String),
    LeadingZero,
    #[allow(dead_code)]
    InvalidVersion(String),
}

impl fmt::Display for RangeParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFormat(msg) => write!(f, "invalid range format: {msg}"),
            Self::LeadingZero => write!(f, "leading zero in version number"),
            Self::InvalidVersion(msg) => write!(f, "invalid version: {msg}"),
        }
    }
}

impl std::error::Error for RangeParseError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    // NPM tests

    #[test]
    fn test_npm_caret() {
        let range = parse_range("^1.2.3", Ecosystem::Npm).unwrap();
        assert!(range.contains(&v("1.2.3")));
        assert!(range.contains(&v("1.9.0")));
        assert!(!range.contains(&v("2.0.0")));
        assert!(!range.contains(&v("1.2.2")));
    }

    #[test]
    fn test_npm_caret_zero_major() {
        let range = parse_range("^0.2.3", Ecosystem::Npm).unwrap();
        assert!(range.contains(&v("0.2.3")));
        assert!(range.contains(&v("0.2.9")));
        assert!(!range.contains(&v("0.3.0")));

        let range = parse_range("^0.0.3", Ecosystem::Npm).unwrap();
        assert!(range.contains(&v("0.0.3")));
        assert!(!range.contains(&v("0.0.4")));
    }

    #[test]
    fn test_npm_tilde() {
        let range = parse_range("~1.2.3", Ecosystem::Npm).unwrap();
        assert!(range.contains(&v("1.2.3")));
        assert!(range.contains(&v("1.2.9")));
        assert!(!range.contains(&v("1.3.0")));

        let range = parse_range("~1.2", Ecosystem::Npm).unwrap();
        assert!(range.contains(&v("1.2.0")));
        assert!(range.contains(&v("1.2.9")));
        assert!(!range.contains(&v("1.3.0")));

        let range = parse_range("~1", Ecosystem::Npm).unwrap();
        assert!(range.contains(&v("1.0.0")));
        assert!(range.contains(&v("1.9.9")));
        assert!(!range.contains(&v("2.0.0")));
    }

    #[test]
    fn test_npm_comparators() {
        let range = parse_range(">=1.2.3", Ecosystem::Npm).unwrap();
        assert!(range.contains(&v("1.2.3")));
        assert!(range.contains(&v("2.0.0")));
        assert!(!range.contains(&v("1.2.2")));

        let range = parse_range(">1.2.3", Ecosystem::Npm).unwrap();
        assert!(!range.contains(&v("1.2.3")));
        assert!(range.contains(&v("1.2.4")));

        let range = parse_range("<=2.0.0", Ecosystem::Npm).unwrap();
        assert!(range.contains(&v("2.0.0")));
        assert!(range.contains(&v("1.0.0")));
        assert!(!range.contains(&v("2.0.1")));

        let range = parse_range("<2.0.0", Ecosystem::Npm).unwrap();
        assert!(!range.contains(&v("2.0.0")));
        assert!(range.contains(&v("1.9.9")));

        let range = parse_range("=1.2.3", Ecosystem::Npm).unwrap();
        assert!(range.contains(&v("1.2.3")));
        assert!(!range.contains(&v("1.2.4")));
    }

    #[test]
    fn test_npm_union() {
        let range = parse_range(">=1.0.0 <2.0.0 || >=3.0.0", Ecosystem::Npm).unwrap();
        assert!(range.contains(&v("1.5.0")));
        assert!(!range.contains(&v("2.5.0")));
        assert!(range.contains(&v("3.0.0")));
        assert!(range.contains(&v("4.0.0")));
    }

    #[test]
    fn test_npm_intersection() {
        let range = parse_range(">=1.0.0 <2.0.0", Ecosystem::Npm).unwrap();
        assert!(range.contains(&v("1.5.0")));
        assert!(!range.contains(&v("0.5.0")));
        assert!(!range.contains(&v("2.0.0")));
    }

    #[test]
    fn test_npm_wildcard() {
        let range = parse_range("*", Ecosystem::Npm).unwrap();
        assert!(range.contains(&v("0.0.1")));
        assert!(range.contains(&v("99.99.99")));
    }

    // Cargo tests

    #[test]
    fn test_caret_cargo() {
        let range = parse_range("^1.2.3", Ecosystem::Cargo).unwrap();
        assert!(range.contains(&v("1.2.3")));
        assert!(range.contains(&v("1.9.0")));
        assert!(!range.contains(&v("2.0.0")));
    }

    #[test]
    fn test_cargo_comma() {
        let range = parse_range(">=1.0.0, <2.0.0", Ecosystem::Cargo).unwrap();
        assert!(range.contains(&v("1.5.0")));
        assert!(!range.contains(&v("2.0.0")));
    }

    // Pip tests

    #[test]
    fn test_pip_compat() {
        let range = parse_range("~=1.2.3", Ecosystem::Pip).unwrap();
        assert!(range.contains(&v("1.2.3")));
        assert!(range.contains(&v("1.2.9")));
        assert!(!range.contains(&v("1.3.0")));

        let range = parse_range("~=1.2", Ecosystem::Pip).unwrap();
        assert!(range.contains(&v("1.2.0")));
        assert!(range.contains(&v("1.9.0")));
        assert!(!range.contains(&v("2.0.0")));
    }

    #[test]
    fn test_pip_neq() {
        let range = parse_range("!=1.2.3", Ecosystem::Pip).unwrap();
        assert!(!range.contains(&v("1.2.3")));
        assert!(range.contains(&v("1.2.4")));
        assert!(range.contains(&v("1.2.2")));
    }

    #[test]
    fn test_pip_comma() {
        let range = parse_range(">=1.0.0, <2.0.0", Ecosystem::Pip).unwrap();
        assert!(range.contains(&v("1.5.0")));
        assert!(!range.contains(&v("2.0.0")));
    }

    // Gem tests

    #[test]
    fn test_gem_pessimistic() {
        let range = parse_range("~>1.2.3", Ecosystem::Gem).unwrap();
        assert!(range.contains(&v("1.2.3")));
        assert!(range.contains(&v("1.2.9")));
        assert!(!range.contains(&v("1.3.0")));

        let range = parse_range("~>1.2", Ecosystem::Gem).unwrap();
        assert!(range.contains(&v("1.2.0")));
        assert!(range.contains(&v("1.9.0")));
        assert!(!range.contains(&v("2.0.0")));
    }

    #[test]
    fn test_gem_comma() {
        let range = parse_range(">=1.0.0, <2.0.0", Ecosystem::Gem).unwrap();
        assert!(range.contains(&v("1.5.0")));
        assert!(!range.contains(&v("2.0.0")));
    }

    #[test]
    fn test_ecosystem_parse() {
        assert_eq!(Ecosystem::from_str("npm").unwrap(), Ecosystem::Npm);
        assert_eq!(Ecosystem::from_str("cargo").unwrap(), Ecosystem::Cargo);
        assert_eq!(Ecosystem::from_str("pip").unwrap(), Ecosystem::Pip);
        assert_eq!(Ecosystem::from_str("gem").unwrap(), Ecosystem::Gem);
        assert!(Ecosystem::from_str("unknown").is_err());
    }
}
