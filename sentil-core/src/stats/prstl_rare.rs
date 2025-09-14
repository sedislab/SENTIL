//! Rare-event PrSTL by adaptive multilevel splitting.

use std::collections::BTreeSet;

use rand::RngCore;

use super::{adaptive_multilevel_splitting, RareEventSimulator};
use crate::error::{Error, Result};
use crate::formula::Formula;
use crate::signal::Trace;

/// Draws an initial packed sample.
type InitFn = Box<dyn Fn(&mut dyn RngCore) -> Vec<f64>>;
type StepFn = Box<dyn Fn(&[f64], f64, &mut dyn RngCore) -> Vec<f64>>;

/// A user-defined stochastic system the splitter can drive.
pub struct StochasticSystem {
    variables: Vec<String>,
    dt: f64,
    horizon: usize,
    init: InitFn,
    step: StepFn,
}

impl StochasticSystem {
    /// Builds a system.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidConfig`] if `dt` is not finite and positive, `horizon` is zero, or `variables` is empty or holds a duplicate.
    pub fn new(
        variables: impl IntoIterator<Item = impl Into<String>>,
        dt: f64,
        horizon: usize,
        init: impl Fn(&mut dyn RngCore) -> Vec<f64> + 'static,
        step: impl Fn(&[f64], f64, &mut dyn RngCore) -> Vec<f64> + 'static,
    ) -> Result<Self> {
        let variables: Vec<String> = variables.into_iter().map(Into::into).collect();
        if !(dt.is_finite() && dt > 0.0) {
            return Err(config_error(format!(
                "dt must be finite and positive, got {dt}"
            )));
        }
        if horizon == 0 {
            return Err(config_error("horizon must be positive".to_owned()));
        }
        if variables.is_empty() {
            return Err(config_error("at least one variable is required".to_owned()));
        }
        let mut seen = BTreeSet::new();
        if let Some(dup) = variables.iter().find(|name| !seen.insert((*name).clone())) {
            return Err(config_error(format!(
                "variable names must be unique, but `{dup}` is repeated"
            )));
        }
        Ok(Self {
            variables,
            dt,
            horizon,
            init: Box::new(init),
            step: Box::new(step),
        })
    }

    /// The packed signal order both closures emit.
    #[must_use]
    pub fn variables(&self) -> &[String] {
        &self.variables
    }

    /// The spacing between successive steps.
    #[must_use]
    pub fn dt(&self) -> f64 {
        self.dt
    }

    /// The trajectory length, in steps.
    #[must_use]
    pub fn horizon(&self) -> usize {
        self.horizon
    }

    /// Draws an initial packed sample from the system.
    pub fn initial(&self, rng: &mut dyn RngCore) -> Vec<f64> {
        (self.init)(rng)
    }

    /// Advances the system one step from `previous` at `time`.
    pub fn advance(&self, previous: &[f64], time: f64, rng: &mut dyn RngCore) -> Vec<f64> {
        (self.step)(previous, time, rng)
    }

    /// Simulates one full-horizon trajectory into a trace.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidConfig`] if the initial draw does not emit one value per variable, or if the samples cannot form a trace.
    pub fn simulate(&self, rng: &mut dyn RngCore) -> Result<Trace> {
        let mut state = self.initial(rng);
        if state.len() != self.variables.len() {
            return Err(config_error(format!(
                "the system emitted {} values but has {} variables",
                state.len(),
                self.variables.len()
            )));
        }
        let mut columns: Vec<Vec<f64>> = (0..self.variables.len())
            .map(|_| Vec::with_capacity(self.horizon + 1))
            .collect();
        let mut times = Vec::with_capacity(self.horizon + 1);
        let mut time = 0.0;
        for step in 0..=self.horizon {
            for (col, value) in columns.iter_mut().zip(&state) {
                col.push(*value);
            }
            times.push(time);
            if step < self.horizon {
                state = self.advance(&state, time, rng);
                time += self.dt;
            }
        }
        let mut trace = Trace::new(times)?;
        for (name, column) in self.variables.iter().zip(columns) {
            trace.add_signal(name, column)?;
        }
        Ok(trace)
    }
}

/// Tuning for a rare-event run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RareEventConfig {
    /// The particle population.
    pub particles: usize,
    /// The violation margin; the rare event is robustness at or below `-margin`.
    pub margin: f64,
    /// The seed.
    pub seed: u64,
}

impl Default for RareEventConfig {
    fn default() -> Self {
        Self {
            particles: 4096,
            margin: 0.0,
            seed: 42,
        }
    }
}

/// The outcome of a rare-event run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RareEventResult {
    /// The estimated satisfaction probability, `1 - violation_probability`.
    pub probability: f64,
    /// The raw splitting estimate of the tail violation probability.
    pub violation_probability: f64,
    /// Whether the satisfaction probability meets the operator's threshold.
    pub holds: bool,
    /// The total simulation steps run.
    pub simulations: u64,
}

#[derive(Clone)]
struct WalkState {
    samples: Vec<Vec<f64>>,
    last_value: Vec<f64>,
    step_index: usize,
    time: f64,
    score: f64,
}

#[derive(Clone, Copy)]
enum ScoreMode<'a> {
    Always(&'a Formula),
    Eventually(&'a Formula),
    Full,
}

fn classify(inner: &Formula) -> ScoreMode<'_> {
    match inner {
        Formula::Always(iv, body) if iv.is_unbounded() && is_atemporal(body) => {
            ScoreMode::Always(body)
        }
        Formula::Eventually(iv, body) if iv.is_unbounded() && is_atemporal(body) => {
            ScoreMode::Eventually(body)
        }
        _ => ScoreMode::Full,
    }
}

fn is_atemporal(f: &Formula) -> bool {
    match f {
        Formula::Predicate(_) => true,
        Formula::Not(a) => is_atemporal(a),
        Formula::And(a, b) | Formula::Or(a, b) | Formula::Implies(a, b) => {
            is_atemporal(a) && is_atemporal(b)
        }
        _ => false,
    }
}

struct PrstlWalk<'a> {
    inner: &'a Formula,
    system: &'a StochasticSystem,
    margin: f64,
    mode: ScoreMode<'a>,
}

impl RareEventSimulator for PrstlWalk<'_> {
    type State = WalkState;

    fn initial_state(&self, rng: &mut dyn RngCore) -> WalkState {
        let v0 = self.system.initial(rng);
        let (samples, score) = match self.mode {
            ScoreMode::Full => {
                let samples = vec![v0.clone()];
                let score = self.compute_score(&samples);
                (samples, score)
            }
            ScoreMode::Always(psi) | ScoreMode::Eventually(psi) => {
                (Vec::new(), self.point_score(psi, &v0))
            }
        };
        WalkState {
            samples,
            last_value: v0,
            step_index: 0,
            time: 0.0,
            score,
        }
    }

    fn step(&self, state: &WalkState, rng: &mut dyn RngCore) -> WalkState {
        if state.step_index >= self.system.horizon() {
            return state.clone();
        }
        let next = self.system.advance(&state.last_value, state.time, rng);
        let (samples, score) = match self.mode {
            ScoreMode::Full => {
                let mut samples = state.samples.clone();
                samples.push(next.clone());
                let score = self.compute_score(&samples);
                (samples, score)
            }
            ScoreMode::Always(psi) => (
                Vec::new(),
                fold_extremum(true, state.score, self.point_violation(psi, &next)),
            ),
            ScoreMode::Eventually(psi) => (
                Vec::new(),
                fold_extremum(false, state.score, self.point_violation(psi, &next)),
            ),
        };
        WalkState {
            samples,
            last_value: next,
            step_index: state.step_index + 1,
            time: state.time + self.system.dt(),
            score,
        }
    }

    fn is_terminal(&self, state: &WalkState) -> (bool, bool) {
        let violated = state.score >= self.margin;
        (violated, violated)
    }

    fn score(&self, state: &WalkState) -> f64 {
        state.score
    }
}

impl PrstlWalk<'_> {
    fn compute_score(&self, samples: &[Vec<f64>]) -> f64 {
        let vars = self.system.variables();
        let mut times = Vec::with_capacity(samples.len());
        let mut t = 0.0;
        for _ in samples {
            times.push(t);
            t += self.system.dt();
        }
        let Ok(mut trace) = Trace::new(times) else {
            return f64::NAN;
        };
        for (i, var) in vars.iter().enumerate() {
            let column: Vec<f64> = samples
                .iter()
                .map(|s| s.get(i).copied().unwrap_or(f64::NAN))
                .collect();
            if trace.add_signal(var, column).is_err() {
                return f64::NAN;
            }
        }
        match self.inner.robustness(&trace) {
            Ok(rho) => (-rho).clamp(-1e12, 1e12),
            Err(_) => f64::NAN,
        }
    }

    fn point_violation(&self, psi: &Formula, values: &[f64]) -> f64 {
        let Ok(mut trace) = Trace::new(vec![0.0]) else {
            return f64::NAN;
        };
        for (i, var) in self.system.variables().iter().enumerate() {
            let value = values.get(i).copied().unwrap_or(f64::NAN);
            if trace.add_signal(var, vec![value]).is_err() {
                return f64::NAN;
            }
        }
        match psi.robustness(&trace) {
            Ok(rho) => -rho,
            Err(_) => f64::NAN,
        }
    }

    fn point_score(&self, psi: &Formula, values: &[f64]) -> f64 {
        let v = self.point_violation(psi, values);
        if v.is_nan() {
            v
        } else {
            v.clamp(-1e12, 1e12)
        }
    }
}

/// Folds a per-sample violation into the running extremum, max for always, min for
/// eventually. NaN propagates; the clamp matches the full recompute's single clamp.
fn fold_extremum(is_always: bool, running: f64, term: f64) -> f64 {
    if running.is_nan() || term.is_nan() {
        return f64::NAN;
    }
    let combined = if is_always {
        running.max(term)
    } else {
        running.min(term)
    };
    combined.clamp(-1e12, 1e12)
}

impl Formula {
    /// Estimates `P~p(phi)` over a user-defined stochastic `system` by adaptive multilevel splitting.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotProbabilistic`] unless the formula is `P~p(phi)`, [`Error::InvalidConfig`] if the inner formula is not safety shaped, and any [`Error::Splitting`] from the run.
    pub fn check_rare_event(
        &self,
        system: &StochasticSystem,
        config: &RareEventConfig,
    ) -> Result<RareEventResult> {
        let Formula::Probabilistic(op, threshold, inner) = self else {
            return Err(Error::NotProbabilistic);
        };
        let walk = PrstlWalk {
            inner,
            system,
            margin: config.margin,
            mode: classify(inner),
        };
        let est = adaptive_multilevel_splitting(
            &walk,
            config.particles,
            config.margin,
            system.horizon() as u64,
            config.seed,
        )?;
        let probability = 1.0 - est.probability;
        Ok(RareEventResult {
            probability,
            violation_probability: est.probability,
            holds: super::decides(*op, probability, *threshold),
            simulations: est.simulations,
        })
    }
}

fn config_error(message: String) -> Error {
    Error::InvalidConfig {
        context: "stochastic system",
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_distr::{Distribution, StandardNormal};

    fn random_walk(horizon: usize) -> StochasticSystem {
        StochasticSystem::new(
            ["x"],
            1.0,
            horizon,
            |rng| vec![StandardNormal.sample(rng)],
            |prev, _t, rng| {
                let step: f64 = StandardNormal.sample(rng);
                vec![prev[0] + step]
            },
        )
        .unwrap()
    }

    #[test]
    fn rare_event_lands_near_ground_truth() {
        // P(the walk's running max reaches 8 within 15 steps) ~ 0.028 by 4e5-sample MC.
        let phi = Formula::parse("P>=0.99(always(x < 8.0))").unwrap();
        let config = RareEventConfig {
            particles: 4000,
            margin: 0.0,
            seed: 1,
        };
        let result = phi.check_rare_event(&random_walk(15), &config).unwrap();
        let truth = 0.0281;
        assert!(
            result.violation_probability > truth / 2.0
                && result.violation_probability < truth * 2.0,
            "got {}",
            result.violation_probability
        );
    }

    #[test]
    fn resolves_a_tail_event_monte_carlo_reports_as_zero() {
        let phi = Formula::parse("P>=0.999(always(x < 8.0))").unwrap();
        let config = RareEventConfig {
            particles: 4000,
            margin: 0.0,
            seed: 2,
        };
        let result = phi.check_rare_event(&random_walk(6), &config).unwrap();
        assert!(result.violation_probability > 0.0);
        assert!(result.probability < 1.0);
    }

    #[test]
    fn simulate_builds_a_full_horizon_trace() {
        use rand::SeedableRng;
        let system =
            StochasticSystem::new(["x"], 1.0, 3, |_| vec![0.0], |p, _, _| vec![p[0] + 1.0])
                .unwrap();
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(0);
        let trace = system.simulate(&mut rng).unwrap();
        assert_eq!(trace.times(), &[0.0, 1.0, 2.0, 3.0]);
        assert_eq!(trace.signals()["x"], vec![0.0, 1.0, 2.0, 3.0]);
    }

    #[test]
    fn the_incremental_score_matches_the_full_recompute() {
        use rand::SeedableRng;

        let inner = Formula::parse("always((x < 8.0) and (x > -8.0))").unwrap();
        assert!(matches!(classify(&inner), ScoreMode::Always(_)));
        let system = random_walk(40);
        let fast = PrstlWalk {
            inner: &inner,
            system: &system,
            margin: 0.0,
            mode: classify(&inner),
        };
        let full = PrstlWalk {
            inner: &inner,
            system: &system,
            margin: 0.0,
            mode: ScoreMode::Full,
        };
        let mut rng_fast = rand_chacha::ChaCha8Rng::seed_from_u64(7);
        let mut rng_full = rand_chacha::ChaCha8Rng::seed_from_u64(7);
        let mut sa = fast.initial_state(&mut rng_fast);
        let mut sb = full.initial_state(&mut rng_full);
        assert_eq!(sa.score.to_bits(), sb.score.to_bits());
        for step in 0..40 {
            sa = fast.step(&sa, &mut rng_fast);
            sb = full.step(&sb, &mut rng_full);
            assert_eq!(
                sa.score.to_bits(),
                sb.score.to_bits(),
                "scores diverged at step {step}: fast {} full {}",
                sa.score,
                sb.score
            );
        }
    }

    #[test]
    fn an_eventually_inner_takes_the_fast_path() {
        let inner = Formula::parse("eventually(x > 8.0)").unwrap();
        assert!(matches!(classify(&inner), ScoreMode::Eventually(_)));
        assert!(matches!(
            classify(&Formula::parse("always[0, 3](x < 8.0)").unwrap()),
            ScoreMode::Full
        ));
        assert!(matches!(
            classify(&Formula::parse("always(eventually(x < 8.0))").unwrap()),
            ScoreMode::Full
        ));
    }

    #[test]
    fn a_non_probabilistic_formula_is_rejected() {
        let phi = Formula::parse("always(x < 8.0)").unwrap();
        let err = phi.check_rare_event(&random_walk(5), &RareEventConfig::default());
        assert!(matches!(err, Err(Error::NotProbabilistic)));
    }

    #[test]
    fn an_invalid_system_is_rejected() {
        let build = StochasticSystem::new(["x"], 0.0, 5, |_| vec![0.0], |p, _, _| p.to_vec());
        assert!(build.is_err());
    }
}