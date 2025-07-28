//! Adaptive multilevel splitting for estimating rare-event probabilities.
//!
//! When the event of interest sits far out in the tail (below about `1e-6`),
//! plain Monte Carlo almost never observes it. AMS instead evolves a population
//! of particles through a ladder of rising score thresholds, clones the
//! survivors at each level, and multiplies the per-level survival fractions into
//! an estimate that stays accurate where Monte Carlo would report zero.

use rand::RngCore;

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