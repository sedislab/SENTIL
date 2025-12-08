//! Mining the tightest parameter of a parametric specification from data.

#[cfg(not(feature = "std"))]
use crate::prelude::*;
use crate::error::{Error, Result};
use crate::formula::Formula;
use crate::signal::Trace;

/// Finds the tightest value of a single parameter for which the formula holds on
/// every trace, where `make(theta)` builds the formula and `lower`/`upper` bracket
/// the boundary.
///
/// # Errors
///
/// Returns [`Error::InvalidConfig`] if there are no traces or the range does not
/// bracket the boundary, and propagates any error from building or evaluating the
/// formula.
pub fn mine_tightest_parameter<M>(
    make: M,
    traces: &[Trace],
    lower: f64,
    upper: f64,
) -> Result<f64>
where
    M: Fn(f64) -> Result<Formula>,
{
    if traces.is_empty() {
        return Err(config_error("mining needs at least one trace".to_owned()));
    }
    let worst = |theta: f64| -> Result<f64> {
        let phi = make(theta)?;
        let mut margin = f64::INFINITY;
        for trace in traces {
            margin = margin.min(phi.robustness(trace)?);
        }
        Ok(margin)
    };

    let mut lo = lower;
    let mut hi = upper;
    let lo_holds = worst(lo)? >= 0.0;
    if lo_holds == (worst(hi)? >= 0.0) {
        return Err(config_error(
            "the parameter range does not bracket a holding and a failing value".to_owned(),
        ));
    }
    for _ in 0..60 {
        let mid = f64::midpoint(lo, hi);
        if (worst(mid)? >= 0.0) == lo_holds {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    if worst(lo)? >= 0.0 {
        Ok(lo)
    } else {
        Ok(hi)
    }
}

fn config_error(message: String) -> Error {
    Error::InvalidConfig {
        context: "parameter mining",
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trace(values: &[f64]) -> Trace {
        let mut tr = Trace::indexed(values.len());
        tr.add_signal("x", values.to_vec()).unwrap();
        tr
    }

    #[test]
    fn mines_the_tightest_upper_threshold() {
        let traces = [trace(&[1.0, 5.0, 3.0]), trace(&[2.0, 7.0, 4.0])];
        let c = mine_tightest_parameter(
            |c| Formula::parse(&format!("always(x < {c})")),
            &traces,
            0.0,
            100.0,
        )
        .unwrap();
        assert!((c - 7.0).abs() < 1e-3, "mined {c}");
    }

    #[test]
    fn mines_the_tightest_lower_threshold() {
        let traces = [trace(&[3.0, 5.0, 1.0]), trace(&[2.0, 4.0, 6.0])];
        let c = mine_tightest_parameter(
            |c| Formula::parse(&format!("always(x > {c})")),
            &traces,
            -100.0,
            100.0,
        )
        .unwrap();
        assert!((c - 1.0).abs() < 1e-3, "mined {c}");
    }

    #[test]
    fn an_unbracketed_range_is_rejected() {
        let traces = [trace(&[1.0, 2.0])];
        let err = mine_tightest_parameter(
            |c| Formula::parse(&format!("always(x < {c})")),
            &traces,
            50.0,
            100.0,
        );
        assert!(matches!(err, Err(Error::InvalidConfig { .. })));
    }

    #[test]
    fn no_traces_is_rejected() {
        let err = mine_tightest_parameter(|c| Formula::parse(&format!("x < {c}")), &[], 0.0, 1.0);
        assert!(matches!(err, Err(Error::InvalidConfig { .. })));
    }
}