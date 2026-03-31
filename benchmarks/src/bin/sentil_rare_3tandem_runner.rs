//! Three queues in series. Arrivals at rate 3 feed queue one, which drains into queue two at rate 2, queue two into queue three at rate 4, and queue three
//! out at rate 6, all exponential, each bounded at capacity c. The rare event is queue three overflowing to c before it empties, from one packet in queue three.

use std::io::Write;
use std::time::Instant;

use rand::RngCore;
use sentil::stats::{adaptive_multilevel_splitting, RareEventSimulator};

struct ThreeTandem {
    c: u32,
}

impl RareEventSimulator for ThreeTandem {
    type State = (u32, u32, u32);
    fn initial_state(&self, _rng: &mut dyn RngCore) -> (u32, u32, u32) {
        (0, 0, 1)
    }
    fn step(&self, s: &(u32, u32, u32), rng: &mut dyn RngCore) -> (u32, u32, u32) {
        let (q1, q2, q3) = *s;
        let (lam, mu1, mu2, mu3) = (3.0, 2.0, 4.0, 6.0);
        let r1 = if q1 > 0 { mu1 } else { 0.0 };
        let r2 = if q2 > 0 { mu2 } else { 0.0 };
        let r3 = if q3 > 0 { mu3 } else { 0.0 };
        let total = lam + r1 + r2 + r3;
        let u = (rng.next_u64() as f64 / u64::MAX as f64) * total;
        if u < lam {
            (if q1 < self.c { q1 + 1 } else { q1 }, q2, q3)
        } else if u < lam + r1 {
            (q1 - 1, if q2 < self.c { q2 + 1 } else { q2 }, q3)
        } else if u < lam + r1 + r2 {
            (q1, q2 - 1, if q3 < self.c { q3 + 1 } else { q3 })
        } else {
            (q1, q2, q3 - 1)
        }
    }
    fn is_terminal(&self, s: &(u32, u32, u32)) -> (bool, bool) {
        (s.2 == 0 || s.2 == self.c, s.2 == self.c)
    }
    fn score(&self, s: &(u32, u32, u32)) -> f64 {
        f64::from(s.2)
    }
}

#[allow(clippy::cast_precision_loss)]
fn main() {
    let c: u32 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(6);
    let particles: usize = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(20_000);
    let truth: f64 = std::env::args().nth(3).and_then(|s| s.parse().ok()).unwrap_or(f64::NAN);
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    let sim = ThreeTandem { c };
    let seeds = 20u64;
    let mut ests = Vec::new();
    let mut total_ms = 0.0;
    let mut sims = 0u64;
    for seed in 0..seeds {
        let start = Instant::now();
        let e = adaptive_multilevel_splitting(&sim, particles, f64::from(c), 4_000_000, seed)
            .expect("estimate");
        total_ms += start.elapsed().as_secs_f64() * 1e3;
        ests.push(e.probability);
        sims = e.simulations;
    }
    let mean = ests.iter().sum::<f64>() / ests.len() as f64;
    let var = ests.iter().map(|e| (e - mean).powi(2)).sum::<f64>() / ests.len() as f64;
    let rel_err = if truth.is_finite() { (mean - truth).abs() / truth } else { f64::NAN };
    let _ = writeln!(
        out,
        "{{\"tool\":\"sentil\",\"benchmark\":\"rare_event/three_tandem\",\"model\":\"three_tandem\",\"c\":{c},\"particles\":{particles},\"estimate\":{mean:.6e},\"truth\":{truth:.6e},\"rel_error\":{rel_err:.4},\"rel_std\":{:.4},\"simulations\":{sims},\"time_ms\":{:.3},\"seeds\":{seeds}}}",
        var.sqrt() / mean,
        total_ms / seeds as f64
    );
}