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

pub struct Case {
    pub id: &'static str,
    pub formula: &'static str,
    pub signals: &'static [(&'static str, &'static [f64])],
    pub expected: &'static [f64],
}

pub const DETERMINISTIC: &[Case] = &[
    Case { id: "gt", formula: "x > 0", signals: &[("x", &[5.0, -3.0, 2.0])], expected: &[5.0, -3.0, 2.0] },
    Case { id: "ge_at_boundary", formula: "x >= 5", signals: &[("x", &[5.0, 5.0])], expected: &[0.0, 0.0] },
    Case { id: "gt_at_boundary", formula: "x > 5", signals: &[("x", &[5.0, 5.0])], expected: &[0.0, 0.0] },
    Case { id: "lt", formula: "x < 5", signals: &[("x", &[3.0, 7.0])], expected: &[2.0, -2.0] },
    Case { id: "le_at_boundary", formula: "x <= 5", signals: &[("x", &[5.0])], expected: &[0.0] },
    Case { id: "eq", formula: "x == 5", signals: &[("x", &[5.0, 3.0])], expected: &[-0.0, -2.0] },
    Case { id: "neq", formula: "x != 5", signals: &[("x", &[3.0, 5.0])], expected: &[2.0, 0.0] },
    Case { id: "scaled_predicate", formula: "x * 2 > 5", signals: &[("x", &[4.0])], expected: &[3.0] },
    Case { id: "shifted_predicate", formula: "x - 3 < 5", signals: &[("x", &[10.0])], expected: &[-2.0] },
    Case { id: "abs_predicate", formula: "abs(x) < 5", signals: &[("x", &[-3.0, 6.0])], expected: &[2.0, -1.0] },
    Case { id: "difference_predicate", formula: "x - y > 0", signals: &[("x", &[5.0]), ("y", &[2.0])], expected: &[3.0] },
    Case { id: "and", formula: "(x > 0) and (y > 0)", signals: &[("x", &[5.0, 3.0]), ("y", &[2.0, -1.0])], expected: &[2.0, -1.0] },
    Case { id: "or", formula: "(x > 0) or (y > 0)", signals: &[("x", &[-5.0, 1.0]), ("y", &[2.0, -3.0])], expected: &[2.0, 1.0] },
    Case { id: "not", formula: "not(x > 0)", signals: &[("x", &[5.0, -3.0])], expected: &[-5.0, 3.0] },
    Case { id: "implies_holds", formula: "(x > 0) implies (y > 0)", signals: &[("x", &[5.0]), ("y", &[2.0])], expected: &[2.0] },
    Case { id: "implies_fails", formula: "(x > 0) implies (y > 0)", signals: &[("x", &[5.0]), ("y", &[-2.0])], expected: &[-2.0] },
    Case { id: "nested_boolean", formula: "((x > 0) and (y > 0)) or (z > 0)", signals: &[("x", &[5.0]), ("y", &[3.0]), ("z", &[-1.0])], expected: &[3.0] },
];

#[must_use]
pub fn case_trace(case: &Case) -> Trace {
    let n = case.signals[0].1.len();
    let mut trace = Trace::indexed(n);
    for (name, values) in case.signals {
        trace
            .add_signal(name, values.to_vec())
            .expect("oracle signal matches the grid");
    }
    trace
}

#[cfg(test)]
mod tests {
    use super::{case_trace, trace, CANONICAL, DETERMINISTIC};
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

    #[test]
    fn sentil_reproduces_the_deterministic_oracle() {
        for case in DETERMINISTIC {
            let phi = Formula::parse(case.formula)
                .unwrap_or_else(|e| panic!("{}: parse {e}", case.id));
            let tr = case_trace(case);
            let got = phi
                .robustness_signal(&tr)
                .unwrap_or_else(|e| panic!("{}: {e}", case.id));
            assert_eq!(got.len(), case.expected.len(), "{}: signal length", case.id);
            for (i, (g, e)) in got.iter().zip(case.expected).enumerate() {
                assert_eq!(
                    g.to_bits(),
                    e.to_bits(),
                    "{} at sample {i}: got {g}, want {e}",
                    case.id
                );
            }
        }
    }
}