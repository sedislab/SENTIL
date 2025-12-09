//! Open-loop trajectory synthesis: find an input sequence that satisfies a spec.

#[cfg(not(feature = "std"))]
use crate::prelude::*;
use super::cmaes::{cma_es, CmaConfig};
#[cfg(feature = "std")]
use super::milp::solve_milp;
use super::model::{Bounds, SystemModel};
use super::pgrad::maximize;
use super::smooth::SmoothConfig;
#[cfg(any(feature = "std", test))]
use crate::error::Error;
use crate::error::Result;
use crate::formula::Formula;

#[cfg(feature = "std")]
const MILP_MAX_NODES: usize = 200_000;

/// The search backend a synthesis problem uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Backend {
    /// MILP for an affine model with a finite input box and an encodable STL spec,
    /// otherwise the gradient search.
    #[default]
    Auto,
    /// Projected gradient ascent on the smooth robustness.
    Gradient,
    /// CMA-ES, a gradient-free search for a rugged landscape.
    CmaEs,
    /// A big-M mixed-integer encoding of robustness solved by branch and bound, for
    /// a model that exposes [`affine_form`](SystemModel::affine_form).
    Milp,
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
    #[cfg(feature = "synthesis-gpu")]
    on_gpu: bool,
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
            #[cfg(feature = "synthesis-gpu")]
            on_gpu: false,
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

    /// Scores the CMA-ES population's smooth robustness on the GPU, falling back to
    /// the CPU when no device is present or the problem is ineligible.
    #[cfg(feature = "synthesis-gpu")]
    #[must_use]
    pub fn on_gpu(mut self, enable: bool) -> Self {
        self.on_gpu = enable;
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
    pub holds: bool,
    /// The backend that produced this result, with `Auto` resolved to its choice.
    pub backend: Backend,
}

/// Solves [`SynthesisProblem`]s.
pub struct Synthesizer;

impl Synthesizer {
    /// Finds the best input within the problem's budget, scored by the exact
    /// robustness. An infeasible spec gives a minimally violating input.
    ///
    /// # Errors
    ///
    /// Propagates a specification error surfaced while scoring, and returns
    /// [`Error::Unsupported`] when [`Backend::Milp`] cannot encode the model or spec.
    pub fn solve<M: SystemModel>(problem: &SynthesisProblem<'_, M>) -> Result<SynthesisResult> {
        let model = problem.model;
        let spec = problem.spec;
        let initial = model.initial_state();
        let start = vec![0.0; model.input_dimension()];
        let backend = if problem.backend == Backend::Auto {
            #[cfg(feature = "std")]
            let chosen = {
                let finite_box = problem.bounds.lower().iter().all(|b| b.is_finite())
                    && problem.bounds.upper().iter().all(|b| b.is_finite());
                if finite_box && model.affine_form().is_some() && super::milp::supports(spec) {
                    Backend::Milp
                } else {
                    Backend::Gradient
                }
            };
            #[cfg(not(feature = "std"))]
            let chosen = Backend::Gradient;
            chosen
        } else {
            problem.backend
        };
        let (input, backend) = match backend {
            #[cfg(feature = "std")]
            Backend::Milp => {
                let affine = model.affine_form().ok_or(Error::Unsupported {
                    feature: "the MILP backend needs an affine model; use Gradient or CmaEs",
                })?;
                (
                    solve_milp(&affine, spec, &problem.bounds, MILP_MAX_NODES)?,
                    Backend::Milp,
                )
            }
            Backend::CmaEs => {
                let config = CmaConfig {
                    max_generations: problem.max_iters,
                    population: problem.population,
                    ..CmaConfig::default()
                };
                (cma_es_input(problem, &start, config)?, Backend::CmaEs)
            }
            _ => {
                let objective = |u: &[f64]| spec.smooth_gradient(model, initial, u, problem.smooth);
                (
                    maximize(objective, &start, &problem.bounds, problem.max_iters)?.0,
                    Backend::Gradient,
                )
            }
        };
        let robustness = spec.robustness(&model.rollout_from(initial, &input)?)?;
        Ok(SynthesisResult {
            holds: robustness >= 0.0,
            robustness,
            input,
            backend,
        })
    }
}

fn cma_es_input<M: SystemModel>(
    problem: &SynthesisProblem<'_, M>,
    start: &[f64],
    config: CmaConfig,
) -> Result<Vec<f64>> {
    let model = problem.model;
    let spec = problem.spec;
    #[cfg(feature = "synthesis-gpu")]
    if problem.on_gpu {
        if let Some(result) = super::synth_gpu::solve_cmaes_gpu(
            model,
            spec,
            problem.smooth,
            &problem.bounds,
            start,
            config,
        ) {
            return result;
        }
    }
    let initial = model.initial_state();
    let objective =
        |u: &[f64]| spec.smooth_robustness(&model.rollout_from(initial, u)?, problem.smooth);
    cma_es(objective, start, &problem.bounds, config).map(|(input, _)| input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signal::Trace;
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
        assert!(result.holds, "robustness {}", result.robustness);
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
        assert!(result.holds, "robustness {}", result.robustness);
        assert_eq!(result.backend, Backend::CmaEs);
    }

    #[test]
    fn auto_picks_milp_for_a_finite_box_affine_spec() {
        let model = integrator(5);
        let spec = Formula::parse("eventually[0, 5](pos > 2)").unwrap();
        let problem = SynthesisProblem::new(&model, &spec)
            .with_bounds(Bounds::new(vec![-1.0; 5], vec![1.0; 5]).unwrap());
        let result = Synthesizer::solve(&problem).unwrap();
        assert_eq!(result.backend, Backend::Milp);
        assert!(result.holds);
    }

    #[test]
    fn auto_falls_back_to_gradient_without_a_finite_box() {
        let model = integrator(5);
        let spec = Formula::parse("eventually[0, 5](pos > 2)").unwrap();
        let problem = SynthesisProblem::new(&model, &spec);
        let result = Synthesizer::solve(&problem).unwrap();
        assert_eq!(result.backend, Backend::Gradient);
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
        assert!(result.holds, "robustness {}", result.robustness);
    }

    #[test]
    fn an_infeasible_spec_returns_a_minimally_violating_input() {
        let model = integrator(5);
        let spec = Formula::parse("eventually[0, 5](pos > 10)").unwrap();
        let problem = SynthesisProblem::new(&model, &spec)
            .with_bounds(Bounds::new(vec![-1.0; 5], vec![1.0; 5]).unwrap())
            .with_budget(400);
        let result = Synthesizer::solve(&problem).unwrap();
        assert!(!result.holds);
        assert!(result.robustness < 0.0 && result.robustness > -6.0);
    }

    #[test]
    fn the_milp_backend_solves_the_reachable_spec_to_optimality() {
        let model = integrator(5);
        let spec = Formula::parse("eventually[0, 5](pos > 2)").unwrap();
        let problem = SynthesisProblem::new(&model, &spec)
            .with_bounds(Bounds::new(vec![-1.0; 5], vec![1.0; 5]).unwrap())
            .with_backend(Backend::Milp);
        let result = Synthesizer::solve(&problem).unwrap();
        assert!(result.holds, "robustness {}", result.robustness);
        assert_eq!(result.backend, Backend::Milp);
    }

    #[test]
    fn the_milp_backend_matches_or_beats_the_gradient_robustness() {
        let model = integrator(5);
        let spec = Formula::parse("eventually[0, 5](pos > 2) and always[0, 5](pos > -3)").unwrap();
        let bounds = || Bounds::new(vec![-1.0; 5], vec![1.0; 5]).unwrap();
        let gradient = Synthesizer::solve(
            &SynthesisProblem::new(&model, &spec)
                .with_bounds(bounds())
                .with_budget(800),
        )
        .unwrap();
        let milp = Synthesizer::solve(
            &SynthesisProblem::new(&model, &spec)
                .with_bounds(bounds())
                .with_backend(Backend::Milp),
        )
        .unwrap();
        assert!(milp.holds);
        assert!(
            milp.robustness >= gradient.robustness - 1e-6,
            "milp {} gradient {}",
            milp.robustness,
            gradient.robustness
        );
    }

    #[test]
    fn the_milp_backend_needs_an_affine_model() {
        struct Custom;
        impl SystemModel for Custom {
            fn input_dimension(&self) -> usize {
                1
            }
            fn initial_state(&self) -> &[f64] {
                &[0.0]
            }
            fn rollout_from(&self, _initial: &[f64], input: &[f64]) -> Result<Trace> {
                let mut trace = Trace::new([0.0, 1.0])?;
                trace.add_signal("pos", [0.0, input[0]])?;
                Ok(trace)
            }
        }
        let model = Custom;
        let spec = Formula::parse("eventually[0, 1](pos > 0)").unwrap();
        let problem = SynthesisProblem::new(&model, &spec).with_backend(Backend::Milp);
        assert!(matches!(
            Synthesizer::solve(&problem),
            Err(Error::Unsupported { .. })
        ));
    }
}