//! A declarative stochastic system the GPU rare-event splitter can transpile.

use rand::RngCore;

use super::noise::NoiseModel;
use super::prstl_rare::StochasticSystem;
use crate::error::{Error, Result};
use crate::signal::Trace;

/// One node of a [`SimModel`] dynamics expression.
#[derive(Debug, Clone)]
pub enum SimExpr {
    /// The previous step's value of variable `d`.
    Prev(usize),
    /// The current time.
    Time,
    /// A constant.
    Const(f64),
    /// The sum of two subexpressions.
    Add(Box<SimExpr>, Box<SimExpr>),
    /// The difference of two subexpressions.
    Sub(Box<SimExpr>, Box<SimExpr>),
    /// The product of two subexpressions.
    Mul(Box<SimExpr>, Box<SimExpr>),
    /// The quotient of two subexpressions.
    Div(Box<SimExpr>, Box<SimExpr>),
    /// A function applied to its arguments.
    Call(String, Vec<SimExpr>),
    /// A fresh residual drawn from the model's noise source `id`.
    Noise(usize),
}

const UNARY: [&str; 10] = [
    "abs", "sqrt", "exp", "ln", "log", "sin", "cos", "tan", "floor", "ceil",
];

const BINARY: [&str; 3] = ["min", "max", "pow"];

/// A declarative stochastic system, the GPU-transpilable twin of [`StochasticSystem`](super::StochasticSystem).
#[derive(Debug, Clone)]
pub struct SimModel {
    variables: Vec<String>,
    dt: f64,
    horizon: usize,
    init: Vec<SimExpr>,
    advance: Vec<SimExpr>,
    noise: Vec<NoiseModel>,
}

impl SimModel {
    /// Builds a model.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidConfig`] if `dt`, `horizon`, `variables`, the expression counts, or any expression reference is invalid.
    pub fn new(
        variables: impl IntoIterator<Item = impl Into<String>>,
        dt: f64,
        horizon: usize,
        init: Vec<SimExpr>,
        advance: Vec<SimExpr>,
        noise: Vec<NoiseModel>,
    ) -> Result<Self> {
        let variables: Vec<String> = variables.into_iter().map(Into::into).collect();
        let n = variables.len();
        if !(dt.is_finite() && dt > 0.0) {
            return Err(config_error(format!(
                "dt must be finite and positive, got {dt}"
            )));
        }
        if horizon == 0 {
            return Err(config_error("horizon must be positive".to_owned()));
        }
        if n == 0 {
            return Err(config_error("at least one variable is required".to_owned()));
        }
        let mut seen = std::collections::BTreeSet::new();
        if let Some(dup) = variables.iter().find(|name| !seen.insert((*name).clone())) {
            return Err(config_error(format!(
                "variable names must be unique, but `{dup}` is repeated"
            )));
        }
        if init.len() != n || advance.len() != n {
            return Err(config_error(format!(
                "init and advance each need one expression per variable, got {} and {} for {n} variables",
                init.len(),
                advance.len()
            )));
        }
        for expr in &init {
            validate_expr(expr, n, noise.len(), false)?;
        }
        for expr in &advance {
            validate_expr(expr, n, noise.len(), true)?;
        }
        Ok(Self {
            variables,
            dt,
            horizon,
            init,
            advance,
            noise,
        })
    }

    /// The packed variable order.
    #[must_use]
    pub fn variables(&self) -> &[String] {
        &self.variables
    }

    /// The spacing between successive steps.
    #[must_use]
    pub fn dt(&self) -> f64 {
        self.dt
    }

    /// The trajectory length, in steps.
    #[must_use]
    pub fn horizon(&self) -> usize {
        self.horizon
    }

    pub(crate) fn init_exprs(&self) -> &[SimExpr] {
        &self.init
    }

    pub(crate) fn advance_exprs(&self) -> &[SimExpr] {
        &self.advance
    }

    pub(crate) fn noise(&self) -> &[NoiseModel] {
        &self.noise
    }

    /// Simulates one full-horizon trajectory into a trace.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidConfig`] if the samples cannot form a trace.
    pub fn simulate(&self, rng: &mut dyn RngCore) -> Result<Trace> {
        let n = self.variables.len();
        let mut state: Vec<f64> = self
            .init
            .iter()
            .map(|expr| eval(expr, &[], 0.0, &self.noise, rng))
            .collect();
        let mut columns: Vec<Vec<f64>> = vec![Vec::with_capacity(self.horizon + 1); n];
        let mut times = Vec::with_capacity(self.horizon + 1);
        let mut time = 0.0;
        for step in 0..=self.horizon {
            for (col, value) in columns.iter_mut().zip(&state) {
                col.push(*value);
            }
            times.push(time);
            if step < self.horizon {
                state = self
                    .advance
                    .iter()
                    .map(|expr| eval(expr, &state, time, &self.noise, rng))
                    .collect();
                time += self.dt;
            }
        }
        let mut trace = Trace::new(times)?;
        for (name, column) in self.variables.iter().zip(columns) {
            trace.add_signal(name, column)?;
        }
        Ok(trace)
    }

    /// Builds the closure-based [`StochasticSystem`] that interprets this model.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidConfig`] on an invariant [`new`](Self::new) already rules out.
    pub fn to_stochastic_system(&self) -> Result<StochasticSystem> {
        let init = self.init.clone();
        let init_noise = self.noise.clone();
        let advance = self.advance.clone();
        let step_noise = self.noise.clone();
        StochasticSystem::new(
            self.variables.clone(),
            self.dt,
            self.horizon,
            move |rng| {
                init.iter()
                    .map(|expr| eval(expr, &[], 0.0, &init_noise, rng))
                    .collect()
            },
            move |prev, t, rng| {
                advance
                    .iter()
                    .map(|expr| eval(expr, prev, t, &step_noise, rng))
                    .collect()
            },
        )
    }
}

fn validate_expr(
    expr: &SimExpr,
    num_vars: usize,
    num_noise: usize,
    allow_prev: bool,
) -> Result<()> {
    match expr {
        SimExpr::Prev(d) => {
            if !allow_prev {
                return Err(config_error(
                    "an init expression cannot read Prev; the initial state has no previous step"
                        .to_owned(),
                ));
            }
            if *d >= num_vars {
                return Err(config_error(format!(
                    "Prev({d}) is out of range for {num_vars} variables"
                )));
            }
            Ok(())
        }
        SimExpr::Noise(id) => {
            if *id >= num_noise {
                return Err(config_error(format!(
                    "Noise({id}) is out of range for {num_noise} noise sources"
                )));
            }
            Ok(())
        }
        SimExpr::Time | SimExpr::Const(_) => Ok(()),
        SimExpr::Add(a, b) | SimExpr::Sub(a, b) | SimExpr::Mul(a, b) | SimExpr::Div(a, b) => {
            validate_expr(a, num_vars, num_noise, allow_prev)?;
            validate_expr(b, num_vars, num_noise, allow_prev)
        }
        SimExpr::Call(name, args) => {
            let arity = if UNARY.contains(&name.as_str()) {
                1
            } else if BINARY.contains(&name.as_str()) {
                2
            } else {
                return Err(config_error(format!("unknown function `{name}`")));
            };
            if args.len() != arity {
                return Err(config_error(format!(
                    "`{name}` takes {arity} argument(s), got {}",
                    args.len()
                )));
            }
            for arg in args {
                validate_expr(arg, num_vars, num_noise, allow_prev)?;
            }
            Ok(())
        }
    }
}

fn eval(
    expr: &SimExpr,
    prev: &[f64],
    time: f64,
    noise: &[NoiseModel],
    rng: &mut dyn RngCore,
) -> f64 {
    match expr {
        SimExpr::Prev(d) => prev.get(*d).copied().unwrap_or(f64::NAN),
        SimExpr::Time => time,
        SimExpr::Const(c) => *c,
        SimExpr::Add(a, b) => eval(a, prev, time, noise, rng) + eval(b, prev, time, noise, rng),
        SimExpr::Sub(a, b) => eval(a, prev, time, noise, rng) - eval(b, prev, time, noise, rng),
        SimExpr::Mul(a, b) => eval(a, prev, time, noise, rng) * eval(b, prev, time, noise, rng),
        SimExpr::Div(a, b) => {
            let (l, r) = (
                eval(a, prev, time, noise, rng),
                eval(b, prev, time, noise, rng),
            );
            if r.abs() < 1e-9 {
                1e38
            } else {
                l / r
            }
        }
        SimExpr::Call(name, args) => eval_call(name, args, prev, time, noise, rng),
        SimExpr::Noise(id) => noise.get(*id).map_or(f64::NAN, |model| model.sample(rng)),
    }
}

fn eval_call(
    name: &str,
    args: &[SimExpr],
    prev: &[f64],
    time: f64,
    noise: &[NoiseModel],
    rng: &mut dyn RngCore,
) -> f64 {
    let mut values = args.iter().map(|arg| eval(arg, prev, time, noise, rng));
    let a = values.next().unwrap_or(f64::NAN);
    match name {
        "abs" => a.abs(),
        "sqrt" => a.max(0.0).sqrt(),
        "exp" => a.clamp(-87.0, 87.0).exp(),
        "ln" => a.max(1e-38).ln(),
        "log" => a.max(1e-38).log10(),
        "sin" => a.sin(),
        "cos" => a.cos(),
        "tan" => a.tan(),
        "floor" => a.floor(),
        "ceil" => a.ceil(),
        "min" => a.min(values.next().unwrap_or(f64::NAN)),
        "max" => a.max(values.next().unwrap_or(f64::NAN)),
        "pow" => a.powf(values.next().unwrap_or(f64::NAN)),
        _ => f64::NAN,
    }
}

fn config_error(message: String) -> Error {
    Error::InvalidConfig {
        context: "sim model",
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    fn boxed(expr: SimExpr) -> Box<SimExpr> {
        Box::new(expr)
    }

    #[test]
    fn a_drifting_random_walk_simulates() {
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
            5,
            vec![SimExpr::Const(0.0)],
            vec![advance],
            vec![NoiseModel::gaussian(0.0, 1.0).unwrap()],
        )
        .unwrap();
        let mut rng = ChaCha8Rng::seed_from_u64(7);
        let trace = model.simulate(&mut rng).unwrap();
        assert_eq!(trace.times().len(), 6);
        assert_eq!(trace.signals().get("x").unwrap().len(), 6);
        assert_eq!(trace.times(), &[0.0, 1.0, 2.0, 3.0, 4.0, 5.0]);
    }

    #[test]
    fn an_init_expression_cannot_read_prev() {
        let result = SimModel::new(
            ["x"],
            1.0,
            3,
            vec![SimExpr::Prev(0)],
            vec![SimExpr::Prev(0)],
            vec![],
        );
        assert!(matches!(result, Err(Error::InvalidConfig { .. })));
    }

    #[test]
    fn out_of_range_references_and_unknown_functions_are_rejected() {
        let bad_noise = SimModel::new(
            ["x"],
            1.0,
            3,
            vec![SimExpr::Const(0.0)],
            vec![SimExpr::Noise(2)],
            vec![],
        );
        assert!(matches!(bad_noise, Err(Error::InvalidConfig { .. })));
        let bad_var = SimModel::new(
            ["x"],
            1.0,
            3,
            vec![SimExpr::Const(0.0)],
            vec![SimExpr::Prev(3)],
            vec![],
        );
        assert!(matches!(bad_var, Err(Error::InvalidConfig { .. })));
        let bad_fn = SimModel::new(
            ["x"],
            1.0,
            3,
            vec![SimExpr::Const(0.0)],
            vec![SimExpr::Call("tanh".to_owned(), vec![SimExpr::Prev(0)])],
            vec![],
        );
        assert!(matches!(bad_fn, Err(Error::InvalidConfig { .. })));
    }

    #[test]
    fn a_repeated_variable_name_is_named_in_the_error() {
        let dup = SimModel::new(
            ["x", "x"],
            1.0,
            3,
            vec![SimExpr::Const(0.0), SimExpr::Const(0.0)],
            vec![SimExpr::Prev(0), SimExpr::Prev(1)],
            vec![],
        );
        let Err(Error::InvalidConfig { message, .. }) = dup else {
            panic!("expected an invalid-config error");
        };
        assert!(message.contains("`x`"), "should name the duplicate: {message}");
    }

    #[test]
    fn the_advance_is_deterministic_without_noise() {
        let advance = SimExpr::Mul(boxed(SimExpr::Const(2.0)), boxed(SimExpr::Prev(0)));
        let model = SimModel::new(
            ["x"],
            1.0,
            4,
            vec![SimExpr::Const(1.0)],
            vec![advance],
            vec![],
        )
        .unwrap();
        let mut rng = ChaCha8Rng::seed_from_u64(1);
        let trace = model.simulate(&mut rng).unwrap();
        assert_eq!(
            trace.signals().get("x").unwrap(),
            &[1.0, 2.0, 4.0, 8.0, 16.0]
        );
    }

    #[test]
    fn the_stochastic_system_bridge_reproduces_the_interpreter() {
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
            6,
            vec![SimExpr::Const(0.0)],
            vec![advance],
            vec![NoiseModel::gaussian(0.0, 1.0).unwrap()],
        )
        .unwrap();
        let system = model.to_stochastic_system().unwrap();
        let mut interp_rng = ChaCha8Rng::seed_from_u64(9);
        let mut system_rng = ChaCha8Rng::seed_from_u64(9);
        let interpreted = model.simulate(&mut interp_rng).unwrap();
        let bridged = system.simulate(&mut system_rng).unwrap();
        assert_eq!(interpreted.signals().get("x"), bridged.signals().get("x"));
    }
}