//! core-math: platform-agnostic distance, diameter, uncertainty, and
//! temporal-smoothing calculations shared by the Android app (via JNI) and
//! the Web app (via WASM).
//!
//! This crate intentionally contains NO camera access, NO model inference,
//! and NO UI code — it is pure numeric/geometric logic so it can be tested,
//! reasoned about, and reused identically on both platforms. See
//! docs/ARCHITECTURE.md for how this fits into the full system.

pub mod diameter;
pub mod distance;
pub mod tracking;
pub mod uncertainty;

pub use diameter::diameter_from_distance;
pub use distance::{
    distance_from_depth_sensor, distance_from_known_width, distance_from_stereo_disparity,
    CameraIntrinsics, MeasurementError,
};
pub use tracking::RollingHistory;
pub use uncertainty::Measurement;

#[cfg(test)]
mod integration_tests {
    use super::*;

    /// End-to-end sanity check: known-width distance estimation feeding
    /// into diameter estimation for a *different* object in the same
    /// frame, using the same camera intrinsics. This exercises the crate
    /// the way a real caller would: distance first, then diameter from
    /// that distance.
    #[test]
    fn distance_then_diameter_pipeline() {
        let intrinsics = CameraIntrinsics {
            focal_length_px: 1000.0,
            cx: 320.0,
            cy: 240.0,
        };

        // Reference object: known width 0.08m (e.g. a soda can diameter),
        // measured at 200px wide in-frame.
        let distance =
            distance_from_known_width(&intrinsics, 0.08, 200.0, 2.0).expect("distance calc");
        assert!((distance.value - 0.4).abs() < 1e-9);

        // Target object of unknown size, at the same distance, measured at
        // 150px wide.
        let diameter = diameter_from_distance(intrinsics.focal_length_px, &distance, 150.0, 1.5)
            .expect("diameter calc");
        // diameter = (150 * 0.4) / 1000 = 0.06 m
        assert!((diameter.value - 0.06).abs() < 1e-9);
        // uncertainty must be non-zero since both inputs had noise
        assert!(diameter.std_dev > 0.0);
    }

    /// End-to-end sanity check for the smoothing layer: several noisy
    /// single-frame diameter measurements for the same tracked object
    /// should converge to a stable smoothed estimate close to the true
    /// value, with reported uncertainty reflecting the noise.
    #[test]
    fn smoothing_pipeline_stabilizes_noisy_frames() {
        let mut history = RollingHistory::new(5);
        let noisy_frames = [0.058, 0.062, 0.059, 0.061, 0.060];
        for v in noisy_frames {
            history.push(Measurement { value: v, std_dev: 0.002 });
        }
        let smoothed = history.smoothed().expect("history has frames");
        assert!(
            (smoothed.value - 0.06).abs() < 0.005,
            "smoothed value {} too far from expected ~0.06",
            smoothed.value
        );
    }
}
