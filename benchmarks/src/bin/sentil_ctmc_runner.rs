//! SENTIL against PRISM, UPPAAL-SMC and Modest on the shared CTMC models.
//!
//! Each model is simulated exactly with Gillespie's direct method inside the system's
//! step, and its satisfaction probability is estimated by direct Monte Carlo, which suits
//! a discrete-state model where the splitter's level selection would bias on tied scores.
//! Both models carry a latched indicator so the property sees a crossing that happens
//! between grid points, the way a continuous time bound does.
//! Run as `sentil_ctmc_runner [circadian|tandem_queue]`. Prints the shared JSON record.

use std::io::Write;
use std::time::Instant;

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use sentil::{Formula, StochasticSystem};
use sentil_benchmarks::measure::{hardware, peak_rss_bytes};

const CAP: f64 = 1000.0;
const DT: f64 = 1.0;
const SAMPLES: u64 = 10_000;

const CIRCADIAN_HORIZON: usize = 20;
const QUEUE_CAP: f64 = 20.0;
const TANDEM_HORIZON: usize = 50;

fn fire(rates: &[f64], clock: &mut f64, rng: &mut dyn rand::RngCore) -> Option<usize> {
    let total: f64 = rates.iter().sum();
    if total <= 0.0 {
        return None;
    }
    *clock += -(1.0 - rng.random::<f64>()).ln() / total;
    if *clock > DT {
        return None;
    }
    let mut pick = rng.random::<f64>() * total;
    let mut which = 0;
    while which < rates.len() - 1 && pick >= rates[which] {
        pick -= rates[which];
        which += 1;
    }
    Some(which)
}

fn advance_circadian(state: &[f64], rng: &mut dyn rand::RngCore) -> Vec<f64> {
    let (mut a, mut r, mut ma, mut mr, mut peak) = (state[0], state[1], state[2], state[3], state[4]);
    let mut clock = 0.0;
    loop {
        let rates = [
            if a < CAP { 1.0 * (1.0 + a) } else { 0.0 }, // transcription A -> ma
            if ma > 0.0 { 0.5 * ma } else { 0.0 },       // translation A -> a
            if a > 0.0 { 0.2 * a } else { 0.0 },         // degradation A
            if r < CAP { 1.0 * (1.0 + a) } else { 0.0 }, // transcription R -> mr
            if mr > 0.0 { 0.5 * mr } else { 0.0 },       // translation R -> r
            if r > 0.0 { 0.2 * r } else { 0.0 },         // degradation R
            if a > 0.0 && r > 0.0 { 10.0 * a * r } else { 0.0 }, // complex formation
        ];
        let Some(which) = fire(&rates, &mut clock, rng) else {
            break;
        };
        match which {
            0 => ma = (ma + 1.0).min(CAP),
            1 => a = (a + 1.0).min(CAP),
            2 => a -= 1.0,
            3 => mr = (mr + 1.0).min(CAP),
            4 => r = (r + 1.0).min(CAP),
            5 => r -= 1.0,
            _ => {
                a -= 1.0;
                r -= 1.0;
            }
        }
        if a > peak {
            peak = a;
        }
    }
    vec![a, r, ma, mr, peak]
}

fn advance_tandem(state: &[f64], rng: &mut dyn rand::RngCore) -> Vec<f64> {
    let (mut q1, mut q2, mut full) = (state[0], state[1], state[2]);
    let mut clock = 0.0;
    loop {
        let rates = [
            if q1 < QUEUE_CAP { 4.8 } else { 0.0 },             // arrival
            if q1 > 0.0 && q2 < QUEUE_CAP { 5.0 } else { 0.0 }, // serve into queue 2
            if q2 > 0.0 { 5.0 } else { 0.0 },                   // departure
        ];
        let Some(which) = fire(&rates, &mut clock, rng) else {
            break;
        };
        match which {
            0 => q1 += 1.0,
            1 => {
                q1 -= 1.0;
                q2 += 1.0;
            }
            _ => q2 -= 1.0,
        }
        if q1 >= QUEUE_CAP || q2 >= QUEUE_CAP {
            full = 1.0;
        }
    }
    vec![q1, q2, full]
}

fn main() {
    let model = std::env::args().nth(1).unwrap_or_else(|| "circadian".to_owned());
    let (benchmark, name, shown, system, formula) = match model.as_str() {
        "circadian" => (
            "smc/circadian",
            "barkai_leibler_ctmc",
            "eventually[0,20](a>=100)",
            StochasticSystem::new(
                ["a", "r", "ma", "mr", "peak_a"],
                DT,
                CIRCADIAN_HORIZON,
                |_rng| vec![10.0, 10.0, 5.0, 5.0, 10.0],
                |prev, _t, rng| advance_circadian(prev, rng),
            )
            .expect("the circadian system is well formed"),
            "eventually[0, 20](peak_a >= 100)",
        ),
        "tandem_queue" => (
            "smc/tandem_queue",
            "tandem_queue",
            "eventually[0,50](q1=20 or q2=20)",
            StochasticSystem::new(
                ["q1", "q2", "full"],
                DT,
                TANDEM_HORIZON,
                |_rng| vec![0.0, 0.0, 0.0],
                |prev, _t, rng| advance_tandem(prev, rng),
            )
            .expect("the tandem system is well formed"),
            "eventually[0, 50](full >= 1)",
        ),
        other => {
            eprintln!("no such model: {other}");
            std::process::exit(2);
        }
    };
    let phi = Formula::parse(formula).expect("a valid formula");

    let mut rng = ChaCha8Rng::seed_from_u64(20260304);
    let start = Instant::now();
    let mut hits = 0u64;
    for _ in 0..SAMPLES {
        let trace = system.simulate(&mut rng).expect("a full trajectory");
        if phi.robustness(&trace).expect("a finite robustness") >= 0.0 {
            hits += 1;
        }
    }
    let elapsed_ms = start.elapsed().as_secs_f64() * 1e3;
    let p = hits as f64 / SAMPLES as f64;
    let hw = hardware();
    let stdout = std::io::stdout();
    let _ = writeln!(
        stdout.lock(),
        "{{\"tool\":\"sentil\",\"benchmark\":\"{benchmark}\",\"model\":\"{name}\",\"property\":\"{shown}\",\"probability\":{p:.6},\"samples\":{SAMPLES},\"time_ms\":{elapsed_ms:.1},\"peak_rss_bytes\":{},\"cpu\":{:?}}}",
        peak_rss_bytes().unwrap_or(0),
        hw.cpu
    );
}