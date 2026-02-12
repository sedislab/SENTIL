//! SENTIL against PRISM on one shared model.
//!
//! PRISM checks the Barkai-Leibler circadian CTMC by statistical model checking; this
//! runs SENTIL on the identical model and property so the two are comparable. The CTMC
//! is simulated exactly with Gillespie's direct method inside the system's step, and
//! the satisfaction probability of `eventually[0,20](a >= 100)` is estimated by direct
//! Monte Carlo, which suits a discrete-state model where the splitter's level selection
//! would bias on tied scores. Prints the shared JSON record.

use std::io::Write;
use std::time::Instant;

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use sentil::{Formula, StochasticSystem};
use sentil_benchmarks::measure::{hardware, peak_rss_bytes};

const CAP: f64 = 1000.0;
const DT: f64 = 1.0;
const HORIZON: usize = 20;
const SAMPLES: u64 = 10_000;

// One Gillespie step from `t` to `t + DT` over the state [a, r, ma, mr], carrying the
// running peak of `a` in the fifth slot so the property sees a crossing that happens
// between grid points, the way PRISM's continuous F<=20 does.
fn advance(state: &[f64], rng: &mut dyn rand::RngCore) -> Vec<f64> {
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
        let total: f64 = rates.iter().sum();
        if total <= 0.0 {
            break;
        }
        clock += -(1.0 - rng.random::<f64>()).ln() / total;
        if clock > DT {
            break;
        }
        let mut pick = rng.random::<f64>() * total;
        let mut which = 0;
        while which < rates.len() - 1 && pick >= rates[which] {
            pick -= rates[which];
            which += 1;
        }
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

fn main() {
    let system = StochasticSystem::new(
        ["a", "r", "ma", "mr", "peak_a"],
        DT,
        HORIZON,
        |_rng| vec![10.0, 10.0, 5.0, 5.0, 10.0],
        |prev, _t, rng| advance(prev, rng),
    )
    .expect("the circadian system is well formed");
    let phi = Formula::parse("eventually[0, 20](peak_a >= 100)").expect("a valid formula");

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
        "{{\"tool\":\"sentil\",\"benchmark\":\"smc/circadian\",\"model\":\"barkai_leibler_ctmc\",\"property\":\"eventually[0,20](a>=100)\",\"probability\":{p:.6},\"samples\":{SAMPLES},\"time_ms\":{elapsed_ms:.1},\"peak_rss_bytes\":{},\"cpu\":{:?}}}",
        peak_rss_bytes().unwrap_or(0),
        hw.cpu
    );
}