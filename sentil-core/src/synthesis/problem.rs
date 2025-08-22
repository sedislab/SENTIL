//! Open-loop trajectory synthesis: find an input sequence that satisfies a spec.

use super::cmaes::{cma_es, CmaConfig};
use super::model::{Bounds, SystemModel};
use super::pgrad::maximize;
use super::smooth::SmoothConfig;
use crate::error::Result;
use crate::formula::Formula;

/// The search backend a synthesis problem uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Backend {
    /// Choose automatically; currently the gradient search.
    #[default]
    Auto,
    /// Projected gradient ascent on the smooth robustness.
    Gradient,
    /// CMA-ES, a gradient-free search for a rugged landscape.
    CmaEs,
}

/// An open-loop search for an input sequence that makes `spec` hold on `model`.
pub struct SynthesisProblem<'a, M: SystemModel> {
    model: &'a M,
    spec: &'a Formula,
    bounds: Bounds,
    smooth: SmoothConfig,
    max_iters: usize,
    backend: Backend,
    population: usize,
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
            backend: Backend::Auto,
            population: 0,
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

    /// Sets the maximum number of search steps.
    #[must_use]
    pub fn with_budget(mut self, max_iters: usize) -> Self {
        self.max_iters = max_iters;
        self
    }

    /// Selects the search backend.
    #[must_use]
    pub fn with_backend(mut self, backend: Backend) -> Self {
        self.backend = backend;
        self
    }

    /// Sets the CMA-ES population per generation; `0` keeps the default.
    #[must_use]
    pub fn with_population(mut self, population: usize) -> Self {
        self.population = population;
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
    /// The backend that produced this result, with `Auto` resolved to its choice.
    pub backend: Backend,
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
        let initial = model.initial_state();
        let start = vec![0.0; model.input_dimension()];
        let (input, backend) = if problem.backend == Backend::CmaEs {
            let objective = |u: &[f64]| {
                spec.smooth_robustness(&model.rollout_from(initial, u)?, problem.smooth)
            };
            let config = CmaConfig {
                max_generations: problem.max_iters,
                population: problem.population,
                ..CmaConfig::default()
            };
            (
                cma_es(objective, &start, &problem.bounds, config)?.0,
                Backend::CmaEs,
            )
        } else {
            let objective = |u: &[f64]| spec.smooth_gradient(model, initial, u, problem.smooth);
            (
                maximize(objective, &start, &problem.bounds, problem.max_iters)?.0,
                Backend::Gradient,
            )
        };
        let robustness = spec.robustness(&model.rollout_from(initial, &input)?)?;
        Ok(SynthesisResult {
            satisfies: robustness >= 0.0,
            robustness,
            input,
            backend,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synthesis::LinearModel;

    fn integrator(horizon: usize) -> LinearModel {
        LinearModel::new(
            vec![vec![1.0]],
            vec![vec![1.0]],
            [0.0],
            ["pos"],
            1.0,
            horizon,
        )
        .unwrap()
    }

    #[test]
    fn synthesizes_an_input_that_satisfies_a_reachable_spec() {
        let model = integrator(5);
        let spec = Formula::parse("eventually[0, 5](pos > 2)").unwrap();
        let problem = SynthesisProblem::new(&model, &spec)
            .with_bounds(Bounds::new(vec![-1.0; 5], vec![1.0; 5]).unwrap())
            .with_budget(400);
        let result = Synthesizer::solve(&problem).unwrap();
        assert!(result.satisfies, "robustness {}", result.robustness);
        assert!(result.robustness >= 0.0);
    }

    #[test]
    fn the_cma_es_backend_also_satisfies_the_spec() {
        let model = integrator(5);
        let spec = Formula::parse("eventually[0, 5](pos > 2)").unwrap();
        let problem = SynthesisProblem::new(&model, &spec)
            .with_bounds(Bounds::new(vec![-1.0; 5], vec![1.0; 5]).unwrap())
            .with_backend(Backend::CmaEs);
        let result = Synthesizer::solve(&problem).unwrap();
        assert!(result.satisfies, "robustness {}", result.robustness);
        assert_eq!(result.backend, Backend::CmaEs);
    }

    #[test]
    fn an_explicit_population_still_satisfies_the_spec() {
        let model = integrator(5);
        let spec = Formula::parse("eventually[0, 5](pos > 2)").unwrap();
        let problem = SynthesisProblem::new(&model, &spec)
            .with_bounds(Bounds::new(vec![-1.0; 5], vec![1.0; 5]).unwrap())
            .with_backend(Backend::CmaEs)
            .with_population(32);
        let result = Synthesizer::solve(&problem).unwrap();
        assert!(result.satisfies, "robustness {}", result.robustness);
    }

    #[test]
    fn an_infeasible_spec_returns_a_minimally_violating_input() {
        let model = integrator(5);
        let spec = Formula::parse("eventually[0, 5](pos > 10)").unwrap();
        let problem = SynthesisProblem::new(&model, &spec)
            .with_bounds(Bounds::new(vec![-1.0; 5], vec![1.0; 5]).unwrap())
            .with_budget(400);
        let result = Synthesizer::solve(&problem).unwrap();
        assert!(!result.satisfies);
        assert!(result.robustness < 0.0 && result.robustness > -6.0);
    }
}