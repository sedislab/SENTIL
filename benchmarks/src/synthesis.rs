use std::time::{Duration, Instant};

use sentil::{Backend, Bounds, Controller, Formula, LinearModel, SynthesisProblem, Synthesizer};

use crate::measure::{hardware, summarize, time_runs};
use crate::schema::SynthRecord;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// `x_{t+1} = x_t + u_t`
fn integrator(x0: f64, horizon: usize) -> LinearModel {
    LinearModel::new(vec![vec![1.0]], vec![vec![1.0]], vec![x0], ["x"], 1.0, horizon)
        .expect("the integrator model is well formed")
}

struct OpenLoop {
    id: &'static str,
    x0: f64,
    formula: &'static str,
    backend: Backend,
    horizon: usize,
}

const OPEN_LOOP: &[OpenLoop] = &[
    OpenLoop { id: "hold_gradient", x0: 0.5, formula: "always (x > 0)", backend: Backend::Gradient, horizon: 5 },
    OpenLoop { id: "hold_cmaes", x0: 0.5, formula: "always (x > 0)", backend: Backend::CmaEs, horizon: 5 },
    OpenLoop { id: "reach_gradient", x0: -1.0, formula: "eventually (x > 3)", backend: Backend::Gradient, horizon: 8 },
    OpenLoop { id: "bounded_gradient", x0: 0.0, formula: "always[0, 4] (x > -0.4)", backend: Backend::Gradient, horizon: 6 },
];

fn backend_name(backend: Backend) -> &'static str {
    match backend {
        Backend::Gradient => "gradient",
        Backend::CmaEs => "cmaes",
        Backend::Milp => "milp",
        Backend::Auto => "auto",
    }
}

fn open_loop_record(case: &OpenLoop) -> SynthRecord {
    let model = integrator(case.x0, case.horizon);
    let spec = Formula::parse(case.formula).expect("the benchmark spec parses");
    let (horizon, backend) = (case.horizon, case.backend);
    let solve = || {
        let bounds = Bounds::new(vec![-1.0; horizon], vec![1.0; horizon])
            .expect("the input bounds are well formed");
        let problem = SynthesisProblem::new(&model, &spec)
            .with_backend(backend)
            .with_bounds(bounds)
            .with_budget(400);
        Synthesizer::solve(&problem).expect("the solve succeeds")
    };
    let timing = time_runs(20, &solve);
    let result = solve();
    SynthRecord {
        tool: "sentil".to_owned(),
        version: VERSION.to_owned(),
        language: "rust".to_owned(),
        mode: "open_loop".to_owned(),
        case: case.id.to_owned(),
        formula: case.formula.to_owned(),
        backend: backend_name(result.backend).to_owned(),
        timing,
        robustness: result.robustness,
        holds: result.holds,
        deadline_ms: None,
        deadline_misses: None,
        steps: None,
        runs: 20,
        hardware: hardware(),
    }
}

fn receding_horizon_record() -> SynthRecord {
    let formula = "always (x > 0)";
    let horizon = 4;
    let deadline = Duration::from_millis(5);
    let model = integrator(0.5, horizon);
    let spec = Formula::parse(formula).expect("the benchmark spec parses");
    let bounds = Bounds::new(vec![-1.0; horizon], vec![1.0; horizon])
        .expect("the input bounds are well formed");
    let mut controller = Controller::new(&model, &spec, 1, deadline).with_bounds(bounds);

    let steps = 200u64;
    let deadline_ms = deadline.as_secs_f64() * 1e3;
    let mut latencies = Vec::with_capacity(steps as usize);
    let mut misses = 0u64;
    let mut worst = f64::INFINITY;
    let mut state = vec![0.5f64];
    for _ in 0..steps {
        let start = Instant::now();
        let input = controller.control(&state).expect("the controller returns an input");
        let elapsed = start.elapsed();
        let ms = elapsed.as_secs_f64() * 1e3;
        latencies.push(ms);
        if elapsed > deadline {
            misses += 1;
        }
        state[0] += input.first().copied().unwrap_or(0.0);
        worst = worst.min(state[0]);
    }
    let timing = summarize(&mut latencies);
    SynthRecord {
        tool: "sentil".to_owned(),
        version: VERSION.to_owned(),
        language: "rust".to_owned(),
        mode: "receding_horizon".to_owned(),
        case: "integrator_hold".to_owned(),
        formula: formula.to_owned(),
        backend: "gradient".to_owned(),
        timing,
        robustness: worst,
        holds: worst >= 0.0,
        deadline_ms: Some(deadline_ms),
        deadline_misses: Some(misses),
        steps: Some(steps),
        runs: steps,
        hardware: hardware(),
    }
}

#[must_use]
pub fn run() -> Vec<SynthRecord> {
    let mut records: Vec<SynthRecord> = OPEN_LOOP.iter().map(open_loop_record).collect();
    records.push(receding_horizon_record());
    records
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_synthesis_case_produces_a_record() {
        let records = super::run();
        assert_eq!(records.len(), super::OPEN_LOOP.len() + 1);
        let online = records.last().expect("a record exists");
        assert!(online.deadline_ms.is_some() && online.deadline_misses.is_some());
    }
}