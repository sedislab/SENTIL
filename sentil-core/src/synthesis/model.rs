//! System models and input bounds for synthesis.

use crate::error::{Error, Result};
use crate::signal::Trace;

/// A system whose input sequence rolls forward into a trace of named signals.
pub trait SystemModel {
    /// The length of the packed input vector the model expects.
    fn input_dimension(&self) -> usize;

    /// The model's default initial state.
    fn initial_state(&self) -> &[f64];

    /// Rolls the packed `input` forward from `initial` into the trace its signals
    /// are read from.
    ///
    /// # Errors
    ///
    /// Returns an error if `input` is not [`input_dimension`](Self::input_dimension)
    /// long, or if the trace cannot be built.
    fn rollout_from(&self, initial: &[f64], input: &[f64]) -> Result<Trace>;

    /// The model's affine structure, when it has one, for the MILP backend.
    fn affine_form(&self) -> Option<AffineForm> {
        None
    }
}

/// The affine dynamics `x_{t+1} = A x_t + B u_t` a model exposes for the MILP
/// backend.
#[derive(Debug, Clone, PartialEq)]
pub struct AffineForm {
    /// The square state matrix `A`, one row per state component.
    pub a: Vec<Vec<f64>>,
    /// The input matrix `B`, one row per state component, one column per input.
    pub b: Vec<Vec<f64>>,
    /// The initial state `x_0`.
    pub x0: Vec<f64>,
    /// The per-state signal names, matching the trace the model rolls out.
    pub variables: Vec<String>,
    /// The step spacing.
    pub dt: f64,
    /// The number of steps.
    pub horizon: usize,
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
    /// Returns [`Error::InvalidConfig`] if the two limits differ in length, any
    /// limit is `NaN`, or any lower limit exceeds its upper limit.
    pub fn new(lower: impl Into<Vec<f64>>, upper: impl Into<Vec<f64>>) -> Result<Self> {
        let (lower, upper) = (lower.into(), upper.into());
        if lower.len() != upper.len() {
            return Err(config_error(
                "lower and upper bounds must have the same length",
            ));
        }
        for (i, (&lo, &hi)) in lower.iter().zip(&upper).enumerate() {
            if lo.is_nan() || hi.is_nan() {
                return Err(config_error(&format!(
                    "bound at coordinate {i} is NaN (lower {lo}, upper {hi}); use a finite value or an infinity"
                )));
            }
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

    /// Bounds that constrain nothing, over `dimension` coordinates.
    #[must_use]
    pub fn unbounded(dimension: usize) -> Self {
        Self {
            lower: vec![f64::NEG_INFINITY; dimension],
            upper: vec![f64::INFINITY; dimension],
        }
    }

    /// The number of coordinates the bounds constrain.
    #[must_use]
    pub fn dimension(&self) -> usize {
        self.lower.len()
    }

    /// The per-coordinate lower limits.
    #[must_use]
    pub fn lower(&self) -> &[f64] {
        &self.lower
    }

    /// The per-coordinate upper limits.
    #[must_use]
    pub fn upper(&self) -> &[f64] {
        &self.upper
    }

    /// The box as `G u <= h` rows for a quadratic program, one row per finite side.
    pub(super) fn constraint_rows(&self) -> (Vec<Vec<f64>>, Vec<f64>) {
        let n = self.dimension();
        let mut g = Vec::new();
        let mut h = Vec::new();
        for (i, (&lo, &hi)) in self.lower.iter().zip(&self.upper).enumerate() {
            if hi.is_finite() {
                let mut row = vec![0.0; n];
                row[i] = 1.0;
                g.push(row);
                h.push(hi);
            }
            if lo.is_finite() {
                let mut row = vec![0.0; n];
                row[i] = -1.0;
                g.push(row);
                h.push(-lo);
            }
        }
        (g, h)
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
        if n == 0 {
            return Err(config_error(
                "the initial state must have at least one component",
            ));
        }
        if a.len() != n || a.iter().any(|row| row.len() != n) {
            return Err(config_error(&format!(
                "A must be {n}x{n} to match the {n}-component state, but it is {}x{}",
                a.len(),
                a.first().map_or(0, Vec::len)
            )));
        }
        let width = b.first().map_or(0, Vec::len);
        if b.len() != n || b.iter().any(|row| row.len() != width) {
            return Err(config_error(&format!(
                "B must have {n} rows, one per state, each the same width, but it has {} rows",
                b.len()
            )));
        }
        if variables.len() != n {
            return Err(config_error(&format!(
                "there must be one variable name per state component: {n} expected, {} given",
                variables.len()
            )));
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

    fn initial_state(&self) -> &[f64] {
        &self.x0
    }

    fn affine_form(&self) -> Option<AffineForm> {
        Some(AffineForm {
            a: self.a.clone(),
            b: self.b.clone(),
            x0: self.x0.clone(),
            variables: self.variables.clone(),
            dt: self.dt,
            horizon: self.horizon,
        })
    }

    fn rollout_from(&self, initial: &[f64], input: &[f64]) -> Result<Trace> {
        if input.len() != self.input_dimension() {
            return Err(Error::PackedLength {
                expected: self.input_dimension(),
                found: input.len(),
            });
        }
        let width = self.b.first().map_or(0, Vec::len);
        let mut state = initial.to_vec();
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

    #[test]
    fn nan_bounds_are_rejected_so_clamp_cannot_panic() {
        assert!(Bounds::new([f64::NAN], [1.0]).is_err());
        assert!(Bounds::new([0.0], [f64::NAN]).is_err());
        assert!(Bounds::new([f64::NEG_INFINITY], [f64::INFINITY]).is_ok());
    }

    #[test]
    fn integrator_rolls_input_to_the_expected_trace() {
        let model =
            LinearModel::new(vec![vec![1.0]], vec![vec![1.0]], [0.0], ["pos"], 1.0, 3).unwrap();
        assert_eq!(model.input_dimension(), 3);
        assert_eq!(model.initial_state(), &[0.0]);
        let trace = model.rollout_from(&[0.0], &[1.0, 1.0, 1.0]).unwrap();
        assert_eq!(trace.times(), &[0.0, 1.0, 2.0, 3.0]);
        assert_eq!(trace.signals()["pos"], vec![0.0, 1.0, 2.0, 3.0]);
        assert!(model.rollout_from(&[0.0], &[1.0]).is_err());
    }

    #[test]
    fn a_shape_mismatch_names_the_dimensions() {
        let bad = LinearModel::new(vec![vec![1.0, 0.0]], vec![vec![1.0]], [0.0], ["x"], 1.0, 2);
        let Err(Error::InvalidConfig { message, .. }) = bad else {
            panic!("expected an invalid-config error");
        };
        assert!(message.contains("1x1"), "message should name the shape: {message}");
    }
}