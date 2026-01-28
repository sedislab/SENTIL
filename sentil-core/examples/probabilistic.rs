//! Probabilistic monitoring: lift a noisy sensor into an ensemble and estimate the satisfaction probability with a Wilson interval.
//!
//! Run with `cargo run --example probabilistic`.

use sentil::{LiftingRegistry, Monitor, MonitorConfig, NoiseInteraction, NoiseModel, SmcConfig};

fn main() -> sentil::Result<()> {
    let times: Vec<f64> = (0..20).map(f64::from).collect();
    let mut trace = sentil::Trace::new(times)?;
    trace.add_signal("x", (0..20).map(|i| 0.4 + 0.05 * f64::from(i)).collect::<Vec<_>>())?;

    let mut lifting = LiftingRegistry::new();
    lifting.register("x", NoiseModel::gaussian(0.0, 0.3)?, NoiseInteraction::Additive);

    let config = MonitorConfig::new().smc(SmcConfig {
        samples: 5000,
        ..SmcConfig::default()
    });
    let monitor = Monitor::new("P>=0.9 (always (x > 0))", config)?;
    let result = monitor.check(&trace, &lifting)?;
    println!(
        "probability {:.3}, interval [{:.3}, {:.3}], holds {}",
        result.probability, result.interval.lower, result.interval.upper, result.holds
    );
    Ok(())
}