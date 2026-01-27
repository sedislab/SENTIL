//! No input a user can supply may panic the public API.

use proptest::prelude::*;
use sentil::{Formula, Trace};

fn arb_formula_string() -> impl Strategy<Value = String> {
    let atom = (
        prop::sample::select(vec!["x", "y", "z"]),
        prop::sample::select(vec![">", ">=", "<", "<=", "==", "!="]),
        -5.0f64..5.0f64,
    )
        .prop_map(|(var, op, c)| format!("{var} {op} {c:.3}"));
    atom.prop_recursive(4, 48, 4, |inner| {
        prop_oneof![
            inner.clone().prop_map(|f| format!("not ({f})")),
            inner.clone().prop_map(|f| format!("always ({f})")),
            inner.clone().prop_map(|f| format!("eventually ({f})")),
            inner.clone().prop_map(|f| format!("historically ({f})")),
            inner.clone().prop_map(|f| format!("once ({f})")),
            inner.clone().prop_map(|f| format!("next ({f})")),
            (0u32..10, 10u32..30, inner.clone())
                .prop_map(|(a, b, f)| format!("always[{a}, {b}] ({f})")),
            (0u32..10, 10u32..30, inner.clone())
                .prop_map(|(a, b, f)| format!("eventually[{a}, {b}] ({f})")),
            (inner.clone(), inner.clone()).prop_map(|(a, b)| format!("({a}) and ({b})")),
            (inner.clone(), inner.clone()).prop_map(|(a, b)| format!("({a}) or ({b})")),
            (inner.clone(), inner.clone()).prop_map(|(a, b)| format!("({a}) implies ({b})")),
            (inner.clone(), inner.clone()).prop_map(|(a, b)| format!("({a}) until ({b})")),
            (inner.clone(), inner.clone()).prop_map(|(a, b)| format!("({a}) since ({b})")),
            (0.5f64..1.0, inner).prop_map(|(p, f)| format!("P>={p:.3}({f})")),
        ]
    })
}

fn build_trace(rows: &[(f64, f64, f64)]) -> Option<Trace> {
    let times: Vec<f64> = (0..rows.len()).map(|i| i as f64).collect();
    let mut trace = Trace::new(times).ok()?;
    trace.add_signal("x", rows.iter().map(|r| r.0).collect::<Vec<_>>()).ok()?;
    trace.add_signal("y", rows.iter().map(|r| r.1).collect::<Vec<_>>()).ok()?;
    trace.add_signal("z", rows.iter().map(|r| r.2).collect::<Vec<_>>()).ok()?;
    Some(trace)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(400))]

    #[test]
    fn parsing_never_panics(text in ".{0,120}") {
        let _ = Formula::parse(&text);
    }

    #[test]
    #[cfg(feature = "ingest")]
    fn ingest_never_panics(text in ".{0,200}") {
        let _ = Trace::from_csv_str(&text);
        let _ = Trace::from_tsv_str(&text);
    }

    #[test]
    fn trace_construction_never_panics(
        times in prop::collection::vec(prop::num::f64::ANY, 0..24),
        values in prop::collection::vec(prop::num::f64::ANY, 0..24),
    ) {
        if let Ok(mut trace) = Trace::new(times) {
            let _ = trace.add_signal("x", values);
        }
    }

    #[test]
    fn evaluation_never_panics(
        text in arb_formula_string(),
        rows in prop::collection::vec(
            (prop::num::f64::ANY, prop::num::f64::ANY, prop::num::f64::ANY),
            0..24,
        ),
    ) {
        let Ok(formula) = Formula::parse(&text) else { return Ok(()); };
        let Some(trace) = build_trace(&rows) else { return Ok(()); };
        let _ = formula.robustness(&trace);
        let _ = formula.robustness_dense(&trace);
        let _ = formula.robustness_signal(&trace);
    }
}