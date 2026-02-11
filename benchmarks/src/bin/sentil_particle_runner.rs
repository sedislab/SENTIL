//! Particle-count sweep for the rare-event AMS bench

use std::io::Write;
use std::time::Instant;

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rand_distr::{Distribution, StandardNormal};
use sentil::{Monitor, MonitorConfig, RareEventConfig, StochasticSystem};

struct Event {
    id: &'static str,
    level: f64,
    horizon: usize,
}

const EVENTS: [Event; 2] = [
    Event { id: "moderate", level: 8.0, horizon: 15 },
    Event { id: "rare", level: 14.0, horizon: 12 },
];

const PARTICLES: [usize; 7] = [100, 250, 500, 1000, 2000, 4000, 8000];
const SEEDS: u64 = 5;
const TRUTH_SAMPLES: u64 = 8_000_000;

/// x_0 ~ N(0, 1), then x_{t+1} = x_t + N(0, 1)
fn walk(horizon: usize) -> StochasticSystem {
    StochasticSystem::new(
        ["x"],
        1.0,
        horizon,
        |rng| vec![StandardNormal.sample(rng)],
        |prev, _t, rng| {
            let step: f64 = StandardNormal.sample(rng);
            vec![prev[0] + step]
        },
    )
    .expect("the walk system is well formed")
}

fn monte_carlo_truth(event: &Event) -> f64 {
    let mut rng = ChaCha8Rng::seed_from_u64(0x5e7);
    let mut hits = 0u64;
    for _ in 0..TRUTH_SAMPLES {
        let mut x: f64 = StandardNormal.sample(&mut rng);
        let mut peak = x;
        for _ in 0..event.horizon {
            let step: f64 = StandardNormal.sample(&mut rng);
            x += step;
            if x > peak {
                peak = x;
            }
        }
        if peak >= event.level {
            hits += 1;
        }
    }
    hits as f64 / TRUTH_SAMPLES as f64
}

fn record(event: &Event, truth: f64, particles: usize, seed: u64, out: &mut impl Write) {
    let formula = format!("P>=0.99(always(x < {}))", event.level);
    let config = MonitorConfig::new().rare(RareEventConfig { particles, margin: 0.0, seed });
    let monitor = Monitor::new(&formula, config).expect("the rare-event formula parses");
    let system = walk(event.horizon);
    let start = Instant::now();
    let result = monitor.check_rare(&system).expect("the splitter returns an estimate");
    let elapsed_ms = start.elapsed().as_secs_f64() * 1e3;
    let estimate = result.violation_probability;
    let rel_error = if truth > 0.0 { (estimate - truth).abs() / truth } else { 0.0 };
    let _ = writeln!(
        out,
        "{{\"tool\":\"sentil\",\"benchmark\":\"rare_event/particles\",\"event\":\"{}\",\"level\":{},\"horizon\":{},\"particles\":{},\"seed\":{},\"estimate\":{:.6e},\"truth\":{:.6e},\"rel_error\":{:.4},\"simulations\":{},\"time_ms\":{:.3}}}",
        event.id, event.level, event.horizon, particles, seed, estimate, truth, rel_error, result.simulations, elapsed_ms
    );
}

fn main() {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    for event in &EVENTS {
        let truth = monte_carlo_truth(event);
        for &particles in &PARTICLES {
            for seed in 0..SEEDS {
                record(event, truth, particles, seed, &mut out);
            }
        }
    }
}