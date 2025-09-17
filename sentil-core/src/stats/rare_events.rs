//! Adaptive multilevel splitting for estimating rare-event probabilities.

use rand::{Rng, RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::error::{Error, Result};

/// A system that adaptive splitting can simulate.
pub trait RareEventSimulator {
    /// The simulation state carried from one step to the next.
    type State: Clone;

    /// Draws an initial state.
    fn initial_state(&self, rng: &mut dyn RngCore) -> Self::State;

    /// Advances the state by one step.
    fn step(&self, state: &Self::State, rng: &mut dyn RngCore) -> Self::State;

    /// Reports `(ended, in_the_rare_event)`.
    fn is_terminal(&self, state: &Self::State) -> (bool, bool);

    /// Scores a state; a higher score is closer to the rare event.
    fn score(&self, state: &Self::State) -> f64;
}

/// The result of an adaptive-splitting run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RareEventEstimate {
    /// The estimated probability of the rare event.
    pub probability: f64,
    /// How many simulation steps were run in total.
    pub simulations: u64,
}

struct Trajectory<S> {
    states: Vec<S>,
    z: f64,
}

/// Estimates the probability that `simulator` reaches the rare event, defined as a
/// score of at least `target_score`, by the last-particle form of adaptive
/// multilevel splitting.
///
/// Each iteration drops the single worst trajectory and regenerates it by
/// branching a survivor from where it first crossed the current level, so the
/// level rises one trajectory at a time and the estimate stays unbiased far out in
/// the tail where a fixed-fraction scheme would not. `particles` sets the
/// population (larger is tighter), `max_steps` caps a trajectory's length, and
/// `seed` makes the run reproducible.
///
/// The removal loop is sequential: each step's level depends on the whole current population.
///
/// # Errors
///
/// Returns [`Error::InvalidConfig`] if `particles` is zero, and
/// [`Error::Splitting`] if a trajectory's score becomes non-finite.
#[allow(
    clippy::cast_precision_loss,
    reason = "particle counts are small positive integers, exact in f64"
)]
pub fn adaptive_multilevel_splitting<S: RareEventSimulator>(
    simulator: &S,
    particles: usize,
    target_score: f64,
    max_steps: u64,
    seed: u64,
) -> Result<RareEventEstimate> {
    // Bound the run so a target that can never be reached still terminates. Each
    // removal multiplies the estimate by `ratio`, so after this many removals per
    // particle the estimate has fallen to about `e^-MAX_LEVELS_PER_PARTICLE`, far
    // below any probability worth resolving.
    const MAX_LEVELS_PER_PARTICLE: u64 = 28;
    if particles == 0 {
        return Err(Error::InvalidConfig {
            context: "adaptive splitting",
            message: "particle count must be positive".to_owned(),
        });
    }
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut simulations = 0u64;
    let mut population: Vec<Trajectory<S::State>> = Vec::with_capacity(particles);
    for i in 0..particles {
        let mut prng = ChaCha8Rng::seed_from_u64(rng.next_u64());
        let start = simulator.initial_state(&mut prng);
        population.push(simulate(
            simulator,
            start,
            &mut prng,
            max_steps,
            &mut simulations,
            i,
        )?);
    }

    let ratio = 1.0 - 1.0 / particles as f64;
    let cap = (particles as u64).saturating_mul(MAX_LEVELS_PER_PARTICLE);
    let mut removed = 0u64;
    while removed < cap {
        let level = population.iter().fold(f64::INFINITY, |m, t| m.min(t.z));
        if level >= target_score {
            break;
        }
        let survivors: Vec<usize> = (0..particles)
            .filter(|&i| population[i].z > level)
            .collect();
        if survivors.is_empty() {
            return Ok(RareEventEstimate {
                probability: 0.0,
                simulations,
            });
        }
        let doomed: Vec<usize> = (0..particles)
            .filter(|&i| population[i].z <= level)
            .collect();
        for dead in doomed {
            let src = survivors[rng.random_range(0..survivors.len())];
            let mut prng = ChaCha8Rng::seed_from_u64(rng.next_u64());
            population[dead] = branch(
                simulator,
                &population[src],
                level,
                &mut prng,
                max_steps,
                &mut simulations,
                dead,
            )?;
            removed += 1;
            if removed >= cap {
                break;
            }
        }
    }

    Ok(RareEventEstimate {
        probability: ratio.powf(removed as f64),
        simulations,
    })
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "a trajectory's length fits u64"
)]
fn extend<S: RareEventSimulator>(
    simulator: &S,
    mut states: Vec<S::State>,
    mut z: f64,
    max_steps: u64,
    rng: &mut dyn RngCore,
    simulations: &mut u64,
    index: usize,
) -> Result<Trajectory<S::State>> {
    while (states.len() as u64) <= max_steps {
        let last = &states[states.len() - 1];
        if simulator.is_terminal(last).0 {
            break;
        }
        *simulations += 1;
        let next = simulator.step(last, rng);
        let score = simulator.score(&next);
        if !score.is_finite() {
            return Err(splitting_error(index, states.len(), "score is not finite"));
        }
        z = z.max(score);
        states.push(next);
    }
    Ok(Trajectory { states, z })
}

fn simulate<S: RareEventSimulator>(
    simulator: &S,
    start: S::State,
    rng: &mut dyn RngCore,
    max_steps: u64,
    simulations: &mut u64,
    index: usize,
) -> Result<Trajectory<S::State>> {
    let z = simulator.score(&start);
    if !z.is_finite() {
        return Err(splitting_error(index, 0, "score is not finite"));
    }
    extend(
        simulator,
        vec![start],
        z,
        max_steps,
        rng,
        simulations,
        index,
    )
}

fn branch<S: RareEventSimulator>(
    simulator: &S,
    source: &Trajectory<S::State>,
    level: f64,
    rng: &mut dyn RngCore,
    max_steps: u64,
    simulations: &mut u64,
    index: usize,
) -> Result<Trajectory<S::State>> {
    let cross = source
        .states
        .iter()
        .position(|s| simulator.score(s) > level)
        .unwrap_or(0);
    let prefix: Vec<S::State> = source.states[..=cross].to_vec();
    let z = prefix
        .iter()
        .map(|s| simulator.score(s))
        .fold(f64::NEG_INFINITY, f64::max);
    extend(simulator, prefix, z, max_steps, rng, simulations, index)
}

fn splitting_error(particle: usize, level: usize, message: &str) -> Error {
    Error::Splitting {
        particle,
        level,
        message: message.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::float_cmp,
        reason = "the unreachable-event probability is exactly zero"
    )]

    use super::*;
    use rand::Rng;

    #[derive(Clone)]
    struct GamblersRuin {
        target: f64,
        failure: f64,
        step_prob: f64,
    }

    impl RareEventSimulator for GamblersRuin {
        type State = f64;

        fn initial_state(&self, _rng: &mut dyn RngCore) -> f64 {
            0.0
        }

        fn step(&self, state: &f64, rng: &mut dyn RngCore) -> f64 {
            if rng.random_bool(self.step_prob) {
                state + 1.0
            } else {
                state - 1.0
            }
        }

        fn is_terminal(&self, state: &f64) -> (bool, bool) {
            let violation = *state >= self.target;
            (violation || *state <= self.failure, violation)
        }

        fn score(&self, state: &f64) -> f64 {
            *state
        }
    }

    #[test]
    fn symmetric_walk_matches_the_analytic_half() {
        let sim = GamblersRuin {
            target: 3.0,
            failure: -3.0,
            step_prob: 0.5,
        };
        let est = adaptive_multilevel_splitting(&sim, 1000, 3.0, 100, 42).unwrap();
        assert!((est.probability - 0.5).abs() < 0.05);
        assert!(est.simulations > 1000);
    }

    #[test]
    fn biased_walk_matches_the_gamblers_ruin_formula() {
        // The analytic probability is 0.11636.
        let sim = GamblersRuin {
            target: 5.0,
            failure: -5.0,
            step_prob: 0.4,
        };
        let est = adaptive_multilevel_splitting(&sim, 1000, 5.0, 1000, 42).unwrap();
        assert!((est.probability - 0.11636).abs() < 0.05);
    }

    #[test]
    fn an_unreachable_event_has_probability_zero() {
        let sim = GamblersRuin {
            target: 10.0,
            failure: -10.0,
            step_prob: 0.0,
        };
        let est = adaptive_multilevel_splitting(&sim, 100, 10.0, 100, 42).unwrap();
        assert_eq!(est.probability, 0.0);
    }

    #[test]
    fn zero_particles_is_rejected() {
        let sim = GamblersRuin {
            target: 5.0,
            failure: -5.0,
            step_prob: 0.5,
        };
        assert!(adaptive_multilevel_splitting(&sim, 0, 5.0, 10, 1).is_err());
    }

    struct RareFlagCleared<S>(S);

    impl<S: RareEventSimulator> RareEventSimulator for RareFlagCleared<S> {
        type State = S::State;

        fn initial_state(&self, rng: &mut dyn RngCore) -> S::State {
            self.0.initial_state(rng)
        }

        fn step(&self, state: &S::State, rng: &mut dyn RngCore) -> S::State {
            self.0.step(state, rng)
        }

        fn is_terminal(&self, state: &S::State) -> (bool, bool) {
            (self.0.is_terminal(state).0, false)
        }

        fn score(&self, state: &S::State) -> f64 {
            self.0.score(state)
        }
    }

    #[test]
    fn clearing_the_rare_event_flag_leaves_the_estimate_alone() {
        let sim = GamblersRuin {
            target: 5.0,
            failure: -5.0,
            step_prob: 0.4,
        };
        let honest = adaptive_multilevel_splitting(&sim, 500, 5.0, 1000, 19).unwrap();
        let cleared =
            adaptive_multilevel_splitting(&RareFlagCleared(sim), 500, 5.0, 1000, 19).unwrap();
        assert_eq!(honest, cleared);
    }

    #[derive(Clone)]
    struct GoesNaN {
        steps_until_nan: u64,
    }

    impl RareEventSimulator for GoesNaN {
        type State = u64;

        fn initial_state(&self, _rng: &mut dyn RngCore) -> u64 {
            0
        }

        fn step(&self, state: &u64, _rng: &mut dyn RngCore) -> u64 {
            state + 1
        }

        fn is_terminal(&self, _state: &u64) -> (bool, bool) {
            (false, false)
        }

        #[allow(clippy::cast_precision_loss, reason = "test step counts are tiny")]
        fn score(&self, state: &u64) -> f64 {
            if *state >= self.steps_until_nan {
                f64::NAN
            } else {
                *state as f64
            }
        }
    }

    #[test]
    fn a_non_finite_score_is_reported() {
        let sim = GoesNaN { steps_until_nan: 5 };
        let result = adaptive_multilevel_splitting(&sim, 100, 100.0, 1000, 42);
        assert!(matches!(result, Err(Error::Splitting { level, .. }) if level >= 1));
    }
}