//! Lowering a formula to a WGSL compute shader.

use crate::error::{Error, Result};
use crate::formula::{BinaryOp, ComparisonOp, Expr, Formula};
#[cfg(not(feature = "std"))]
use crate::prelude::*;
use core::fmt::Write as _;

pub(crate) struct TranspiledShader {
    pub state_size: usize,
    pub evaluate_formula: String,
}

/// Emits one SSA register per value.
pub(super) struct Ssa {
    pub body: String,
    next: usize,
}

impl Ssa {
    pub(super) fn new() -> Self {
        Self {
            body: String::new(),
            next: 0,
        }
    }

    pub(super) fn bind(&mut self, value: &str) -> String {
        let name = format!("v{}", self.next);
        self.next += 1;
        let _ = writeln!(self.body, "    let {name} = {value};");
        name
    }
}

/// Transpiles a non-temporal `formula` into a WGSL `evaluate_formula` function,
/// resolving variables against `symbols`.
///
/// # Errors
///
/// Returns [`Error::Transpilation`] for an operator the GPU path cannot evaluate, [`Error::UnknownVariable`] for a variable absent from `symbols`, and [`Error::UnknownFunction`] for an unknown function or wrong arity.
pub(crate) fn transpile_atemporal(
    formula: &Formula,
    symbols: &[String],
) -> Result<TranspiledShader> {
    let mut ssa = Ssa::new();
    let result = emit_formula(&mut ssa, formula, symbols, &|slot| {
        format!("state[{slot}u]")
    })?;
    let size = symbols.len().max(1);
    let mut source = format!("fn evaluate_formula(state: array<f32, {size}>) -> f32 {{\n");
    source.push_str(&ssa.body);
    let _ = write!(source, "    return {result};\n}}");
    validate(&source)?;
    Ok(TranspiledShader {
        state_size: symbols.len(),
        evaluate_formula: source,
    })
}

/// Parses and validates WGSL on the CPU.
pub(super) fn validate(source: &str) -> Result<()> {
    let module = naga::front::wgsl::parse_str(source).map_err(|e| Error::Transpilation {
        message: format!("generated shader did not parse: {e}"),
    })?;
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .map_err(|e| Error::Transpilation {
        message: format!("generated shader did not validate: {e:?}"),
    })?;
    Ok(())
}

/// Emits the boolean core of a formula into `ssa` and returns the result register.
pub(super) fn emit_formula(
    ssa: &mut Ssa,
    formula: &Formula,
    symbols: &[String],
    access: &dyn Fn(usize) -> String,
) -> Result<String> {
    match formula {
        Formula::Predicate(p) => {
            let lhs = emit_expr(ssa, &p.lhs, symbols, access)?;
            let rhs = emit_expr(ssa, &p.rhs, symbols, access)?;
            let margin = match p.op {
                ComparisonOp::Less | ComparisonOp::LessEqual => format!("{rhs} - {lhs}"),
                ComparisonOp::Greater | ComparisonOp::GreaterEqual => format!("{lhs} - {rhs}"),
                ComparisonOp::Equal => format!("-abs({lhs} - {rhs})"),
                ComparisonOp::NotEqual => format!("abs({lhs} - {rhs})"),
            };
            Ok(ssa.bind(&margin))
        }
        Formula::Not(f) => {
            let v = emit_formula(ssa, f, symbols, access)?;
            Ok(ssa.bind(&format!("-{v}")))
        }
        Formula::And(l, r) => {
            let lv = emit_formula(ssa, l, symbols, access)?;
            let rv = emit_formula(ssa, r, symbols, access)?;
            Ok(ssa.bind(&format!("min({lv}, {rv})")))
        }
        Formula::Or(l, r) => {
            let lv = emit_formula(ssa, l, symbols, access)?;
            let rv = emit_formula(ssa, r, symbols, access)?;
            Ok(ssa.bind(&format!("max({lv}, {rv})")))
        }
        Formula::Implies(l, r) => {
            let lv = emit_formula(ssa, l, symbols, access)?;
            let rv = emit_formula(ssa, r, symbols, access)?;
            Ok(ssa.bind(&format!("max(-{lv}, {rv})")))
        }
        other => Err(Error::Transpilation {
            message: format!(
                "the GPU path cannot evaluate `{}` here; run this formula on the CPU monitor",
                formula_kind(other)
            ),
        }),
    }
}

fn emit_expr(
    ssa: &mut Ssa,
    expr: &Expr,
    symbols: &[String],
    access: &dyn Fn(usize) -> String,
) -> Result<String> {
    match expr {
        Expr::Literal(v) => Ok(ssa.bind(&format!("f32({v:?})"))),
        Expr::Variable(name) => {
            let slot = symbols
                .iter()
                .position(|n| n == name)
                .ok_or_else(|| Error::UnknownVariable { name: name.clone() })?;
            Ok(ssa.bind(&access(slot)))
        }
        Expr::Binary(op, lhs, rhs) => {
            let l = emit_expr(ssa, lhs, symbols, access)?;
            let r = emit_expr(ssa, rhs, symbols, access)?;
            let value = match op {
                BinaryOp::Add => format!("{l} + {r}"),
                BinaryOp::Sub => format!("{l} - {r}"),
                BinaryOp::Mul => format!("{l} * {r}"),
                BinaryOp::Pow => format!("pow({l}, {r})"),
                BinaryOp::Div => format!("select({l} / {r}, 1e38, abs({r}) < 1e-9)"),
                BinaryOp::Mod => format!("select({l} % {r}, 0.0, abs({r}) < 1e-9)"),
            };
            Ok(ssa.bind(&value))
        }
        Expr::Call(name, args) => emit_call(ssa, name, args, symbols, access),
    }
}

fn emit_call(
    ssa: &mut Ssa,
    name: &str,
    args: &[Expr],
    symbols: &[String],
    access: &dyn Fn(usize) -> String,
) -> Result<String> {
    let unary = |ssa: &mut Ssa, wrap: &dyn Fn(&str) -> String| -> Result<String> {
        let [arg] = args else {
            return Err(Error::UnknownFunction {
                name: name.to_owned(),
                arity: args.len(),
            });
        };
        let a = emit_expr(ssa, arg, symbols, access)?;
        Ok(ssa.bind(&wrap(&a)))
    };
    match name {
        "abs" => unary(ssa, &|a| format!("abs({a})")),
        // The GPU clamps and guards the transcendentals the same way storm does,
        // so a domain edge yields a finite value instead of a NaN.
        "sqrt" => unary(ssa, &|a| format!("sqrt(max({a}, 0.0))")),
        "exp" => unary(ssa, &|a| format!("exp(clamp({a}, -87.0, 87.0))")),
        "ln" => unary(ssa, &|a| format!("log(max({a}, 1e-38))")),
        "log" => unary(ssa, &|a| format!("log(max({a}, 1e-38)) / 2.302585093")),
        "sin" => unary(ssa, &|a| format!("sin({a})")),
        "cos" => unary(ssa, &|a| format!("cos({a})")),
        "tan" => unary(ssa, &|a| format!("tan({a})")),
        "floor" => unary(ssa, &|a| format!("floor({a})")),
        "ceil" => unary(ssa, &|a| format!("ceil({a})")),
        "min" | "max" => {
            let [lhs, rhs] = args else {
                return Err(Error::UnknownFunction {
                    name: name.to_owned(),
                    arity: args.len(),
                });
            };
            let l = emit_expr(ssa, lhs, symbols, access)?;
            let r = emit_expr(ssa, rhs, symbols, access)?;
            Ok(ssa.bind(&format!("{name}({l}, {r})")))
        }
        _ => Err(Error::UnknownFunction {
            name: name.to_owned(),
            arity: args.len(),
        }),
    }
}

fn formula_kind(formula: &Formula) -> &'static str {
    match formula {
        Formula::Always(..) => "always",
        Formula::Eventually(..) => "eventually",
        Formula::Until(..) => "until",
        Formula::Since(..) => "since",
        Formula::Historically(..) => "historically",
        Formula::Once(..) => "once",
        Formula::Next(_) => "next",
        Formula::Probabilistic(..) => "the probabilistic operator P",
        _ => "this operator",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn transpiled(formula: &str) -> String {
        let f = Formula::parse(formula).unwrap();
        let symbols = f.variables();
        transpile_atemporal(&f, &symbols).unwrap().evaluate_formula
    }

    #[test]
    fn predicate_margins_transpile_and_validate() {
        assert!(transpiled("x > 5").contains("= v0 - v1"));
        assert!(transpiled("x < 5").contains("= v1 - v0"));
        assert!(transpiled("x == 5").contains("= -abs(v0 - v1)"));
        assert!(transpiled("x != 5").contains("= abs(v0 - v1)"));
    }

    #[test]
    fn boolean_and_arithmetic_transpile_and_validate() {
        transpiled("x > 0 and y < 10");
        transpiled("x > 0 or y > 0");
        transpiled("not(x > 5)");
        transpiled("(x > 10) implies (y > 0)");
        assert!(transpiled("x / y > 0").contains("select("));
        transpiled("abs(x - 10) + sqrt(y) * 2 > exp(x) / ln(y)");
        transpiled("min(x, y) > max(x, 3)");
    }

    #[test]
    fn temporal_operator_is_a_clear_transpilation_error() {
        let f = Formula::parse("always[0, 5](x > 0)").unwrap();
        let result = transpile_atemporal(&f, &["x".to_string()]);
        assert!(matches!(result, Err(Error::Transpilation { .. })));
    }
}