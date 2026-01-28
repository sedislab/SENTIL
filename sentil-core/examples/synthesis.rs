//! Synthesize a control-input sequence that satisfies a spec on a linear model.
//!
//! Run with `cargo run --example synthesis`.

use sentil::synthesis::{Backend, Bounds, LinearModel, SynthesisProblem, Synthesizer};
use sentil::Formula;

fn main() -> sentil::Result<()> {
    // x_{t+1} = x_t + u_t over three steps; keep x above zero.
    let model = LinearModel::new(vec![vec![1.0]], vec![vec![1.0]], vec![1.0], ["x"], 1.0, 3)?;
    let spec = Formula::parse("always (x > 0)")?;
    let bounds = Bounds::new(vec![-1.0, -1.0, -1.0], vec![1.0, 1.0, 1.0])?;

    let problem = SynthesisProblem::new(&model, &spec)
        .with_bounds(bounds)
        .with_backend(Backend::Gradient)
        .with_budget(200);
    let result = Synthesizer::solve(&problem)?;
    println!(
        "input {:?}, robustness {:.4}, holds {}",
        result.input, result.robustness, result.holds
    );
    Ok(())
}