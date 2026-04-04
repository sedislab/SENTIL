//! A complete MILP backend for affine dynamics with STL, using the big-M encoding
//! of Raman et al. over a dense two-phase primal simplex.

#![allow(
    clippy::many_single_char_names,
    clippy::cast_precision_loss,
    reason = "the encoding uses the domain's short names (state s, step t, matrices a/b, robustness m), and step indices stay far below 2^53 so the time cast is exact"
)]

use super::model::{AffineForm, Bounds};
use crate::error::{Error, Result};
use crate::formula::{BinaryOp, ComparisonOp, Expr, Formula, Predicate};
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

const DEFAULT_MILP_BUDGET: Duration = Duration::from_secs(5);

const PIVOT_EPS: f64 = 1e-9;

const TIME_EPS: f64 = 1e-9;

/// Solves the affine-plus-STL synthesis problem, returning the packed input of
/// greatest robustness within `bounds`, capped by `max_nodes` and a default
/// wall-clock budget.
///
/// # Errors
///
/// Returns [`Error::Unsupported`] if `spec` uses the probabilistic operator or a
/// predicate term that is not affine in the state, and [`Error::InvalidConfig`] if
/// `bounds` does not match the input dimension.
///
/// ```
/// use sentil::{AffineForm, Bounds, Formula, LinearModel, SystemModel};
/// use sentil::synthesis::solve_milp;
///
/// let model = LinearModel::new(vec![vec![1.0]], vec![vec![1.0]], [0.0], ["pos"], 1.0, 5)?;
/// let affine = model.affine_form().unwrap();
/// let spec = Formula::parse("eventually[0, 5](pos > 2)")?;
/// let bounds = Bounds::new(vec![-1.0; 5], vec![1.0; 5])?;
/// let input = solve_milp(&affine, &spec, &bounds, 10_000)?;
/// assert_eq!(input.len(), 5);
/// # Ok::<(), sentil::Error>(())
/// ```
pub fn solve_milp(
    affine: &AffineForm,
    spec: &Formula,
    bounds: &Bounds,
    max_nodes: usize,
) -> Result<Vec<f64>> {
    solve_milp_within(affine, spec, bounds, max_nodes, DEFAULT_MILP_BUDGET)
}

/// [`solve_milp`] with an explicit wall-clock budget for the branch-and-bound loop.
fn solve_milp_within(
    affine: &AffineForm,
    spec: &Formula,
    bounds: &Bounds,
    max_nodes: usize,
    budget: Duration,
) -> Result<Vec<f64>> {
    let mut enc = Encoder::new(affine, spec, bounds)?;
    let root = enc.encode(spec, 0)?;
    let var = match root {
        Node::Static(_) => {
            let mut input = vec![0.0; enc.inputs.len()];
            bounds.clamp(&mut input);
            return Ok(input);
        }
        Node::Lp { var } => var,
    };
    enc.lp.objective[var] = 1.0;
    let deadline = Instant::now() + budget;
    let Some(solution) = branch_and_bound(&enc.lp, &enc.binaries, max_nodes, deadline) else {
        return Err(Error::InvalidConfig {
            context: "MILP synthesis",
            message: "the encoded program was infeasible at the root, which the dynamics \
                      and bounds should never produce".to_owned(),
        });
    };
    Ok(enc
        .inputs
        .iter()
        .map(|&v| solution.values[v])
        .collect())
}

/// The linear program under construction and the variables it indexes.
struct Encoder {
    lp: LinearProgram,
    inputs: Vec<usize>,
    /// `state[t][s]` is the LP variable for state component `s` at step `t`.
    state: Vec<Vec<usize>>,
    /// The big-M bound on any robustness value.
    big_m: f64,
    variables: Vec<String>,
    horizon: usize,
    dt: f64,
    binaries: Vec<usize>,
}

/// A subformula's robustness in the encoding.
#[derive(Clone, Copy)]
enum Node {
    /// A robustness fixed independently of the input.
    Static(f64),
    /// An LP variable holding the robustness.
    Lp { var: usize },
}

impl Encoder {
    fn new(affine: &AffineForm, spec: &Formula, bounds: &Bounds) -> Result<Self> {
        let n = affine.x0.len();
        let width = affine.b.first().map_or(0, Vec::len);
        let input_dim = width * affine.horizon;
        if bounds.dimension() != input_dim {
            return Err(Error::InvalidConfig {
                context: "MILP synthesis",
                message: format!(
                    "bounds cover {} inputs but the {}-step horizon with {width} inputs per step needs {input_dim}",
                    bounds.dimension(),
                    affine.horizon
                ),
            });
        }
        if let Some(i) = (0..input_dim)
            .find(|&i| !bounds.lower()[i].is_finite() || !bounds.upper()[i].is_finite())
        {
            return Err(Error::InvalidConfig {
                context: "MILP synthesis",
                message: format!(
                    "input coordinate {i} is unbounded, so the optimal robustness is \
                     unbounded and the big-M encoding has no finite value; pass a finite \
                     input box or use Backend::Gradient"
                ),
            });
        }
        let mut lp = LinearProgram::default();
        let inputs: Vec<usize> = (0..input_dim)
            .map(|i| lp.add_variable(bounds.lower()[i], bounds.upper()[i]))
            .collect();
        let state: Vec<Vec<usize>> = (0..=affine.horizon)
            .map(|_| (0..n).map(|_| lp.add_variable(f64::NEG_INFINITY, f64::INFINITY)).collect())
            .collect();

        for (s, &x0s) in affine.x0.iter().enumerate() {
            lp.equal(&[(state[0][s], 1.0)], x0s);
        }
        for t in 0..affine.horizon {
            for s in 0..n {
                let mut terms = vec![(state[t + 1][s], 1.0)];
                for (k, &a) in affine.a[s].iter().enumerate() {
                    terms.push((state[t][k], -a));
                }
                for (k, &b) in affine.b[s].iter().enumerate() {
                    terms.push((inputs[t * width + k], -b));
                }
                lp.equal(&terms, 0.0);
            }
        }

        let big_m = big_m_bound(affine, spec, bounds);
        if !big_m.is_finite() {
            return Err(Error::InvalidConfig {
                context: "MILP synthesis",
                message: "the model's reachable state span is unbounded, so the big-M \
                          encoding has no finite value; the MILP backend needs finite \
                          dynamics and a finite input box"
                    .to_owned(),
            });
        }
        Ok(Self {
            lp,
            inputs,
            state,
            big_m,
            variables: affine.variables.clone(),
            horizon: affine.horizon,
            dt: affine.dt,
            binaries: Vec::new(),
        })
    }

    /// Encodes `formula` evaluated at step `t`.
    fn encode(&mut self, formula: &Formula, t: usize) -> Result<Node> {
        match formula {
            Formula::Predicate(p) => self.predicate(p, t),
            Formula::Not(f) => {
                let child = self.encode(f, t)?;
                Ok(self.negate(child))
            }
            Formula::And(l, r) => {
                let a = self.encode(l, t)?;
                let b = self.encode(r, t)?;
                Ok(self.min_of(&[a, b]))
            }
            Formula::Or(l, r) => {
                let a = self.encode(l, t)?;
                let b = self.encode(r, t)?;
                Ok(self.max_of(&[a, b]))
            }
            Formula::Implies(l, r) => {
                let a = self.encode(l, t)?;
                let b = self.encode(r, t)?;
                let not_a = self.negate(a);
                Ok(self.max_of(&[not_a, b]))
            }
            Formula::Always(iv, f) => {
                let members = self.window(f, t, iv.lower(), iv.upper_or_infinity())?;
                Ok(self.reduce(&members, Reduce::Min))
            }
            Formula::Eventually(iv, f) => {
                let members = self.window(f, t, iv.lower(), iv.upper_or_infinity())?;
                Ok(self.reduce(&members, Reduce::Max))
            }
            Formula::Until(iv, l, r) => self.until(l, r, t, iv.lower(), iv.upper_or_infinity()),
            Formula::Next(f) => {
                if t < self.horizon {
                    self.encode(f, t + 1)
                } else {
                    Ok(self.constant(f64::NEG_INFINITY))
                }
            }
            Formula::Historically(..) | Formula::Once(..) | Formula::Since(..) => {
                Err(Error::Unsupported {
                    feature: "the MILP backend encodes only future-time STL; \
                              past operators (historically, once, since) are not yet encoded",
                })
            }
            Formula::Probabilistic(..) => Err(Error::Unsupported {
                feature: "the MILP backend does not encode the probabilistic operator",
            }),
        }
    }

    /// The robustness of `f(x_t) ~ c`, where `f` is affine in the state.
    fn predicate(&mut self, p: &Predicate, t: usize) -> Result<Node> {
        let lhs = self.affine_term(&p.lhs, t)?;
        let rhs = self.affine_term(&p.rhs, t)?;
        let margin = lhs.sub(&rhs);
        match p.op {
            ComparisonOp::Greater | ComparisonOp::GreaterEqual => Ok(self.bind(&margin)),
            ComparisonOp::Less | ComparisonOp::LessEqual => Ok(self.bind(&margin.scaled(-1.0))),
            ComparisonOp::Equal => {
                let pos = self.bind(&margin);
                let neg = self.bind(&margin.scaled(-1.0));
                let lower = self.min_of(&[pos, neg]);
                Ok(lower)
            }
            ComparisonOp::NotEqual => {
                let pos = self.bind(&margin);
                let neg = self.bind(&margin.scaled(-1.0));
                Ok(self.max_of(&[pos, neg]))
            }
        }
    }

    /// Reads an arithmetic term as an affine form over the state at step `t`.
    fn affine_term(&self, expr: &Expr, t: usize) -> Result<Affine> {
        match expr {
            Expr::Literal(v) => Ok(Affine::constant(*v)),
            Expr::Variable(name) => {
                let s = self.variables.iter().position(|v| v == name).ok_or(
                    Error::Unsupported {
                        feature: "the MILP backend's predicate names a signal the affine model \
                                  does not carry as a state component",
                    },
                )?;
                Ok(Affine::variable(self.state[t][s]))
            }
            Expr::Binary(op, l, r) => {
                let a = self.affine_term(l, t)?;
                let b = self.affine_term(r, t)?;
                match op {
                    BinaryOp::Add => Ok(a.add(&b)),
                    BinaryOp::Sub => Ok(a.sub(&b)),
                    BinaryOp::Mul => a.mul(&b).ok_or(Error::Unsupported {
                        feature: "the MILP backend supports only affine predicate terms; \
                                  a product of two state-dependent terms is nonlinear",
                    }),
                    BinaryOp::Div => a.div(&b).ok_or(Error::Unsupported {
                        feature: "the MILP backend divides an affine term only by a nonzero \
                                  constant; a variable divisor is nonlinear",
                    }),
                    BinaryOp::Mod | BinaryOp::Pow => Err(Error::Unsupported {
                        feature: "the MILP backend supports only affine predicate terms; \
                                  modulo and power are nonlinear",
                    }),
                }
            }
            Expr::Call(..) => Err(Error::Unsupported {
                feature: "the MILP backend supports only affine predicate terms; \
                          a function call is not affine",
            }),
        }
    }

    /// The encoded child at every step whose time falls in `[t*dt + a, t*dt + b]`.
    fn window(&mut self, f: &Formula, t: usize, a: f64, b: f64) -> Result<Vec<Node>> {
        let mut members = Vec::new();
        for j in self.window_indices(t, a, b) {
            members.push(self.encode(f, j)?);
        }
        Ok(members)
    }

    /// The step indices whose times fall in the window `[t*dt + a, t*dt + b]`.
    fn window_indices(&self, t: usize, a: f64, b: f64) -> impl Iterator<Item = usize> {
        let center = t as f64 * self.dt;
        let lo = center + a;
        let hi = center + b;
        let dt = self.dt;
        let last = self.horizon;
        (0..=last).filter(move |&j| {
            let tj = j as f64 * dt;
            tj >= lo - TIME_EPS && (hi.is_infinite() || tj <= hi + TIME_EPS)
        })
    }

    /// `phi until[a, b] psi` at step `t`.
    fn until(&mut self, l: &Formula, r: &Formula, t: usize, a: f64, b: f64) -> Result<Node> {
        let witnesses: Vec<usize> = self.window_indices(t, a, b).collect();
        let mut candidates = Vec::with_capacity(witnesses.len());
        for &s in &witnesses {
            let psi = self.encode(r, s)?;
            let mut prefix = vec![psi];
            for k in t..s {
                prefix.push(self.encode(l, k)?);
            }
            candidates.push(self.reduce(&prefix, Reduce::Min));
        }
        Ok(self.reduce(&candidates, Reduce::Max))
    }

    /// A fresh free robustness variable constrained to equal the affine form.
    fn bind(&mut self, value: &Affine) -> Node {
        let var = self.lp.add_variable(f64::NEG_INFINITY, f64::INFINITY);
        let mut terms = vec![(var, 1.0)];
        for (&col, &coeff) in &value.terms {
            terms.push((col, -coeff));
        }
        self.lp.equal(&terms, value.constant);
        Node::Lp { var }
    }

    fn negate(&mut self, child: Node) -> Node {
        match child {
            Node::Static(value) => Node::Static(-value),
            Node::Lp { var } => {
                let neg = self.lp.add_variable(f64::NEG_INFINITY, f64::INFINITY);
                self.lp.equal(&[(neg, 1.0), (var, 1.0)], 0.0);
                Node::Lp { var: neg }
            }
        }
    }

    fn constant(&mut self, value: f64) -> Node {
        if value.is_finite() {
            Node::Lp {
                var: self.lp.add_variable(value, value),
            }
        } else {
            Node::Static(value)
        }
    }

    fn min_of(&mut self, members: &[Node]) -> Node {
        self.reduce(members, Reduce::Min)
    }

    fn max_of(&mut self, members: &[Node]) -> Node {
        self.reduce(members, Reduce::Max)
    }

    /// The big-M reduction of `members` to their min or max.
    fn reduce(&mut self, members: &[Node], kind: Reduce) -> Node {
        let mut lp_vars = Vec::with_capacity(members.len());
        for m in members {
            match *m {
                Node::Lp { var } => lp_vars.push(var),
                Node::Static(value) => match kind.fold(value) {
                    Fold::Decides => return Node::Static(value),
                    Fold::Drops => {}
                    Fold::Member => lp_vars.push(self.lp.add_variable(value, value)),
                },
            }
        }
        match lp_vars.as_slice() {
            [] => Node::Static(kind.identity()),
            [single] => Node::Lp { var: *single },
            _ => {
                let rho = self.lp.add_variable(f64::NEG_INFINITY, f64::INFINITY);
                let mut selectors = Vec::with_capacity(lp_vars.len());
                for &m in &lp_vars {
                    let b = self.lp.add_variable(0.0, 1.0);
                    self.binaries.push(b);
                    selectors.push(b);
                    match kind {
                        Reduce::Min => {
                            self.lp.less_equal(&[(rho, 1.0), (m, -1.0)], 0.0);
                            self.lp.greater_equal(
                                &[(rho, 1.0), (m, -1.0), (b, -self.big_m)],
                                -self.big_m,
                            );
                        }
                        Reduce::Max => {
                            self.lp.greater_equal(&[(rho, 1.0), (m, -1.0)], 0.0);
                            self.lp.less_equal(
                                &[(rho, 1.0), (m, -1.0), (b, self.big_m)],
                                self.big_m,
                            );
                        }
                    }
                }
                let select: Vec<(usize, f64)> = selectors.iter().map(|&b| (b, 1.0)).collect();
                self.lp.equal(&select, 1.0);
                Node::Lp { var: rho }
            }
        }
    }
}

#[derive(Clone, Copy)]
enum Reduce {
    Min,
    Max,
}

impl Reduce {
    /// The reducer's identity over an empty set.
    fn identity(self) -> f64 {
        match self {
            Reduce::Min => f64::INFINITY,
            Reduce::Max => f64::NEG_INFINITY,
        }
    }

    /// How a static member folds into this reduction.
    fn fold(self, value: f64) -> Fold {
        if value.is_finite() {
            Fold::Member
        } else if (value > 0.0) == matches!(self, Reduce::Max) {
            Fold::Decides
        } else {
            Fold::Drops
        }
    }
}

/// How a static `+/-inf` member folds into a min or max reduction.
enum Fold {
    /// It fixes the result on its own.
    Decides,
    /// It leaves the reduction unchanged.
    Drops,
    /// A finite static, joining as an ordinary pinned member.
    Member,
}

/// An affine form over the LP variables: `sum coeff * var + constant`.
#[derive(Clone, Default)]
struct Affine {
    terms: BTreeMap<usize, f64>,
    constant: f64,
}

impl Affine {
    fn constant(c: f64) -> Self {
        Self {
            terms: BTreeMap::new(),
            constant: c,
        }
    }

    fn variable(col: usize) -> Self {
        let mut terms = BTreeMap::new();
        terms.insert(col, 1.0);
        Self {
            terms,
            constant: 0.0,
        }
    }

    fn add(&self, other: &Self) -> Self {
        let mut out = self.clone();
        for (&col, &c) in &other.terms {
            *out.terms.entry(col).or_insert(0.0) += c;
        }
        out.constant += other.constant;
        out
    }

    fn sub(&self, other: &Self) -> Self {
        self.add(&other.scaled(-1.0))
    }

    fn scaled(&self, factor: f64) -> Self {
        Self {
            terms: self.terms.iter().map(|(&col, &c)| (col, c * factor)).collect(),
            constant: self.constant * factor,
        }
    }

    /// The product, defined only when one side is a bare constant.
    fn mul(&self, other: &Self) -> Option<Self> {
        match (self.terms.is_empty(), other.terms.is_empty()) {
            (true, _) => Some(other.scaled(self.constant)),
            (_, true) => Some(self.scaled(other.constant)),
            _ => None,
        }
    }

    /// The quotient, defined only when the divisor is a nonzero bare constant.
    fn div(&self, other: &Self) -> Option<Self> {
        (other.terms.is_empty() && other.constant != 0.0).then(|| self.scaled(1.0 / other.constant))
    }
}

/// Box-bounded variables, linear constraints, and a linear objective to maximize.
#[derive(Default, Clone)]
struct LinearProgram {
    lower: Vec<f64>,
    upper: Vec<f64>,
    objective: Vec<f64>,
    rows: Vec<Constraint>,
}

/// A linear constraint `sum coeff * var (relation) rhs`.
#[derive(Clone)]
struct Constraint {
    terms: Vec<(usize, f64)>,
    relation: Relation,
    rhs: f64,
}

#[derive(Clone, Copy, PartialEq)]
enum Relation {
    LessEqual,
    GreaterEqual,
    Equal,
}

impl LinearProgram {
    fn add_variable(&mut self, lower: f64, upper: f64) -> usize {
        self.lower.push(lower);
        self.upper.push(upper);
        self.objective.push(0.0);
        self.lower.len() - 1
    }

    fn push(&mut self, terms: &[(usize, f64)], relation: Relation, rhs: f64) {
        self.rows.push(Constraint {
            terms: terms.to_vec(),
            relation,
            rhs,
        });
    }

    fn less_equal(&mut self, terms: &[(usize, f64)], rhs: f64) {
        self.push(terms, Relation::LessEqual, rhs);
    }

    fn greater_equal(&mut self, terms: &[(usize, f64)], rhs: f64) {
        self.push(terms, Relation::GreaterEqual, rhs);
    }

    fn equal(&mut self, terms: &[(usize, f64)], rhs: f64) {
        self.push(terms, Relation::Equal, rhs);
    }
}

struct LpSolution {
    objective: f64,
    values: Vec<f64>,
}

/// How a bounded variable maps onto the non-negative standard-form columns.
#[derive(Clone, Copy)]
enum VarMap {
    /// `v = lower + s`.
    Shifted { col: usize, lower: f64 },
    /// `v = upper - s`.
    Reflected { col: usize, upper: f64 },
    /// `v = pos - neg`.
    Free { pos: usize, neg: usize },
}

/// Solves a linear program by the two-phase primal simplex, or `None` when it is
/// infeasible or unbounded.
fn solve_lp(lp: &LinearProgram) -> Option<LpSolution> {
    let n = lp.lower.len();
    let mut maps = Vec::with_capacity(n);
    let mut width = 0;
    for i in 0..n {
        let (lo, hi) = (lp.lower[i], lp.upper[i]);
        let map = if lo.is_finite() {
            let m = VarMap::Shifted { col: width, lower: lo };
            width += 1;
            m
        } else if hi.is_finite() {
            let m = VarMap::Reflected { col: width, upper: hi };
            width += 1;
            m
        } else {
            let m = VarMap::Free { pos: width, neg: width + 1 };
            width += 2;
            m
        };
        maps.push(map);
    }

    let mut rows: Vec<(Vec<f64>, f64)> = Vec::new();
    let mut relations: Vec<Relation> = Vec::new();
    for (i, &map) in maps.iter().enumerate() {
        if let VarMap::Shifted { col, lower } = map {
            if lp.upper[i].is_finite() {
                let mut row = vec![0.0; width];
                row[col] = 1.0;
                rows.push((row, lp.upper[i] - lower));
                relations.push(Relation::LessEqual);
            }
        }
    }
    for c in &lp.rows {
        let mut row = vec![0.0; width];
        let constant = place(&mut row, &maps, &c.terms);
        rows.push((row, c.rhs - constant));
        relations.push(c.relation);
    }

    let mut objective = vec![0.0; width];
    let obj_constant = place_objective(&mut objective, &maps, &lp.objective);

    let solution = simplex(&rows, &relations, &objective)?;
    let values = (0..n)
        .map(|i| recover(maps[i], &solution))
        .collect();
    Some(LpSolution {
        objective: solution.iter().zip(&objective).map(|(x, c)| x * c).sum::<f64>() + obj_constant,
        values,
    })
}

/// Writes `terms` into a standard-form `row`, returning the constant the shift and
/// reflection contribute.
fn place(row: &mut [f64], maps: &[VarMap], terms: &[(usize, f64)]) -> f64 {
    let mut constant = 0.0;
    for &(i, coeff) in terms {
        match maps[i] {
            VarMap::Shifted { col, lower } => {
                row[col] += coeff;
                constant += coeff * lower;
            }
            VarMap::Reflected { col, upper } => {
                row[col] -= coeff;
                constant += coeff * upper;
            }
            VarMap::Free { pos, neg } => {
                row[pos] += coeff;
                row[neg] -= coeff;
            }
        }
    }
    constant
}

/// Writes the objective onto the standard-form columns, returning the constant the
/// shift and reflection add.
fn place_objective(out: &mut [f64], maps: &[VarMap], objective: &[f64]) -> f64 {
    let mut constant = 0.0;
    for (i, &coeff) in objective.iter().enumerate() {
        if coeff == 0.0 {
            continue;
        }
        match maps[i] {
            VarMap::Shifted { col, lower } => {
                out[col] += coeff;
                constant += coeff * lower;
            }
            VarMap::Reflected { col, upper } => {
                out[col] -= coeff;
                constant += coeff * upper;
            }
            VarMap::Free { pos, neg } => {
                out[pos] += coeff;
                out[neg] -= coeff;
            }
        }
    }
    constant
}

/// Recovers an original variable's value from the standard-form solution.
fn recover(map: VarMap, x: &[f64]) -> f64 {
    match map {
        VarMap::Shifted { col, lower } => lower + x[col],
        VarMap::Reflected { col, upper } => upper - x[col],
        VarMap::Free { pos, neg } => x[pos] - x[neg],
    }
}

/// Maximizes `c x` subject to the standard-form rows and `x >= 0`, returning the
/// optimal point or `None` if infeasible or unbounded.
fn simplex(rows: &[(Vec<f64>, f64)], relations: &[Relation], objective: &[f64]) -> Option<Vec<f64>> {
    let m = rows.len();
    let structural = objective.len();
    let slack_count = relations.iter().filter(|r| **r != Relation::Equal).count();
    let total = structural + slack_count;

    let mut a = vec![vec![0.0; total + m]; m];
    let mut b = vec![0.0; m];
    let mut artificial = vec![false; total + m];
    let mut slack_at = structural;
    for (r, ((coeffs, rhs), relation)) in rows.iter().zip(relations).enumerate() {
        let flip = *rhs < 0.0;
        let sign = if flip { -1.0 } else { 1.0 };
        for (c, &v) in coeffs.iter().enumerate() {
            a[r][c] = sign * v;
        }
        b[r] = sign * rhs;
        match relation {
            Relation::LessEqual => {
                a[r][slack_at] = sign;
                slack_at += 1;
            }
            Relation::GreaterEqual => {
                a[r][slack_at] = -sign;
                slack_at += 1;
            }
            Relation::Equal => {}
        }
    }

    let mut basis = vec![0usize; m];
    for r in 0..m {
        let col = total + r;
        a[r][col] = 1.0;
        artificial[col] = true;
        basis[r] = col;
    }

    let phase_one: Vec<f64> = (0..total + m)
        .map(|c| if artificial[c] { -1.0 } else { 0.0 })
        .collect();
    run_simplex(&mut a, &mut b, &mut basis, &phase_one)?;
    let artificial_value: f64 = (0..m)
        .filter(|&r| artificial[basis[r]])
        .map(|r| b[r])
        .sum();
    if artificial_value > 1e-6 {
        return None;
    }

    for r in 0..m {
        if artificial[basis[r]] {
            if let Some(col) = (0..total).find(|&c| a[r][c].abs() > PIVOT_EPS) {
                pivot(&mut a, &mut b, &mut basis, r, col);
            }
        }
    }

    let mut cost = vec![0.0; total + m];
    cost[..structural].copy_from_slice(objective);
    for (c, pinned) in artificial.iter().enumerate() {
        if *pinned {
            cost[c] = f64::NEG_INFINITY;
        }
    }
    run_simplex(&mut a, &mut b, &mut basis, &cost)?;

    let mut x = vec![0.0; structural];
    for r in 0..m {
        if basis[r] < structural {
            x[basis[r]] = b[r];
        }
    }
    Some(x)
}

/// Runs primal-simplex pivots to optimality from the given basis, or `None` if the
/// objective is unbounded above.
fn run_simplex(
    a: &mut [Vec<f64>],
    b: &mut [f64],
    basis: &mut [usize],
    cost: &[f64],
) -> Option<()> {
    let m = a.len();
    let cols = cost.len();
    loop {
        let basic_cost: Vec<f64> = basis.iter().map(|&col| finite(cost[col])).collect();
        let mut entering = None;
        for j in 0..cols {
            if cost[j] == f64::NEG_INFINITY {
                continue;
            }
            let reduced =
                cost[j] - (0..m).map(|r| basic_cost[r] * a[r][j]).sum::<f64>();
            if reduced > PIVOT_EPS {
                entering = Some(j);
                break;
            }
        }
        let Some(col) = entering else {
            return Some(());
        };

        let mut leaving = None;
        let mut best_ratio = f64::INFINITY;
        for r in 0..m {
            if a[r][col] > PIVOT_EPS {
                let ratio = b[r] / a[r][col];
                if ratio < best_ratio - PIVOT_EPS
                    || (ratio < best_ratio + PIVOT_EPS
                        && leaving.is_some_and(|l| basis[r] < basis[l]))
                {
                    best_ratio = ratio;
                    leaving = Some(r);
                }
            }
        }
        let row = leaving?;
        pivot(a, b, basis, row, col);
    }
}

/// A pinned `NEG_INFINITY` cost as a large finite penalty for the pricing.
fn finite(cost: f64) -> f64 {
    if cost == f64::NEG_INFINITY {
        -1e18
    } else {
        cost
    }
}

fn pivot(a: &mut [Vec<f64>], b: &mut [f64], basis: &mut [usize], row: usize, col: usize) {
    let p = a[row][col];
    for entry in &mut a[row] {
        *entry /= p;
    }
    b[row] /= p;
    let b_pivot = b[row];
    let (head, tail) = a.split_at_mut(row);
    let (pivot_row, rest) = tail.split_first_mut().expect("row is a valid index");
    for (r, target) in head.iter_mut().chain(rest).enumerate() {
        let idx = if r < row { r } else { r + 1 };
        let factor = target[col];
        if factor == 0.0 {
            continue;
        }
        for (entry, &p) in target.iter_mut().zip(pivot_row.iter()) {
            *entry -= factor * p;
        }
        b[idx] -= factor * b_pivot;
    }
    basis[row] = col;
}

/// Branch and bound over the binary variables on top of the LP relaxation, capped
/// by `max_nodes` and `deadline`.
fn branch_and_bound(
    lp: &LinearProgram,
    binaries: &[usize],
    max_nodes: usize,
    deadline: Instant,
) -> Option<LpSolution> {
    let mut incumbent: Option<LpSolution> = None;
    let mut fallback: Option<LpSolution> = None;
    let mut stack = vec![lp.clone()];
    let mut nodes = 0;
    while let Some(node) = stack.pop() {
        if nodes >= max_nodes || Instant::now() >= deadline {
            break;
        }
        nodes += 1;
        let Some(relaxed) = solve_lp(&node) else {
            continue;
        };
        if fallback
            .as_ref()
            .is_none_or(|best| relaxed.objective > best.objective)
        {
            fallback = Some(LpSolution {
                objective: relaxed.objective,
                values: relaxed.values.clone(),
            });
        }
        if incumbent
            .as_ref()
            .is_some_and(|best| relaxed.objective <= best.objective + PIVOT_EPS)
        {
            continue;
        }
        let fractional = binaries.iter().find(|&&v| {
            let value = relaxed.values[v];
            value > PIVOT_EPS && value < 1.0 - PIVOT_EPS
        });
        match fractional {
            None => {
                if incumbent
                    .as_ref()
                    .is_none_or(|best| relaxed.objective > best.objective)
                {
                    incumbent = Some(relaxed);
                }
            }
            Some(&v) => {
                let mut zero = node.clone();
                zero.lower[v] = 0.0;
                zero.upper[v] = 0.0;
                let mut one = node;
                one.lower[v] = 1.0;
                one.upper[v] = 1.0;
                stack.push(zero);
                stack.push(one);
            }
        }
    }
    incumbent.or(fallback)
}

/// A finite big-M that strictly dominates the largest member spread the encoding
/// can produce.
fn big_m_bound(affine: &AffineForm, spec: &Formula, bounds: &Bounds) -> f64 {
    let reach = state_reach(affine, bounds);
    let names = &affine.variables;
    let max_magnitude = predicate_magnitudes(spec)
        .map(|margin| margin.magnitude(names, &reach))
        .fold(0.0, f64::max);
    (4.0 * max_magnitude + 1.0).max(1.0)
}

/// A per-state bound on the reachable magnitude `|x_s|` over the horizon.
fn state_reach(affine: &AffineForm, bounds: &Bounds) -> Vec<f64> {
    let width = affine.b.first().map_or(0, Vec::len);
    let input_cap = |i: usize| bounds.lower()[i].abs().max(bounds.upper()[i].abs());
    let mut reach: Vec<f64> = affine.x0.iter().map(|x| x.abs()).collect();
    let mut peak = reach.clone();
    for t in 0..affine.horizon {
        let next: Vec<f64> = affine
            .a
            .iter()
            .zip(&affine.b)
            .map(|(arow, brow)| {
                let drift: f64 = arow.iter().zip(&reach).map(|(c, r)| c.abs() * r).sum();
                let control: f64 = brow
                    .iter()
                    .enumerate()
                    .map(|(k, c)| c.abs() * input_cap(t * width + k))
                    .sum();
                drift + control
            })
            .collect();
        for (p, &v) in peak.iter_mut().zip(&next) {
            *p = p.max(v);
        }
        reach = next;
    }
    peak
}

/// The affine margin `lhs - rhs` of every predicate in `spec`.
fn predicate_magnitudes(spec: &Formula) -> impl Iterator<Item = Margin> + '_ {
    fn walk(formula: &Formula, out: &mut Vec<Margin>) {
        match formula {
            Formula::Predicate(p) => {
                if let (Some(lhs), Some(rhs)) = (margin_of(&p.lhs), margin_of(&p.rhs)) {
                    out.push(lhs.sub(&rhs));
                }
            }
            Formula::Not(f)
            | Formula::Always(_, f)
            | Formula::Eventually(_, f)
            | Formula::Historically(_, f)
            | Formula::Once(_, f)
            | Formula::Next(f)
            | Formula::Probabilistic(_, _, f) => walk(f, out),
            Formula::And(l, r)
            | Formula::Or(l, r)
            | Formula::Implies(l, r)
            | Formula::Until(_, l, r)
            | Formula::Since(_, l, r) => {
                walk(l, out);
                walk(r, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(spec, &mut out);
    out.into_iter()
}

/// An affine form over named state components, used to size the big-M.
struct Margin {
    terms: BTreeMap<String, f64>,
    constant: f64,
}

impl Margin {
    fn add(&self, other: &Self) -> Self {
        let mut terms = self.terms.clone();
        for (name, &c) in &other.terms {
            *terms.entry(name.clone()).or_insert(0.0) += c;
        }
        Self {
            terms,
            constant: self.constant + other.constant,
        }
    }

    fn sub(&self, other: &Self) -> Self {
        let mut terms = self.terms.clone();
        for (name, &c) in &other.terms {
            *terms.entry(name.clone()).or_insert(0.0) -= c;
        }
        Self {
            terms,
            constant: self.constant - other.constant,
        }
    }

    /// The product, defined only when one side is a bare constant.
    fn mul(&self, other: &Self) -> Option<Self> {
        let scale = |m: &Self, factor: f64| Self {
            terms: m.terms.iter().map(|(n, &c)| (n.clone(), c * factor)).collect(),
            constant: m.constant * factor,
        };
        match (self.terms.is_empty(), other.terms.is_empty()) {
            (true, _) => Some(scale(other, self.constant)),
            (_, true) => Some(scale(self, other.constant)),
            _ => None,
        }
    }

    /// The quotient, defined only when the divisor is a nonzero bare constant.
    fn div(&self, other: &Self) -> Option<Self> {
        (other.terms.is_empty() && other.constant != 0.0).then(|| Self {
            terms: self
                .terms
                .iter()
                .map(|(n, &c)| (n.clone(), c / other.constant))
                .collect(),
            constant: self.constant / other.constant,
        })
    }

    /// The largest `|margin|` over the reachable box.
    fn magnitude(&self, names: &[String], reach: &[f64]) -> f64 {
        let bound: f64 = self
            .terms
            .iter()
            .map(|(name, coeff)| {
                let r = names
                    .iter()
                    .position(|n| n == name)
                    .map_or(0.0, |s| reach[s]);
                coeff.abs() * r
            })
            .sum();
        bound + self.constant.abs()
    }
}

/// The affine form of an arithmetic term over named state components, or `None`
/// when the term is not affine.
fn margin_of(expr: &Expr) -> Option<Margin> {
    match expr {
        Expr::Literal(v) => Some(Margin {
            terms: BTreeMap::new(),
            constant: *v,
        }),
        Expr::Variable(name) => {
            let mut terms = BTreeMap::new();
            terms.insert(name.clone(), 1.0);
            Some(Margin {
                terms,
                constant: 0.0,
            })
        }
        Expr::Binary(op, l, r) => {
            let a = margin_of(l)?;
            let b = margin_of(r)?;
            match op {
                BinaryOp::Add => Some(a.add(&b)),
                BinaryOp::Sub => Some(a.sub(&b)),
                BinaryOp::Mul => a.mul(&b),
                BinaryOp::Div => a.div(&b),
                BinaryOp::Mod | BinaryOp::Pow => None,
            }
        }
        Expr::Call(..) => None,
    }
}

/// Whether [`solve_milp`] can encode `spec`.
pub(crate) fn supports(spec: &Formula) -> bool {
    match spec {
        Formula::Predicate(p) => margin_of(&p.lhs).is_some() && margin_of(&p.rhs).is_some(),
        Formula::Not(a)
        | Formula::Always(_, a)
        | Formula::Eventually(_, a)
        | Formula::Next(a) => supports(a),
        Formula::And(a, b)
        | Formula::Or(a, b)
        | Formula::Implies(a, b)
        | Formula::Until(_, a, b) => supports(a) && supports(b),
        Formula::Historically(..)
        | Formula::Once(..)
        | Formula::Since(..)
        | Formula::Probabilistic(..) => false,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp, reason = "the asserted LP vertices are exact")]

    use super::*;
    use crate::synthesis::{LinearModel, SystemModel};

    fn integrator(horizon: usize) -> LinearModel {
        LinearModel::new(vec![vec![1.0]], vec![vec![1.0]], [0.0], ["pos"], 1.0, horizon).unwrap()
    }

    fn box_bounds(horizon: usize) -> Bounds {
        Bounds::new(vec![-1.0; horizon], vec![1.0; horizon]).unwrap()
    }

    fn exact(model: &LinearModel, spec: &Formula, input: &[f64]) -> f64 {
        let trace = model.rollout_from(model.initial_state(), input).unwrap();
        spec.robustness(&trace).unwrap()
    }

    #[test]
    fn simplex_finds_a_known_lp_optimum() {
        let mut lp = LinearProgram::default();
        let x = lp.add_variable(0.0, f64::INFINITY);
        let y = lp.add_variable(0.0, f64::INFINITY);
        lp.objective[x] = 1.0;
        lp.objective[y] = 1.0;
        lp.less_equal(&[(x, 1.0), (y, 2.0)], 14.0);
        lp.greater_equal(&[(x, 3.0), (y, -1.0)], 0.0);
        lp.less_equal(&[(x, 1.0), (y, -1.0)], 2.0);
        let solution = solve_lp(&lp).unwrap();
        assert!((solution.objective - 10.0).abs() < 1e-7);
        assert!((solution.values[x] - 6.0).abs() < 1e-6);
        assert!((solution.values[y] - 4.0).abs() < 1e-6);
    }

    #[test]
    fn simplex_reports_an_unbounded_program() {
        let mut lp = LinearProgram::default();
        let x = lp.add_variable(0.0, f64::INFINITY);
        lp.objective[x] = 1.0;
        assert!(solve_lp(&lp).is_none());
    }

    #[test]
    fn simplex_reports_an_infeasible_program() {
        let mut lp = LinearProgram::default();
        let x = lp.add_variable(0.0, f64::INFINITY);
        lp.less_equal(&[(x, 1.0)], 1.0);
        lp.greater_equal(&[(x, 1.0)], 3.0);
        assert!(solve_lp(&lp).is_none());
    }

    #[test]
    fn milp_satisfies_an_eventually_spec() {
        let model = integrator(5);
        let affine = model.affine_form().unwrap();
        let spec = Formula::parse("eventually[0, 5](pos > 2)").unwrap();
        let input = solve_milp(&affine, &spec, &box_bounds(5), 50_000).unwrap();
        assert!(exact(&model, &spec, &input) >= 0.0, "input {input:?}");
    }

    #[test]
    fn milp_satisfies_an_always_spec() {
        let model = integrator(5);
        let affine = model.affine_form().unwrap();
        let spec = Formula::parse("always[0, 5](pos < 5)").unwrap();
        let input = solve_milp(&affine, &spec, &box_bounds(5), 50_000).unwrap();
        assert!(exact(&model, &spec, &input) >= 0.0, "input {input:?}");
    }

    #[test]
    fn milp_satisfies_a_conjunctive_spec() {
        let model = integrator(5);
        let affine = model.affine_form().unwrap();
        let spec = Formula::parse("eventually[0, 5](pos > 2) and always[0, 5](pos > -3)").unwrap();
        let input = solve_milp(&affine, &spec, &box_bounds(5), 100_000).unwrap();
        assert!(exact(&model, &spec, &input) >= 0.0, "input {input:?}");
    }

    #[test]
    fn milp_exercises_binary_branching_on_a_disjunction() {
        let model = integrator(5);
        let affine = model.affine_form().unwrap();
        let spec = Formula::parse("eventually[0, 5](pos > 4) or eventually[0, 5](pos < -4)").unwrap();
        let input = solve_milp(&affine, &spec, &box_bounds(5), 100_000).unwrap();
        assert!(exact(&model, &spec, &input) >= 0.0, "input {input:?}");
    }

    #[test]
    fn milp_returns_a_minimally_violating_input_for_an_infeasible_spec() {
        let model = integrator(5);
        let affine = model.affine_form().unwrap();
        let spec = Formula::parse("eventually[0, 5](pos > 10)").unwrap();
        let input = solve_milp(&affine, &spec, &box_bounds(5), 100_000).unwrap();
        let rho = exact(&model, &spec, &input);
        assert!(rho < 0.0 && rho.is_finite(), "rho {rho}");
        assert!((rho + 5.0).abs() < 1e-6, "expected the least violation -5, got {rho}");
    }

    #[test]
    fn the_encoded_optimum_matches_the_exact_robustness() {
        let model = integrator(5);
        let affine = model.affine_form().unwrap();
        let specs = [
            "eventually[0, 5](pos > 2)",
            "always[0, 5](pos < 5)",
            "eventually[0, 5](pos > 2) and always[0, 5](pos > -3)",
            "not(always[0, 5](pos < 1))",
            "(pos > -4) until[0, 5] (pos > 3)",
            "always[0, 3](eventually[0, 2](pos > 1))",
        ];
        for text in specs {
            let spec = Formula::parse(text).unwrap();
            let mut enc = Encoder::new(&affine, &spec, &box_bounds(5)).unwrap();
            let Node::Lp { var } = enc.encode(&spec, 0).unwrap() else {
                panic!("{text}: expected a non-static root");
            };
            enc.lp.objective[var] = 1.0;
            let deadline = Instant::now() + Duration::from_secs(30);
            let solution = branch_and_bound(&enc.lp, &enc.binaries, 200_000, deadline).unwrap();
            let input: Vec<f64> = enc.inputs.iter().map(|&v| solution.values[v]).collect();
            let rho = exact(&model, &spec, &input);
            assert!(
                (solution.objective - rho).abs() < 1e-6,
                "{text}: encoded {} vs exact {rho}",
                solution.objective
            );
        }
    }

    #[test]
    fn the_probabilistic_operator_is_rejected() {
        let model = integrator(5);
        let affine = model.affine_form().unwrap();
        let spec = Formula::parse("P>=0.9(eventually[0, 5](pos > 2))").unwrap();
        assert!(matches!(
            solve_milp(&affine, &spec, &box_bounds(5), 1000),
            Err(Error::Unsupported { .. })
        ));
    }

    #[test]
    fn a_nonaffine_predicate_is_rejected() {
        let model = integrator(5);
        let affine = model.affine_form().unwrap();
        let spec = Formula::parse("eventually[0, 5](pos * pos > 2)").unwrap();
        assert!(matches!(
            solve_milp(&affine, &spec, &box_bounds(5), 1000),
            Err(Error::Unsupported { .. })
        ));
    }

    #[test]
    fn supports_agrees_with_the_encoder_on_every_operator() {
        let model = integrator(5);
        let affine = model.affine_form().unwrap();
        let specs = [
            "pos > 2",
            "pos * pos > 2",
            "not (pos > 2)",
            "(pos > 2) and (pos < 4)",
            "(pos > 2) or (pos < 4)",
            "(pos > 2) implies (pos < 4)",
            "always[0, 2](pos > 2)",
            "eventually[0, 2](pos > 2)",
            "(pos > -3) until[0, 2] (pos > 2)",
            "next (pos > 2)",
            "historically[0, 2](pos > 2)",
            "once[0, 2](pos > 2)",
            "(pos > -3) since[0, 2] (pos > 2)",
            "P>=0.9(pos > 2)",
        ];
        for text in specs {
            let spec = Formula::parse(text).unwrap();
            let mut enc = Encoder::new(&affine, &spec, &box_bounds(5)).unwrap();
            let encodable = enc.encode(&spec, 0).is_ok();
            assert_eq!(
                supports(&spec),
                encodable,
                "{text}: supports says {} but the encoder says {encodable}",
                supports(&spec)
            );
        }
    }

    #[test]
    fn a_past_operator_is_rejected() {
        let model = integrator(5);
        let affine = model.affine_form().unwrap();
        let spec = Formula::parse("once[0, 2](pos > 2)").unwrap();
        assert!(!supports(&spec));
        assert!(matches!(
            solve_milp(&affine, &spec, &box_bounds(5), 1000),
            Err(Error::Unsupported { .. })
        ));
    }

    #[test]
    fn an_unbounded_input_box_is_rejected_rather_than_solved_wrong() {
        let model = integrator(5);
        let affine = model.affine_form().unwrap();
        let spec = Formula::parse("eventually[0, 5](pos > 2)").unwrap();
        assert!(matches!(
            solve_milp(&affine, &spec, &Bounds::unbounded(5), 100_000),
            Err(Error::InvalidConfig { .. })
        ));
    }

    #[test]
    fn bounds_that_do_not_cover_the_inputs_are_rejected() {
        let model = integrator(5);
        let affine = model.affine_form().unwrap();
        let spec = Formula::parse("eventually[0, 5](pos > 2)").unwrap();
        let bounds = Bounds::new(vec![-1.0; 3], vec![1.0; 3]).unwrap();
        assert!(matches!(
            solve_milp(&affine, &spec, &bounds, 1000),
            Err(Error::InvalidConfig { .. })
        ));
    }

    #[test]
    fn an_affine_predicate_with_a_constant_factor_is_accepted() {
        let model = integrator(5);
        let affine = model.affine_form().unwrap();
        let spec = Formula::parse("eventually[0, 5](2 * pos > 4)").unwrap();
        let input = solve_milp(&affine, &spec, &box_bounds(5), 50_000).unwrap();
        assert!(exact(&model, &spec, &input) >= 0.0, "input {input:?}");
    }

    #[test]
    fn an_affine_predicate_with_a_constant_divisor_is_accepted() {
        let model = integrator(5);
        let affine = model.affine_form().unwrap();
        let spec = Formula::parse("eventually[0, 5](pos / 2 > 1)").unwrap();
        let input = solve_milp(&affine, &spec, &box_bounds(5), 50_000).unwrap();
        assert!(exact(&model, &spec, &input) >= 0.0, "input {input:?}");
    }

    #[test]
    fn a_tight_node_cap_still_returns_a_valid_input() {
        let model = integrator(5);
        let affine = model.affine_form().unwrap();
        let spec = Formula::parse("eventually[0, 5](pos > 2)").unwrap();
        let input = solve_milp(&affine, &spec, &box_bounds(5), 1).unwrap();
        assert_eq!(input.len(), 5);
        assert!(input.iter().all(|&u| (-1.0..=1.0).contains(&u)), "input {input:?}");
    }

    #[test]
    fn mixed_large_and_small_thresholds_stay_feasible() {
        let model = integrator(5);
        let affine = model.affine_form().unwrap();
        let spec =
            Formula::parse("(eventually[0, 5](pos > 1000)) and (always[0, 5](pos < 5))").unwrap();
        let input = solve_milp(&affine, &spec, &box_bounds(5), 100_000).unwrap();
        let rho = exact(&model, &spec, &input);
        assert!(rho < 0.0 && rho.is_finite(), "rho {rho}");
        assert!((rho + 995.0).abs() < 1e-6, "expected -995, got {rho}");
    }

    #[test]
    fn an_empty_always_window_holds() {
        let model = integrator(5);
        let affine = model.affine_form().unwrap();
        let spec = Formula::parse("always[10, 12](pos > 2)").unwrap();
        let input = solve_milp(&affine, &spec, &box_bounds(5), 100_000).unwrap();
        assert_eq!(input.len(), 5);
        let rho = exact(&model, &spec, &input);
        assert!(rho.is_infinite() && rho > 0.0, "rho {rho}");
    }

    #[test]
    fn an_empty_eventually_window_does_not_hold() {
        let model = integrator(5);
        let affine = model.affine_form().unwrap();
        let spec = Formula::parse("eventually[10, 12](pos > 2)").unwrap();
        let input = solve_milp(&affine, &spec, &box_bounds(5), 100_000).unwrap();
        assert_eq!(input.len(), 5);
        let rho = exact(&model, &spec, &input);
        assert!(rho.is_infinite() && rho < 0.0, "rho {rho}");
    }

    #[test]
    fn a_partially_in_range_window_still_encodes_its_steps() {
        let model = integrator(5);
        let affine = model.affine_form().unwrap();
        let spec = Formula::parse("eventually[3, 12](pos > 2)").unwrap();
        let input = solve_milp(&affine, &spec, &box_bounds(5), 100_000).unwrap();
        let rho = exact(&model, &spec, &input);
        assert!(rho.is_finite() && rho >= 0.0, "rho {rho}");
    }

    #[test]
    fn a_large_horizon_until_returns_within_the_budget() {
        let model = integrator(8);
        let affine = model.affine_form().unwrap();
        let spec = Formula::parse("(pos > -3) until[0, 8] (pos > 2)").unwrap();
        let bounds = box_bounds(8);
        let start = Instant::now();
        let input =
            solve_milp_within(&affine, &spec, &bounds, 1_000_000, Duration::from_millis(500))
                .unwrap();
        assert!(start.elapsed() < Duration::from_secs(5), "ran for {:?}", start.elapsed());
        assert_eq!(input.len(), 8);
        assert!(input.iter().all(|&u| (-1.0..=1.0).contains(&u)), "input {input:?}");
    }
}