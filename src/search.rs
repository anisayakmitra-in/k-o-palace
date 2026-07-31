//! Search helpers and indexing.

use crate::models::Package;

/// Score a package against a query. Higher is better.
pub fn score(query: &str, pkg: &Package) -> f64 {
    let q = query.to_lowercase();
    let mut score = 0.0;
    if pkg.id.to_lowercase() == q {
        score += 100.0;
    } else if pkg.id.to_lowercase().contains(&q) {
        score += 50.0;
    }
    if pkg.name.to_lowercase().contains(&q) {
        score += 20.0;
    }
    if pkg.description.to_lowercase().contains(&q) {
        score += 10.0;
    }
    if pkg.tags.iter().any(|t| t.to_lowercase().contains(&q)) {
        score += 5.0;
    }
    score += (pkg.downloads as f64).ln_1p() * 0.5;
    score
}

/// Rank a list of packages by relevance.
pub fn rank_results(query: &str, packages: &mut [Package]) {
    let q = query.to_lowercase();
    packages.sort_by(|a, b| {
        let sa = score(&q, a);
        let sb = score(&q, b);
        sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
    });
}
