//! GPU adaptive multilevel splitting for rare-event probabilities.

// The transpiler and packing are consumed by the splitting kernels and context as
// they land; until then they are exercised through this module's tests.
#![allow(
    dead_code,
    reason = "consumed by the splitting kernels and context as they land"
)]

use core::fmt::Write as _;

use super::monte_carlo::{write_draw_residual, GpuMcError, NOISE_RECORD, PRNG_WGSL};
use super::transpiler::{emit_formula, Ssa};
use crate::error::{Error, Result};
use crate::formula::Formula;
use crate::stats::{GpuSampler, NoiseModel, SimExpr, SimModel};

/// Packs the model's noise sources into the device buffer, one record per source.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "the device runs in f32; the family tag and parameters fit it"
)]
fn pack_sim_noise(noise: &[NoiseModel]) -> core::result::Result<Vec<f32>, GpuMcError> {
    let mut packed = vec![0.0f32; noise.len().max(1) * NOISE_RECORD];
    for (id, model) in noise.iter().enumerate() {
        let (family, params) = match model.gpu_sampler() {
            GpuSampler::Device { family, params } => (family, params),
            GpuSampler::Cpu { family } => {
                return Err(GpuMcError::UnsupportedNoiseFamily { family })
            }
        };
        let base = id * NOISE_RECORD;
        packed[base] = family as f32;
        for (i, &p) in params.iter().enumerate() {
            packed[base + 2 + i] = p as f32;
        }
    }
    Ok(packed)
}

/// Lowers a [`SimExpr`] to a WGSL value.
fn emit_sim_expr(ssa: &mut Ssa, expr: &SimExpr) -> Result<String> {
    match expr {
        SimExpr::Prev(d) => Ok(ssa.bind(&format!("(*prev)[{d}u]"))),
        SimExpr::Time => Ok(ssa.bind("t")),
        SimExpr::Const(c) => Ok(ssa.bind(&format!("f32({c:?})"))),
        SimExpr::Add(a, b) => {
            let (l, r) = (emit_sim_expr(ssa, a)?, emit_sim_expr(ssa, b)?);
            Ok(ssa.bind(&format!("{l} + {r}")))
        }
        SimExpr::Sub(a, b) => {
            let (l, r) = (emit_sim_expr(ssa, a)?, emit_sim_expr(ssa, b)?);
            Ok(ssa.bind(&format!("{l} - {r}")))
        }
        SimExpr::Mul(a, b) => {
            let (l, r) = (emit_sim_expr(ssa, a)?, emit_sim_expr(ssa, b)?);
            Ok(ssa.bind(&format!("{l} * {r}")))
        }
        SimExpr::Div(a, b) => {
            let (l, r) = (emit_sim_expr(ssa, a)?, emit_sim_expr(ssa, b)?);
            Ok(ssa.bind(&format!("select({l} / {r}, 1e38, abs({r}) < 1e-9)")))
        }
        SimExpr::Call(name, args) => emit_sim_call(ssa, name, args),
        SimExpr::Noise(id) => Ok(ssa.bind(&format!("draw_residual({id}u, rng)"))),
    }
}

/// Lowers a whitelisted function call.
fn emit_sim_call(ssa: &mut Ssa, name: &str, args: &[SimExpr]) -> Result<String> {
    let lowered = args
        .iter()
        .map(|arg| emit_sim_expr(ssa, arg))
        .collect::<Result<Vec<_>>>()?;
    let value = match (name, lowered.as_slice()) {
        ("abs", [a]) => format!("abs({a})"),
        ("sqrt", [a]) => format!("sqrt(max({a}, 0.0))"),
        ("exp", [a]) => format!("exp(clamp({a}, -87.0, 87.0))"),
        ("ln", [a]) => format!("log(max({a}, 1e-38))"),
        ("log", [a]) => format!("log(max({a}, 1e-38)) / 2.302585093"),
        ("sin", [a]) => format!("sin({a})"),
        ("cos", [a]) => format!("cos({a})"),
        ("tan", [a]) => format!("tan({a})"),
        ("floor", [a]) => format!("floor({a})"),
        ("ceil", [a]) => format!("ceil({a})"),
        ("min", [a, b]) => format!("min({a}, {b})"),
        ("max", [a, b]) => format!("max({a}, {b})"),
        ("pow", [a, b]) => format!("pow({a}, {b})"),
        _ => {
            return Err(Error::Transpilation {
                message: format!(
                    "the splitting path cannot lower `{name}` with {} arguments",
                    args.len()
                ),
            })
        }
    };
    Ok(ssa.bind(&value))
}

/// Emits `advance`, `init_state`, and `psi_margin`.
///
/// # Errors
///
/// Returns [`Error::Transpilation`] when an expression or `psi` cannot be lowered.
fn build_dynamics(model: &SimModel, psi: &Formula, symbols: &[String]) -> Result<String> {
    let v = symbols.len().max(1);
    let mut source = String::from(PRNG_WGSL);
    write_draw_residual(&mut source);

    let mut advance_ssa = Ssa::new();
    let mut advance_assigns = String::new();
    for (d, expr) in model.advance_exprs().iter().enumerate() {
        let result = emit_sim_expr(&mut advance_ssa, expr)?;
        let _ = writeln!(advance_assigns, "    out[{d}u] = {result};");
    }
    let _ = writeln!(
        source,
        "\nfn advance(prev: ptr<function, array<f32, {v}>>, t: f32, rng: ptr<function, u32>) -> array<f32, {v}> {{\n    var out: array<f32, {v}>;\n{body}{advance_assigns}    return out;\n}}",
        body = advance_ssa.body,
    );

    let mut init_ssa = Ssa::new();
    let mut init_assigns = String::new();
    for (d, expr) in model.init_exprs().iter().enumerate() {
        let result = emit_sim_expr(&mut init_ssa, expr)?;
        let _ = writeln!(init_assigns, "    out[{d}u] = {result};");
    }
    let _ = writeln!(
        source,
        "\nfn init_state(rng: ptr<function, u32>) -> array<f32, {v}> {{\n    var out: array<f32, {v}>;\n{body}{init_assigns}    return out;\n}}",
        body = init_ssa.body,
    );

    let mut margin_ssa = Ssa::new();
    let margin = emit_formula(&mut margin_ssa, psi, symbols, &|slot| {
        format!("(*state)[{slot}u]")
    })?;
    let _ = writeln!(
        source,
        "\nfn psi_margin(state: ptr<function, array<f32, {v}>>) -> f32 {{\n{body}    return {margin};\n}}",
        body = margin_ssa.body,
    );
    Ok(source)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp, reason = "the packed records are exact f32 values")]

    use super::*;
    use crate::stats::NoiseModel;

    fn boxed(expr: SimExpr) -> Box<SimExpr> {
        Box::new(expr)
    }

    fn validated_dynamics(model: &SimModel, psi: &Formula, symbols: &[String]) -> String {
        let dynamics = build_dynamics(model, psi, symbols).unwrap();
        let source = format!(
            "@group(0) @binding(4) var<storage, read> noise_params: array<f32>;\n\n{dynamics}"
        );
        super::super::transpiler::validate(&source).unwrap();
        source
    }

    #[test]
    fn a_noisy_advance_lowers_and_validates() {
        let advance = SimExpr::Add(
            boxed(SimExpr::Add(
                boxed(SimExpr::Prev(0)),
                boxed(SimExpr::Const(0.1)),
            )),
            boxed(SimExpr::Noise(0)),
        );
        let model = SimModel::new(
            ["x"],
            1.0,
            8,
            vec![SimExpr::Const(0.0)],
            vec![advance],
            vec![NoiseModel::gaussian(0.0, 1.0).unwrap()],
        )
        .unwrap();
        let psi = Formula::parse("x > 0").unwrap();
        let source = validated_dynamics(&model, &psi, &["x".to_owned()]);
        assert!(source.contains("fn advance(prev: ptr<function, array<f32, 1>>"));
        assert!(source.contains("draw_residual(0u, rng)"));
        assert!(source.contains("(*prev)[0u]"));
        assert!(source.contains("fn psi_margin"));
    }

    #[test]
    fn calls_and_two_variables_lower_and_validate() {
        let advance_x = SimExpr::Add(
            boxed(SimExpr::Prev(0)),
            boxed(SimExpr::Mul(
                boxed(SimExpr::Const(0.5)),
                boxed(SimExpr::Prev(1)),
            )),
        );
        let advance_y = SimExpr::Add(
            boxed(SimExpr::Call("cos".to_owned(), vec![SimExpr::Time])),
            boxed(SimExpr::Noise(0)),
        );
        let model = SimModel::new(
            ["x", "y"],
            0.5,
            6,
            vec![SimExpr::Const(0.0), SimExpr::Const(1.0)],
            vec![advance_x, advance_y],
            vec![NoiseModel::gaussian(0.0, 0.2).unwrap()],
        )
        .unwrap();
        let psi = Formula::parse("x > 0 and y < 5").unwrap();
        let source = validated_dynamics(&model, &psi, &["x".to_owned(), "y".to_owned()]);
        assert!(source.contains("array<f32, 2>"));
        assert!(source.contains("cos("));
    }

    #[test]
    fn a_temporal_psi_cannot_be_lowered() {
        let model = SimModel::new(
            ["x"],
            1.0,
            4,
            vec![SimExpr::Const(0.0)],
            vec![SimExpr::Prev(0)],
            vec![],
        )
        .unwrap();
        let psi = Formula::parse("always[0, 2](x > 0)").unwrap();
        let result = build_dynamics(&model, &psi, &["x".to_owned()]);
        assert!(matches!(result, Err(Error::Transpilation { .. })));
    }

    #[test]
    fn the_noise_buffer_packs_one_record_per_source() {
        let packed = pack_sim_noise(&[
            NoiseModel::gaussian(0.5, 2.0).unwrap(),
            NoiseModel::exponential(1.5).unwrap(),
        ])
        .unwrap();
        assert_eq!(packed.len(), 2 * NOISE_RECORD);
        // Gaussian is family 1 with mean and standard deviation in slots 2 and 3.
        assert_eq!(packed[0], 1.0);
        assert_eq!(packed[2], 0.5);
        assert_eq!(packed[3], 2.0);
        assert_eq!(packed[NOISE_RECORD + 2], 1.5);
    }
}