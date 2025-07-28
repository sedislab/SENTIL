//! Adaptive multilevel splitting for estimating rare-event probabilities.
//!
//! When the event of interest sits far out in the tail (below about `1e-6`),
//! plain Monte Carlo almost never observes it. AMS instead evolves a population
//! of particles through a ladder of rising score thresholds, clones the
//! survivors at each level, and multiplies the per-level survival fractions into
//! an estimate that stays accurate where Monte Carlo would report zero.

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

#[derive(Clone)]
struct Particle<S> {
    state: S,
    max_score: f64,
}

struct LevelOutcome<S> {
    /// The particle to carry forward, absent if it was absorbed into failure.
    survivor: Option<Particle<S>>,
    max_score: f64,
}

/// Estimates the probability that `simulator` reaches the rare event, defined as
/// a score of at least `target_score`, by adaptive multilevel splitting.
///
/// `particles` is the population size; a larger population gives a tighter
/// estimate. `max_steps_per_level` caps each particle's simulation per level, and
/// `seed` makes the run reproducible.
///
/// # Errors
///
/// Returns [`Error::InvalidConfig`] if `particles` is zero.
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "particle counts are small positive integers, exact in f64 and back"
)]
pub fn adaptive_multilevel_splitting<S: RareEventSimulator>(
    simulator: &S,
    particles: usize,
    target_score: f64,
    max_steps_per_level: u64,
    seed: u64,
) -> Result<RareEventEstimate> {
    if particles == 0 {
        return Err(Error::InvalidConfig {
            context: "adaptive splitting",
            message: "particle count must be positive".to_owned(),
        });
    }
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut simulations = 0u64;

    let mut population: Vec<Particle<S::State>> = (0..particles)
        .map(|_| {
            let mut prng = ChaCha8Rng::seed_from_u64(rng.next_u64());
            let state = simulator.initial_state(&mut prng);
            let max_score = simulator.score(&state);
            Particle { state, max_score }
        })
        .collect();
    simulations += particles as u64;

    let mut probability = 1.0;
    loop {
        let mut outcomes = Vec::with_capacity(particles);
        for p in &population {
            let mut prng = ChaCha8Rng::seed_from_u64(rng.next_u64());
            outcomes.push(run_level(
                simulator,
                p,
                max_steps_per_level,
                &mut prng,
                &mut simulations,
            ));
        }

        if outcomes
            .iter()
            .filter(|o| o.max_score >= target_score)
            .count()
            == particles
        {
            break;
        }

        // Promote the threshold to the score that about half the population clears.
        let mut scores: Vec<f64> = outcomes.iter().map(|o| o.max_score).collect();
        scores.sort_by(f64::total_cmp);
        let survivor_target = ((particles as f64) * 0.5).max(1.0) as usize;
        let threshold = scores[particles - survivor_target].min(target_score);

        let survivors: Vec<Particle<S::State>> = outcomes
            .into_iter()
            .filter(|o| o.max_score >= threshold)
            .filter_map(|o| o.survivor)
            .collect();
        let k = survivors.len();
        if k == 0 {
            return Ok(RareEventEstimate {
                probability: 0.0,
                simulations,
            });
        }
        probability *= k as f64 / particles as f64;
        if threshold >= target_score {
            break;
        }
        population = (0..particles)
            .map(|_| survivors[rng.random_range(0..k)].clone())
            .collect();
    }

    Ok(RareEventEstimate {
        probability,
        simulations,
    })
}

/// Runs one particle for a single level, tracking its highest score and stopping
/// early if it is absorbed into a terminal state. A particle absorbed into
/// failure leaves no survivor.
fn run_level<S: RareEventSimulator>(
    simulator: &S,
    particle: &Particle<S::State>,
    max_steps: u64,
    rng: &mut dyn RngCore,
    simulations: &mut u64,
) -> LevelOutcome<S::State> {
    let mut state = particle.state.clone();
    let mut max_score = particle.max_score;
    for _ in 0..max_steps {
        *simulations += 1;
        state = simulator.step(&state, rng);
        max_score = max_score.max(simulator.score(&state));
        let (terminal, violation) = simulator.is_terminal(&state);
        if terminal {
            return LevelOutcome {
                survivor: violation.then(|| Particle { state, max_score }),
                max_score,
            };
        }
    }
    LevelOutcome {
        survivor: Some(Particle { state, max_score }),
        max_score,
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
}