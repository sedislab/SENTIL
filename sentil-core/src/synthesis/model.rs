//! System models and input bounds for synthesis.

use crate::error::{Error, Result};

/// Box bounds on a packed input vector: a lower and upper limit per coordinate.
pub struct Bounds {
    lower: Vec<f64>,
    upper: Vec<f64>,
}

impl Bounds {
    /// Builds bounds from per-coordinate limits.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidConfig`] if the two limits differ in length or any
    /// lower limit exceeds its upper limit.
    pub fn new(lower: impl Into<Vec<f64>>, upper: impl Into<Vec<f64>>) -> Result<Self> {
        let (lower, upper) = (lower.into(), upper.into());
        if lower.len() != upper.len() {
            return Err(config_error(
                "lower and upper bounds must have the same length",
            ));
        }
        if lower.iter().zip(&upper).any(|(lo, hi)| lo > hi) {
            return Err(config_error(
                "every lower bound must be at most its upper bound",
            ));
        }
        Ok(Self { lower, upper })
    }

    /// Projects `point` into the box in place, clamping each coordinate.
    pub fn clamp(&self, point: &mut [f64]) {
        for ((value, &lo), &hi) in point.iter_mut().zip(&self.lower).zip(&self.upper) {
            *value = value.clamp(lo, hi);
        }
    }

    /// The number of coordinates the bounds constrain.
    #[must_use]
    pub fn dimension(&self) -> usize {
        self.lower.len()
    }
}

fn config_error(message: &str) -> Error {
    Error::InvalidConfig {
        context: "synthesis",
        message: message.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp, reason = "the clamped coordinates are exact")]

    use super::*;

    #[test]
    fn clamp_keeps_every_coordinate_in_range() {
        let bounds = Bounds::new([-1.0, 0.0], [1.0, 2.0]).unwrap();
        let mut point = [-5.0, 3.0];
        bounds.clamp(&mut point);
        assert_eq!(point, [-1.0, 2.0]);
        assert_eq!(bounds.dimension(), 2);
    }

    #[test]
    fn mismatched_or_inverted_bounds_are_rejected() {
        assert!(Bounds::new([0.0], [1.0, 2.0]).is_err());
        assert!(Bounds::new([1.0], [0.0]).is_err());
    }
}