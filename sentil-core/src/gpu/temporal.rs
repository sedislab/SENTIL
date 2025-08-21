//! Lowering a temporal formula to WGSL.

// The emitters are built up operator by operator and become reachable from the
// Monte Carlo path once it dispatches the temporal case.
#![allow(dead_code)]

use core::fmt::Write as _;

use super::transpiler::{emit_formula, validate, Ssa};
use crate::error::{Error, Result};
use crate::formula::{Formula, Interval};
#[cfg(not(feature = "std"))]
use crate::prelude::*;

/// The longest trace the temporal path evaluates on the device.
pub(crate) const MAX_TEMPORAL_LEN: usize = 512;

/// The most `variables * trace_length` cells of thread-private trace storage a formula may need.
pub(crate) const MAX_TEMPORAL_CELLS: usize = 8192;

pub(crate) struct TemporalShader {
    pub state_size: usize,
    pub trace_len: usize,
    pub evaluate_temporal: String,
}

fn validate_dims(trace_len: usize, num_vars: usize) -> Result<()> {
    if trace_len < 2 {
        return Err(Error::Transpilation {
            message: format!("a temporal window needs at least two samples, got {trace_len}"),
        });
    }
    if trace_len > MAX_TEMPORAL_LEN {
        return Err(Error::Transpilation {
            message: format!(
                "trace length {trace_len} exceeds the GPU temporal limit of {MAX_TEMPORAL_LEN}; this runs on the CPU"
            ),
        });
    }
    if num_vars.max(1) * trace_len > MAX_TEMPORAL_CELLS {
        return Err(Error::Transpilation {
            message: format!(
                "a {num_vars}-variable formula over {trace_len} samples exceeds the GPU temporal cell limit; this runs on the CPU"
            ),
        });
    }
    Ok(())
}

struct Builder<'a> {
    symbols: &'a [String],
    trace_len: usize,
    num_vars: usize,
    helpers: String,
    calls: String,
    next: usize,
}

impl<'a> Builder<'a> {
    fn new(symbols: &'a [String], trace_len: usize) -> Self {
        Self {
            symbols,
            trace_len,
            num_vars: symbols.len().max(1),
            helpers: String::new(),
            calls: String::new(),
            next: 0,
        }
    }

    fn emit(&mut self, formula: &Formula) -> Result<String> {
        match formula {
            // always over [t+a, t+b] is the windowed minimum; eventually the maximum.
            Formula::Always(interval, inner) => {
                let child = self.emit(inner)?;
                Ok(self.emit_forward_window(interval, &child, "min", "0x7f800000u"))
            }
            Formula::Eventually(interval, inner) => {
                let child = self.emit(inner)?;
                Ok(self.emit_forward_window(interval, &child, "max", "0xff800000u"))
            }
            _ if !formula.has_temporal() => self.emit_atemporal(formula),
            _ => Err(Error::Transpilation {
                message: "this temporal operator is not yet on the GPU temporal path".to_owned(),
            }),
        }
    }

    /// Emits a forward windowed reduction (always or eventually): for each index
    /// `i`, reduce the child over `{ j : t[i]+a <= t[j] <= t[i]+b }`. Both interval
    /// bounds are applied; the empty window yields the seed. `seed_bits` is the f32
    /// bit pattern of the seed (`+inf` for min, `-inf` for max), set at runtime
    /// since naga rejects `bitcast` in a constant.
    fn emit_forward_window(
        &mut self,
        interval: &Interval,
        child: &str,
        reduce: &str,
        seed_bits: &str,
    ) -> String {
        let k = self.next;
        self.next += 1;
        let l = self.trace_len;
        let membership = match interval.upper {
            Some(b) => format!("tj >= lo && tj <= (*times)[i] + f32({b:?})"),
            None => "tj >= lo".to_owned(),
        };
        let _ = write!(
            self.helpers,
            "fn node_{k}(times: ptr<function, array<f32, {l}>>, child: ptr<function, array<f32, {l}>>, out: ptr<function, array<f32, {l}>>) {{\n    for (var i = 0u; i < {l}u; i = i + 1u) {{\n        let lo = (*times)[i] + f32({lower:?});\n        var acc = bitcast<f32>({seed_bits});\n        for (var j = 0u; j < {l}u; j = j + 1u) {{\n            let tj = (*times)[j];\n            if ({membership}) {{ acc = {reduce}(acc, (*child)[j]); }}\n        }}\n        (*out)[i] = acc;\n    }}\n}}\n\n",
            lower = interval.lower,
        );
        let _ = write!(
            self.calls,
            "    var n{k}: array<f32, {l}>;\n    node_{k}(times, &{child}, &n{k});\n",
        );
        format!("n{k}")
    }

    /// Emits one node for a boolean combination of predicates.
    fn emit_atemporal(&mut self, formula: &Formula) -> Result<String> {
        let k = self.next;
        self.next += 1;
        let l = self.trace_len;
        let v = self.num_vars;
        let mut ssa = Ssa::new();
        let result = emit_formula(&mut ssa, formula, self.symbols, &|slot| {
            format!("(*trace)[{slot}u][i]")
        })?;
        let _ = write!(
            self.helpers,
            "fn node_{k}(trace: ptr<function, array<array<f32, {l}>, {v}>>, out: ptr<function, array<f32, {l}>>) {{\n    for (var i = 0u; i < {l}u; i = i + 1u) {{\n{body}        (*out)[i] = {result};\n    }}\n}}\n\n",
            body = ssa.body,
        );
        let _ = write!(
            self.calls,
            "    var n{k}: array<f32, {l}>;\n    node_{k}(trace, &n{k});\n",
        );
        Ok(format!("n{k}"))
    }

    /// Assembles the full `evaluate_temporal` function returning `root[0]`.
    fn finish(self, root: &str) -> String {
        let l = self.trace_len;
        let v = self.num_vars;
        let mut source = self.helpers;
        let _ = write!(
            source,
            "fn evaluate_temporal(trace: ptr<function, array<array<f32, {l}>, {v}>>, times: ptr<function, array<f32, {l}>>) -> f32 {{\n{calls}    return {root}[0];\n}}",
            calls = self.calls,
        );
        source
    }
}

/// Transpiles a temporal `formula` into a WGSL `evaluate_temporal` function.
///
/// # Errors
///
/// Returns [`Error::Transpilation`] when the dimensions exceed the device limits or the formula cannot be lowered.
pub(crate) fn transpile_temporal(
    formula: &Formula,
    symbols: &[String],
    trace_len: usize,
) -> Result<TemporalShader> {
    validate_dims(trace_len, symbols.len())?;
    let mut builder = Builder::new(symbols, trace_len);
    let root = builder.emit(formula)?;
    let source = builder.finish(&root);
    validate(&source)?;
    Ok(TemporalShader {
        state_size: symbols.len(),
        trace_len,
        evaluate_temporal: source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn transpiled(formula: &str, trace_len: usize) -> String {
        let f = Formula::parse(formula).unwrap();
        let symbols = f.variables();
        transpile_temporal(&f, &symbols, trace_len)
            .unwrap()
            .evaluate_temporal
    }

    #[test]
    fn a_boolean_subtree_lowers_per_index_and_validates() {
        let source = transpiled("x > 3 and y < 5", 4);
        assert!(source.contains("fn node_0"));
        assert!(source.contains("fn evaluate_temporal"));
        assert!(source.contains("return n0[0]"));
    }

    #[test]
    fn degenerate_or_oversized_dimensions_decline() {
        let f = Formula::parse("x > 0").unwrap();
        let symbols = f.variables();
        assert!(transpile_temporal(&f, &symbols, 1).is_err());
        assert!(transpile_temporal(&f, &symbols, MAX_TEMPORAL_LEN + 1).is_err());
    }

    #[test]
    fn always_and_eventually_lower_to_window_scans() {
        let always = transpiled("always[0, 2](x > 0)", 8);
        assert!(always.contains("acc = min(acc, (*child)[j])"));
        assert!(always.contains("bitcast<f32>(0x7f800000u)")); // +inf seed
        assert!(always.contains("tj >= lo && tj <= (*times)[i] + f32(2.0)"));

        let eventually = transpiled("eventually[1, 3](x > 0)", 8);
        assert!(eventually.contains("acc = max(acc, (*child)[j])"));
        assert!(eventually.contains("bitcast<f32>(0xff800000u)")); // -inf seed

        let unbounded = transpiled("always[0, inf](x > 0)", 8);
        assert!(unbounded.contains("if (tj >= lo)"));
    }
}