use serde::{Deserialize, Serialize};

/// Result of a Mann-Whitney U test comparing two distributions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MannWhitneyResult {
    pub u_statistic: f64,
    pub z_score: f64,
    pub p_value: f64,
    pub significant: bool,
}

/// Perform the Mann-Whitney U test (two-tailed) on two independent samples.
///
/// Uses the normal approximation for the p-value (valid for n > 20).
/// Handles tied ranks with the average-rank method and applies tie correction
/// to the variance.
pub fn mann_whitney_u(baseline: &[f64], canary: &[f64]) -> MannWhitneyResult {
    let n1 = baseline.len();
    let n2 = canary.len();

    if n1 == 0 || n2 == 0 {
        return MannWhitneyResult {
            u_statistic: 0.0,
            z_score: 0.0,
            p_value: 1.0,
            significant: false,
        };
    }

    // Combine all values with group tags (0 = baseline, 1 = canary)
    let mut combined: Vec<(f64, usize)> = Vec::with_capacity(n1 + n2);
    for &v in baseline {
        combined.push((v, 0));
    }
    for &v in canary {
        combined.push((v, 1));
    }

    // Sort by value (stable sort preserves order within ties)
    combined.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    // Assign ranks with average-rank method for ties
    let n = combined.len();
    let mut ranks = vec![0.0f64; n];
    let mut tie_sizes: Vec<usize> = Vec::new();
    let mut i = 0;

    while i < n {
        let mut j = i;
        while j < n && combined[j].0 == combined[i].0 {
            j += 1;
        }
        // Positions i..j are tied; average rank is (i+1 + j) / 2 (1-based)
        let avg_rank = (i + 1 + j) as f64 / 2.0;
        for rank in ranks.iter_mut().take(j).skip(i) {
            *rank = avg_rank;
        }
        tie_sizes.push(j - i);
        i = j;
    }

    // Sum of ranks for baseline (group 0)
    let r1: f64 = combined
        .iter()
        .enumerate()
        .filter(|(_, (_, group))| *group == 0)
        .map(|(idx, _)| ranks[idx])
        .sum();

    // U statistics
    let u1 = r1 - (n1 as f64 * (n1 as f64 + 1.0)) / 2.0;
    let u2 = (n1 as f64 * n2 as f64) - u1;
    let u = u1.min(u2);

    // Expected value under H0
    let mu = (n1 as f64 * n2 as f64) / 2.0;

    // Variance with tie correction
    let n_total = n as f64;
    let tie_correction: f64 = tie_sizes
        .iter()
        .map(|&t| {
            let t = t as f64;
            (t * t * t - t) / (n_total * (n_total - 1.0))
        })
        .sum();

    let sigma = ((n1 as f64 * n2 as f64 / 12.0) * (n_total + 1.0 - tie_correction)).sqrt();

    if sigma == 0.0 {
        return MannWhitneyResult {
            u_statistic: u,
            z_score: 0.0,
            p_value: 1.0,
            significant: false,
        };
    }

    // Continuity correction: subtract 0.5 from |U - μ|
    let z = ((u - mu).abs() - 0.5).max(0.0) / sigma;

    // Two-tailed p-value
    let p_value = 2.0 * (1.0 - normal_cdf(z));

    MannWhitneyResult {
        u_statistic: u,
        z_score: z,
        p_value,
        significant: p_value < 0.05,
    }
}

/// Standard normal CDF approximation using Abramowitz and Stegun formula.
///
/// Accuracy: ~1e-5 for all x.
pub fn normal_cdf(x: f64) -> f64 {
    if x < 0.0 {
        return 1.0 - normal_cdf(-x);
    }

    let p = 0.33267;
    let b1 = 0.4361836;
    let b2 = -0.1201676;
    let b3 = 0.9372980;

    let t = 1.0 / (1.0 + p * x);
    let phi = (-x * x / 2.0).exp() / (2.0 * std::f64::consts::PI).sqrt();

    1.0 - phi * (b1 * t + b2 * t * t + b3 * t * t * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_cdf_at_zero() {
        let result = normal_cdf(0.0);
        assert!((result - 0.5).abs() < 1e-4);
    }

    #[test]
    fn normal_cdf_large_positive() {
        let result = normal_cdf(4.0);
        assert!((result - 1.0).abs() < 1e-4);
    }

    #[test]
    fn normal_cdf_large_negative() {
        let result = normal_cdf(-4.0);
        assert!(result < 1e-4);
    }

    #[test]
    fn normal_cdf_known_values() {
        // Φ(1.96) ≈ 0.975
        assert!((normal_cdf(1.96) - 0.975).abs() < 1e-3);
        // Φ(1.0) ≈ 0.8413
        assert!((normal_cdf(1.0) - 0.8413).abs() < 1e-3);
        // Φ(-1.0) ≈ 0.1587
        assert!((normal_cdf(-1.0) - 0.1587).abs() < 1e-3);
        // Φ(2.576) ≈ 0.995
        assert!((normal_cdf(2.576) - 0.995).abs() < 1e-3);
    }
}
