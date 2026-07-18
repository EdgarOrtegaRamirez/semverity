//! Version range parsing and resolution using interval arithmetic.
//!
//! A range represents a set of versions. We model ranges as a union of
//! intervals, each with optional lower and upper bounds (inclusive or exclusive).

use crate::version::Version;
use std::cmp::Ordering;
use std::fmt;

/// A bound on a version interval.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Bound {
    /// Inclusive lower bound (>= version)
    InclusiveLower(Version),
    /// Exclusive lower bound (> version)
    ExclusiveLower(Version),
    /// Inclusive upper bound (<= version)
    InclusiveUpper(Version),
    /// Exclusive upper bound (< version)
    ExclusiveUpper(Version),
}

/// A single interval of versions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interval {
    pub lower: Option<(Version, bool)>, // (version, inclusive)
    pub upper: Option<(Version, bool)>, // (version, inclusive)
}

impl Interval {
    /// Create an unbounded interval (matches all versions).
    pub fn all() -> Self {
        Self {
            lower: None,
            upper: None,
        }
    }

    /// Create an interval from a lower bound.
    pub fn lower_bound(version: Version, inclusive: bool) -> Self {
        Self {
            lower: Some((version, inclusive)),
            upper: None,
        }
    }

    /// Create an interval from an upper bound.
    pub fn upper_bound(version: Version, inclusive: bool) -> Self {
        Self {
            lower: None,
            upper: Some((version, inclusive)),
        }
    }

    /// Create an exact match interval (== version).
    pub fn exact(version: Version) -> Self {
        Self {
            lower: Some((version.clone(), true)),
            upper: Some((version, true)),
        }
    }

    /// Create a bounded interval.
    pub fn bounded(
        lower: Version,
        lower_inclusive: bool,
        upper: Version,
        upper_inclusive: bool,
    ) -> Self {
        Self {
            lower: Some((lower, lower_inclusive)),
            upper: Some((upper, upper_inclusive)),
        }
    }

    /// Check if a version falls within this interval.
    pub fn contains(&self, version: &Version) -> bool {
        if let Some((lower, inclusive)) = &self.lower {
            match version.cmp(lower) {
                Ordering::Less => return false,
                Ordering::Equal if !inclusive => return false,
                _ => {}
            }
        }
        if let Some((upper, inclusive)) = &self.upper {
            match version.cmp(upper) {
                Ordering::Greater => return false,
                Ordering::Equal if !inclusive => return false,
                _ => {}
            }
        }
        true
    }

    /// Intersect two intervals. Returns None if the intersection is empty.
    pub fn intersect(&self, other: &Self) -> Option<Self> {
        // Compute the tighter lower bound
        let lower = tighter_lower(&self.lower, &other.lower);

        // Compute the tighter upper bound
        let upper = tighter_upper(&self.upper, &other.upper);

        // Check if the interval is valid (lower <= upper)
        if let (Some((lv, li)), Some((uv, ui))) = (&lower, &upper) {
            match lv.cmp(uv) {
                Ordering::Greater => return None,
                Ordering::Equal if !(*li && *ui) => return None,
                _ => {}
            }
        }

        Some(Self { lower, upper })
    }
}

/// A version range: a union of intervals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionRange {
    pub intervals: Vec<Interval>,
}

impl VersionRange {
    /// Create a range matching all versions.
    pub fn all() -> Self {
        Self {
            intervals: vec![Interval::all()],
        }
    }

    /// Create a range matching no versions.
    pub fn none() -> Self {
        Self { intervals: vec![] }
    }

    /// Create a range from a single interval.
    pub fn from_interval(interval: Interval) -> Self {
        Self {
            intervals: vec![interval],
        }
    }

    /// Create a range matching an exact version.
    pub fn exact(version: Version) -> Self {
        Self::from_interval(Interval::exact(version))
    }

    /// Check if a version satisfies this range.
    pub fn contains(&self, version: &Version) -> bool {
        self.intervals.iter().any(|i| i.contains(version))
    }

    /// Union with another range.
    pub fn union(&self, other: &Self) -> Self {
        let mut intervals = self.intervals.clone();
        intervals.extend(other.intervals.clone());
        Self {
            intervals: simplify_intervals(intervals),
        }
    }

    /// Intersect with another range. Returns empty range if no overlap.
    pub fn intersect(&self, other: &Self) -> Self {
        let mut result = Vec::new();
        for a in &self.intervals {
            for b in &other.intervals {
                if let Some(intersection) = a.intersect(b) {
                    result.push(intersection);
                }
            }
        }
        Self { intervals: result }
    }

    /// Check if the range is empty (matches no versions).
    pub fn is_empty(&self) -> bool {
        self.intervals.is_empty()
    }

    /// Negate the range (complement).
    pub fn negate(&self) -> Self {
        // The complement of a union of intervals is the intersection of complements.
        // For simplicity, we compute the complement of each interval and intersect them.
        if self.intervals.is_empty() {
            return Self::all();
        }

        let mut result = vec![Interval::all()];
        for interval in &self.intervals {
            let complements = complement_interval(interval);
            let mut new_result = Vec::new();
            for r in &result {
                for c in &complements {
                    if let Some(i) = r.intersect(c) {
                        new_result.push(i);
                    }
                }
            }
            result = new_result;
        }
        Self { intervals: result }
    }
}

/// Compute the complement of an interval (returns 0, 1, or 2 intervals).
fn complement_interval(interval: &Interval) -> Vec<Interval> {
    let mut result = Vec::new();
    if let Some((lower, inclusive)) = &interval.lower {
        // Complement of >= v is < v; complement of > v is <= v
        result.push(Interval::upper_bound(lower.clone(), !*inclusive));
    }
    if let Some((upper, inclusive)) = &interval.upper {
        // Complement of <= v is > v; complement of < v is >= v
        result.push(Interval::lower_bound(upper.clone(), !*inclusive));
    }
    if result.is_empty() {
        // The interval was unbounded (matches all), complement is empty
        return vec![];
    }
    result
}

/// Find the tighter (higher) lower bound.
fn tighter_lower(
    a: &Option<(Version, bool)>,
    b: &Option<(Version, bool)>,
) -> Option<(Version, bool)> {
    match (a, b) {
        (None, None) => None,
        (Some(x), None) => Some(x.clone()),
        (None, Some(x)) => Some(x.clone()),
        (Some((av, ai)), Some((bv, bi))) => match av.cmp(bv) {
            Ordering::Greater => Some((av.clone(), *ai)),
            Ordering::Less => Some((bv.clone(), *bi)),
            Ordering::Equal => Some((av.clone(), *ai && *bi)),
        },
    }
}

/// Find the tighter (lower) upper bound.
fn tighter_upper(
    a: &Option<(Version, bool)>,
    b: &Option<(Version, bool)>,
) -> Option<(Version, bool)> {
    match (a, b) {
        (None, None) => None,
        (Some(x), None) => Some(x.clone()),
        (None, Some(x)) => Some(x.clone()),
        (Some((av, ai)), Some((bv, bi))) => match av.cmp(bv) {
            Ordering::Less => Some((av.clone(), *ai)),
            Ordering::Greater => Some((bv.clone(), *bi)),
            Ordering::Equal => Some((av.clone(), *ai && *bi)),
        },
    }
}

/// Simplify and sort intervals (merge overlapping ones).
fn simplify_intervals(mut intervals: Vec<Interval>) -> Vec<Interval> {
    if intervals.is_empty() {
        return intervals;
    }

    // Sort by lower bound
    intervals.sort_by(|a, b| match (&a.lower, &b.lower) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Less,
        (Some(_), None) => Ordering::Greater,
        (Some((av, _)), Some((bv, _))) => av.cmp(bv),
    });

    let mut result: Vec<Interval> = Vec::with_capacity(intervals.len());
    for interval in intervals {
        if let Some(last) = result.last_mut() {
            // Check if intervals overlap or are adjacent
            if let (Some((last_upper, last_inclusive)), Some((cur_lower, cur_inclusive))) =
                (&last.upper, &interval.lower)
            {
                match last_upper.cmp(cur_lower) {
                    Ordering::Less => result.push(interval),
                    Ordering::Equal => {
                        // Adjacent: merge if either is inclusive
                        if *last_inclusive || *cur_inclusive {
                            last.upper = interval.upper;
                        } else {
                            result.push(interval);
                        }
                    }
                    Ordering::Greater => {
                        // Overlapping: merge by taking the wider upper bound
                        match (&last.upper, &interval.upper) {
                            (None, _) => {} // last is unbounded above, keep it
                            (Some(_), None) => last.upper = None,
                            (Some((lu, li)), Some((cu, ci))) => {
                                match lu.cmp(cu) {
                                    Ordering::Less => last.upper = Some((cu.clone(), *ci)),
                                    Ordering::Equal => last.upper = Some((cu.clone(), *li || *ci)),
                                    Ordering::Greater => {} // keep last
                                }
                            }
                        }
                    }
                }
            } else {
                // At least one is unbounded; merge
                match (&last.upper, &interval.upper) {
                    (None, _) => {}
                    (Some(_), None) => last.upper = None,
                    (Some((lu, li)), Some((cu, ci))) => match lu.cmp(cu) {
                        Ordering::Less => last.upper = Some((cu.clone(), *ci)),
                        Ordering::Equal => last.upper = Some((cu.clone(), *li || *ci)),
                        Ordering::Greater => {}
                    },
                }
            }
        } else {
            result.push(interval);
        }
    }
    result
}

impl fmt::Display for VersionRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            return write!(f, "(none)");
        }
        let parts: Vec<String> = self.intervals.iter().map(format_interval).collect();
        write!(f, "{}", parts.join(" OR "))
    }
}

fn format_interval(interval: &Interval) -> String {
    let lower = match &interval.lower {
        Some((v, true)) => format!(">={v}"),
        Some((v, false)) => format!(">{v}"),
        None => "*".to_string(),
    };
    let upper = match &interval.upper {
        Some((v, true)) => format!("<={v}"),
        Some((v, false)) => format!("<{v}"),
        None => "*".to_string(),
    };
    match (&interval.lower, &interval.upper) {
        (None, None) => "*".to_string(),
        (Some(_), None) => lower,
        (None, Some(_)) => upper,
        (Some(_), Some(_)) => format!("{lower} AND {upper}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    #[test]
    fn test_interval_contains() {
        let interval = Interval::bounded(v("1.0.0"), true, v("2.0.0"), false);
        assert!(interval.contains(&v("1.0.0")));
        assert!(interval.contains(&v("1.5.0")));
        assert!(!interval.contains(&v("2.0.0")));
        assert!(!interval.contains(&v("0.9.0")));
    }

    #[test]
    fn test_interval_exact() {
        let interval = Interval::exact(v("1.2.3"));
        assert!(interval.contains(&v("1.2.3")));
        assert!(!interval.contains(&v("1.2.4")));
        assert!(!interval.contains(&v("1.2.2")));
    }

    #[test]
    fn test_interval_intersect() {
        let a = Interval::bounded(v("1.0.0"), true, v("3.0.0"), false);
        let b = Interval::bounded(v("2.0.0"), true, v("4.0.0"), false);
        let result = a.intersect(&b).unwrap();
        assert!(result.contains(&v("2.0.0")));
        assert!(result.contains(&v("2.5.0")));
        assert!(!result.contains(&v("1.5.0")));
        assert!(!result.contains(&v("3.0.0")));
    }

    #[test]
    fn test_interval_no_intersect() {
        let a = Interval::bounded(v("1.0.0"), true, v("2.0.0"), false);
        let b = Interval::bounded(v("3.0.0"), true, v("4.0.0"), false);
        assert!(a.intersect(&b).is_none());
    }

    #[test]
    fn test_range_contains() {
        let range =
            VersionRange::from_interval(Interval::bounded(v("1.0.0"), true, v("2.0.0"), false));
        assert!(range.contains(&v("1.0.0")));
        assert!(range.contains(&v("1.5.0")));
        assert!(!range.contains(&v("2.0.0")));
    }

    #[test]
    fn test_range_union() {
        let a = VersionRange::from_interval(Interval::bounded(v("1.0.0"), true, v("2.0.0"), false));
        let b = VersionRange::from_interval(Interval::bounded(v("3.0.0"), true, v("4.0.0"), false));
        let union = a.union(&b);
        assert!(union.contains(&v("1.5.0")));
        assert!(union.contains(&v("3.5.0")));
        assert!(!union.contains(&v("2.5.0")));
    }

    #[test]
    fn test_range_intersect() {
        let a = VersionRange::from_interval(Interval::bounded(v("1.0.0"), true, v("3.0.0"), false));
        let b = VersionRange::from_interval(Interval::bounded(v("2.0.0"), true, v("4.0.0"), false));
        let result = a.intersect(&b);
        assert!(result.contains(&v("2.0.0")));
        assert!(result.contains(&v("2.5.0")));
        assert!(!result.contains(&v("1.5.0")));
        assert!(!result.contains(&v("3.0.0")));
    }

    #[test]
    fn test_range_negate() {
        let range =
            VersionRange::from_interval(Interval::bounded(v("1.0.0"), true, v("2.0.0"), false));
        let negated = range.negate();
        assert!(!negated.contains(&v("1.5.0")));
        assert!(negated.contains(&v("0.5.0")));
        assert!(negated.contains(&v("2.0.0")));
        assert!(negated.contains(&v("3.0.0")));
    }

    #[test]
    fn test_range_exact() {
        let range = VersionRange::exact(v("1.2.3"));
        assert!(range.contains(&v("1.2.3")));
        assert!(!range.contains(&v("1.2.4")));
    }

    #[test]
    fn test_range_empty() {
        let range = VersionRange::none();
        assert!(range.is_empty());
        assert!(!range.contains(&v("1.0.0")));
    }

    #[test]
    fn test_range_all() {
        let range = VersionRange::all();
        assert!(range.contains(&v("0.0.1")));
        assert!(range.contains(&v("99.99.99")));
    }
}
