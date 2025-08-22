//! Statistical model checking for the probabilistic operator.

mod confidence;
mod lifting;
mod noise;
mod prstl_rare;
mod rare_events;
#[cfg(feature = "gpu")]
mod sim_model;
mod smc;
mod sprt;

pub use confidence::{
    chernoff_hoeffding_samples, clopper_pearson, wilson_interval, z_score, ConfidenceInterval,
};
pub use lifting::LiftingRegistry;
#[cfg(feature = "gpu")]
pub(crate) use noise::GpuSampler;
pub use noise::{NoiseInteraction, NoiseModel};
pub use prstl_rare::{RareEventConfig, RareEventResult, StochasticSystem};
pub use rare_events::{adaptive_multilevel_splitting, RareEventEstimate, RareEventSimulator};
#[cfg(feature = "gpu")]
pub use sim_model::{SimExpr, SimModel};
pub use smc::{SmcConfig, SmcResult};
pub use sprt::{sequential_test, SprtConfig, SprtResult};

use crate::error::{Error, Result};
use crate::formula::{Formula, ProbabilityOp};
use crate::signal::Trace;

pub(crate) fn decides(op: ProbabilityOp, probability: f64, threshold: f64) -> bool {
    match op {
        ProbabilityOp::GreaterEqual => probability >= threshold,
        ProbabilityOp::Greater => probability > threshold,
        ProbabilityOp::LessEqual => probability <= threshold,
        ProbabilityOp::Less => probability < threshold,
    }
}

impl Formula {
    /// Decides a probabilistic specification `P~p(phi)` over a trace by sampling.
    ///
    /// ```
    /// use sentil::{Formula, Trace, stats::{LiftingRegistry, NoiseModel, NoiseInteraction, SmcConfig}};
    ///
    /// let phi = Formula::parse("P>=0.9(always[0, 2](x > 0))")?;
    /// let mut trace = Trace::new([0.0, 1.0, 2.0])?;
    /// trace.add_signal("x", [5.0, 5.0, 5.0])?;
    /// let mut lifting = LiftingRegistry::new();
    /// lifting.register("x", NoiseModel::gaussian(0.0, 0.5)?, NoiseInteraction::Additive);
    ///
    /// let result = phi.check(&trace, &lifting, &SmcConfig::default())?;
    /// assert!(result.holds);
    /// assert!(result.probability > 0.99);
    /// # Ok::<(), sentil::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotProbabilistic`] if the formula is not wrapped in `P`.
    pub fn check(
        &self,
        trace: &Trace,
        lifting: &LiftingRegistry,
        config: &SmcConfig,
    ) -> Result<SmcResult> {
        match self {
            Formula::Probabilistic(op, threshold, inner) => {
                smc::check(*op, *threshold, inner, trace, lifting, config)
            }
            _ => Err(Error::NotProbabilistic),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::float_cmp,
        reason = "the asserted probabilities are exact bounds"
    )]
    #![allow(
        clippy::cast_precision_loss,
        reason = "test trace indices are tiny, so the index-to-time cast is exact"
    )]

    use super::*;

    fn trace(values: &[f64]) -> Trace {
        let times: Vec<f64> = (0..values.len()).map(|i| i as f64).collect();
        let mut t = Trace::new(times).unwrap();
        t.add_signal("x", values.to_vec()).unwrap();
        t
    }

    #[test]
    fn a_property_with_wide_margin_holds_with_near_certainty() {
        let phi = Formula::parse("P>=0.9(always[0, 2](x > 0))").unwrap();
        let mut lifting = LiftingRegistry::new();
        lifting.register(
            "x",
            NoiseModel::gaussian(0.0, 0.5).unwrap(),
            NoiseInteraction::Additive,
        );
        let result = phi
            .check(&trace(&[5.0, 5.0, 5.0]), &lifting, &SmcConfig::default())
            .unwrap();
        assert!(result.holds);
        assert!(result.probability > 0.99);
    }

    #[test]
    fn a_borderline_property_estimates_near_one_half() {
        let phi = Formula::parse("P>=0.4(x > 0)").unwrap();
        let mut lifting = LiftingRegistry::new();
        lifting.register(
            "x",
            NoiseModel::gaussian(0.0, 1.0).unwrap(),
            NoiseInteraction::Additive,
        );
        let result = phi
            .check(&trace(&[0.0]), &lifting, &SmcConfig::default())
            .unwrap();
        assert!((0.45..=0.55).contains(&result.probability));
        assert!(result.interval.contains(0.5));
    }

    #[test]
    fn the_check_is_reproducible() {
        let phi = Formula::parse("P>=0.5(x > 0)").unwrap();
        let mut lifting = LiftingRegistry::new();
        lifting.register(
            "x",
            NoiseModel::gaussian(0.0, 2.0).unwrap(),
            NoiseInteraction::Additive,
        );
        let config = SmcConfig {
            samples: 5_000,
            confidence: 0.95,
            seed: 7,
        };
        let a = phi.check(&trace(&[1.0]), &lifting, &config).unwrap();
        let b = phi.check(&trace(&[1.0]), &lifting, &config).unwrap();
        assert_eq!(a.satisfactions, b.satisfactions);
    }

    #[test]
    fn boundary_robustness_counts_as_satisfied() {
        let phi = Formula::parse("P>=0.5(x >= 0)").unwrap();
        let mut lifting = LiftingRegistry::new();
        lifting.register(
            "x",
            NoiseModel::dirac(0.0).unwrap(),
            NoiseInteraction::Additive,
        );
        let result = phi
            .check(&trace(&[0.0]), &lifting, &SmcConfig::default())
            .unwrap();
        assert_eq!(result.probability, 1.0);
        assert!(result.holds);
    }

    #[test]
    fn a_non_probabilistic_formula_is_rejected() {
        let phi = Formula::parse("x > 0").unwrap();
        assert!(matches!(
            phi.check(
                &trace(&[1.0]),
                &LiftingRegistry::new(),
                &SmcConfig::default()
            ),
            Err(Error::NotProbabilistic)
        ));
    }

    #[cfg(feature = "gpu")]
    #[test]
    #[ignore = "needs a GPU; run with --ignored on a GPU node"]
    fn gpu_probability_matches_the_normal_cdf() {
        assert!(
            crate::gpu::is_available(),
            "this test must run on a GPU node so the check uses the device"
        );
        let mut lifting = LiftingRegistry::new();
        lifting.register(
            "x",
            NoiseModel::gaussian(0.0, 1.0).unwrap(),
            NoiseInteraction::Additive,
        );
        let config = SmcConfig {
            samples: 2_000_000,
            confidence: 0.95,
            seed: 7,
        };
        for (c, expected) in [
            (0.0, 0.5),
            (0.674_490, 0.75),
            (1.281_552, 0.90),
            (-1.0, 0.158_655),
        ] {
            let phi = Formula::parse(&format!("P>=0.5(x <= {c})")).unwrap();
            let result = phi.check(&trace(&[0.0]), &lifting, &config).unwrap();
            assert!(
                (result.probability - expected).abs() < 3e-3,
                "c={c}: got {}, expected {expected}",
                result.probability
            );
        }
    }

    #[cfg(feature = "gpu")]
    #[test]
    #[ignore = "needs a GPU; run with --ignored on a GPU node"]
    fn gpu_closed_form_families_match_their_cdf() {
        assert!(
            crate::gpu::is_available(),
            "this test must run on a GPU node so the check uses the device"
        );
        let config = SmcConfig {
            samples: 2_000_000,
            confidence: 0.95,
            seed: 11,
        };
        let cases: [(NoiseModel, f64, f64); 4] = [
            (
                NoiseModel::weibull(1.0, 1.0).unwrap(),
                1.0,
                1.0 - (-1.0f64).exp(),
            ),
            (
                NoiseModel::rayleigh(1.0).unwrap(),
                1.0,
                1.0 - (-0.5f64).exp(),
            ),
            (NoiseModel::gumbel(0.0, 1.0).unwrap(), 0.0, (-1.0f64).exp()),
            (NoiseModel::cauchy(0.0, 1.0).unwrap(), 0.0, 0.5),
        ];
        for (model, c, expected) in cases {
            let mut lifting = LiftingRegistry::new();
            lifting.register("x", model, NoiseInteraction::Additive);
            let phi = Formula::parse(&format!("P>=0.5(x <= {c})")).unwrap();
            let result = phi.check(&trace(&[0.0]), &lifting, &config).unwrap();
            assert!(
                (result.probability - expected).abs() < 4e-3,
                "CDF({c}): got {}, expected {expected}",
                result.probability
            );
        }
    }

    #[cfg(feature = "gpu")]
    #[test]
    #[ignore = "needs a GPU; run with --ignored on a GPU node"]
    fn gpu_truncated_normal_matches_its_distribution() {
        assert!(
            crate::gpu::is_available(),
            "this test must run on a GPU node so the check uses the device"
        );
        let mut lifting = LiftingRegistry::new();
        lifting.register(
            "x",
            NoiseModel::truncated_normal(0.0, 1.0, -1.0, 1.0).unwrap(),
            NoiseInteraction::Additive,
        );
        let config = SmcConfig {
            samples: 2_000_000,
            confidence: 0.95,
            seed: 13,
        };
        let median = Formula::parse("P>=0.5(x <= 0)").unwrap();
        let p = median.check(&trace(&[0.0]), &lifting, &config).unwrap();
        assert!(
            (p.probability - 0.5).abs() < 3e-3,
            "median: got {}",
            p.probability
        );
        let bounded = Formula::parse("P>=0.5(x <= 1.001)").unwrap();
        let q = bounded.check(&trace(&[0.0]), &lifting, &config).unwrap();
        assert!(q.probability > 0.999, "bound: got {}", q.probability);
    }

    #[cfg(feature = "gpu")]
    #[test]
    #[ignore = "needs a GPU; run with --ignored on a GPU node"]
    fn gpu_gamma_based_families_match() {
        assert!(
            crate::gpu::is_available(),
            "this test must run on a GPU node so the check uses the device"
        );
        let config = SmcConfig {
            samples: 2_000_000,
            confidence: 0.95,
            seed: 17,
        };
        let cases: [(NoiseModel, f64, f64); 3] = [
            (
                NoiseModel::gamma(2.0, 1.0).unwrap(),
                2.0,
                1.0 - 3.0 * (-2.0f64).exp(),
            ),
            (NoiseModel::beta(2.0, 2.0).unwrap(), 0.5, 0.5),
            (NoiseModel::student_t(10.0, 0.0, 1.0).unwrap(), 0.0, 0.5),
        ];
        for (model, c, expected) in cases {
            let mut lifting = LiftingRegistry::new();
            lifting.register("x", model, NoiseInteraction::Additive);
            let phi = Formula::parse(&format!("P>=0.5(x <= {c})")).unwrap();
            let result = phi.check(&trace(&[0.0]), &lifting, &config).unwrap();
            assert!(
                (result.probability - expected).abs() < 4e-3,
                "P(x <= {c}): got {}, expected {expected}",
                result.probability
            );
        }
    }

    #[cfg(feature = "gpu")]
    #[test]
    #[ignore = "needs a GPU; run with --ignored on a GPU node"]
    fn gpu_discrete_families_match() {
        assert!(
            crate::gpu::is_available(),
            "this test must run on a GPU node so the check uses the device"
        );
        let config = SmcConfig {
            samples: 2_000_000,
            confidence: 0.95,
            seed: 19,
        };
        let cases: [(NoiseModel, f64, f64); 2] = [
            (NoiseModel::poisson(4.0).unwrap(), 4.0, 0.628_837),
            (NoiseModel::binomial(10, 0.5).unwrap(), 5.0, 638.0 / 1024.0),
        ];
        for (model, c, expected) in cases {
            let mut lifting = LiftingRegistry::new();
            lifting.register("x", model, NoiseInteraction::Additive);
            let phi = Formula::parse(&format!("P>=0.5(x <= {c})")).unwrap();
            let result = phi.check(&trace(&[0.0]), &lifting, &config).unwrap();
            assert!(
                (result.probability - expected).abs() < 4e-3,
                "P(x <= {c}): got {}, expected {expected}",
                result.probability
            );
        }
    }
}