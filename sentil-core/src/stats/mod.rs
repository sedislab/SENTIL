//! Statistical model checking for the probabilistic operator.

mod confidence;
mod lifting;
mod noise;
mod smc;

pub use confidence::{wilson_interval, z_score, ConfidenceInterval};
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