//! System models and input bounds for synthesis.

use crate::error::{Error, Result};
use crate::signal::Trace;

/// A system whose input sequence rolls forward into a trace of named signals.
pub trait SystemModel {
    /// The length of the packed input vector the model expects.
    fn input_dimension(&self) -> usize;

    /// Rolls the packed `input` into the trace its signals are read from.
    ///
    /// # Errors
    ///
    /// Returns an error if `input` is not [`input_dimension`](Self::input_dimension)
    /// long, or if the trace cannot be built.
    fn rollout(&self, input: &[f64]) -> Result<Trace>;
}

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

/// A discrete linear time-invariant model `x_{t+1} = A x_t + B u_t`, emitting each
/// state component as a named signal.
pub struct LinearModel {
    a: Vec<Vec<f64>>,
    b: Vec<Vec<f64>>,
    x0: Vec<f64>,
    variables: Vec<String>,
    dt: f64,
    horizon: usize,
}

impl LinearModel {
    /// Builds the model from the state matrix `a`, the input matrix `b`, the
    /// initial state `x0`, the per-state signal `variables`, the step spacing `dt`,
    /// and the step count `horizon`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidConfig`] on any shape mismatch, a non-positive `dt`,
    /// or a zero `horizon`.
    pub fn new(
        a: Vec<Vec<f64>>,
        b: Vec<Vec<f64>>,
        x0: impl Into<Vec<f64>>,
        variables: impl IntoIterator<Item = impl Into<String>>,
        dt: f64,
        horizon: usize,
    ) -> Result<Self> {
        let x0 = x0.into();
        let variables: Vec<String> = variables.into_iter().map(Into::into).collect();
        let n = x0.len();
        if n == 0 || a.len() != n || a.iter().any(|row| row.len() != n) {
            return Err(config_error("A must be square and match the initial state"));
        }
        let width = b.first().map_or(0, Vec::len);
        if b.len() != n || b.iter().any(|row| row.len() != width) {
            return Err(config_error(
                "B must have one row per state with a uniform width",
            ));
        }
        if variables.len() != n {
            return Err(config_error(
                "there must be one variable name per state component",
            ));
        }
        if !(dt.is_finite() && dt > 0.0) {
            return Err(config_error("dt must be finite and positive"));
        }
        if horizon == 0 {
            return Err(config_error("horizon must be positive"));
        }
        Ok(Self {
            a,
            b,
            x0,
            variables,
            dt,
            horizon,
        })
    }

    fn advance(&self, state: &[f64], input: &[f64]) -> Vec<f64> {
        self.a
            .iter()
            .zip(&self.b)
            .map(|(arow, brow)| {
                let drift: f64 = arow.iter().zip(state).map(|(c, s)| c * s).sum();
                let control: f64 = brow.iter().zip(input).map(|(c, u)| c * u).sum();
                drift + control
            })
            .collect()
    }
}

impl SystemModel for LinearModel {
    fn input_dimension(&self) -> usize {
        self.horizon * self.b.first().map_or(0, Vec::len)
    }

    fn rollout(&self, input: &[f64]) -> Result<Trace> {
        if input.len() != self.input_dimension() {
            return Err(Error::PackedLength {
                expected: self.input_dimension(),
                found: input.len(),
            });
        }
        let width = self.b.first().map_or(0, Vec::len);
        let mut state = self.x0.clone();
        let mut columns: Vec<Vec<f64>> = (0..state.len())
            .map(|_| Vec::with_capacity(self.horizon + 1))
            .collect();
        let mut times = Vec::with_capacity(self.horizon + 1);
        let mut time = 0.0;
        for step in 0..=self.horizon {
            for (col, &s) in columns.iter_mut().zip(&state) {
                col.push(s);
            }
            times.push(time);
            time += self.dt;
            if step < self.horizon {
                state = self.advance(&state, &input[step * width..(step + 1) * width]);
            }
        }
        let mut trace = Trace::new(times)?;
        for (name, column) in self.variables.iter().zip(columns) {
            trace.add_signal(name, column)?;
        }
        Ok(trace)
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