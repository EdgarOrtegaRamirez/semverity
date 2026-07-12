//! Version bumping with conventional commit support.

use crate::version::Version;

/// The type of version bump to apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BumpType {
    Major,
    Minor,
    Patch,
    Prerelease,
}

impl BumpType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Major => "major",
            Self::Minor => "minor",
            Self::Patch => "patch",
            Self::Prerelease => "prerelease",
        }
    }
}

/// Bump a version according to the bump type.
pub fn bump(version: &Version, bump_type: BumpType, prerelease_label: &str) -> Version {
    match bump_type {
        BumpType::Major => version.bump_major(),
        BumpType::Minor => version.bump_minor(),
        BumpType::Patch => version.bump_patch(),
        BumpType::Prerelease => version.bump_prerelease(prerelease_label),
    }
}

/// Determine the appropriate bump type from a conventional commit message.
///
/// Conventional commit format: `type(scope): description`
/// - `feat:` or `feat(scope):` → minor bump
/// - `fix:` → patch bump
/// - `BREAKING CHANGE:` in footer or `!:` in header → major bump
/// - Other types (docs, chore, refactor, etc.) → patch bump (or no bump)
pub fn bump_from_commit(message: &str) -> BumpType {
    let message = message.trim();

    // Check for breaking change indicator
    if message.contains("BREAKING CHANGE:") || message.contains("BREAKING-CHANGE:") {
        return BumpType::Major;
    }

    // Check for !: in the header (breaking change shorthand)
    let first_line = message.lines().next().unwrap_or("");
    if let Some(colon_pos) = first_line.find(':') {
        let type_scope = &first_line[..colon_pos];
        if type_scope.ends_with('!') {
            return BumpType::Major;
        }

        // Extract the type (before any scope in parentheses)
        let commit_type = type_scope.split('(').next().unwrap_or(type_scope).trim();

        match commit_type {
            "feat" => BumpType::Minor,
            "fix" => BumpType::Patch,
            "perf" => BumpType::Patch,
            "refactor" => BumpType::Patch,
            "docs" => BumpType::Patch,
            "style" => BumpType::Patch,
            "test" => BumpType::Patch,
            "chore" => BumpType::Patch,
            "build" => BumpType::Patch,
            "ci" => BumpType::Patch,
            _ => BumpType::Patch,
        }
    } else {
        BumpType::Patch
    }
}

/// Determine the bump type from a list of commit messages.
/// Returns the highest bump needed.
pub fn bump_from_commits(messages: &[&str]) -> BumpType {
    let mut result = BumpType::Patch;
    for msg in messages {
        let bump = bump_from_commit(msg);
        match (result, bump) {
            (BumpType::Major, _) | (_, BumpType::Major) => return BumpType::Major,
            (BumpType::Minor, _) | (_, BumpType::Minor) => result = BumpType::Minor,
            _ => result = BumpType::Patch,
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    #[test]
    fn test_bump_major() {
        assert_eq!(bump(&v("1.2.3"), BumpType::Major, "").to_string(), "2.0.0");
    }

    #[test]
    fn test_bump_minor() {
        assert_eq!(bump(&v("1.2.3"), BumpType::Minor, "").to_string(), "1.3.0");
    }

    #[test]
    fn test_bump_patch() {
        assert_eq!(bump(&v("1.2.3"), BumpType::Patch, "").to_string(), "1.2.4");
    }

    #[test]
    fn test_bump_prerelease() {
        assert_eq!(
            bump(&v("1.0.0"), BumpType::Prerelease, "alpha").to_string(),
            "1.0.0-alpha"
        );
        assert_eq!(
            bump(&v("1.0.0-alpha"), BumpType::Prerelease, "alpha").to_string(),
            "1.0.0-alpha.0"
        );
    }

    #[test]
    fn test_bump_from_commit_feat() {
        assert_eq!(bump_from_commit("feat: add new feature"), BumpType::Minor);
        assert_eq!(
            bump_from_commit("feat(api): add new endpoint"),
            BumpType::Minor
        );
    }

    #[test]
    fn test_bump_from_commit_fix() {
        assert_eq!(bump_from_commit("fix: resolve bug"), BumpType::Patch);
        assert_eq!(
            bump_from_commit("fix(auth): handle edge case"),
            BumpType::Patch
        );
    }

    #[test]
    fn test_bump_from_commit_breaking() {
        assert_eq!(
            bump_from_commit("feat: new API\n\nBREAKING CHANGE: removed old API"),
            BumpType::Major
        );
        assert_eq!(bump_from_commit("feat!: redesign API"), BumpType::Major);
        assert_eq!(
            bump_from_commit("feat(api)!: change response format"),
            BumpType::Major
        );
    }

    #[test]
    fn test_bump_from_commit_other() {
        assert_eq!(bump_from_commit("docs: update README"), BumpType::Patch);
        assert_eq!(bump_from_commit("chore: update deps"), BumpType::Patch);
        assert_eq!(bump_from_commit("refactor: clean up code"), BumpType::Patch);
    }

    #[test]
    fn test_bump_from_commits_multiple() {
        let commits = vec!["fix: bug fix", "feat: new feature", "docs: update"];
        assert_eq!(bump_from_commits(&commits), BumpType::Minor);

        let commits = vec!["fix: bug fix", "feat!: breaking change"];
        assert_eq!(bump_from_commits(&commits), BumpType::Major);
    }
}
