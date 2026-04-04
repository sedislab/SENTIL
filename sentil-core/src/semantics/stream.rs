//! Online monitoring, folding in one timed sample at a time.

#[cfg(not(feature = "std"))]
use crate::prelude::*;
#[cfg(feature = "std")]
use std::collections::VecDeque;
#[cfg(feature = "std")]
use std::sync::Arc;

use super::robustness::Robustness;
use super::window::MonotonicDeque;
use crate::error::{Error, Result};
use crate::expr::Program;
use crate::formula::Formula;
use crate::signal::Trace;
#[cfg(feature = "statistical")]
use crate::formula::ProbabilityOp;
#[cfg(feature = "statistical")]
use crate::stats::{LiftingRegistry, SmcConfig};
#[cfg(feature = "statistical")]
use rand::SeedableRng;
#[cfg(feature = "statistical")]
use rand_chacha::ChaCha8Rng;

use super::WINDOW_EPSILON;

/// Dense indices for a formula's variables, in sorted order.
#[derive(Debug)]
struct SymbolTable {
    names: Vec<String>,
}

impl SymbolTable {
    fn from_formula(formula: &Formula) -> Self {
        Self {
            names: formula.variables(),
        }
    }

    fn index(&self, name: &str) -> Option<usize> {
        self.names.iter().position(|n| n == name)
    }

    fn len(&self) -> usize {
        self.names.len()
    }
}

/// One stateful node in the compiled monitor.
trait Node {
    /// Folds in the sample at `time`, whose values are packed in `state` by
    /// symbol index.
    fn update(&mut self, time: f64, state: &[f64]) -> Result<Robustness>;
    fn reset(&mut self);
    /// The satisfaction probability estimated at the last update, for a `P~p` node.
    #[cfg(feature = "statistical")]
    fn probability_estimate(&self) -> Option<f64> {
        None
    }
}

/// A maximal non-temporal subformula, compiled to a flat program.
struct AtomicNode {
    program: Program,
    scratch: Vec<f64>,
}

impl Node for AtomicNode {
    fn update(&mut self, _time: f64, state: &[f64]) -> Result<Robustness> {
        Ok(Robustness::Concrete(
            self.program.eval(&mut self.scratch, state)?,
        ))
    }

    fn reset(&mut self) {}
}

struct NotNode {
    child: Box<dyn Node>,
}

impl Node for NotNode {
    fn update(&mut self, time: f64, state: &[f64]) -> Result<Robustness> {
        Ok(self.child.update(time, state)?.negate())
    }

    fn reset(&mut self) {
        self.child.reset();
    }
}

/// Holds a branch's output back until the other branch reaches the same instant.
struct DelayLine {
    lag: f64,
    seen: VecDeque<(f64, Robustness)>,
}

impl DelayLine {
    fn new(lag: f64) -> Self {
        Self {
            lag,
            seen: VecDeque::new(),
        }
    }

    fn push_read(&mut self, time: f64, value: Robustness) -> Robustness {
        self.seen.push_back((time, value));
        let target = time - self.lag + WINDOW_EPSILON;
        while self.seen.len() > 1 && self.seen[1].0 <= target {
            self.seen.pop_front();
        }
        let (seen_at, held) = self.seen[0];
        if seen_at <= target {
            held
        } else {
            Robustness::UNKNOWN
        }
    }

    fn reset(&mut self) {
        self.seen.clear();
    }
}

/// Whether a junction's branches describe the same instant, and which one waits.
enum Align {
    Ready,
    Unknown,
    Left(DelayLine),
    Right(DelayLine),
}

struct BinaryNode {
    left: Box<dyn Node>,
    right: Box<dyn Node>,
    combine: fn(Robustness, Robustness) -> Robustness,
    align: Align,
}

impl Node for BinaryNode {
    fn update(&mut self, time: f64, state: &[f64]) -> Result<Robustness> {
        let l = self.left.update(time, state)?;
        let r = self.right.update(time, state)?;
        Ok(match &mut self.align {
            Align::Ready => (self.combine)(l, r),
            Align::Unknown => Robustness::UNKNOWN,
            Align::Left(delay) => (self.combine)(delay.push_read(time, l), r),
            Align::Right(delay) => (self.combine)(l, delay.push_read(time, r)),
        })
    }

    fn reset(&mut self) {
        self.left.reset();
        self.right.reset();
        match &mut self.align {
            Align::Ready | Align::Unknown => {}
            Align::Left(delay) | Align::Right(delay) => delay.reset(),
        }
    }
}

fn alignment(left: &Formula, right: &Formula) -> Align {
    match (settle_lag(left), settle_lag(right)) {
        (Some((lt, ls)), Some((rt, rs))) if ls == rs => {
            if lt < rt {
                Align::Left(DelayLine::new(rt - lt))
            } else if rt < lt {
                Align::Right(DelayLine::new(lt - rt))
            } else {
                Align::Ready
            }
        }
        (None, Some(_)) | (Some(_), None) => Align::Unknown,
        _ => Align::Ready,
    }
}

/// The settled value a past-time operator or an until operand needs each step.
fn extract_concrete(robustness: Robustness) -> Result<f64> {
    match robustness {
        Robustness::Concrete(v) => Ok(v),
        Robustness::Interval(..) => Err(Error::Unsupported {
            feature: "a future-time operator nested where only present data is available",
        }),
    }
}

/// `historically[a, b] phi` and `once[a, b] phi`.
struct PastWindowNode {
    child: Box<dyn Node>,
    delay: VecDeque<(f64, f64)>,
    window: MonotonicDeque,
    push: fn(&mut MonotonicDeque, f64, f64),
    neutral: f64,
    offset_lower: f64,
    width: f64,
}

impl Node for PastWindowNode {
    fn update(&mut self, time: f64, state: &[f64]) -> Result<Robustness> {
        let value = extract_concrete(self.child.update(time, state)?)?;
        self.delay.push_back((time, value));
        let maturity = time - self.offset_lower + WINDOW_EPSILON;
        while let Some(&(t, v)) = self.delay.front() {
            if t > maturity {
                break;
            }
            (self.push)(&mut self.window, t, v);
            self.delay.pop_front();
        }
        self.window.evict_before(time - self.width - WINDOW_EPSILON);
        Ok(Robustness::Concrete(
            self.window.front_value().unwrap_or(self.neutral),
        ))
    }

    fn reset(&mut self) {
        self.delay.clear();
        self.window.clear();
        self.child.reset();
    }
}

/// `phi since[a, b] psi`, the past-time mirror of `until`.
struct SinceNode {
    phi: Box<dyn Node>,
    psi: Box<dyn Node>,
    candidates: VecDeque<(f64, f64, f64)>,
    delay: VecDeque<(f64, f64, f64)>,
    offset_lower: f64,
    width: f64,
}

impl Node for SinceNode {
    fn update(&mut self, time: f64, state: &[f64]) -> Result<Robustness> {
        let r_phi = extract_concrete(self.phi.update(time, state)?)?;
        let r_psi = extract_concrete(self.psi.update(time, state)?)?;

        for entry in &mut self.delay {
            entry.2 = entry.2.min(r_phi);
        }
        for entry in &mut self.candidates {
            entry.2 = entry.2.min(r_phi);
        }
        self.delay.push_back((time, r_psi, f64::INFINITY));

        let maturity = time - self.offset_lower + WINDOW_EPSILON;
        while let Some(&(t, psi_val, min_phi)) = self.delay.front() {
            if t > maturity {
                break;
            }
            self.delay.pop_front();
            while let Some(&(_, last_psi, last_phi)) = self.candidates.back() {
                let score_last = last_psi.min(last_phi);
                let score_new = psi_val.min(min_phi);
                if score_last <= score_new && last_phi <= min_phi {
                    self.candidates.pop_back();
                } else {
                    break;
                }
            }
            self.candidates.push_back((t, psi_val, min_phi));
        }

        let start = time - self.width - WINDOW_EPSILON;
        while self.candidates.front().is_some_and(|&(t, _, _)| t < start) {
            self.candidates.pop_front();
        }

        let best = self
            .candidates
            .iter()
            .map(|&(_, psi, phi)| psi.min(phi))
            .fold(f64::NEG_INFINITY, f64::max);
        Ok(Robustness::Concrete(best))
    }

    fn reset(&mut self) {
        self.candidates.clear();
        self.delay.clear();
        self.phi.reset();
        self.psi.reset();
    }
}

/// `always[a, b] phi`, settling the verdict for the time `b` behind the present.
struct FutureAlwaysNode {
    child: Box<dyn Node>,
    window: MonotonicDeque,
    offset_start: f64,
    offset_end: f64,
    bounded: bool,
    /// First update at which the child returned a settled value.
    settled_at: Option<f64>,
    global_min: f64,
}

impl Node for FutureAlwaysNode {
    fn update(&mut self, time: f64, state: &[f64]) -> Result<Robustness> {
        let child = self.child.update(time, state)?;
        let (lo, hi) = (child.lower(), child.upper());
        if self.bounded {
            self.window.push_min(time, lo);
        }

        if self.settled_at.is_none() && child.is_resolved() {
            self.settled_at = Some(time);
        }
        let Some(anchor) = self.settled_at else {
            return Ok(Robustness::UNKNOWN);
        };

        if !self.bounded {
            if time >= anchor + self.offset_start - WINDOW_EPSILON {
                self.global_min = self.global_min.min(hi);
            }
            return Ok(Robustness::Interval(f64::NEG_INFINITY, self.global_min));
        }

        let query_time = time - self.offset_end;
        if query_time < anchor - WINDOW_EPSILON {
            self.window
                .evict_before(anchor + self.offset_start - WINDOW_EPSILON);
            let partial = self.window.front_value().unwrap_or(f64::INFINITY);
            return Ok(Robustness::Interval(f64::NEG_INFINITY, partial));
        }

        let window_start = query_time + self.offset_start;
        self.window.evict_before(window_start - WINDOW_EPSILON);
        Ok(Robustness::Concrete(
            self.window.front_value().unwrap_or(f64::INFINITY),
        ))
    }

    fn reset(&mut self) {
        self.window.clear();
        self.settled_at = None;
        self.global_min = f64::INFINITY;
        self.child.reset();
    }
}

/// `eventually[a, b] phi`: the dual of `always`, settling on a supremum.
struct FutureEventuallyNode {
    child: Box<dyn Node>,
    window: MonotonicDeque,
    offset_start: f64,
    offset_end: f64,
    bounded: bool,
    /// See [`FutureAlwaysNode::settled_at`].
    settled_at: Option<f64>,
    global_max: f64,
}

impl Node for FutureEventuallyNode {
    fn update(&mut self, time: f64, state: &[f64]) -> Result<Robustness> {
        let child = self.child.update(time, state)?;
        let (lo, hi) = (child.lower(), child.upper());
        if self.bounded {
            self.window.push_max(time, hi);
        }

        if self.settled_at.is_none() && child.is_resolved() {
            self.settled_at = Some(time);
        }
        let Some(anchor) = self.settled_at else {
            return Ok(Robustness::UNKNOWN);
        };

        if !self.bounded {
            if time >= anchor + self.offset_start - WINDOW_EPSILON {
                self.global_max = self.global_max.max(lo);
            }
            return Ok(Robustness::Interval(self.global_max, f64::INFINITY));
        }

        let query_time = time - self.offset_end;
        if query_time < anchor - WINDOW_EPSILON {
            self.window
                .evict_before(anchor + self.offset_start - WINDOW_EPSILON);
            let partial = self.window.front_value().unwrap_or(f64::NEG_INFINITY);
            return Ok(Robustness::Interval(partial, f64::INFINITY));
        }

        let window_start = query_time + self.offset_start;
        self.window.evict_before(window_start - WINDOW_EPSILON);
        Ok(Robustness::Concrete(
            self.window.front_value().unwrap_or(f64::NEG_INFINITY),
        ))
    }

    fn reset(&mut self) {
        self.window.clear();
        self.settled_at = None;
        self.global_max = f64::NEG_INFINITY;
        self.child.reset();
    }
}

/// `phi until[a, b] psi`.
struct UntilNode {
    phi: Box<dyn Node>,
    psi: Box<dyn Node>,
    buffer: VecDeque<(f64, f64, f64)>,
    offset_start: f64,
    offset_end: f64,
    bounded: bool,
    first_time: Option<f64>,
    unbounded_dp: f64,
    pending_min_phi: f64,
    pending_best: f64,
}

impl Node for UntilNode {
    fn update(&mut self, time: f64, state: &[f64]) -> Result<Robustness> {
        let r_phi = extract_concrete(self.phi.update(time, state)?)?;
        let r_psi = extract_concrete(self.psi.update(time, state)?)?;
        let first = *self.first_time.get_or_insert(time);
        if self.bounded {
            self.buffer.push_back((time, r_phi, r_psi));
        }

        if !self.bounded {
            // Maler-Nickovic recurrence.
            self.unbounded_dp = r_psi.max(r_phi.min(self.unbounded_dp));
            return Ok(Robustness::Interval(self.unbounded_dp, f64::INFINITY));
        }

        let query_time = time - self.offset_end;
        if query_time < first - WINDOW_EPSILON {
            if time >= first + self.offset_start - WINDOW_EPSILON {
                self.pending_best = self.pending_best.max(r_psi.min(self.pending_min_phi));
            }
            self.pending_min_phi = self.pending_min_phi.min(r_phi);
            return Ok(Robustness::Interval(self.pending_best, f64::INFINITY));
        }

        let trim = query_time - WINDOW_EPSILON;
        while self.buffer.front().is_some_and(|&(t, _, _)| t < trim) {
            self.buffer.pop_front();
        }
        Ok(Robustness::Concrete(self.resolve(query_time)))
    }

    fn reset(&mut self) {
        self.buffer.clear();
        self.first_time = None;
        self.unbounded_dp = f64::NEG_INFINITY;
        self.pending_min_phi = f64::INFINITY;
        self.pending_best = f64::NEG_INFINITY;
        self.phi.reset();
        self.psi.reset();
    }
}

impl UntilNode {
    /// The resolved until at `query`, read off the trimmed buffer.
    fn resolve(&self, query: f64) -> f64 {
        let lower = query + self.offset_start;
        let mut best = f64::NEG_INFINITY;
        let mut min_phi = f64::INFINITY;
        for &(t, phi, psi) in &self.buffer {
            if t >= lower - WINDOW_EPSILON {
                best = best.max(psi.min(min_phi));
            }
            min_phi = min_phi.min(phi);
        }
        best
    }
}

/// `next phi`, reporting the child's value one step behind.
struct NextNode {
    child: Box<dyn Node>,
    initialized: bool,
}

impl Node for NextNode {
    fn update(&mut self, time: f64, state: &[f64]) -> Result<Robustness> {
        let child = extract_concrete(self.child.update(time, state)?)?;
        if self.initialized {
            Ok(Robustness::Concrete(child))
        } else {
            self.initialized = true;
            Ok(Robustness::UNKNOWN)
        }
    }

    fn reset(&mut self) {
        self.initialized = false;
        self.child.reset();
    }
}

/// `P~p(phi)`, estimating online how often phi holds across a particle ensemble.
#[cfg(feature = "statistical")]
struct ProbabilisticNode {
    op: ProbabilityOp,
    threshold: f64,
    particles: Vec<Box<dyn Node>>,
    rngs: Vec<ChaCha8Rng>,
    lifting: Arc<LiftingRegistry>,
    symbols: Arc<SymbolTable>,
    scratch: Vec<f64>,
    last_estimate: Option<f64>,
}

#[cfg(feature = "statistical")]
impl Node for ProbabilisticNode {
    #[allow(
        clippy::cast_precision_loss,
        reason = "the particle count stays far below 2^53, so the count cast is exact"
    )]
    fn update(&mut self, time: f64, state: &[f64]) -> Result<Robustness> {
        let mut satisfied = 0u64;
        let mut possible = 0u64;
        for p in 0..self.particles.len() {
            for (i, name) in self.symbols.names.iter().enumerate() {
                self.scratch[i] = match self.lifting.model_for(name) {
                    Some((model, interaction)) => {
                        interaction.apply(state[i], model.sample(&mut self.rngs[p]))
                    }
                    None => state[i],
                };
            }
            let rho = self.particles[p].update(time, &self.scratch)?;
            if rho.lower() >= 0.0 {
                satisfied += 1;
            }
            if rho.upper() >= 0.0 {
                possible += 1;
            }
        }
        let count = self.particles.len() as f64;
        let low = satisfied as f64 / count;
        let high = possible as f64 / count;
        self.last_estimate = Some(low);
        let holds = crate::stats::decides(self.op, low, self.threshold);
        if holds != crate::stats::decides(self.op, high, self.threshold) {
            return Ok(Robustness::UNKNOWN);
        }
        Ok(Robustness::Concrete(if holds {
            f64::INFINITY
        } else {
            f64::NEG_INFINITY
        }))
    }

    fn reset(&mut self) {
        for particle in &mut self.particles {
            particle.reset();
        }
        self.last_estimate = None;
    }

    fn probability_estimate(&self) -> Option<f64> {
        self.last_estimate
    }
}

/// An online monitor that evaluates a formula incrementally.
pub struct StreamMonitor {
    root: Box<dyn Node>,
    symbols: Arc<SymbolTable>,
    buffer: Vec<f64>,
    last_time: Option<f64>,
}

impl StreamMonitor {
    /// Builds a monitor for a formula given as text.
    ///
    /// ```
    /// use sentil::StreamMonitor;
    ///
    /// let mut monitor = StreamMonitor::new("x > 0 and y < 10")?;
    /// let rho = monitor.update(0.0, &[("x", 5.0), ("y", 2.0)])?;
    /// assert_eq!(rho.value(), 5.0);
    /// # Ok::<(), sentil::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns a parse error for malformed input, and [`Error::Unsupported`] for an
    /// operator this monitor does not handle.
    pub fn new(formula: &str) -> Result<Self> {
        Self::from_formula(&Formula::parse(formula)?)
    }

    /// Builds a monitor from an already-parsed formula.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Unsupported`] for an operator this monitor does not handle.
    pub fn from_formula(formula: &Formula) -> Result<Self> {
        validate_streaming(formula)?;
        let symbols = Arc::new(SymbolTable::from_formula(formula));
        let root = build_node(formula, &symbols)?;
        let buffer = vec![0.0; symbols.len()];
        Ok(Self {
            root,
            symbols,
            buffer,
            last_time: None,
        })
    }

    /// Builds a monitor that can decide a top-level probabilistic operator online,
    /// lifting each reading into `config.samples` particles with `lifting`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidConfig`] for a `config` that cannot produce a
    /// verdict, and [`Error::Unsupported`] for a nested probabilistic operator or an
    /// inner formula that is not streamable.
    #[cfg(feature = "statistical")]
    pub fn with_lifting(
        formula: &Formula,
        lifting: &LiftingRegistry,
        config: &SmcConfig,
    ) -> Result<Self> {
        config.validate()?;
        validate_streaming(formula)?;
        let symbols = Arc::new(SymbolTable::from_formula(formula));
        let root: Box<dyn Node> = match formula {
            Formula::Probabilistic(op, threshold, inner) => {
                let particles = (0..config.samples)
                    .map(|_| build_node(inner, &symbols))
                    .collect::<Result<Vec<_>>>()?;
                let rngs = (0..config.samples)
                    .map(|p| ChaCha8Rng::seed_from_u64(config.seed.wrapping_add(p)))
                    .collect::<Vec<_>>();
                Box::new(ProbabilisticNode {
                    op: *op,
                    threshold: *threshold,
                    particles,
                    rngs,
                    lifting: Arc::new(lifting.clone()),
                    symbols: symbols.clone(),
                    scratch: vec![0.0; symbols.len()],
                    last_estimate: None,
                })
            }
            _ => build_node(formula, &symbols)?,
        };
        let buffer = vec![0.0; symbols.len()];
        Ok(Self {
            root,
            symbols,
            buffer,
            last_time: None,
        })
    }

    fn accept_time(&mut self, time: f64) -> Result<()> {
        if !time.is_finite() {
            return Err(Error::NonFiniteSample {
                kind: "time",
                value: time,
            });
        }
        if let Some(previous) = self.last_time {
            if time <= previous {
                return Err(Error::NonMonotonicTime { previous, time });
            }
        }
        self.last_time = Some(time);
        Ok(())
    }

    /// Folds in one timestep, given the current value of each variable, and
    /// returns the robustness so far.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownVariable`] if a variable the formula needs has no
    /// value in `values`, [`Error::NonFiniteSample`] if `time` or a reading is not
    /// finite, or [`Error::NonMonotonicTime`] if the time does not follow the
    /// previous step.
    pub fn update(&mut self, time: f64, values: &[(&str, f64)]) -> Result<Robustness> {
        for (idx, name) in self.symbols.names.iter().enumerate() {
            match values.iter().find(|(n, _)| n == name) {
                Some(&(_, v)) => {
                    accept_value(v)?;
                    self.buffer[idx] = v;
                }
                None => return Err(Error::UnknownVariable { name: name.clone() }),
            }
        }
        self.accept_time(time)?;
        let buffer = core::mem::take(&mut self.buffer);
        let result = self.root.update(time, &buffer);
        self.buffer = buffer;
        result
    }

    /// Folds in one timestep from values packed by [`StreamMonitor::symbol_index`]
    /// order.
    ///
    /// # Errors
    ///
    /// Returns [`Error::PackedLength`] if `values` is shorter than the number of
    /// variables the formula needs, [`Error::NonFiniteSample`] if `time` or a reading
    /// is not finite, or [`Error::NonMonotonicTime`] if the time does not follow the
    /// previous step.
    pub fn update_packed(&mut self, time: f64, values: &[f64]) -> Result<Robustness> {
        if values.len() < self.symbols.len() {
            return Err(Error::PackedLength {
                expected: self.symbols.len(),
                found: values.len(),
            });
        }
        for &value in &values[..self.symbols.len()] {
            accept_value(value)?;
        }
        self.accept_time(time)?;
        self.root.update(time, values)
    }

    /// Replays a recorded `trace` through the monitor in time order, returning the
    /// per-step robustness.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownVariable`] if the trace lacks a signal the formula
    /// needs.
    pub fn run(&mut self, trace: &Trace) -> Result<Vec<Robustness>> {
        let signals = trace.signals();
        let columns = self
            .symbols
            .names
            .iter()
            .map(|name| {
                signals
                    .get(name)
                    .map(Vec::as_slice)
                    .ok_or_else(|| Error::UnknownVariable { name: name.clone() })
            })
            .collect::<Result<Vec<_>>>()?;
        let mut out = Vec::with_capacity(trace.len());
        let mut packed = vec![0.0; columns.len()];
        for (i, &time) in trace.times().iter().enumerate() {
            for (slot, column) in packed.iter_mut().zip(&columns) {
                *slot = column[i];
            }
            out.push(self.update_packed(time, &packed)?);
        }
        Ok(out)
    }

    /// The packed-slice index of a variable, for use with
    /// [`StreamMonitor::update_packed`].
    pub fn symbol_index(&self, name: &str) -> Option<usize> {
        self.symbols.index(name)
    }

    /// The number of variables the formula references.
    pub fn variable_count(&self) -> usize {
        self.symbols.len()
    }

    /// The satisfaction probability estimated at the last update, or `None` for a
    /// deterministic formula.
    #[cfg(feature = "statistical")]
    pub fn last_probability(&self) -> Option<f64> {
        self.root.probability_estimate()
    }

    /// Clears all state.
    pub fn reset(&mut self) {
        self.root.reset();
        self.last_time = None;
    }
}

/// Whether a formula carries any temporal or probabilistic operator.
fn is_temporal(formula: &Formula) -> bool {
    match formula {
        Formula::Predicate(_) => false,
        Formula::Not(f) => is_temporal(f),
        Formula::And(l, r) | Formula::Or(l, r) | Formula::Implies(l, r) => {
            is_temporal(l) || is_temporal(r)
        }
        _ => true,
    }
}

fn accept_value(value: f64) -> Result<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(Error::NonFiniteSample {
            kind: "value",
            value,
        })
    }
}

fn atomic_node(formula: &Formula, names: &[String]) -> Result<Box<dyn Node>> {
    let program = Program::compile(formula, names)?;
    let scratch = program.scratch();
    Ok(Box::new(AtomicNode { program, scratch }))
}

fn offsets(interval: crate::formula::Interval) -> (f64, f64, bool) {
    match interval.upper() {
        Some(b) => (interval.lower(), b, true),
        None => (interval.lower(), 0.0, false),
    }
}

/// Samples-per-unit-time assumed when pre-sizing a window.
const ASSUMED_RATE: f64 = 256.0;
const MAX_PREALLOC: usize = 4096;
const MIN_PREALLOC: usize = 16;

/// Slots to pre-size a window of the given width for.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "the estimate is clamped into [MIN_PREALLOC, MAX_PREALLOC], both small non-negative integers, so the round trip is exact and the cast cannot lose sign or magnitude"
)]
fn prealloc_for(width: f64) -> usize {
    if !width.is_finite() || width <= 0.0 {
        return MIN_PREALLOC;
    }
    let est = (width * ASSUMED_RATE).ceil();
    est.clamp(MIN_PREALLOC as f64, MAX_PREALLOC as f64) as usize
}

fn future_cap(offset_end: f64, bounded: bool) -> usize {
    if bounded {
        prealloc_for(offset_end)
    } else {
        MIN_PREALLOC
    }
}

fn junction_node(
    l: &Formula,
    r: &Formula,
    symbols: &Arc<SymbolTable>,
    combine: fn(Robustness, Robustness) -> Robustness,
) -> Result<Box<dyn Node>> {
    Ok(Box::new(BinaryNode {
        left: build_node(l, symbols)?,
        right: build_node(r, symbols)?,
        combine,
        align: alignment(l, r),
    }))
}

fn build_node(formula: &Formula, symbols: &Arc<SymbolTable>) -> Result<Box<dyn Node>> {
    if !is_temporal(formula) {
        return atomic_node(formula, &symbols.names);
    }
    match formula {
        Formula::Not(inner) => Ok(Box::new(NotNode {
            child: build_node(inner, symbols)?,
        })),
        Formula::And(l, r) => junction_node(l, r, symbols, Robustness::min),
        Formula::Or(l, r) => junction_node(l, r, symbols, Robustness::max),
        Formula::Implies(l, r) => junction_node(l, r, symbols, Robustness::implies),
        Formula::Historically(interval, inner) => Ok(Box::new(PastWindowNode {
            child: build_node(inner, symbols)?,
            delay: VecDeque::with_capacity(prealloc_for(interval.lower())),
            window: MonotonicDeque::with_capacity(prealloc_for(interval.upper_or_infinity())),
            push: MonotonicDeque::push_min,
            neutral: f64::INFINITY,
            offset_lower: interval.lower(),
            width: interval.upper_or_infinity(),
        })),
        Formula::Once(interval, inner) => Ok(Box::new(PastWindowNode {
            child: build_node(inner, symbols)?,
            delay: VecDeque::with_capacity(prealloc_for(interval.lower())),
            window: MonotonicDeque::with_capacity(prealloc_for(interval.upper_or_infinity())),
            push: MonotonicDeque::push_max,
            neutral: f64::NEG_INFINITY,
            offset_lower: interval.lower(),
            width: interval.upper_or_infinity(),
        })),
        Formula::Since(interval, l, r) => Ok(Box::new(SinceNode {
            phi: build_node(l, symbols)?,
            psi: build_node(r, symbols)?,
            candidates: VecDeque::with_capacity(prealloc_for(interval.upper_or_infinity())),
            delay: VecDeque::with_capacity(prealloc_for(interval.lower())),
            offset_lower: interval.lower(),
            width: interval.upper_or_infinity(),
        })),
        Formula::Always(interval, inner) => {
            let (offset_start, offset_end, bounded) = offsets(*interval);
            let cap = future_cap(offset_end, bounded);
            Ok(Box::new(FutureAlwaysNode {
                child: build_node(inner, symbols)?,
                window: MonotonicDeque::with_capacity(cap),
                offset_start,
                offset_end,
                bounded,
                settled_at: None,
                global_min: f64::INFINITY,
            }))
        }
        Formula::Eventually(interval, inner) => {
            let (offset_start, offset_end, bounded) = offsets(*interval);
            let cap = future_cap(offset_end, bounded);
            Ok(Box::new(FutureEventuallyNode {
                child: build_node(inner, symbols)?,
                window: MonotonicDeque::with_capacity(cap),
                offset_start,
                offset_end,
                bounded,
                settled_at: None,
                global_max: f64::NEG_INFINITY,
            }))
        }
        Formula::Until(interval, l, r) => {
            let (offset_start, offset_end, bounded) = offsets(*interval);
            let cap = future_cap(offset_end, bounded);
            Ok(Box::new(UntilNode {
                phi: build_node(l, symbols)?,
                psi: build_node(r, symbols)?,
                buffer: VecDeque::with_capacity(cap),
                offset_start,
                offset_end,
                bounded,
                first_time: None,
                unbounded_dp: f64::NEG_INFINITY,
                pending_min_phi: f64::INFINITY,
                pending_best: f64::NEG_INFINITY,
            }))
        }
        Formula::Next(inner) => Ok(Box::new(NextNode {
            child: build_node(inner, symbols)?,
            initialized: false,
        })),
        Formula::Probabilistic(..) => Err(Error::Unsupported {
            feature: "probabilistic operators need the statistical monitor",
        }),
        Formula::Predicate(_) => atomic_node(formula, &symbols.names),
    }
}

/// Whether a future-time operator appears anywhere in the subtree.
fn contains_future(formula: &Formula) -> bool {
    match formula {
        Formula::Always(..) | Formula::Eventually(..) | Formula::Until(..) | Formula::Next(..) => {
            true
        }
        Formula::Predicate(_) => false,
        Formula::Not(g)
        | Formula::Historically(_, g)
        | Formula::Once(_, g)
        | Formula::Probabilistic(_, _, g) => contains_future(g),
        Formula::And(l, r)
        | Formula::Or(l, r)
        | Formula::Implies(l, r)
        | Formula::Since(_, l, r) => contains_future(l) || contains_future(r),
    }
}

/// How far behind the present a formula's verdict settles, as a time offset and a
/// count of whole samples, or `None` when it never settles.
fn settle_lag(formula: &Formula) -> Option<(f64, usize)> {
    match formula {
        Formula::Predicate(_)
        | Formula::Historically(..)
        | Formula::Once(..)
        | Formula::Since(..) => Some((0.0, 0)),
        Formula::Not(g) | Formula::Probabilistic(_, _, g) => settle_lag(g),
        Formula::And(l, r) | Formula::Or(l, r) | Formula::Implies(l, r) => {
            let (lt, ls) = settle_lag(l)?;
            let (rt, rs) = settle_lag(r)?;
            Some((lt.max(rt), ls.max(rs)))
        }
        Formula::Always(interval, g) | Formula::Eventually(interval, g) => {
            let (time, samples) = settle_lag(g)?;
            Some((time + interval.upper()?, samples))
        }
        Formula::Until(interval, ..) => Some((interval.upper()?, 0)),
        Formula::Next(g) => {
            let (time, samples) = settle_lag(g)?;
            Some((time, samples + 1))
        }
    }
}

/// Rejects compositions the online monitor cannot evaluate.
fn validate_streaming(formula: &Formula) -> Result<()> {
    let present = |f: &Formula| -> Result<()> {
        if contains_future(f) {
            Err(Error::Unsupported {
                feature: "a future-time operator nested inside a past-time operator or the \
                          operand of until, since, or next; evaluate this formula offline instead",
            })
        } else {
            Ok(())
        }
    };
    match formula {
        Formula::Predicate(_) => Ok(()),
        Formula::Not(g) | Formula::Always(_, g) | Formula::Eventually(_, g) => {
            validate_streaming(g)
        }
        Formula::And(l, r) | Formula::Or(l, r) | Formula::Implies(l, r) => {
            validate_streaming(l)?;
            validate_streaming(r)?;
            match (settle_lag(l), settle_lag(r)) {
                (Some((_, ls)), Some((_, rs))) if ls != rs => Err(Error::Unsupported {
                    feature: "a boolean junction mixing a next-delayed branch with a \
                              time-delayed one; monitor them as separate formulas, or \
                              evaluate this one offline",
                }),
                _ => Ok(()),
            }
        }
        Formula::Historically(_, g) | Formula::Once(_, g) | Formula::Next(g) => {
            present(g)?;
            validate_streaming(g)
        }
        Formula::Until(_, l, r) | Formula::Since(_, l, r) => {
            present(l)?;
            present(r)?;
            validate_streaming(l)?;
            validate_streaming(r)
        }
        Formula::Probabilistic(_, _, g) => validate_streaming(g),
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::float_cmp,
        reason = "streaming robustness here matches the offline reference exactly"
    )]
    #![allow(
        clippy::cast_precision_loss,
        reason = "test trace indices are tiny, so the index-to-time cast is exact"
    )]

    use super::*;
    use proptest::prelude::*;

    #[test]
    fn streams_a_boolean_formula() {
        let mut monitor = StreamMonitor::new("x > 0 and y < 10").unwrap();
        assert_eq!(
            monitor
                .update(0.0, &[("x", 5.0), ("y", 2.0)])
                .unwrap()
                .value(),
            5.0
        );
        assert_eq!(
            monitor
                .update(1.0, &[("x", 1.0), ("y", 12.0)])
                .unwrap()
                .value(),
            -2.0
        );
    }

    #[test]
    fn future_always_partial_upper_bound_stays_sound() {
        let mut m = StreamMonitor::new("always[1, 3](x > 0)").unwrap();
        for (t, &x) in [-5.0, 10.0, 10.0, 10.0].iter().enumerate() {
            let r = m.update(t as f64, &[("x", x)]).unwrap();
            if let Robustness::Interval(_, hi) = r {
                assert!(hi >= 10.0, "step {t}: provisional upper {hi} below the resolved 10");
            }
        }
    }

    #[test]
    fn future_eventually_partial_lower_bound_stays_sound() {
        let mut m = StreamMonitor::new("eventually[1, 3](x > 0)").unwrap();
        for (t, &x) in [10.0, -5.0, -5.0, -5.0].iter().enumerate() {
            let r = m.update(t as f64, &[("x", x)]).unwrap();
            if let Robustness::Interval(lo, _) = r {
                assert!(lo <= -5.0, "step {t}: provisional lower {lo} above the resolved -5");
            }
        }
    }

    #[test]
    fn dense_update_matches_named_update() {
        let mut monitor = StreamMonitor::new("x > 0 and y > 0").unwrap();
        let xi = monitor.symbol_index("x").unwrap();
        let yi = monitor.symbol_index("y").unwrap();
        let mut packed = vec![0.0; monitor.variable_count()];
        packed[xi] = 3.0;
        packed[yi] = 7.0;
        assert_eq!(monitor.update_packed(0.0, &packed).unwrap().value(), 3.0);
    }

    #[test]
    fn negation_and_implication_stream() {
        let mut monitor = StreamMonitor::new("not(x > 5)").unwrap();
        assert_eq!(monitor.update(0.0, &[("x", 3.0)]).unwrap().value(), 2.0);

        let mut imp = StreamMonitor::new("(x > 10) implies (y > 0)").unwrap();
        assert_eq!(
            imp.update(0.0, &[("x", 15.0), ("y", 3.0)]).unwrap().value(),
            3.0
        );
    }

    #[test]
    fn missing_value_is_an_error() {
        let mut monitor = StreamMonitor::new("x > 0 and y > 0").unwrap();
        assert!(matches!(
            monitor.update(0.0, &[("x", 1.0)]),
            Err(Error::UnknownVariable { .. })
        ));
    }

    #[test]
    fn future_inside_past_is_rejected() {
        assert!(matches!(
            StreamMonitor::new("historically[0, 5](eventually[0, 2](x > 0))"),
            Err(Error::Unsupported { .. })
        ));
        assert!(matches!(
            StreamMonitor::new("(eventually[0, 2](x > 0)) until[0, 3] (y > 0)"),
            Err(Error::Unsupported { .. })
        ));
        assert!(StreamMonitor::new("always[0, 2](eventually[0, 1](x > 0))").is_ok());
        assert!(StreamMonitor::new("always[0, 2](historically[0, 1](x > 0))").is_ok());
    }

    #[test]
    fn a_junction_aligns_its_branches_and_refuses_only_mixed_units() {
        for formula in [
            "always[0, 1](x > 0) and always[0, 3](y > 0)",
            "(x > 0) and eventually[0, 2](y > 0)",
            "always[0, 2]((x > 0) implies eventually[0, 2](y > 0))",
            "eventually[0, 1](x > 2) or always[0, 3](x > 0)",
            "always[0, 2](x > 0) and always[0, 2](y > 0)",
            "historically[0, 4](x > 0) and (y > 0)",
            "always(x > 0) and always(y > 0)",
        ] {
            assert!(StreamMonitor::new(formula).is_ok(), "{formula} should stream");
        }
        for formula in ["next(x > 0) and always[0, 1](y > 0)", "always[0, 1](next(x > 0)) or eventually[0, 1](y > 0)"] {
            assert!(
                matches!(StreamMonitor::new(formula), Err(Error::Unsupported { .. })),
                "{formula} mixes a sample delay with a time delay and cannot be aligned"
            );
        }
    }

    fn future_resolves_to_offline(
        formula: &str,
        delay: usize,
        times: &[f64],
        signals: &[(&str, &[f64])],
    ) {
        let offline = offline_values(formula, times, signals);
        let mut monitor = StreamMonitor::new(formula).unwrap();
        let slots: Vec<usize> = signals
            .iter()
            .map(|(n, _)| monitor.symbol_index(n).unwrap())
            .collect();
        let mut packed = vec![0.0; monitor.variable_count()];
        for (i, &t) in times.iter().enumerate() {
            for (s, (_, values)) in signals.iter().enumerate() {
                packed[slots[s]] = values[i];
            }
            let robustness = monitor.update_packed(t, &packed).unwrap();
            if i >= delay {
                assert!(
                    robustness.lower() == robustness.upper(),
                    "{formula} should have resolved by step {i}"
                );
                assert_eq!(
                    robustness.value(),
                    offline[i - delay],
                    "{formula} at step {i}"
                );
            }
        }
    }

    #[test]
    fn future_operators_resolve_to_the_offline_value() {
        let times = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let x = [3.0, -1.0, 4.0, 2.0, -5.0, 1.0, 6.0];
        future_resolves_to_offline("always[0, 3](x > 0)", 3, &times, &[("x", &x)]);
        future_resolves_to_offline("eventually[0, 2](x > 5)", 2, &times, &[("x", &x)]);
        future_resolves_to_offline("next(x > 0)", 1, &times, &[("x", &x)]);
        let y = [-1.0, -1.0, 2.0, -1.0, 3.0, -1.0, 1.0];
        future_resolves_to_offline(
            "x > 0 until[0, 3] y > 0",
            3,
            &times,
            &[("x", &x), ("y", &y)],
        );
    }

    fn accumulated_times(n: usize, step: f64) -> Vec<f64> {
        let mut times = Vec::with_capacity(n);
        let mut t = 0.0;
        for _ in 0..n {
            times.push(t);
            t += step;
        }
        times
    }

    #[test]
    fn a_drifted_window_edge_keeps_the_sample_on_it() {
        let times = accumulated_times(8, 0.1);
        let dip = [-5.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0];
        future_resolves_to_offline("always[0, 0.3](x > 0)", 3, &times, &[("x", &dip)]);
        let peak = [7.0, -2.0, -2.0, -2.0, -2.0, -2.0, -2.0, -2.0];
        future_resolves_to_offline("eventually[0, 0.3](x > 0)", 3, &times, &[("x", &peak)]);
    }

    #[test]
    fn a_delayed_window_start_survives_the_unresolved_phase() {
        let times = accumulated_times(20, 0.1);
        let mut x = [10.0; 20];
        x[8] = -5.0;
        future_resolves_to_offline("always[0.8, 1.4](x > 0)", 14, &times, &[("x", &x)]);
    }

    #[test]
    fn a_query_time_that_drifts_below_zero_still_resolves() {
        let times = accumulated_times(12, 0.2);
        let mut x = [5.0; 12];
        x[3] = -2.0;
        future_resolves_to_offline("always[0.6000000000000001, 1.4000000000000001](x > 0)", 7, &times, &[("x", &x)]);
        let mut y = [-5.0; 12];
        y[3] = 2.0;
        future_resolves_to_offline(
            "eventually[0.6000000000000001, 1.4000000000000001](y > 0)",
            7,
            &times,
            &[("y", &y)],
        );
    }

    #[test]
    fn until_keeps_the_sample_its_query_starts_on() {
        let times = accumulated_times(8, 0.1);
        let dip = [-3.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0];
        let late = [-1.0, -1.0, -1.0, 7.0, -1.0, -1.0, -1.0, -1.0];
        future_resolves_to_offline(
            "x > 0 until[0, 0.3] y > 0",
            3,
            &times,
            &[("x", &dip), ("y", &late)],
        );
        let flat = [10.0; 8];
        let early = [7.0, -1.0, -1.0, -1.0, -1.0, -1.0, -1.0, -1.0];
        future_resolves_to_offline(
            "x > 0 until[0, 0.3] y > 0",
            3,
            &times,
            &[("x", &flat), ("y", &early)],
        );
    }

    fn stream_values(formula: &str, times: &[f64], signals: &[(&str, &[f64])]) -> Vec<f64> {
        let mut monitor = StreamMonitor::new(formula).unwrap();
        let slots: Vec<usize> = signals
            .iter()
            .map(|(n, _)| monitor.symbol_index(n).unwrap())
            .collect();
        let mut packed = vec![0.0; monitor.variable_count()];
        let mut out = Vec::with_capacity(times.len());
        for (i, &t) in times.iter().enumerate() {
            for (s, (_, values)) in signals.iter().enumerate() {
                packed[slots[s]] = values[i];
            }
            out.push(monitor.update_packed(t, &packed).unwrap().value());
        }
        out
    }

    fn offline_values(formula: &str, times: &[f64], signals: &[(&str, &[f64])]) -> Vec<f64> {
        let phi = crate::Formula::parse(formula).unwrap();
        let map: std::collections::BTreeMap<String, Vec<f64>> = signals
            .iter()
            .map(|(n, v)| ((*n).to_string(), v.to_vec()))
            .collect();
        super::super::discrete::robustness_trace(&phi, times, &map).unwrap()
    }

    #[test]
    fn presizing_does_not_change_verdicts_even_past_the_estimate() {
        let n = 600usize;
        let times: Vec<f64> = (0..n).map(|i| i as f64).collect();
        let x: Vec<f64> = (0..n).map(|i| ((i % 7) as f64) - 3.0).collect();
        let f = "historically[0, 50](x > 0)";
        assert_eq!(
            stream_values(f, &times, &[("x", &x)]),
            offline_values(f, &times, &[("x", &x)])
        );
    }

    #[test]
    fn prealloc_for_clamps_to_the_sane_range() {
        assert_eq!(prealloc_for(0.0), MIN_PREALLOC);
        assert_eq!(prealloc_for(f64::INFINITY), MIN_PREALLOC);
        assert_eq!(prealloc_for(-1.0), MIN_PREALLOC);
        assert_eq!(prealloc_for(f64::NAN), MIN_PREALLOC);
        assert_eq!(prealloc_for(1e9), MAX_PREALLOC);
        assert_eq!(prealloc_for(1.0), 256);
        for &w in &[0.01, 0.5, 2.0, 17.0, 1e6, f64::INFINITY] {
            let c = prealloc_for(w);
            assert!((MIN_PREALLOC..=MAX_PREALLOC).contains(&c));
        }
    }

    #[test]
    fn historically_streams_the_running_minimum() {
        let times = [0.0, 1.0, 2.0, 3.0, 4.0];
        let x = [2.0, -1.0, 3.0, 5.0, 1.0];
        assert_eq!(
            stream_values("historically[0, 2](x > 0)", &times, &[("x", &x)]),
            offline_values("historically[0, 2](x > 0)", &times, &[("x", &x)])
        );
    }

    #[test]
    fn once_and_since_stream_like_offline() {
        let times = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
        let x = [-3.0, 1.0, 8.0, -2.0, 4.0, 0.5];
        let y = [-1.0, -1.0, 2.0, -1.0, -1.0, 3.0];
        assert_eq!(
            stream_values("once[0, 3](x > 5)", &times, &[("x", &x)]),
            offline_values("once[0, 3](x > 5)", &times, &[("x", &x)])
        );
        assert_eq!(
            stream_values("x > 0 since y > 0", &times, &[("x", &x), ("y", &y)]),
            offline_values("x > 0 since y > 0", &times, &[("x", &x), ("y", &y)])
        );
    }

    proptest! {
        #[test]
        fn past_operators_stream_exactly_like_offline(
            values in prop::collection::vec(-20.0f64..20.0, 1..40),
        ) {
            let times: Vec<f64> = (0..values.len()).map(|i| i as f64).collect();
            for formula in [
                "historically[0, 3](x > 0)",
                "once[1, 4](x > 5)",
                "historically[0, 1](x > -10)",
            ] {
                let online = stream_values(formula, &times, &[("x", &values)]);
                let offline = offline_values(formula, &times, &[("x", &values)]);
                prop_assert_eq!(online, offline, "mismatch for {}", formula);
            }
        }

        #[test]
        fn future_operators_resolve_to_offline(
            xs in prop::collection::vec(-20.0f64..20.0, 1..40),
            ys in prop::collection::vec(-20.0f64..20.0, 1..40),
        ) {
            let n = xs.len().min(ys.len());
            let times: Vec<f64> = (0..n).map(|i| i as f64).collect();
            let x = &xs[..n];
            let y = &ys[..n];
            for (formula, delay, signals) in [
                ("always[0, 3](x > 0)", 3usize, &[("x", x)][..]),
                ("eventually[1, 4](x > 5)", 4, &[("x", x)][..]),
                ("next(x > 0)", 1, &[("x", x)][..]),
                ("x > 0 until[0, 3] y > 0", 3, &[("x", x), ("y", y)][..]),
                ("x > 0 until[1, 4] y > 0", 4, &[("x", x), ("y", y)][..]),
                ("x > 0 until[2, 2] y > 0", 2, &[("x", x), ("y", y)][..]),
                ("always[0, 1](eventually[0, 1](x > 0))", 2, &[("x", x)][..]),
                ("eventually[0, 1](always[0, 1](x > 0))", 2, &[("x", x)][..]),
                ("always[0, 2](always[0, 1](x > 0))", 3, &[("x", x)][..]),
                ("eventually[0, 1](eventually[0, 1](x > 0))", 2, &[("x", x)][..]),
                ("always[1, 2](eventually[0, 1](x > 0))", 3, &[("x", x)][..]),
                ("always[0, 1](next(x > 0))", 2, &[("x", x)][..]),
                ("always[0, 1](x > 0 until[0, 2] y > 0)", 3, &[("x", x), ("y", y)][..]),
                ("eventually[0, 2](x > 0 until[1, 2] y > 0)", 4, &[("x", x), ("y", y)][..]),
                ("always[0, 2](always[0, 1](eventually[0, 1](x > 0)))", 4, &[("x", x)][..]),
                ("always[0, 2]((x > 0) and (eventually[0, 2](y > 0)))", 4, &[("x", x), ("y", y)][..]),
                ("always[0, 2]((x > 0) implies (eventually[0, 2](y > 0)))", 4, &[("x", x), ("y", y)][..]),
                ("always[0, 1]((x > 0) and (eventually[0, 3](y > 0)))", 4, &[("x", x), ("y", y)][..]),
                ("(always[0, 1](x > 0)) and (always[0, 3](y > 0))", 3, &[("x", x), ("y", y)][..]),
                ("(x > 0) and (eventually[0, 2](y > 0))", 2, &[("x", x), ("y", y)][..]),
                ("(eventually[0, 2](x > 0)) or (always[0, 1](y > 0))", 2, &[("x", x), ("y", y)][..]),
            ] {
                let offline = offline_values(formula, &times, signals);
                let mut monitor = StreamMonitor::new(formula).unwrap();
                let slots: Vec<usize> =
                    signals.iter().map(|(s, _)| monitor.symbol_index(s).unwrap()).collect();
                let mut packed = vec![0.0; monitor.variable_count()];
                for (i, &t) in times.iter().enumerate() {
                    for (s, (_, vals)) in signals.iter().enumerate() {
                        packed[slots[s]] = vals[i];
                    }
                    let robustness = monitor.update_packed(t, &packed).unwrap();
                    if i < delay {
                        prop_assert!(!robustness.is_resolved(), "{} settled at {}, before its window closed", formula, i);
                        continue;
                    }
                    prop_assert!(robustness.is_resolved(), "{} unresolved at {}", formula, i);
                    prop_assert_eq!(robustness.value(), offline[i - delay], "{} at {}", formula, i);
                }
            }
        }

        #[test]
        fn unbounded_future_operators_bracket_the_offline_value(
            xs in prop::collection::vec(-20.0f64..20.0, 4..40),
        ) {
            let times: Vec<f64> = (0..xs.len()).map(|i| i as f64).collect();
            let signals = &[("x", &xs[..])][..];
            for formula in [
                "always[2, inf](x > 0)",
                "eventually[2, inf](x > 0)",
                "always[0, inf](x > 0)",
                "eventually[1, inf](x > 5)",
            ] {
                let offline = offline_values(formula, &times, signals);
                let mut monitor = StreamMonitor::new(formula).unwrap();
                let slot = monitor.symbol_index("x").unwrap();
                let mut packed = vec![0.0; monitor.variable_count()];
                for (i, &t) in times.iter().enumerate() {
                    packed[slot] = xs[i];
                    let robustness = monitor.update_packed(t, &packed).unwrap();
                    prop_assert!(!robustness.is_resolved(), "{} settled at {}", formula, i);
                    prop_assert!(
                        robustness.lower() <= offline[0] && offline[0] <= robustness.upper(),
                        "{} at {}: [{}, {}] excludes the answer {}",
                        formula, i, robustness.lower(), robustness.upper(), offline[0]
                    );
                }
            }
        }

        #[test]
        fn every_arithmetic_function_streams_exactly_like_offline(
            xs in prop::collection::vec(-5.0f64..5.0, 1..30),
            ys in prop::collection::vec(-5.0f64..5.0, 1..30),
        ) {
            let n = xs.len().min(ys.len());
            let times: Vec<f64> = (0..n).map(|i| i as f64).collect();
            let x = &xs[..n];
            let y = &ys[..n];
            let f = "abs(x) + sqrt(abs(x) + 1) + exp(sin(x)) + ln(abs(y) + 2) + log(abs(y) + 3) + cos(x) + tan(x) + floor(x) + ceil(y) + min(x, y) + max(x, y) > 0";
            let online = stream_values(f, &times, &[("x", x), ("y", y)]);
            let offline = offline_values(f, &times, &[("x", x), ("y", y)]);
            for (o, ofl) in online.iter().zip(&offline) {
                if o.is_finite() && ofl.is_finite() {
                    prop_assert_eq!(o.to_bits(), ofl.to_bits());
                }
            }
        }
    }

    #[test]
    fn a_wide_until_bound_stays_linear() {
        let n: usize = 200_000;
        let mut monitor = StreamMonitor::new("x > 0 until[0, 100000000] y > 0").unwrap();
        let xi = monitor.symbol_index("x").unwrap();
        let yi = monitor.symbol_index("y").unwrap();
        let mut packed = vec![0.0; monitor.variable_count()];
        let start = std::time::Instant::now();
        for i in 0..n {
            packed[xi] = ((i * 7) % 13) as f64 - 6.0;
            packed[yi] = ((i * 5) % 11) as f64 - 5.0;
            monitor.update_packed(i as f64, &packed).unwrap();
        }
        assert!(start.elapsed() < std::time::Duration::from_secs(5));
    }

    #[test]
    fn run_replays_a_trace_through_the_monitor() {
        let trace = Trace::from_signal(vec![0.0, 1.0, 2.0], "x", vec![3.0, -1.0, 4.0]).unwrap();
        let mut monitor = StreamMonitor::new("x > 0").unwrap();
        let signal = monitor.run(&trace).unwrap();
        let values: Vec<f64> = signal.iter().map(Robustness::value).collect();
        assert_eq!(values, vec![3.0, -1.0, 4.0]);
    }

    #[test]
    fn run_reports_a_missing_signal() {
        let trace = Trace::from_signal(vec![0.0, 1.0], "y", vec![1.0, 2.0]).unwrap();
        let mut monitor = StreamMonitor::new("x > 0").unwrap();
        assert!(matches!(
            monitor.run(&trace),
            Err(Error::UnknownVariable { .. })
        ));
    }

    #[cfg(feature = "statistical")]
    #[test]
    fn online_probabilistic_decides_a_clear_case() {
        use crate::stats::{LiftingRegistry, NoiseInteraction, NoiseModel, SmcConfig};
        let phi = Formula::parse("P>=0.5(x > 0)").unwrap();
        let mut lifting = LiftingRegistry::new();
        lifting.register(
            "x",
            NoiseModel::gaussian(0.0, 1.0).unwrap(),
            NoiseInteraction::Additive,
        );
        let config = SmcConfig {
            samples: 2000,
            confidence: 0.95,
            seed: 7,
            ..Default::default()
        };
        let mut holds = StreamMonitor::with_lifting(&phi, &lifting, &config).unwrap();
        assert!(holds.update(0.0, &[("x", 2.0)]).unwrap().value() > 0.0);
        let mut fails = StreamMonitor::with_lifting(&phi, &lifting, &config).unwrap();
        assert!(fails.update(0.0, &[("x", -2.0)]).unwrap().value() < 0.0);
    }

    #[cfg(feature = "statistical")]
    #[test]
    fn online_probabilistic_tracks_a_temporal_inner() {
        use crate::stats::{LiftingRegistry, NoiseInteraction, NoiseModel, SmcConfig};
        let phi = Formula::parse("P>=0.5(always[0, 1](x > 0))").unwrap();
        let mut lifting = LiftingRegistry::new();
        lifting.register(
            "x",
            NoiseModel::gaussian(0.0, 0.5).unwrap(),
            NoiseInteraction::Additive,
        );
        let config = SmcConfig {
            samples: 2000,
            confidence: 0.95,
            seed: 3,
            ..Default::default()
        };
        let mut monitor = StreamMonitor::with_lifting(&phi, &lifting, &config).unwrap();
        monitor.update(0.0, &[("x", 3.0)]).unwrap();
        assert!(monitor.update(1.0, &[("x", 3.0)]).unwrap().value() > 0.0);
    }

    #[cfg(feature = "statistical")]
    #[test]
    fn a_probabilistic_verdict_waits_for_its_window() {
        use crate::stats::{LiftingRegistry, NoiseInteraction, NoiseModel, SmcConfig};
        let mut lifting = LiftingRegistry::new();
        lifting.register("x", NoiseModel::gaussian(0.0, 0.05).unwrap(), NoiseInteraction::Additive);
        let config = SmcConfig {
            samples: 200,
            confidence: 0.95,
            seed: 11,
            ..Default::default()
        };
        for (formula, settled) in [
            ("P>=0.95(always[0, 4](x > 0.35))", f64::INFINITY),
            ("P<=0.05(always[0, 4](x > 0.35))", f64::NEG_INFINITY),
        ] {
            let phi = Formula::parse(formula).unwrap();
            let mut monitor = StreamMonitor::with_lifting(&phi, &lifting, &config).unwrap();
            for t in 0..4 {
                let rho = monitor.update(f64::from(t), &[("x", 1.0)]).unwrap();
                assert!(!rho.is_resolved(), "{formula} settled at t={t}: {rho:?}");
                assert_eq!(monitor.last_probability(), Some(0.0));
            }
            let rho = monitor.update(4.0, &[("x", 1.0)]).unwrap();
            assert_eq!(rho.concrete(), Some(settled), "{formula} at t=4");
            assert_eq!(monitor.last_probability(), Some(1.0));
        }
        let phi = Formula::parse("P>=0.5(always[0, 2](x > 0))").unwrap();
        let mut early = StreamMonitor::with_lifting(&phi, &lifting, &config).unwrap();
        assert_eq!(early.update(0.0, &[("x", -4.0)]).unwrap().concrete(), Some(f64::NEG_INFINITY));
        let phi = Formula::parse("P>=0.95(always(x > 0.35))").unwrap();
        let mut open = StreamMonitor::with_lifting(&phi, &lifting, &config).unwrap();
        for t in 0..4 {
            let rho = open.update(f64::from(t), &[("x", 1.0)]).unwrap();
            assert!(!rho.is_resolved(), "unbounded always settled at t={t}");
        }
    }

    #[test]
    fn a_non_finite_reading_is_refused_as_the_trace_refuses_it() {
        for formula in ["historically[0, 2](x > 0)", "always[0, 2](x > 0)"] {
            let mut monitor = StreamMonitor::new(formula).unwrap();
            for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
                assert!(matches!(
                    monitor.update(0.0, &[("x", bad)]),
                    Err(Error::NonFiniteSample { kind: "value", .. })
                ));
                assert!(matches!(
                    monitor.update_packed(0.0, &[bad]),
                    Err(Error::NonFiniteSample { kind: "value", .. })
                ));
            }
            assert!(monitor.update(0.0, &[("x", 4.0)]).is_ok());
        }
    }

    #[test]
    fn a_junction_with_one_branch_that_never_settles_reports_no_bound() {
        let mut monitor = StreamMonitor::new("eventually(x > 0) or (y > 0)").unwrap();
        for (t, x, y) in [(0.0, -5.0, 1.0), (1.0, -5.0, 100.0)] {
            let rho = monitor.update(t, &[("x", x), ("y", y)]).unwrap();
            assert!(!rho.is_resolved(), "settled at t={t}");
            assert_eq!(rho, Robustness::UNKNOWN, "at t={t}");
        }
    }

    #[cfg(feature = "statistical")]
    #[test]
    fn a_nested_probabilistic_operator_is_rejected() {
        use crate::stats::{LiftingRegistry, SmcConfig};
        let phi = Formula::parse("always[0, 2](P>=0.5(x > 0))").unwrap();
        let config = SmcConfig {
            samples: 100,
            confidence: 0.95,
            seed: 1,
            ..Default::default()
        };
        assert!(matches!(
            StreamMonitor::with_lifting(&phi, &LiftingRegistry::new(), &config),
            Err(Error::Unsupported { .. })
        ));
    }

    #[cfg(feature = "statistical")]
    #[test]
    fn an_unusable_config_is_rejected_online_as_it_is_offline() {
        use crate::stats::{LiftingRegistry, SmcConfig};
        let phi = Formula::parse("P>=0.5(x > 0)").unwrap();
        let empty = SmcConfig {
            samples: 0,
            ..Default::default()
        };
        assert!(matches!(
            StreamMonitor::with_lifting(&phi, &LiftingRegistry::new(), &empty),
            Err(Error::InvalidConfig { .. })
        ));
        let certain = SmcConfig {
            confidence: 1.0,
            ..Default::default()
        };
        assert!(matches!(
            StreamMonitor::with_lifting(&phi, &LiftingRegistry::new(), &certain),
            Err(Error::InvalidConfig { .. })
        ));
    }
}