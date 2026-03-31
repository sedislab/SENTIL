use std::io::Write;
use std::time::Instant;

use rand::{rngs::StdRng, RngCore, SeedableRng};
use sentil::{Formula, RareEventConfig, StochasticSystem};
#[cfg(feature = "gpu")]
use sentil::{NoiseModel, SimExpr, SimModel};

const A: f64 = 0.9;
const HORIZON: usize = 40;

fn normal(rng: &mut dyn RngCore) -> f64 {
    let u1 = ((rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64).max(1e-12);
    let u2 = (rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
    (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
}

fn system() -> StochasticSystem {
    StochasticSystem::new(
        ["x"],
        1.0,
        HORIZON,
        |_rng| vec![0.0],
        |prev, _t, rng| vec![A * prev[0] + normal(rng)],
    )
    .expect("the AR(1) system is well formed")
}

#[allow(clippy::cast_precision_loss)]
fn monte_carlo_truth(level: f64, samples: u64) -> f64 {
    let mut rng = StdRng::seed_from_u64(0x0177);
    let mut hits = 0u64;
    for _ in 0..samples {
        let mut x = 0.0f64;
        let mut crossed = false;
        for _ in 0..HORIZON {
            x = A * x + normal(&mut rng);
            if x >= level {
                crossed = true;
                break;
            }
        }
        hits += u64::from(crossed);
    }
    hits as f64 / samples as f64
}

#[allow(clippy::cast_precision_loss)]
fn main() {
    let level: f64 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(7.0);
    let particles: usize = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(20_000);
    let truth_samples: u64 = std::env::args().nth(3).and_then(|s| s.parse().ok()).unwrap_or(50_000_000);
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    let formula =
        Formula::parse(&format!("P>=0.99 (always[0,{HORIZON}] (x < {level}))")).expect("parses");
    let config = RareEventConfig { particles, margin: 0.0, seed: 1 };
    let truth = monte_carlo_truth(level, truth_samples);

    let sys = system();
    let mut cpu_ms = 0.0;
    let mut cpu_est = 0.0;
    let seeds = 5u64;
    for seed in 0..seeds {
        let start = Instant::now();
        let r = formula
            .check_rare_event(&sys, &RareEventConfig { seed, ..config })
            .expect("cpu estimate");
        cpu_ms += start.elapsed().as_secs_f64() * 1e3;
        cpu_est += r.violation_probability;
    }
    let _ = writeln!(
        out,
        "{{\"tool\":\"sentil\",\"benchmark\":\"rare_event/ou_walk\",\"model\":\"ar1_walk\",\"device\":\"cpu\",\"level\":{level},\"particles\":{particles},\"estimate\":{:.6e},\"truth\":{truth:.6e},\"time_ms\":{:.3}}}",
        cpu_est / seeds as f64,
        cpu_ms / seeds as f64
    );

    #[cfg(feature = "gpu")]
    if sentil::gpu::is_available() {
        let advance = SimExpr::Add(
            Box::new(SimExpr::Mul(
                Box::new(SimExpr::Const(A)),
                Box::new(SimExpr::Prev(0)),
            )),
            Box::new(SimExpr::Noise(0)),
        );
        let model = SimModel::new(
            ["x"],
            1.0,
            HORIZON,
            vec![SimExpr::Const(0.0)],
            vec![advance],
            vec![NoiseModel::gaussian(0.0, 1.0).expect("gaussian")],
        )
        .expect("sim model");
        let mut gpu_ms = 0.0;
        let mut gpu_est = 0.0;
        for seed in 0..seeds {
            let start = Instant::now();
            let e = formula
                .check_rare_event_gpu(&model, &RareEventConfig { seed, ..config })
                .expect("gpu estimate");
            gpu_ms += start.elapsed().as_secs_f64() * 1e3;
            gpu_est += e.violation_probability;
        }
        let _ = writeln!(
            out,
            "{{\"tool\":\"sentil\",\"benchmark\":\"rare_event/ou_walk\",\"model\":\"ar1_walk\",\"device\":\"gpu\",\"level\":{level},\"particles\":{particles},\"estimate\":{:.6e},\"truth\":{truth:.6e},\"time_ms\":{:.3}}}",
            gpu_est / seeds as f64,
            gpu_ms / seeds as f64
        );
    }
}