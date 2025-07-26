//! Statistical model checking for the probabilistic operator.

mod confidence;
mod lifting;
mod noise;
mod smc;

pub use confidence::{clopper_pearson, wilson_interval, z_score, ConfidenceInterval};
pub use lifting::LiftingRegistry;
pub use noise::{NoiseInteraction, NoiseModel};
pub use smc::{SmcConfig, SmcResult};

use crate::error::{Error, Result};
use crate::formula::Formula;
use crate::signal::Trace;

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
}