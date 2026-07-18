use assert_cmd::Command;
use predicates::ord::eq;
use predicates::str::contains;

fn semverity() -> Command {
    Command::cargo_bin("semverity").unwrap()
}

#[test]
fn test_parse_valid_version() {
    let mut cmd = semverity();
    let assert = cmd.arg("parse").arg("1.2.3").assert();
    assert
        .success()
        .stdout(contains("Version: 1.2.3"))
        .stdout(contains("Major:   1"))
        .stdout(contains("Minor:   2"))
        .stdout(contains("Patch:   3"))
        .stdout(contains("Stable:  yes"));
}

#[test]
fn test_parse_prerelease() {
    let mut cmd = semverity();
    let assert = cmd.arg("parse").arg("2.0.0-beta.1+sha.abc").assert();
    assert
        .success()
        .stdout(contains("Version: 2.0.0-beta.1+sha.abc"))
        .stdout(contains("Pre-release: beta.1"))
        .stdout(contains("Build:   sha.abc"))
        .stdout(contains("Stable:  no"));
}

#[test]
fn test_parse_invalid() {
    let mut cmd = semverity();
    let assert = cmd.arg("parse").arg("not-a-version").assert();
    assert.failure().stderr(contains("Error"));
}

#[test]
fn test_compare_less() {
    let mut cmd = semverity();
    let assert = cmd.arg("compare").arg("1.0.0").arg("2.0.0").assert();
    assert.success().stdout(contains("1.0.0 < 2.0.0"));
}

#[test]
fn test_compare_greater() {
    let mut cmd = semverity();
    let assert = cmd.arg("compare").arg("3.0.0").arg("1.0.0").assert();
    assert.success().stdout(contains("3.0.0 > 1.0.0"));
}

#[test]
fn test_compare_equal() {
    let mut cmd = semverity();
    let assert = cmd.arg("compare").arg("1.0.0").arg("1.0.0").assert();
    assert.success().stdout(contains("1.0.0 == 1.0.0"));
}

#[test]
fn test_compare_prerelease_vs_stable() {
    let mut cmd = semverity();
    let assert = cmd.arg("compare").arg("1.0.0-alpha").arg("1.0.0").assert();
    assert.success().stdout(contains("1.0.0-alpha < 1.0.0"));
}

#[test]
fn test_sort_ascending() {
    let mut cmd = semverity();
    let assert = cmd
        .arg("sort")
        .arg("2.0.0")
        .arg("1.0.0")
        .arg("1.0.0-alpha")
        .arg("1.1.0")
        .assert();
    let output = String::from_utf8(assert.success().get_output().stdout.clone()).unwrap();
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines[0], "1.0.0-alpha");
    assert_eq!(lines[1], "1.0.0");
    assert_eq!(lines[2], "1.1.0");
    assert_eq!(lines[3], "2.0.0");
}

#[test]
fn test_sort_descending() {
    let mut cmd = semverity();
    let assert = cmd
        .arg("sort")
        .arg("--desc")
        .arg("1.0.0")
        .arg("2.0.0")
        .assert();
    let output = String::from_utf8(assert.success().get_output().stdout.clone()).unwrap();
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines[0], "2.0.0");
    assert_eq!(lines[1], "1.0.0");
}

#[test]
fn test_check_satisfies() {
    let mut cmd = semverity();
    let assert = cmd.arg("check").arg("1.5.0").arg("^1.0.0").assert();
    assert
        .success()
        .stdout(contains("✓ 1.5.0 satisfies ^1.0.0"));
}

#[test]
fn test_check_does_not_satisfy() {
    let mut cmd = semverity();
    let assert = cmd.arg("check").arg("2.0.0").arg("^1.0.0").assert();
    assert.failure().stdout(contains("does NOT satisfy"));
}

#[test]
fn test_check_with_ecosystem() {
    let mut cmd = semverity();
    let assert = cmd
        .arg("check")
        .arg("1.5.0")
        .arg(">=1.0.0, <2.0.0")
        .arg("--ecosystem")
        .arg("cargo")
        .assert();
    assert.success().stdout(contains("✓"));
}

#[test]
fn test_resolve_npm_range() {
    let mut cmd = semverity();
    let assert = cmd.arg("resolve").arg("^1.2.3").assert();
    assert.success().stdout(contains("Status: valid"));
}

#[test]
fn test_resolve_empty_range() {
    let mut cmd = semverity();
    let assert = cmd.arg("resolve").arg(">=1.0.0 <1.0.0").assert();
    assert.success().stdout(contains("Status: empty"));
}

#[test]
fn test_intersect_compatible() {
    let mut cmd = semverity();
    let assert = cmd.arg("intersect").arg(">=1.0.0").arg("<3.0.0").assert();
    assert.success().stdout(contains("Status: valid"));
}

#[test]
fn test_intersect_incompatible() {
    let mut cmd = semverity();
    let assert = cmd.arg("intersect").arg("<1.0.0").arg(">=2.0.0").assert();
    assert.success().stdout(contains("Status: empty"));
}

#[test]
fn test_bump_major() {
    let mut cmd = semverity();
    let assert = cmd.arg("bump").arg("1.2.3").arg("major").assert();
    assert.success().stdout(eq("2.0.0\n"));
}

#[test]
fn test_bump_minor() {
    let mut cmd = semverity();
    let assert = cmd.arg("bump").arg("1.2.3").arg("minor").assert();
    assert.success().stdout(eq("1.3.0\n"));
}

#[test]
fn test_bump_patch() {
    let mut cmd = semverity();
    let assert = cmd.arg("bump").arg("1.2.3").arg("patch").assert();
    assert.success().stdout(eq("1.2.4\n"));
}

#[test]
fn test_bump_prerelease() {
    let mut cmd = semverity();
    let assert = cmd
        .arg("bump")
        .arg("1.0.0")
        .arg("prerelease")
        .arg("--label")
        .arg("alpha")
        .assert();
    assert.success().stdout(eq("1.0.0-alpha\n"));
}

#[test]
fn test_diff_major() {
    let mut cmd = semverity();
    let assert = cmd.arg("diff").arg("1.0.0").arg("2.0.0").assert();
    assert
        .success()
        .stdout(contains("upgrade"))
        .stdout(contains("major"));
}

#[test]
fn test_diff_downgrade() {
    let mut cmd = semverity();
    let assert = cmd.arg("diff").arg("2.0.0").arg("1.0.0").assert();
    assert.success().stdout(contains("downgrade"));
}

#[test]
fn test_latest() {
    let mut cmd = semverity();
    let assert = cmd
        .arg("latest")
        .arg("1.0.0")
        .arg("3.0.0")
        .arg("2.0.0")
        .assert();
    assert.success().stdout(eq("3.0.0\n"));
}

#[test]
fn test_lowest() {
    let mut cmd = semverity();
    let assert = cmd
        .arg("lowest")
        .arg("3.0.0")
        .arg("1.0.0")
        .arg("2.0.0")
        .assert();
    assert.success().stdout(eq("1.0.0\n"));
}

#[test]
fn test_bump_from_commits_feat() {
    let mut cmd = semverity();
    let assert = cmd
        .arg("bump-from-commits")
        .arg("1.0.0")
        .arg("fix: bug fix")
        .arg("feat: new feature")
        .assert();
    assert.success().stdout(eq("1.1.0\n"));
}

#[test]
fn test_bump_from_commits_breaking() {
    let mut cmd = semverity();
    let assert = cmd
        .arg("bump-from-commits")
        .arg("1.0.0")
        .arg("feat!: breaking change")
        .assert();
    assert.success().stdout(eq("2.0.0\n"));
}

#[test]
fn test_analyze_no_file() {
    let mut cmd = semverity();
    let assert = cmd
        .arg("analyze")
        .arg("/nonexistent/path/to/package.json")
        .assert();
    assert.failure().stderr(contains("Error"));
}

#[test]
fn test_help() {
    let mut cmd = semverity();
    let assert = cmd.arg("--help").assert();
    assert
        .success()
        .stdout(contains("Parse, compare, resolve ranges"))
        .stdout(contains("parse"))
        .stdout(contains("compare"))
        .stdout(contains("sort"))
        .stdout(contains("check"))
        .stdout(contains("bump"))
        .stdout(contains("diff"))
        .stdout(contains("analyze"))
        .stdout(contains("latest"))
        .stdout(contains("lowest"));
}

#[test]
fn test_version_flag() {
    let mut cmd = semverity();
    let assert = cmd.arg("--version").assert();
    assert.success().stdout(contains("1.0.0"));
}
