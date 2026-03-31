//! Rare-event estimation on the tandem queue

use std::io::Write;
use std::time::Instant;

use rand::RngCore;
use sentil::stats::{adaptive_multilevel_splitting, RareEventSimulator};

struct Tandem {
    c: u32,
}

impl RareEventSimulator for Tandem {
    type State = (u32, u32);
    fn initial_state(&self, _rng: &mut dyn RngCore) -> (u32, u32) {
        (0, 1)
    }
    fn step(&self, s: &(u32, u32), rng: &mut dyn RngCore) -> (u32, u32) {
        let (q1, q2) = *s;
        let (lam, mu1, mu2) = (3.0, 2.0, 6.0);
        let rq1 = if q1 > 0 { mu1 } else { 0.0 };
        let rq2 = if q2 > 0 { mu2 } else { 0.0 };
        let total = lam + rq1 + rq2;
        let u = (rng.next_u64() as f64 / u64::MAX as f64) * total;
        if u < lam {
            (if q1 < self.c { q1 + 1 } else { q1 }, q2)
        } else if u < lam + rq1 {
            (q1 - 1, if q2 < self.c { q2 + 1 } else { q2 })
        } else {
            (q1, q2 - 1)
        }
    }
    fn is_terminal(&self, s: &(u32, u32)) -> (bool, bool) {
        (s.1 == 0 || s.1 == self.c, s.1 == self.c)
    }
    fn score(&self, s: &(u32, u32)) -> f64 {
        f64::from(s.1)
    }
}

// results
fn exact(c: u32) -> f64 {
    match c {
        8 => 5.602364e-6,
        10 => 3.147278e-7,
        12 => 1.860151e-8,
        _ => f64::NAN,
    }
}

#[allow(clippy::cast_precision_loss)]
fn main() {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    for &(c, particles) in &[
        (8u32, 4000usize),
        (8, 16000),
        (8, 64000),
        (10, 8000),
        (12, 16000),
    ] {
        let sim = Tandem { c };
        let truth = exact(c);
        let seeds = 20u64;
        let mut ests = Vec::new();
        let mut total_ms = 0.0;
        let mut sims = 0u64;
        for seed in 0..seeds {
            let start = Instant::now();
            let e = adaptive_multilevel_splitting(&sim, particles, f64::from(c), 2_000_000, seed)
                .expect("estimate");
            total_ms += start.elapsed().as_secs_f64() * 1e3;
            ests.push(e.probability);
            sims = e.simulations;
        }
        let mean = ests.iter().sum::<f64>() / ests.len() as f64;
        let var = ests.iter().map(|e| (e - mean).powi(2)).sum::<f64>() / ests.len() as f64;
        let rel_std = var.sqrt() / mean;
        let _ = writeln!(
            out,
            "{{\"tool\":\"sentil\",\"benchmark\":\"rare_event/tandem\",\"model\":\"tandem_queue\",\"c\":{c},\"particles\":{particles},\"estimate\":{mean:.6e},\"truth\":{truth:.6e},\"rel_error\":{:.4},\"rel_std\":{rel_std:.4},\"simulations\":{sims},\"time_ms\":{:.3},\"seeds\":{seeds}}}",
            (mean - truth).abs() / truth,
            total_ms / seeds as f64
        );
    }
}