//! Distance estimation backends.
//!
//! All distance methods ultimately rely on the pinhole camera model:
//!
//! ```text
//! pixel_size = (real_size * focal_length_px) / distance
//! ```
//!
//! Rearranged for distance:
//!
//! ```text
//! distance = (real_size * focal_length_px) / pixel_size
//! ```
//!
//! `focal_length_px` is the focal length expressed in *pixels*, i.e.
//! `focal_length_mm * (image_width_px / sensor_width_mm)`. Callers must
//! convert from datasheet mm values before calling into this module.
//!
//! IMPORTANT: a single 2D image cannot resolve distance and real-world size
//! simultaneously (a big object far away is indistinguishable in pixels
//! from a small object close up). Every method below resolves that
//! ambiguity differently. See ARCHITECTURE.md for the accuracy ceiling of
//! each.

use crate::uncertainty::Measurement;

/// Camera intrinsic parameters, in pixel units, after calibration.
#[derive(Debug, Clone, Copy)]
pub struct CameraIntrinsics {
    /// Focal length in pixels (fx). For square pixels fx ≈ fy; if you have
    /// both, prefer using fx for horizontal measurements and fy for vertical.
    pub focal_length_px: f64,
    /// Principal point / image center, not used by the distance formulas
    /// directly but kept here since it's part of a full intrinsics matrix
    /// and downstream undistortion needs it.
    pub cx: f64,
    pub cy: f64,
}

/// Error type shared by all distance/diameter estimators.
#[derive(Debug, Clone, PartialEq)]
pub enum MeasurementError {
    /// A required input was zero, negative, or otherwise physically invalid.
    InvalidInput(String),
}

impl std::fmt::Display for MeasurementError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MeasurementError::InvalidInput(msg) => write!(f, "invalid input: {msg}"),
        }
    }
}
impl std::error::Error for MeasurementError {}

/// Distance estimation using a known real-world reference width.
///
/// Accuracy ceiling: ~3-8% under good calibration, dominated by pixel-edge
/// localization error at the object boundary. Requires the real width of
/// the object class to actually be known and fixed (fails on objects with
/// variable size within a class).
///
/// `pixel_width_std_dev` is the estimated 1-sigma noise in the pixel-width
/// measurement (from detector/segmentation boundary jitter), used to
/// propagate uncertainty into the result.
pub fn distance_from_known_width(
    intrinsics: &CameraIntrinsics,
    real_width_m: f64,
    pixel_width: f64,
    pixel_width_std_dev: f64,
) -> Result<Measurement, MeasurementError> {
    if real_width_m <= 0.0 {
        return Err(MeasurementError::InvalidInput(
            "real_width_m must be positive".into(),
        ));
    }
    if pixel_width <= 0.0 {
        return Err(MeasurementError::InvalidInput(
            "pixel_width must be positive".into(),
        ));
    }
    if intrinsics.focal_length_px <= 0.0 {
        return Err(MeasurementError::InvalidInput(
            "focal_length_px must be positive".into(),
        ));
    }

    let value = (real_width_m * intrinsics.focal_length_px) / pixel_width;

    // Propagate pixel-width noise through the (nonlinear, 1/x) relation via
    // first-order Taylor expansion:
    //   d(distance)/d(pixel_width) = -(real_width * focal_length) / pixel_width^2
    // std_dev_distance ≈ |derivative| * std_dev_pixel_width
    let derivative =
        (real_width_m * intrinsics.focal_length_px) / (pixel_width * pixel_width);
    let std_dev = derivative * pixel_width_std_dev;

    Ok(Measurement { value, std_dev })
}

/// Distance estimation from stereo disparity (triangulation).
///
/// Accuracy ceiling: ~1-3% at close range, degrading roughly quadratically
/// with distance, since disparity resolution is fixed in pixels but maps to
/// increasingly coarse depth steps as distance grows. Requires a calibrated
/// stereo pair (known baseline) and a valid (non-zero) disparity — flat,
/// textureless surfaces frequently yield unreliable disparity and should be
/// filtered upstream before calling this.
///
/// `disparity_std_dev` is the 1-sigma noise in the disparity measurement
/// (sub-pixel disparity estimation typically achieves ~0.1-0.5 px).
pub fn distance_from_stereo_disparity(
    intrinsics: &CameraIntrinsics,
    baseline_m: f64,
    disparity_px: f64,
    disparity_std_dev: f64,
) -> Result<Measurement, MeasurementError> {
    if baseline_m <= 0.0 {
        return Err(MeasurementError::InvalidInput(
            "baseline_m must be positive".into(),
        ));
    }
    if disparity_px <= 0.0 {
        return Err(MeasurementError::InvalidInput(
            "disparity_px must be positive (non-zero disparity required)".into(),
        ));
    }

    let value = (baseline_m * intrinsics.focal_length_px) / disparity_px;

    // d(distance)/d(disparity) = -(baseline * focal_length) / disparity^2
    let derivative =
        (baseline_m * intrinsics.focal_length_px) / (disparity_px * disparity_px);
    let std_dev = derivative * disparity_std_dev;

    Ok(Measurement { value, std_dev })
}

/// Distance from a direct depth-sensor reading (ToF / structured light /
/// LiDAR). This is the "ground truth" tier — the sensor hardware has
/// already resolved the distance/size ambiguity via physical measurement,
/// so this function is a thin, honest passthrough rather than a geometric
/// derivation. `sensor_std_dev_m` should come from the sensor's own
/// datasheet noise spec (varies by device and range).
pub fn distance_from_depth_sensor(
    depth_reading_m: f64,
    sensor_std_dev_m: f64,
) -> Result<Measurement, MeasurementError> {
    if depth_reading_m <= 0.0 {
        return Err(MeasurementError::InvalidInput(
            "depth_reading_m must be positive".into(),
        ));
    }
    Ok(Measurement {
        value: depth_reading_m,
        std_dev: sensor_std_dev_m.max(0.0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn intrinsics(focal_px: f64) -> CameraIntrinsics {
        CameraIntrinsics {
            focal_length_px: focal_px,
            cx: 0.0,
            cy: 0.0,
        }
    }

    #[test]
    fn known_width_matches_hand_calculation() {
        // real_width=0.08m, focal=1000px, pixel_width=200px
        // distance = (0.08 * 1000) / 200 = 0.4 m
        let m = distance_from_known_width(&intrinsics(1000.0), 0.08, 200.0, 0.0).unwrap();
        assert!((m.value - 0.4).abs() < 1e-9);
    }

    #[test]
    fn known_width_rejects_nonpositive_inputs() {
        assert!(distance_from_known_width(&intrinsics(1000.0), -1.0, 200.0, 0.0).is_err());
        assert!(distance_from_known_width(&intrinsics(1000.0), 0.08, 0.0, 0.0).is_err());
        assert!(distance_from_known_width(&intrinsics(0.0), 0.08, 200.0, 0.0).is_err());
    }

    #[test]
    fn known_width_uncertainty_scales_with_pixel_noise() {
        let low_noise =
            distance_from_known_width(&intrinsics(1000.0), 0.08, 200.0, 1.0).unwrap();
        let high_noise =
            distance_from_known_width(&intrinsics(1000.0), 0.08, 200.0, 5.0).unwrap();
        assert!(high_noise.std_dev > low_noise.std_dev);
    }

    #[test]
    fn stereo_disparity_matches_hand_calculation() {
        // baseline=0.06m, focal=1000px, disparity=30px
        // distance = (0.06*1000)/30 = 2.0 m
        let m = distance_from_stereo_disparity(&intrinsics(1000.0), 0.06, 30.0, 0.0).unwrap();
        assert!((m.value - 2.0).abs() < 1e-9);
    }

    #[test]
    fn stereo_rejects_zero_disparity() {
        assert!(distance_from_stereo_disparity(&intrinsics(1000.0), 0.06, 0.0, 0.0).is_err());
    }

    #[test]
    fn stereo_error_grows_with_distance_for_fixed_disparity_noise() {
        // Simulate two distances by using different disparities from the same baseline/focal.
        // Larger distance => smaller disparity => same absolute disparity noise causes
        // a larger relative distance error (quadratic growth).
        let near = distance_from_stereo_disparity(&intrinsics(1000.0), 0.06, 60.0, 0.5).unwrap(); // ~1m
        let far = distance_from_stereo_disparity(&intrinsics(1000.0), 0.06, 6.0, 0.5).unwrap(); // ~10m
        let near_relative_error = near.std_dev / near.value;
        let far_relative_error = far.std_dev / far.value;
        assert!(far_relative_error > near_relative_error);
    }

    #[test]
    fn depth_sensor_is_passthrough() {
        let m = distance_from_depth_sensor(1.234, 0.01).unwrap();
        assert!((m.value - 1.234).abs() < 1e-9);
        assert!((m.std_dev - 0.01).abs() < 1e-9);
    }

    #[test]
    fn depth_sensor_rejects_nonpositive_reading() {
        assert!(distance_from_depth_sensor(0.0, 0.01).is_err());
        assert!(distance_from_depth_sensor(-1.0, 0.01).is_err());
    }
}
