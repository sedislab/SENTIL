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
            // always/historically reduce by minimum, eventually/once by maximum;
            // always/eventually use the forward window [t+a, t+b], the past mirrors
            // use the backward window [t-b, t-a].
            Formula::Always(interval, inner) => {
                let child = self.emit(inner)?;
                Ok(self.emit_window(interval, &child, "min", "0x7f800000u", true))
            }
            Formula::Eventually(interval, inner) => {
                let child = self.emit(inner)?;
                Ok(self.emit_window(interval, &child, "max", "0xff800000u", true))
            }
            Formula::Historically(interval, inner) => {
                let child = self.emit(inner)?;
                Ok(self.emit_window(interval, &child, "min", "0x7f800000u", false))
            }
            Formula::Once(interval, inner) => {
                let child = self.emit(inner)?;
                Ok(self.emit_window(interval, &child, "max", "0xff800000u", false))
            }
            Formula::Until(interval, lhs, rhs) => {
                let lphi = self.emit(lhs)?;
                let rpsi = self.emit(rhs)?;
                Ok(self.emit_until(interval, &lphi, &rpsi))
            }
            Formula::Since(interval, lhs, rhs) => {
                let lphi = self.emit(lhs)?;
                let rpsi = self.emit(rhs)?;
                Ok(self.emit_since(interval, &lphi, &rpsi))
            }
            Formula::Next(inner) => {
                let child = self.emit(inner)?;
                Ok(self.emit_next(&child))
            }
            // A boolean operator with a temporal child combines the children's
            // per-index arrays. A wholly atemporal one collapses to a single node.
            Formula::Not(inner) if inner.has_temporal() => {
                let child = self.emit(inner)?;
                Ok(self.emit_combine(&[&child], "-(*c0)[i]"))
            }
            Formula::And(l, r) if l.has_temporal() || r.has_temporal() => {
                let (lc, rc) = (self.emit(l)?, self.emit(r)?);
                Ok(self.emit_combine(&[&lc, &rc], "min((*c0)[i], (*c1)[i])"))
            }
            Formula::Or(l, r) if l.has_temporal() || r.has_temporal() => {
                let (lc, rc) = (self.emit(l)?, self.emit(r)?);
                Ok(self.emit_combine(&[&lc, &rc], "max((*c0)[i], (*c1)[i])"))
            }
            Formula::Implies(l, r) if l.has_temporal() || r.has_temporal() => {
                let (lc, rc) = (self.emit(l)?, self.emit(r)?);
                Ok(self.emit_combine(&[&lc, &rc], "max(-(*c0)[i], (*c1)[i])"))
            }
            _ => self.emit_atemporal(formula),
        }
    }

    /// Emits a windowed reduction over `[t[i]+a, t[i]+b]` forward or `[t[i]-b, t[i]-a]` backward.
    /// naga rejects `bitcast` in a constant, so `seed_bits` is applied at runtime.
    fn emit_window(
        &mut self,
        interval: &Interval,
        child: &str,
        reduce: &str,
        seed_bits: &str,
        forward: bool,
    ) -> String {
        let k = self.next;
        self.next += 1;
        let l = self.trace_len;
        let a = interval.lower;
        let membership = match (forward, interval.upper) {
            (true, Some(b)) => {
                format!("tj >= (*times)[i] + f32({a:?}) && tj <= (*times)[i] + f32({b:?})")
            }
            (true, None) => format!("tj >= (*times)[i] + f32({a:?})"),
            (false, Some(b)) => {
                format!("tj >= (*times)[i] - f32({b:?}) && tj <= (*times)[i] - f32({a:?})")
            }
            (false, None) => format!("tj <= (*times)[i] - f32({a:?})"),
        };
        let _ = write!(
            self.helpers,
            "fn node_{k}(times: ptr<function, array<f32, {l}>>, child: ptr<function, array<f32, {l}>>, out: ptr<function, array<f32, {l}>>) {{\n    for (var i = 0u; i < {l}u; i = i + 1u) {{\n        var acc = bitcast<f32>({seed_bits});\n        for (var j = 0u; j < {l}u; j = j + 1u) {{\n            let tj = (*times)[j];\n            if ({membership}) {{ acc = {reduce}(acc, (*child)[j]); }}\n        }}\n        (*out)[i] = acc;\n    }}\n}}\n\n",
        );
        let _ = write!(
            self.calls,
            "    var n{k}: array<f32, {l}>;\n    node_{k}(times, &{child}, &n{k});\n",
        );
        format!("n{k}")
    }

    /// Emits until: for each `i`, the supremum over `s` in `[t[i]+a, t[i]+b]` of `min(psi(s), inf of phi over (t[i], s])`.
    fn emit_until(&mut self, interval: &Interval, lphi: &str, rpsi: &str) -> String {
        let k = self.next;
        self.next += 1;
        let l = self.trace_len;
        let a = interval.lower;
        let upper_break = match interval.upper {
            Some(b) => {
                format!("            if ((*times)[j] > (*times)[i] + f32({b:?})) {{ break; }}\n")
            }
            None => String::new(),
        };
        let _ = write!(
            self.helpers,
            "fn node_{k}(times: ptr<function, array<f32, {l}>>, lphi: ptr<function, array<f32, {l}>>, rpsi: ptr<function, array<f32, {l}>>, out: ptr<function, array<f32, {l}>>) {{\n    for (var i = 0u; i < {l}u; i = i + 1u) {{\n        let ws = (*times)[i] + f32({a:?});\n        var best = bitcast<f32>(0xff800000u);\n        if (ws <= (*times)[{l}u - 1u]) {{\n            var min_phi = bitcast<f32>(0x7f800000u);\n            for (var j = i; j < {l}u; j = j + 1u) {{\n                if (j > i) {{ min_phi = min(min_phi, (*lphi)[j - 1u]); }}\n{upper_break}                if ((*times)[j] >= ws) {{ best = max(best, min((*rpsi)[j], min_phi)); }}\n            }}\n        }}\n        (*out)[i] = best;\n    }}\n}}\n\n",
        );
        let _ = write!(
            self.calls,
            "    var n{k}: array<f32, {l}>;\n    node_{k}(times, &{lphi}, &{rpsi}, &n{k});\n",
        );
        format!("n{k}")
    }

    /// Emits since, the past mirror of until.
    fn emit_since(&mut self, interval: &Interval, lphi: &str, rpsi: &str) -> String {
        let k = self.next;
        self.next += 1;
        let l = self.trace_len;
        let a = interval.lower;
        let lower_break = match interval.upper {
            Some(b) => {
                format!("            if ((*times)[j] < (*times)[i] - f32({b:?})) {{ break; }}\n")
            }
            None => String::new(),
        };
        let _ = write!(
            self.helpers,
            "fn node_{k}(times: ptr<function, array<f32, {l}>>, lphi: ptr<function, array<f32, {l}>>, rpsi: ptr<function, array<f32, {l}>>, out: ptr<function, array<f32, {l}>>) {{\n    for (var i = 0u; i < {l}u; i = i + 1u) {{\n        let we = (*times)[i] - f32({a:?});\n        var best = bitcast<f32>(0xff800000u);\n        if (we >= (*times)[0u]) {{\n            var min_phi = bitcast<f32>(0x7f800000u);\n            for (var jj = 0u; jj <= i; jj = jj + 1u) {{\n                let j = i - jj;\n                if (jj > 0u) {{ min_phi = min(min_phi, (*lphi)[j + 1u]); }}\n{lower_break}                if ((*times)[j] <= we) {{ best = max(best, min((*rpsi)[j], min_phi)); }}\n            }}\n        }}\n        (*out)[i] = best;\n    }}\n}}\n\n",
        );
        let _ = write!(
            self.calls,
            "    var n{k}: array<f32, {l}>;\n    node_{k}(times, &{lphi}, &{rpsi}, &n{k});\n",
        );
        format!("n{k}")
    }

    /// Emits next: shift the child one index earlier, with `-inf` at the last index.
    fn emit_next(&mut self, child: &str) -> String {
        let k = self.next;
        self.next += 1;
        let l = self.trace_len;
        let _ = write!(
            self.helpers,
            "fn node_{k}(child: ptr<function, array<f32, {l}>>, out: ptr<function, array<f32, {l}>>) {{\n    for (var i = 0u; i + 1u < {l}u; i = i + 1u) {{\n        (*out)[i] = (*child)[i + 1u];\n    }}\n    (*out)[{l}u - 1u] = bitcast<f32>(0xff800000u);\n}}\n\n",
        );
        let _ = write!(
            self.calls,
            "    var n{k}: array<f32, {l}>;\n    node_{k}(&{child}, &n{k});\n",
        );
        format!("n{k}")
    }

    /// Emits a per-index boolean combiner over child arrays, read as `(*c0)[i]`.
    fn emit_combine(&mut self, children: &[&str], expr: &str) -> String {
        let k = self.next;
        self.next += 1;
        let l = self.trace_len;
        let mut params = String::new();
        let mut args = String::new();
        for (idx, child) in children.iter().enumerate() {
            let _ = write!(params, "c{idx}: ptr<function, array<f32, {l}>>, ");
            let _ = write!(args, "&{child}, ");
        }
        let _ = write!(
            self.helpers,
            "fn node_{k}({params}out: ptr<function, array<f32, {l}>>) {{\n    for (var i = 0u; i < {l}u; i = i + 1u) {{\n        (*out)[i] = {expr};\n    }}\n}}\n\n",
        );
        let _ = write!(
            self.calls,
            "    var n{k}: array<f32, {l}>;\n    node_{k}({args}&n{k});\n",
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
        assert!(always.contains("tj >= (*times)[i] + f32(0.0) && tj <= (*times)[i] + f32(2.0)"));

        let eventually = transpiled("eventually[1, 3](x > 0)", 8);
        assert!(eventually.contains("acc = max(acc, (*child)[j])"));
        assert!(eventually.contains("bitcast<f32>(0xff800000u)")); // -inf seed

        let unbounded = transpiled("always[0, inf](x > 0)", 8);
        assert!(unbounded.contains("if (tj >= (*times)[i] + f32(0.0))"));
    }

    #[test]
    fn historically_and_once_use_the_backward_window() {
        let hist = transpiled("historically[1, 3](x > 0)", 8);
        assert!(hist.contains("acc = min(acc, (*child)[j])"));
        assert!(hist.contains("tj >= (*times)[i] - f32(3.0) && tj <= (*times)[i] - f32(1.0)"));

        let once = transpiled("once[0, inf](x > 0)", 8);
        assert!(once.contains("acc = max(acc, (*child)[j])"));
        assert!(once.contains("if (tj <= (*times)[i] - f32(0.0))"));
    }

    #[test]
    fn until_and_since_lower_to_the_sup_min_double_loop() {
        let until = transpiled("(x > 0) until[0, 3] (y > 0)", 8);
        assert!(until.contains("min_phi = min(min_phi, (*lphi)[j - 1u])"));
        assert!(until.contains("best = max(best, min((*rpsi)[j], min_phi))"));

        let since = transpiled("(x > 0) since[1, 2] (y > 0)", 8);
        assert!(since.contains("let j = i - jj"));
        assert!(since.contains("min_phi = min(min_phi, (*lphi)[j + 1u])"));
    }

    #[test]
    fn next_shifts_the_index_with_a_minus_inf_tail() {
        let next = transpiled("next(x > 0)", 8);
        assert!(next.contains("(*out)[i] = (*child)[i + 1u]"));
        assert!(next.contains("(*out)[8u - 1u] = bitcast<f32>(0xff800000u)"));
    }

    #[test]
    fn nesting_and_booleans_over_temporal_children_compose() {
        let nested = transpiled("always[0, 2](eventually[0, 1](x > 0))", 8);
        assert!(nested.contains("fn node_0") && nested.contains("fn node_2"));
        let conj = transpiled("always[0, 2](x > 0) and eventually[0, 2](y > 0)", 8);
        assert!(conj.contains("(*out)[i] = min((*c0)[i], (*c1)[i])"));
        let neg = transpiled("not(always[0, 2](x > 0))", 8);
        assert!(neg.contains("(*out)[i] = -(*c0)[i]"));
    }
}