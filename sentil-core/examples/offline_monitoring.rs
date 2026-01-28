//! Offline robustness over a recorded trace, in discrete and dense time.
//!
//! Run with `cargo run --example offline_monitoring`.

use sentil::{Formula, Trace};

fn main() -> sentil::Result<()> {
    let mut trace = Trace::new(vec![0.0, 1.0, 2.0, 3.0, 4.0])?;
    trace.add_signal("speed", vec![12.0, 9.0, 7.0, 4.0, 6.0])?;

    let phi = Formula::parse("always (speed > 5)")?;
    println!("robustness:       {}", phi.robustness(&trace)?);
    println!("per sample:       {:?}", phi.robustness_signal(&trace)?);
    println!("dense robustness: {}", phi.robustness_dense(&trace)?);
    Ok(())
}