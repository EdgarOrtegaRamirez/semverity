//! Semantic Version parsing and comparison (SemVer 2.0.0 compliant).
//!
//! Implements the full SemVer 2.0.0 specification: https://semver.org/
//! Supports pre-release identifiers and build metadata.

use std::cmp::Ordering;
use std::fmt;

/// A semantic version following SemVer 2.0.0.
#[derive(Debug, Clone)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub pre: Vec<String>,
    pub build: Vec<String>,
}

impl PartialEq for Version {
    /// Per SemVer 2.0.0, build metadata is ignored when determining version precedence.
    fn eq(&self, other: &Self) -> bool {
        self.major == other.major
            && self.minor == other.minor
            && self.patch == other.patch
            && self.pre == other.pre
    }
}

impl Eq for Version {}

impl Version {
    /// Create a new simple version (no pre-release or build metadata).
    pub fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
            pre: Vec::new(),
            build: Vec::new(),
        }
    }

    /// Create a version with pre-release identifiers.
    pub fn with_pre(mut self, pre: Vec<String>) -> Self {
        self.pre = pre;
        self
    }

    /// Create a version with build metadata.
    pub fn with_build(mut self, build: Vec<String>) -> Self {
        self.build = build;
        self
    }

    /// Parse a SemVer string into a Version.
    ///
    /// # Errors
    /// Returns an error if the string is not a valid SemVer 2.0.0 version.
    pub fn parse(input: &str) -> Result<Self, ParseError> {
        let input = input.trim();
        if input.is_empty() {
            return Err(ParseError::Empty);
        }

        // Split off build metadata (+)
        let (version_pre, build) = match input.find('+') {
            Some(pos) => (&input[..pos], split_identifiers(&input[pos + 1..])),
            None => (input, Vec::new()),
        };

        // Split off pre-release (-)
        let (version_core, pre) = match version_pre.find('-') {
            Some(pos) => {
                let pre_str = &version_pre[pos + 1..];
                if pre_str.is_empty() {
                    return Err(ParseError::EmptyIdentifier);
                }
                (
                    &version_pre[..pos],
                    split_identifiers(pre_str),
                )
            }
            None => (version_pre, Vec::new()),
        };

        // Parse core version: MAJOR.MINOR.PATCH
        let parts: Vec<&str> = version_core.split('.').collect();
        if parts.len() != 3 {
            return Err(ParseError::InvalidCore(
                "expected MAJOR.MINOR.PATCH".to_string(),
            ));
        }

        // No leading zeros allowed (except "0" itself)
        fn parse_numeric(s: &str, field: &str) -> Result<u64, ParseError> {
            if s.is_empty() {
                return Err(ParseError::InvalidCore(format!("{field} is empty")));
            }
            if s.len() > 1 && s.starts_with('0') {
                return Err(ParseError::LeadingZero(field.to_string()));
            }
            s.parse::<u64>()
                .map_err(|_| ParseError::InvalidCore(format!("{field} not a valid number: {s}")))
        }

        let major = parse_numeric(parts[0], "major")?;
        let minor = parse_numeric(parts[1], "minor")?;
        let patch = parse_numeric(parts[2], "patch")?;

        // Validate pre-release identifiers
        for id in &pre {
            validate_identifier(id, false)?;
        }

        // Validate build identifiers (less strict — alphanumeric + hyphen)
        for id in &build {
            if id.is_empty() {
                return Err(ParseError::EmptyIdentifier);
            }
            if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
                return Err(ParseError::InvalidIdentifier(id.to_string()));
            }
        }

        Ok(Self {
            major,
            minor,
            patch,
            pre,
            build,
        })
    }

    /// Check if this is a pre-release version.
    pub fn is_prerelease(&self) -> bool {
        !self.pre.is_empty()
    }

    /// Check if this is a stable (non-pre-release) version.
    pub fn is_stable(&self) -> bool {
        self.pre.is_empty()
    }

    /// Get the core version string (MAJOR.MINOR.PATCH without pre/build).
    #[allow(dead_code)]
    pub fn core(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }

    /// Bump to the next major version (resets minor, patch, clears pre/build).
    pub fn bump_major(&self) -> Self {
        Self::new(self.major + 1, 0, 0)
    }

    /// Bump to the next minor version (resets patch, clears pre/build).
    pub fn bump_minor(&self) -> Self {
        Self::new(self.major, self.minor + 1, 0)
    }

    /// Bump to the next patch version (clears pre/build).
    pub fn bump_patch(&self) -> Self {
        Self::new(self.major, self.minor, self.patch + 1)
    }

    /// Bump the pre-release version. If no pre-release exists, starts one with the given label.
    /// If a pre-release already exists with a numeric last identifier, increments it.
    /// If a pre-release already exists with a non-numeric last identifier, appends ".0".
    pub fn bump_prerelease(&self, label: &str) -> Self {
        let was_prerelease = self.is_prerelease();
        let mut new_pre = if was_prerelease {
            self.pre.clone()
        } else {
            vec![label.to_string()]
        };

        if was_prerelease {
            // Only append/increment if we already had a pre-release
            if let Some(last) = new_pre.last_mut() {
                if last.chars().all(|c| c.is_ascii_digit()) {
                    if let Ok(n) = last.parse::<u64>() {
                        *last = (n + 1).to_string();
                    }
                } else {
                    new_pre.push("0".to_string());
                }
            }
        }

        Self::new(self.major, self.minor, self.patch).with_pre(new_pre)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if !self.pre.is_empty() {
            write!(f, "-{}", self.pre.join("."))?;
        }
        if !self.build.is_empty() {
            write!(f, "+{}", self.build.join("."))?;
        }
        Ok(())
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    /// Compare two versions per SemVer 2.0.0 precedence rules.
    ///
    /// 1. Major, minor, patch compared numerically.
    /// 2. A version without pre-release > version with pre-release.
    /// 3. Pre-release identifiers compared field by field:
    ///    - Numeric < alphanumeric
    ///    - Numeric compared numerically
    ///    - Alphanumeric compared lexically (ASCII)
    ///    - Fewer fields < more fields (if all preceding are equal)
    /// 4. Build metadata is ignored for precedence.
    fn cmp(&self, other: &Self) -> Ordering {
        // Compare core
        match self.major.cmp(&other.major) {
            Ordering::Equal => {}
            ord => return ord,
        }
        match self.minor.cmp(&other.minor) {
            Ordering::Equal => {}
            ord => return ord,
        }
        match self.patch.cmp(&other.patch) {
            Ordering::Equal => {}
            ord => return ord,
        }

        // Pre-release: no pre > has pre
        match (self.is_prerelease(), other.is_prerelease()) {
            (false, false) => Ordering::Equal,
            (false, true) => Ordering::Greater,
            (true, false) => Ordering::Less,
            (true, true) => compare_pre_release(&self.pre, &other.pre),
        }
    }
}

impl std::str::FromStr for Version {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

/// Compare pre-release identifier lists per SemVer spec.
fn compare_pre_release(a: &[String], b: &[String]) -> Ordering {
    for (i, a_id) in a.iter().enumerate() {
        if i >= b.len() {
            // a has more identifiers → a > b
            return Ordering::Greater;
        }
        let b_id = &b[i];
        match compare_identifier(a_id, b_id) {
            Ordering::Equal => continue,
            ord => return ord,
        }
    }
    // All compared identifiers equal; fewer fields = lower
    a.len().cmp(&b.len())
}

/// Compare two pre-release identifiers.
fn compare_identifier(a: &str, b: &str) -> Ordering {
    let a_num = a.chars().all(|c| c.is_ascii_digit());
    let b_num = b.chars().all(|c| c.is_ascii_digit());

    match (a_num, b_num) {
        (true, true) => {
            // Both numeric → compare numerically
            let a_val: u64 = a.parse().unwrap_or(0);
            let b_val: u64 = b.parse().unwrap_or(0);
            a_val.cmp(&b_val)
        }
        (true, false) => Ordering::Less, // numeric < alphanumeric
        (false, true) => Ordering::Greater,
        (false, false) => a.cmp(b), // both alphanumeric → lexical
    }
}

/// Split dot-separated identifiers.
fn split_identifiers(s: &str) -> Vec<String> {
    if s.is_empty() {
        return Vec::new();
    }
    s.split('.').map(|p| p.to_string()).collect()
}

/// Validate a pre-release or build identifier.
fn validate_identifier(id: &str, _is_build: bool) -> Result<(), ParseError> {
    if id.is_empty() {
        return Err(ParseError::EmptyIdentifier);
    }
    // Pre-release: alphanumeric + hyphen; numeric must not have leading zeros
    if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(ParseError::InvalidIdentifier(id.to_string()));
    }
    // Numeric identifiers must not have leading zeros
    if id.chars().all(|c| c.is_ascii_digit()) && id.len() > 1 && id.starts_with('0') {
        return Err(ParseError::LeadingZero(
            "pre-release identifier".to_string(),
        ));
    }
    Ok(())
}

/// Errors that can occur during version parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    Empty,
    InvalidCore(String),
    LeadingZero(String),
    EmptyIdentifier,
    InvalidIdentifier(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "empty version string"),
            Self::InvalidCore(msg) => write!(f, "invalid version core: {msg}"),
            Self::LeadingZero(field) => write!(f, "{field} has leading zero"),
            Self::EmptyIdentifier => write!(f, "empty identifier in pre-release/build"),
            Self::InvalidIdentifier(id) => write!(f, "invalid identifier: {id}"),
        }
    }
}

impl std::error::Error for ParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let v = Version::parse("1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
        assert!(v.pre.is_empty());
        assert!(v.build.is_empty());
    }

    #[test]
    fn test_parse_prerelease() {
        let v = Version::parse("1.0.0-alpha.1").unwrap();
        assert_eq!(v.pre, vec!["alpha", "1"]);
        assert!(v.is_prerelease());
    }

    #[test]
    fn test_parse_build() {
        let v = Version::parse("1.0.0+build.123").unwrap();
        assert_eq!(v.build, vec!["build", "123"]);
        assert!(v.is_stable());
    }

    #[test]
    fn test_parse_prerelease_and_build() {
        let v = Version::parse("1.0.0-beta.2+exp.sha.5114f85").unwrap();
        assert_eq!(v.pre, vec!["beta", "2"]);
        assert_eq!(v.build, vec!["exp", "sha", "5114f85"]);
    }

    #[test]
    fn test_parse_invalid() {
        assert!(Version::parse("").is_err());
        assert!(Version::parse("1").is_err());
        assert!(Version::parse("1.2").is_err());
        assert!(Version::parse("01.2.3").is_err());
        assert!(Version::parse("1.02.3").is_err());
        assert!(Version::parse("1.2.03").is_err());
        assert!(Version::parse("1.2.3-").is_err());
        assert!(Version::parse("1.2.3-01").is_err()); // leading zero in pre
    }

    #[test]
    fn test_display() {
        assert_eq!(Version::new(1, 2, 3).to_string(), "1.2.3");
        let v = Version::new(1, 0, 0).with_pre(vec!["alpha".into()]);
        assert_eq!(v.to_string(), "1.0.0-alpha");
        let v = Version::new(1, 0, 0)
            .with_pre(vec!["beta".into(), "2".into()])
            .with_build(vec!["sha".into(), "abc".into()]);
        assert_eq!(v.to_string(), "1.0.0-beta.2+sha.abc");
    }

    #[test]
    fn test_compare_basic() {
        assert!(Version::new(2, 0, 0) > Version::new(1, 0, 0));
        assert!(Version::new(1, 1, 0) > Version::new(1, 0, 0));
        assert!(Version::new(1, 0, 1) > Version::new(1, 0, 0));
        assert_eq!(Version::new(1, 0, 0), Version::new(1, 0, 0));
    }

    #[test]
    fn test_compare_prerelease() {
        // Stable > pre-release
        assert!(Version::new(1, 0, 0) > Version::parse("1.0.0-alpha").unwrap());

        // Pre-release ordering
        let alpha = Version::parse("1.0.0-alpha").unwrap();
        let alpha1 = Version::parse("1.0.0-alpha.1").unwrap();
        let beta = Version::parse("1.0.0-beta").unwrap();
        let beta2 = Version::parse("1.0.0-beta.2").unwrap();
        let beta11 = Version::parse("1.0.0-beta.11").unwrap();

        assert!(alpha < alpha1);
        assert!(alpha1 < beta);
        assert!(beta < beta2);
        assert!(beta2 < beta11);
    }

    #[test]
    fn test_compare_numeric_vs_alpha() {
        // Numeric < alphanumeric
        let num = Version::parse("1.0.0-1").unwrap();
        let alpha = Version::parse("1.0.0-alpha").unwrap();
        assert!(num < alpha);
    }

    #[test]
    fn test_build_metadata_ignored() {
        let a = Version::parse("1.0.0+build1").unwrap();
        let b = Version::parse("1.0.0+build2").unwrap();
        assert_eq!(a, b); // Build metadata doesn't affect precedence
    }

    #[test]
    fn test_sort() {
        let mut versions: Vec<Version> = vec![
            Version::parse("1.0.0").unwrap(),
            Version::parse("1.0.0-alpha").unwrap(),
            Version::parse("2.0.0").unwrap(),
            Version::parse("1.0.0-beta").unwrap(),
            Version::parse("1.1.0").unwrap(),
        ];
        versions.sort();
        assert_eq!(versions[0].to_string(), "1.0.0-alpha");
        assert_eq!(versions[1].to_string(), "1.0.0-beta");
        assert_eq!(versions[2].to_string(), "1.0.0");
        assert_eq!(versions[3].to_string(), "1.1.0");
        assert_eq!(versions[4].to_string(), "2.0.0");
    }

    #[test]
    fn test_bump() {
        let v = Version::new(1, 2, 3);
        assert_eq!(v.bump_major().to_string(), "2.0.0");
        assert_eq!(v.bump_minor().to_string(), "1.3.0");
        assert_eq!(v.bump_patch().to_string(), "1.2.4");
    }

    #[test]
    fn test_bump_prerelease() {
        let v = Version::parse("1.0.0-alpha.1").unwrap();
        let bumped = v.bump_prerelease("alpha");
        assert_eq!(bumped.to_string(), "1.0.0-alpha.2");

        let v2 = Version::new(1, 0, 0);
        let bumped2 = v2.bump_prerelease("beta");
        assert_eq!(bumped2.to_string(), "1.0.0-beta");
    }

    #[test]
    fn test_core() {
        let v = Version::parse("1.2.3-alpha+build").unwrap();
        assert_eq!(v.core(), "1.2.3");
    }
}
