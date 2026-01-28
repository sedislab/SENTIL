//! Online streaming: fold one timestamped sample at a time.
//!
//! Run with `cargo run --example online_streaming`.

use sentil::{Monitor, MonitorConfig};

fn main() -> sentil::Result<()> {
    let mut monitor = Monitor::new("always[0, 10] (x > -0.9)", MonitorConfig::new())?;
    for t in 0..60 {
        let x = (f64::from(t) * 0.3).sin();
        let verdict = monitor.update(f64::from(t), &[("x", x)])?;
        if verdict.is_resolved() && verdict.value() < 0.0 {
            println!("violated at t={t}, robustness={:.3}", verdict.value());
            return Ok(());
        }
    }
    println!("held over the whole stream");
    Ok(())
}