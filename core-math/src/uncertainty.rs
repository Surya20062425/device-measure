//! Shared uncertainty representation and propagation helpers.
//!
//! Design principle for this whole project: never report a bare number for
//! a physically-estimated quantity. Every distance/diameter result carries
//! a `std_dev` (1-sigma), computed via first-order error propagation from
//! its inputs. This is what lets the UI honestly show `7.4cm ± 0.3cm`
//! instead of faking precision.

/// A measured value with 1-sigma standard deviation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Measurement {
    pub value: f64,
    pub std_dev: f64,
}

impl Measurement {
    pub fn new(value: f64, std_dev: f64) -> Self {
        Measurement {
            value,
            std_dev: std_dev.max(0.0),
        }
    }

    /// 95% confidence interval, assuming approximately Gaussian error
    /// (reasonable for first-order-propagated pixel/sensor noise, but not
    /// exact — treat as an honest approximation, not a guarantee).
    pub fn confidence_interval_95(&self) -> (f64, f64) {
        let margin = 1.96 * self.std_dev;
        (self.value - margin, self.value + margin)
    }

    /// Relative uncertainty as a fraction of the value (e.g. 0.05 = 5%).
    /// Returns None if value is zero (undefined relative error).
    pub fn relative_uncertainty(&self) -> Option<f64> {
        if self.value == 0.0 {
            None
        } else {
            Some(self.std_dev / self.value.abs())
        }
    }
}

impl std::fmt::Display for Measurement {
    /// Formats as "value ± std_dev", e.g. "7.4 ± 0.3".
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.3} ± {:.3}", self.value, self.std_dev)
    }
}

/// Combine two independent measurements' uncertainties in quadrature.
/// This is the standard rule for propagating uncorrelated errors through
/// a linear combination (used internally by diameter.rs; exposed here for
/// reuse by other modules, e.g. multi-frame fusion in tracking.rs).
pub fn combine_in_quadrature(terms: &[f64]) -> f64 {
    terms.iter().map(|t| t * t).sum::<f64>().sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confidence_interval_is_symmetric_around_value() {
        let m = Measurement::new(10.0, 1.0);
        let (lo, hi) = m.confidence_interval_95();
        assert!((10.0 - lo - (hi - 10.0)).abs() < 1e-9);
        assert!(lo < 10.0 && hi > 10.0);
    }

    #[test]
    fn relative_uncertainty_computed_correctly() {
        let m = Measurement::new(10.0, 2.0);
        assert!((m.relative_uncertainty().unwrap() - 0.2).abs() < 1e-9);
    }

    #[test]
    fn relative_uncertainty_none_for_zero_value() {
        let m = Measurement::new(0.0, 1.0);
        assert!(m.relative_uncertainty().is_none());
    }

    #[test]
    fn std_dev_never_negative_even_if_constructed_negative() {
        let m = Measurement::new(5.0, -3.0);
        assert_eq!(m.std_dev, 0.0);
    }

    #[test]
    fn quadrature_combination_matches_pythagorean_case() {
        // 3-4-5 triangle sanity check
        let combined = combine_in_quadrature(&[3.0, 4.0]);
        assert!((combined - 5.0).abs() < 1e-9);
    }

    #[test]
    fn quadrature_of_single_term_is_itself() {
        assert!((combine_in_quadrature(&[7.5]) - 7.5).abs() < 1e-9);
    }
}
