//! Open-loop trajectory synthesis: find an input sequence that satisfies a spec.

use super::model::{Bounds, SystemModel};
use super::pgrad::maximize;
use super::smooth::SmoothConfig;
use crate::error::Result;
use crate::formula::Formula;

/// An open-loop search for an input sequence that makes `spec` hold on `model`.
pub struct SynthesisProblem<'a, M: SystemModel> {
    model: &'a M,
    spec: &'a Formula,
    bounds: Bounds,
    smooth: SmoothConfig,
    max_iters: usize,
}

impl<'a, M: SystemModel> SynthesisProblem<'a, M> {
    /// Builds a problem with no input bounds and the default temperature and budget.
    #[must_use]
    pub fn new(model: &'a M, spec: &'a Formula) -> Self {
        let bounds = Bounds::unbounded(model.input_dimension());
        Self {
            model,
            spec,
            bounds,
            smooth: SmoothConfig::default(),
            max_iters: 200,
        }
    }

    /// Constrains the input to a box.
    #[must_use]
    pub fn with_bounds(mut self, bounds: Bounds) -> Self {
        self.bounds = bounds;
        self
    }

    /// Sets the smoothing temperature used during the search.
    #[must_use]
    pub fn with_smooth(mut self, smooth: SmoothConfig) -> Self {
        self.smooth = smooth;
        self
    }

    /// Sets the maximum number of ascent steps.
    #[must_use]
    pub fn with_budget(mut self, max_iters: usize) -> Self {
        self.max_iters = max_iters;
        self
    }
}

/// The outcome of open-loop synthesis.
#[derive(Debug, Clone, PartialEq)]
pub struct SynthesisResult {
    /// The packed input sequence found, inside the bounds.
    pub input: Vec<f64>,
    /// The exact robustness of the rollout under `input`.
    pub robustness: f64,
    /// Whether the specification holds.
    pub satisfies: bool,
}

/// Solves [`SynthesisProblem`]s.
pub struct Synthesizer;

impl Synthesizer {
    /// Finds the best input within the problem's budget by climbing the smooth
    /// robustness, then scores it with the exact robustness. An infeasible spec
    /// gives a minimally violating input, never an error or nothing.
    ///
    /// # Errors
    ///
    /// Propagates a specification error (an unknown variable, or a bare
    /// probabilistic operator) surfaced while scoring.
    pub fn solve<M: SystemModel>(problem: &SynthesisProblem<'_, M>) -> Result<SynthesisResult> {
        let model = problem.model;
        let spec = problem.spec;
        let start = vec![0.0; model.input_dimension()];
        let objective = |u: &[f64]| spec.smooth_gradient(model, u, problem.smooth);
        let (input, _) = maximize(objective, &start, &problem.bounds, problem.max_iters)?;
        let robustness = spec.robustness(&model.rollout(&input)?)?;
        Ok(SynthesisResult {
            satisfies: robustness >= 0.0,
            robustness,
            input,
        })
    }
}