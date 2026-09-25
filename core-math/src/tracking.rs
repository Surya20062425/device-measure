//! Per-object temporal smoothing.
//!
//! This module does NOT implement bbox-to-bbox identity assignment
//! (that's SORT/ByteTrack's job, running in the platform-native detection
//! layer). It assumes a stable track ID is already provided per frame, and
//! its only responsibility is: given a stream of noisy single-frame
//! measurements for one object, produce a smoothed, more stable estimate.
//!
//! Rolling median is used (not mean) because detector/segmentation jitter
//! occasionally produces sharp outliers (e.g. a single bad frame with a
//! partially occluded box) that a mean would let skew the result but a
//! median is naturally robust to.

use crate::uncertainty::Measurement;
use std::collections::VecDeque;

/// Fixed-capacity rolling window of measurements for a single tracked
/// object, used to smooth distance/diameter estimates across frames.
#[derive(Debug, Clone)]
pub struct RollingHistory {
    window: VecDeque<Measurement>,
    capacity: usize,
}

impl RollingHistory {
    /// `capacity` is the max number of recent frames retained. Must be >= 1.
    pub fn new(capacity: usize) -> Self {
        RollingHistory {
            window: VecDeque::with_capacity(capacity.max(1)),
            capacity: capacity.max(1),
        }
    }

    /// Push a new single-frame measurement, evicting the oldest if the
    /// window is full.
    pub fn push(&mut self, m: Measurement) {
        if self.window.len() == self.capacity {
            self.window.pop_front();
        }
        self.window.push_back(m);
    }

    pub fn len(&self) -> usize {
        self.window.len()
    }

    pub fn is_empty(&self) -> bool {
        self.window.is_empty()
    }

    /// Smoothed estimate: median of the values currently in the window,
    /// with a std_dev computed as the sample standard deviation of the
    /// windowed values (capturing frame-to-frame jitter) combined in
    /// quadrature with the mean of the individual per-frame std_devs
    /// (capturing each frame's own measurement uncertainty). Returns None
    /// if the window is empty.
    pub fn smoothed(&self) -> Option<Measurement> {
        if self.window.is_empty() {
            return None;
        }

        let mut values: Vec<f64> = self.window.iter().map(|m| m.value).collect();
        values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median = median_of_sorted(&values);

        let jitter_std_dev = sample_std_dev(&values);

        let mean_per_frame_std_dev = self.window.iter().map(|m| m.std_dev).sum::<f64>()
            / self.window.len() as f64;

        let combined_std_dev =
            (jitter_std_dev * jitter_std_dev + mean_per_frame_std_dev * mean_per_frame_std_dev)
                .sqrt();

        Some(Measurement {
            value: median,
            std_dev: combined_std_dev,
        })
    }
}

fn median_of_sorted(sorted_values: &[f64]) -> f64 {
    let n = sorted_values.len();
    if n % 2 == 1 {
        sorted_values[n / 2]
    } else {
        (sorted_values[n / 2 - 1] + sorted_values[n / 2]) / 2.0
    }
}

fn sample_std_dev(values: &[f64]) -> f64 {
    let n = values.len();
    if n < 2 {
        return 0.0;
    }
    let mean = values.iter().sum::<f64>() / n as f64;
    let variance =
        values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n as f64 - 1.0);
    variance.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(value: f64) -> Measurement {
        Measurement { value, std_dev: 0.0 }
    }

    #[test]
    fn empty_history_returns_none() {
        let h = RollingHistory::new(5);
        assert!(h.smoothed().is_none());
    }

    #[test]
    fn single_measurement_passthrough() {
        let mut h = RollingHistory::new(5);
        h.push(Measurement { value: 10.0, std_dev: 0.5 });
        let s = h.smoothed().unwrap();
        assert!((s.value - 10.0).abs() < 1e-9);
    }

    #[test]
    fn median_is_robust_to_single_outlier() {
        let mut h = RollingHistory::new(5);
        for v in [10.0, 10.1, 9.9, 10.0] {
            h.push(m(v));
        }
        h.push(m(100.0)); // one bad frame (e.g. occlusion glitch)
        let s = h.smoothed().unwrap();
        // median should stay near ~10, not be dragged toward 100
        assert!(s.value < 15.0, "median was pulled toward outlier: {}", s.value);
    }

    #[test]
    fn window_evicts_oldest_beyond_capacity() {
        let mut h = RollingHistory::new(3);
        h.push(m(1.0));
        h.push(m(2.0));
        h.push(m(3.0));
        h.push(m(4.0)); // should evict 1.0
        assert_eq!(h.len(), 3);
        let s = h.smoothed().unwrap();
        // remaining values: 2,3,4 -> median 3
        assert!((s.value - 3.0).abs() < 1e-9);
    }

    #[test]
    fn stable_values_produce_low_jitter_std_dev() {
        let mut h = RollingHistory::new(5);
        for _ in 0..5 {
            h.push(Measurement { value: 10.0, std_dev: 0.1 });
        }
        let s = h.smoothed().unwrap();
        // no frame-to-frame jitter, so std_dev should just reflect the
        // per-frame measurement noise (~0.1), not be inflated
        assert!(s.std_dev < 0.2, "unexpected inflated std_dev: {}", s.std_dev);
    }

    #[test]
    fn noisy_values_produce_higher_std_dev_than_stable_values() {
        let mut stable = RollingHistory::new(5);
        let mut noisy = RollingHistory::new(5);
        for v in [10.0, 10.0, 10.0, 10.0, 10.0] {
            stable.push(m(v));
        }
        for v in [8.0, 12.0, 9.0, 13.0, 10.0] {
            noisy.push(m(v));
        }
        let s_stable = stable.smoothed().unwrap();
        let s_noisy = noisy.smoothed().unwrap();
        assert!(s_noisy.std_dev > s_stable.std_dev);
    }
}
