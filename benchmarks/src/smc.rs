//! Statistical model checking benchmark

use std::time::Instant;

use sentil::{Formula, LiftingRegistry, NoiseInteraction, NoiseModel, SmcConfig, SmcResult, Trace};

use crate::measure::summarize;
use crate::schema::Timing;

pub struct SmcModel {
    pub id: &'static str,
    pub signals: &'static [(&'static str, &'static [f64])],
    pub noise: &'static [(&'static str, f64)],
    pub formula: &'static str,
    pub ground_truth: Option<f64>,
}

pub const THROUGHPUT: &[SmcModel] = &[
    SmcModel {
        id: "single_step",
        signals: &[("x", &[0.5])],
        noise: &[("x", 1.0)],
        formula: "P >= 0.5(x > 0)",
        ground_truth: Some(0.691_462_461_274_013_1),
    },
    SmcModel {
        id: "always_ten",
        signals: &[("x", &[1.0; 10])],
        noise: &[("x", 1.0)],
        formula: "P >= 0.5(always[0, 9](x > 0))",
        ground_truth: Some(0.177_721_459_208_027_73),
    },
    SmcModel {
        id: "eventually_ten",
        signals: &[("x", &[-1.0; 10])],
        noise: &[("x", 1.0)],
        formula: "P >= 0.5(eventually[0, 9](x > 0))",
        ground_truth: Some(0.822_278_540_791_972_3),
    },
];

pub struct Outcome {
    pub result: SmcResult,
    pub timing: Timing,
    pub throughput_per_s: f64,
    pub steps: u64,
}

/// Lifts the model's signals under their noise and runs [`Formula::check`] `runs` times at `samples` draws.
///
/// # Panics
///
/// Panics if a model in the suite is malformed (no signals, a parse error, a non-positive noise deviation, or a signal that does not match the grid)
#[must_use]
pub fn estimate(model: &SmcModel, samples: u64, runs: u64, seed: u64) -> Outcome {
    let n = model
        .signals
        .first()
        .expect("a model carries at least one signal")
        .1
        .len();
    let mut trace = Trace::indexed(n);
    for (name, values) in model.signals {
        trace
            .add_signal(name, values.to_vec())
            .expect("a base signal matches the grid");
    }
    let mut lifting = LiftingRegistry::new();
    for (var, std_dev) in model.noise {
        lifting.register(
            var,
            NoiseModel::gaussian(0.0, *std_dev).expect("a positive noise deviation"),
            NoiseInteraction::Additive,
        );
    }
    let config = SmcConfig {
        samples,
        seed,
        ..Default::default()
    };
    let phi = Formula::parse(model.formula).expect("a valid model formula");

    let mut times_ms = Vec::with_capacity(runs as usize);
    let mut result = None;
    for _ in 0..runs {
        let start = Instant::now();
        let r = phi.check(&trace, &lifting, &config).expect("an estimate");
        times_ms.push(start.elapsed().as_secs_f64() * 1000.0);
        result = Some(r);
    }
    let timing = summarize(&mut times_ms);
    #[allow(
        clippy::cast_precision_loss,
        reason = "sample counts and trace lengths stay far below 2^53"
    )]
    let throughput_per_s = (samples as f64 * n as f64) / (timing.mean_ms / 1000.0);
    Outcome {
        result: result.expect("at least one run"),
        timing,
        throughput_per_s,
        steps: n as u64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_throughput_model_estimates_near_its_closed_form_truth() {
        for model in THROUGHPUT {
            let truth = model
                .ground_truth
                .expect("every throughput model carries a closed-form truth");
            let out = estimate(model, 40_000, 1, 7);
            assert!(
                (out.result.probability - truth).abs() < 0.02,
                "{}: estimate {} vs {truth}",
                model.id,
                out.result.probability
            );
            assert!(out.throughput_per_s > 0.0);
        }
    }
}