//! Version diffing — compare two versions and describe the changes.

use crate::version::Version;

/// The type of version change between two versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeType {
    Major,
    Minor,
    Patch,
    Prerelease,
    Build,
    Downgrade,
    Equal,
}

impl ChangeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Major => "major",
            Self::Minor => "minor",
            Self::Patch => "patch",
            Self::Prerelease => "prerelease",
            Self::Build => "build",
            Self::Downgrade => "downgrade",
            Self::Equal => "equal",
        }
    }
}

/// A diff between two versions.
#[derive(Debug, Clone)]
pub struct VersionDiff {
    pub from: Version,
    pub to: Version,
    pub change_type: ChangeType,
    pub major_delta: i64,
    pub minor_delta: i64,
    pub patch_delta: i64,
    #[allow(dead_code)]
    pub prerelease_changed: bool,
    #[allow(dead_code)]
    pub build_changed: bool,
}

/// Compute the diff between two versions.
pub fn diff(from: &Version, to: &Version) -> VersionDiff {
    let major_delta = to.major as i64 - from.major as i64;
    let minor_delta = to.minor as i64 - from.minor as i64;
    let patch_delta = to.patch as i64 - from.patch as i64;
    let prerelease_changed = from.pre != to.pre;
    let build_changed = from.build != to.build;

    let change_type = if from.major == to.major
        && from.minor == to.minor
        && from.patch == to.patch
        && from.pre == to.pre
        && from.build == to.build
    {
        ChangeType::Equal
    } else if to < from {
        ChangeType::Downgrade
    } else if major_delta > 0 {
        ChangeType::Major
    } else if minor_delta > 0 {
        ChangeType::Minor
    } else if patch_delta > 0 {
        ChangeType::Patch
    } else if prerelease_changed {
        ChangeType::Prerelease
    } else {
        ChangeType::Build
    };

    VersionDiff {
        from: from.clone(),
        to: to.clone(),
        change_type,
        major_delta,
        minor_delta,
        patch_delta,
        prerelease_changed,
        build_changed,
    }
}

impl std::fmt::Display for VersionDiff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.change_type == ChangeType::Equal {
            return write!(f, "no change ({} == {})", self.from, self.to);
        }

        let direction = if self.change_type == ChangeType::Downgrade {
            "downgrade"
        } else {
            "upgrade"
        };

        write!(
            f,
            "{direction} {} → {} ({})",
            self.from,
            self.to,
            self.change_type.as_str()
        )?;

        let mut deltas = Vec::new();
        if self.major_delta != 0 {
            deltas.push(format!("major {:+}", self.major_delta));
        }
        if self.minor_delta != 0 {
            deltas.push(format!("minor {:+}", self.minor_delta));
        }
        if self.patch_delta != 0 {
            deltas.push(format!("patch {:+}", self.patch_delta));
        }
        if !deltas.is_empty() {
            write!(f, " [{}]", deltas.join(", "))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    #[test]
    fn test_diff_major() {
        let d = diff(&v("1.0.0"), &v("2.0.0"));
        assert_eq!(d.change_type, ChangeType::Major);
        assert_eq!(d.major_delta, 1);
    }

    #[test]
    fn test_diff_minor() {
        let d = diff(&v("1.0.0"), &v("1.1.0"));
        assert_eq!(d.change_type, ChangeType::Minor);
        assert_eq!(d.minor_delta, 1);
    }

    #[test]
    fn test_diff_patch() {
        let d = diff(&v("1.0.0"), &v("1.0.1"));
        assert_eq!(d.change_type, ChangeType::Patch);
        assert_eq!(d.patch_delta, 1);
    }

    #[test]
    fn test_diff_prerelease() {
        let d = diff(&v("1.0.0-alpha"), &v("1.0.0-beta"));
        assert_eq!(d.change_type, ChangeType::Prerelease);
        assert!(d.prerelease_changed);
    }

    #[test]
    fn test_diff_build() {
        let d = diff(&v("1.0.0+build1"), &v("1.0.0+build2"));
        assert_eq!(d.change_type, ChangeType::Build);
        assert!(d.build_changed);
    }

    #[test]
    fn test_diff_downgrade() {
        let d = diff(&v("2.0.0"), &v("1.0.0"));
        assert_eq!(d.change_type, ChangeType::Downgrade);
        assert_eq!(d.major_delta, -1);
    }

    #[test]
    fn test_diff_equal() {
        let d = diff(&v("1.0.0"), &v("1.0.0"));
        assert_eq!(d.change_type, ChangeType::Equal);
    }

    #[test]
    fn test_diff_display() {
        let d = diff(&v("1.0.0"), &v("2.3.4"));
        let s = d.to_string();
        assert!(s.contains("upgrade"));
        assert!(s.contains("major"));
    }
}
