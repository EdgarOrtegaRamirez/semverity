//! SemVerity — Semantic Version Intelligence Toolkit
//! CLI entry point.

mod analyzer;
mod bumper;
mod differ;
mod ecosystem;
mod range;
mod version;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "semverity",
    version,
    about = "Semantic Version Intelligence Toolkit",
    long_about = "Parse, compare, resolve ranges, check constraints, detect conflicts, bump versions, and analyze package files across ecosystems."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse and display a semantic version
    Parse {
        /// The version string to parse
        version: String,
    },

    /// Compare two versions
    Compare {
        /// First version
        a: String,
        /// Second version
        b: String,
    },

    /// Sort a list of versions
    Sort {
        /// Versions to sort (space-separated)
        #[arg(num_args = 1..)]
        versions: Vec<String>,
        /// Sort in descending order
        #[arg(short, long)]
        desc: bool,
    },

    /// Check if a version satisfies a range
    Check {
        /// The version to check
        version: String,
        /// The range/constraint string
        range: String,
        /// Ecosystem syntax (npm, cargo, pip, gem)
        #[arg(short, long, default_value = "npm")]
        ecosystem: String,
    },

    /// Resolve and display a version range
    Resolve {
        /// The range string
        range: String,
        /// Ecosystem syntax (npm, cargo, pip, gem)
        #[arg(short, long, default_value = "npm")]
        ecosystem: String,
    },

    /// Intersect multiple ranges
    Intersect {
        /// Ranges to intersect
        #[arg(num_args = 1..)]
        ranges: Vec<String>,
        /// Ecosystem syntax
        #[arg(short, long, default_value = "npm")]
        ecosystem: String,
    },

    /// Bump a version
    Bump {
        /// The version to bump
        version: String,
        /// Bump type: major, minor, patch, prerelease
        #[arg(value_parser = ["major", "minor", "patch", "prerelease"])]
        bump_type: String,
        /// Prerelease label (for prerelease bumps)
        #[arg(long, default_value = "alpha")]
        label: String,
    },

    /// Bump based on conventional commit messages
    BumpFromCommits {
        /// The current version
        version: String,
        /// Commit messages (one per line via stdin, or as arguments)
        #[arg(num_args = 0..)]
        commits: Vec<String>,
        /// Prerelease label
        #[arg(long, default_value = "alpha")]
        label: String,
    },

    /// Diff two versions
    Diff {
        /// From version
        from: String,
        /// To version
        to: String,
    },

    /// Analyze a package file for version constraints
    Analyze {
        /// Path to the package file (package.json, Cargo.toml, pyproject.toml, go.mod)
        file: PathBuf,
        /// Output format: text, json
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Find the latest version from a list
    Latest {
        /// Versions to compare
        #[arg(num_args = 1..)]
        versions: Vec<String>,
    },

    /// Find the lowest version from a list
    Lowest {
        /// Versions to compare
        #[arg(num_args = 1..)]
        versions: Vec<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Parse { version } => cmd_parse(&version),
        Commands::Compare { a, b } => cmd_compare(&a, &b),
        Commands::Sort { versions, desc } => cmd_sort(&versions, desc),
        Commands::Check {
            version,
            range,
            ecosystem,
        } => cmd_check(&version, &range, &ecosystem),
        Commands::Resolve { range, ecosystem } => cmd_resolve(&range, &ecosystem),
        Commands::Intersect { ranges, ecosystem } => cmd_intersect(&ranges, &ecosystem),
        Commands::Bump {
            version,
            bump_type,
            label,
        } => cmd_bump(&version, &bump_type, &label),
        Commands::BumpFromCommits {
            version,
            commits,
            label,
        } => cmd_bump_from_commits(&version, &commits, &label),
        Commands::Diff { from, to } => cmd_diff(&from, &to),
        Commands::Analyze { file, format } => cmd_analyze(&file, &format),
        Commands::Latest { versions } => cmd_latest(&versions),
        Commands::Lowest { versions } => cmd_lowest(&versions),
    }
}

fn cmd_parse(input: &str) {
    match version::Version::parse(input) {
        Ok(v) => {
            println!("Version: {v}");
            println!("Major:   {}", v.major);
            println!("Minor:   {}", v.minor);
            println!("Patch:   {}", v.patch);
            if v.is_prerelease() {
                println!("Pre-release: {}", v.pre.join("."));
            }
            if !v.build.is_empty() {
                println!("Build:   {}", v.build.join("."));
            }
            println!("Stable:  {}", if v.is_stable() { "yes" } else { "no" });
        }
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}

fn cmd_compare(a: &str, b: &str) {
    let va = match version::Version::parse(a) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error parsing '{a}': {e}");
            std::process::exit(1);
        }
    };
    let vb = match version::Version::parse(b) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error parsing '{b}': {e}");
            std::process::exit(1);
        }
    };

    use std::cmp::Ordering;
    match va.cmp(&vb) {
        Ordering::Equal => println!("{a} == {b}"),
        Ordering::Less => println!("{a} < {b}"),
        Ordering::Greater => println!("{a} > {b}"),
    }
}

fn cmd_sort(versions: &[String], desc: bool) {
    let mut parsed: Vec<version::Version> = Vec::new();
    for v in versions {
        match version::Version::parse(v) {
            Ok(pv) => parsed.push(pv),
            Err(e) => {
                eprintln!("Error parsing '{v}': {e}");
                std::process::exit(1);
            }
        }
    }
    parsed.sort();
    if desc {
        parsed.reverse();
    }
    for v in parsed {
        println!("{v}");
    }
}

fn cmd_check(version_str: &str, range_str: &str, ecosystem_str: &str) {
    let v = match version::Version::parse(version_str) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error parsing version: {e}");
            std::process::exit(1);
        }
    };

    let eco = match ecosystem_str.parse::<ecosystem::Ecosystem>() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };

    match ecosystem::parse_range(range_str, eco) {
        Ok(range) => {
            let satisfies = range.contains(&v);
            if satisfies {
                println!("✓ {version_str} satisfies {range_str}");
            } else {
                println!("✗ {version_str} does NOT satisfy {range_str}");
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("Error parsing range: {e}");
            std::process::exit(1);
        }
    }
}

fn cmd_resolve(range_str: &str, ecosystem_str: &str) {
    let eco = match ecosystem_str.parse::<ecosystem::Ecosystem>() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };

    match ecosystem::parse_range(range_str, eco) {
        Ok(range) => {
            println!("Range: {range_str}");
            println!("Ecosystem: {eco}");
            println!("Resolved: {range}");
            if range.is_empty() {
                println!("Status: empty (matches no versions)");
            } else {
                println!("Status: valid");
            }
        }
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}

fn cmd_intersect(ranges: &[String], ecosystem_str: &str) {
    let eco = match ecosystem_str.parse::<ecosystem::Ecosystem>() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };

    let mut combined = range::VersionRange::all();
    for r in ranges {
        match ecosystem::parse_range(r, eco) {
            Ok(parsed) => combined = combined.intersect(&parsed),
            Err(e) => {
                eprintln!("Error parsing '{r}': {e}");
                std::process::exit(1);
            }
        }
    }

    println!("Intersection: {combined}");
    if combined.is_empty() {
        println!("Status: empty (no version satisfies all ranges)");
    } else {
        println!("Status: valid");
    }
}

fn cmd_bump(version_str: &str, bump_type: &str, label: &str) {
    let v = match version::Version::parse(version_str) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };

    let bt = match bump_type {
        "major" => bumper::BumpType::Major,
        "minor" => bumper::BumpType::Minor,
        "patch" => bumper::BumpType::Patch,
        "prerelease" => bumper::BumpType::Prerelease,
        _ => unreachable!(),
    };

    let bumped = bumper::bump(&v, bt, label);
    println!("{bumped}");
}

fn cmd_bump_from_commits(version_str: &str, commits: &[String], label: &str) {
    let v = match version::Version::parse(version_str) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };

    // If no commits provided as args, read from stdin
    let commit_strs: Vec<String> = if commits.is_empty() {
        use std::io::Read;
        let mut input = String::new();
        std::io::stdin().read_to_string(&mut input).unwrap_or(0);
        input
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|s| s.to_string())
            .collect()
    } else {
        commits.to_vec()
    };

    let refs: Vec<&str> = commit_strs.iter().map(|s| s.as_str()).collect();
    let bump_type = bumper::bump_from_commits(&refs);
    let bumped = bumper::bump(&v, bump_type, label);
    println!("{bumped}");
    eprintln!(
        "Bump type: {} (from {} commits)",
        bump_type.as_str(),
        refs.len()
    );
}

fn cmd_diff(from: &str, to: &str) {
    let from_v = match version::Version::parse(from) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error parsing '{from}': {e}");
            std::process::exit(1);
        }
    };
    let to_v = match version::Version::parse(to) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error parsing '{to}': {e}");
            std::process::exit(1);
        }
    };

    let d = differ::diff(&from_v, &to_v);
    println!("{d}");
}

fn cmd_analyze(file: &std::path::Path, format: &str) {
    match analyzer::analyze_file(file) {
        Ok(result) => match format {
            "json" => print_analyze_json(&result),
            _ => print_analyze_text(&result),
        },
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}

fn print_analyze_text(result: &analyzer::AnalysisResult) {
    println!("File: {}", result.file);
    println!("Ecosystem: {}", result.ecosystem);
    println!("Dependencies: {}", result.dependencies.len());
    println!();

    if result.dependencies.is_empty() {
        println!("No dependencies found.");
        return;
    }

    // Group by section
    use std::collections::BTreeMap;
    let mut sections: BTreeMap<String, Vec<&analyzer::Dependency>> = BTreeMap::new();
    for dep in &result.dependencies {
        sections.entry(dep.section.clone()).or_default().push(dep);
    }

    for (section, deps) in &sections {
        println!("[{section}]");
        for dep in deps {
            println!("  {} = \"{}\"", dep.name, dep.constraint);
        }
        println!();
    }

    if !result.conflicts.is_empty() {
        println!("⚠ Conflicts detected:");
        for conflict in &result.conflicts {
            println!("  {}: incompatible constraints", conflict.package);
            for c in &conflict.constraints {
                println!("    - {c}");
            }
        }
    } else {
        println!("✓ No conflicts detected.");
    }
}

fn print_analyze_json(result: &analyzer::AnalysisResult) {
    use serde::Serialize;

    #[derive(Serialize)]
    struct OutDep<'a> {
        name: &'a str,
        constraint: &'a str,
        section: &'a str,
    }

    #[derive(Serialize)]
    struct OutConflict<'a> {
        package: &'a str,
        constraints: &'a [String],
    }

    #[derive(Serialize)]
    struct OutResult<'a> {
        file: &'a str,
        ecosystem: &'a str,
        dependencies: Vec<OutDep<'a>>,
        conflicts: Vec<OutConflict<'a>>,
    }

    let out = OutResult {
        file: &result.file,
        ecosystem: result.ecosystem.as_str(),
        dependencies: result
            .dependencies
            .iter()
            .map(|d| OutDep {
                name: &d.name,
                constraint: &d.constraint,
                section: &d.section,
            })
            .collect(),
        conflicts: result
            .conflicts
            .iter()
            .map(|c| OutConflict {
                package: &c.package,
                constraints: &c.constraints,
            })
            .collect(),
    };

    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}

fn cmd_latest(versions: &[String]) {
    let mut parsed: Vec<version::Version> = Vec::new();
    for v in versions {
        match version::Version::parse(v) {
            Ok(pv) => parsed.push(pv),
            Err(e) => {
                eprintln!("Error parsing '{v}': {e}");
                std::process::exit(1);
            }
        }
    }
    if let Some(latest) = parsed.iter().max() {
        println!("{latest}");
    }
}

fn cmd_lowest(versions: &[String]) {
    let mut parsed: Vec<version::Version> = Vec::new();
    for v in versions {
        match version::Version::parse(v) {
            Ok(pv) => parsed.push(pv),
            Err(e) => {
                eprintln!("Error parsing '{v}': {e}");
                std::process::exit(1);
            }
        }
    }
    if let Some(lowest) = parsed.iter().min() {
        println!("{lowest}");
    }
}
