//! Diameter / real-world size estimation.
//!
//! Uses the same pinhole relation as distance, solved the other direction:
//!
//! ```text
//! real_diameter = (pixel_width * distance) / focal_length_px
//! ```
//!
//! IMPORTANT ERROR-PROPAGATION NOTE: diameter accuracy has a hard ceiling
//! set by distance accuracy, since any relative error in distance carries
//! through proportionally into diameter. This module always requires a
//! `Measurement` (value + std_dev) for distance, specifically so that
//! callers can't accidentally treat diameter as more precise than the
//! distance it was derived from.

use crate::distance::MeasurementError;
use crate::uncertainty::Measurement;

/// Estimate real-world diameter from a measured pixel width, a distance
/// measurement (with its own uncertainty), and camera focal length.
///
/// `pixel_width_std_dev` is the 1-sigma noise in the pixel-width
/// measurement of the object itself (boundary/segmentation jitter),
/// independent of whatever noise fed the distance estimate.
///
/// NOTE ON BBOX VS. TRUE WIDTH: if `pixel_width` comes from an axis-aligned
/// bounding box rather than a segmentation mask or fitted contour, this
/// will systematically overestimate diameter for objects that are rotated
/// relative to the camera plane, or underestimate for elongated objects
/// photographed off-axis (foreshortening). That is a systematic bias, not
/// captured by `pixel_width_std_dev`, and must be corrected upstream (e.g.
/// via an orientation/ellipse fit) before calling this function if accuracy
/// matters. See docs/ARCHITECTURE.md.
pub fn diameter_from_distance(
    focal_length_px: f64,
    distance: &Measurement,
    pixel_width: f64,
    pixel_width_std_dev: f64,
) -> Result<Measurement, MeasurementError> {
    if focal_length_px <= 0.0 {
        return Err(MeasurementError::InvalidInput(
            "focal_length_px must be positive".into(),
        ));
    }
    if pixel_width <= 0.0 {
        return Err(MeasurementError::InvalidInput(
            "pixel_width must be positive".into(),
        ));
    }
    if distance.value <= 0.0 {
        return Err(MeasurementError::InvalidInput(
            "distance.value must be positive".into(),
        ));
    }

    let value = (pixel_width * distance.value) / focal_length_px;

    // real_diameter = f(pixel_width, distance) = pixel_width * distance / focal_length
    // This is linear in both inputs, so we propagate uncertainty from BOTH
    // independent sources in quadrature (assuming pixel-width noise and
    // distance noise are uncorrelated, which holds when they come from
    // different upstream measurements):
    //
    //   d(diameter)/d(pixel_width) = distance / focal_length
    //   d(diameter)/d(distance)    = pixel_width / focal_length
    //
    //   std_dev_diameter = sqrt( (d/d(pixel_width) * std_dev_pixel_width)^2
    //                           + (d/d(distance)    * std_dev_distance)^2 )
    let d_wrt_pixel = distance.value / focal_length_px;
    let d_wrt_distance = pixel_width / focal_length_px;

    let term_pixel = d_wrt_pixel * pixel_width_std_dev;
    let term_distance = d_wrt_distance * distance.std_dev;

    let std_dev = (term_pixel * term_pixel + term_distance * term_distance).sqrt();

    Ok(Measurement { value, std_dev })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_hand_calculation() {
        // pixel_width=150px, distance=2.0m, focal=1000px
        // diameter = (150 * 2.0) / 1000 = 0.3 m
        let distance = Measurement {
            value: 2.0,
            std_dev: 0.0,
        };
        let m = diameter_from_distance(1000.0, &distance, 150.0, 0.0).unwrap();
        assert!((m.value - 0.3).abs() < 1e-9);
    }

    #[test]
    fn rejects_invalid_inputs() {
        let distance = Measurement {
            value: 2.0,
            std_dev: 0.0,
        };
        assert!(diameter_from_distance(0.0, &distance, 150.0, 0.0).is_err());
        assert!(diameter_from_distance(1000.0, &distance, 0.0, 0.0).is_err());
        let bad_distance = Measurement {
            value: -1.0,
            std_dev: 0.0,
        };
        assert!(diameter_from_distance(1000.0, &bad_distance, 150.0, 0.0).is_err());
    }

    #[test]
    fn diameter_error_inherits_distance_error() {
        // Same pixel measurement, but distance uncertainty grows =>
        // diameter uncertainty must grow too (this is the "ceiling" property).
        let precise_distance = Measurement {
            value: 2.0,
            std_dev: 0.01,
        };
        let noisy_distance = Measurement {
            value: 2.0,
            std_dev: 0.5,
        };
        let m_precise = diameter_from_distance(1000.0, &precise_distance, 150.0, 1.0).unwrap();
        let m_noisy = diameter_from_distance(1000.0, &noisy_distance, 150.0, 1.0).unwrap();
        assert!(m_noisy.std_dev > m_precise.std_dev);
    }

    #[test]
    fn zero_noise_inputs_give_zero_output_noise() {
        let distance = Measurement {
            value: 2.0,
            std_dev: 0.0,
        };
        let m = diameter_from_distance(1000.0, &distance, 150.0, 0.0).unwrap();
        assert_eq!(m.std_dev, 0.0);
    }
}
