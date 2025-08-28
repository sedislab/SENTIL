//! Fixed signals and formulas with their robustness

use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use sentil::Trace;

pub const SIGNAL_SEED: u64 = 42;

pub const CANONICAL: &[(&str, f64)] = &[
    ("always[0, 10](x < 5)", -7.622_064_772_118_447),
    ("eventually[0, 50](x > 10)", 4.993_604_045_622_577),
    ("always[0, 100](eventually[0, 10](p > 0))", 1.0),
    ("(p > 0) implies (eventually[0, 20](q > 0))", 1.0),
    ("always[0, 200]((p > 0) and (eventually[5, 15](q > 0)))", -1.0),
];

#[must_use]
pub fn trace(n: usize) -> Trace {
    let mut rng = ChaCha8Rng::seed_from_u64(SIGNAL_SEED);
    let mut x = Vec::with_capacity(n);
    let mut p = Vec::with_capacity(n);
    let mut q = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f64 * 0.1;
        x.push(15.0 * t.sin());
        p.push(if (i / 10) % 2 == 0 { 1.0 } else { -1.0 });
        q.push(if rng.random_bool(0.5) { 1.0 } else { -1.0 });
    }
    let mut trace = Trace::indexed(n);
    trace.add_signal("x", x).expect("x matches the time grid");
    trace.add_signal("p", p).expect("p matches the time grid");
    trace.add_signal("q", q).expect("q matches the time grid");
    trace
}

#[cfg(test)]
mod tests {
    use super::{trace, CANONICAL};
    use sentil::Formula;

    #[test]
    fn sentil_reproduces_every_oracle_value() {
        let tr = trace(2001);
        for (formula, expected) in CANONICAL {
            let phi = Formula::parse(formula).expect("a valid oracle formula");
            let got = phi.robustness(&tr).expect("a finite robustness");
            assert_eq!(
                got.to_bits(),
                expected.to_bits(),
                "{formula}: got {got}, expected {expected}"
            );
        }
    }
}