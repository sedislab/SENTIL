//! SENTIL vs PRISM, UPPAAL-SMC and Modest on the shared stochastic models

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

const BIO_DT: f64 = 0.01;
const BIO_HORIZON: usize = 10_000;
const PT_DT: f64 = 0.05;
const PT_HORIZON: usize = 1_000;

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
            if ma > 0.0 { 0.5 * ma } else { 0.0 }, // translation A -> a
            if a > 0.0 { 0.2 * a } else { 0.0 }, // degradation A
            if r < CAP { 1.0 * (1.0 + a) } else { 0.0 }, // transcription R -> mr
            if mr > 0.0 { 0.5 * mr } else { 0.0 }, // translation R -> r
            if r > 0.0 { 0.2 * r } else { 0.0 }, // degradation R
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
            if q1 < QUEUE_CAP { 4.8 } else { 0.0 }, // arrival
            if q1 > 0.0 && q2 < QUEUE_CAP { 5.0 } else { 0.0 }, // serve into queue 2
            if q2 > 0.0 { 5.0 } else { 0.0 }, // departure
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

fn advance_biodiesel(state: &[f64], rng: &mut dyn rand::RngCore) -> Vec<f64> {
    let (mut x_e, mut x_tg, mut temp, mut heater, mut reached) =
        (state[0], state[1], state[2], state[3], state[4]);
    if reached >= 0.5 {
        return vec![x_e, x_tg, temp, heater, reached];
    }
    let r1 = 40000.0 * (-5000.0 / temp).exp() * x_tg * (1.0 - x_e);
    let r2 = 2000.0 * (-5500.0 / temp).exp() * x_tg * x_tg;
    let heat = if heater > 0.5 { 500.0 * (350.0 - temp) } else { 10.0 * (298.0 - temp) };
    if rng.random::<f64>() < 0.0002 {
        heater = 0.0;
    }
    x_e = (x_e + 3.0 * r1 * BIO_DT).max(0.0);
    x_tg = (x_tg + (-r1 - r2) * BIO_DT).max(0.0);
    temp += (0.05 * heat + 10.0 * r1) * BIO_DT;
    if x_e >= 0.99 {
        reached = 1.0;
    }
    vec![x_e, x_tg, temp, heater, reached]
}

fn advance_powertrain(state: &[f64], t: f64, rng: &mut dyn rand::RngCore) -> Vec<f64> {
    let (mut afr, mut throttle, ceff) = (state[0], state[1], state[2]);
    let demand = 40.0 + 25.0 * (t * 0.6).sin() + 10.0 * (t * 1.5).sin();
    let dist_mag = ((demand - throttle) * PT_DT * 12.0).abs() * 0.4;
    let dist_sign = if rng.random::<f64>() < 0.5 { -1.0 } else { 1.0 };
    let noise = (rng.random::<f64>() - 0.5) * 0.15;
    let spike = if rng.random::<f64>() < 0.01 { (rng.random::<f64>() - 0.5) * 0.8 } else { 0.0 };
    throttle += (demand - throttle) * PT_DT * 12.0;
    afr += (ceff * 2.5 * (14.7 - afr) + dist_mag * dist_sign + spike) * PT_DT + noise * PT_DT.sqrt();
    vec![afr, throttle, ceff]
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
        "biodiesel" => (
            "smc/biodiesel",
            "biodiesel_reactor",
            "eventually[0,100](reached)",
            StochasticSystem::new(
                ["x_e", "x_tg", "temp", "heater", "reached"],
                BIO_DT,
                BIO_HORIZON,
                |_rng| vec![0.0, 1.0, 300.0, 1.0, 0.0],
                |prev, _t, rng| advance_biodiesel(prev, rng),
            )
            .expect("the biodiesel system is well formed"),
            "eventually[0, 100](reached >= 0.5)",
        ),
        "powertrain" => (
            "smc/powertrain",
            "powertrain_afr",
            "always[0,50](14.3 < afr < 15.1)",
            StochasticSystem::new(
                ["afr", "throttle", "ceff"],
                PT_DT,
                PT_HORIZON,
                |rng| vec![14.7, 10.0, 0.6 + rng.random::<f64>() * 0.4],
                |prev, t, rng| advance_powertrain(prev, t, rng),
            )
            .expect("the powertrain system is well formed"),
            "always[0, 50]((afr > 14.3) and (afr < 15.1))",
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