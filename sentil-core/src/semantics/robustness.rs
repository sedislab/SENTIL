//! Quantitative robustness, the signed margin by which a formula holds or fails.

/// The robustness of a formula at an instant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Robustness {
    /// A fully determined robustness margin.
    Concrete(f64),
    /// A margin known only to lie within `[lower, upper]`.
    Interval(f64, f64),
}

impl Robustness {
    /// Robustness of a property that holds unconditionally.
    pub const TRUE: Robustness = Robustness::Concrete(f64::INFINITY);
    /// Robustness of a property that fails unconditionally.
    pub const FALSE: Robustness = Robustness::Concrete(f64::NEG_INFINITY);

    /// A single representative value: the margin itself when concrete, or the
    /// midpoint of the interval otherwise.
    pub fn value(&self) -> f64 {
        match *self {
            Robustness::Concrete(r) => r,
            Robustness::Interval(lower, upper) => f64::midpoint(lower, upper),
        }
    }

    /// The greatest lower bound on the robustness.
    pub fn lower(&self) -> f64 {
        match *self {
            Robustness::Concrete(r) | Robustness::Interval(r, _) => r,
        }
    }

    /// The least upper bound on the robustness.
    pub fn upper(&self) -> f64 {
        match *self {
            Robustness::Concrete(r) | Robustness::Interval(_, r) => r,
        }
    }

    /// Whether the property is satisfied, using the representative value.
    ///
    /// A robustness of exactly zero counts as satisfied, matching the convention
    /// that a predicate holds on its boundary.
    pub fn is_satisfied(&self) -> bool {
        self.value() >= 0.0
    }

    /// Negation, flipping the sign.
    #[must_use]
    pub fn negate(self) -> Robustness {
        match self {
            Robustness::Concrete(r) => Robustness::Concrete(-r),
            Robustness::Interval(lower, upper) => Robustness::Interval(-upper, -lower),
        }
    }

    /// Conjunction: the pointwise minimum of two robustness values.
    #[must_use]
    pub fn min(self, other: Robustness) -> Robustness {
        Self::combine(self, other, f64::min)
    }

    /// Disjunction: the pointwise maximum of two robustness values.
    #[must_use]
    pub fn max(self, other: Robustness) -> Robustness {
        Self::combine(self, other, f64::max)
    }

    /// Implication, equal to `max(negate(self), other)`.
    #[must_use]
    pub fn implies(self, other: Robustness) -> Robustness {
        self.negate().max(other)
    }

    /// `op` must be monotone in both arguments.
    fn combine(a: Robustness, b: Robustness, op: fn(f64, f64) -> f64) -> Robustness {
        match (a, b) {
            (Robustness::Concrete(x), Robustness::Concrete(y)) => Robustness::Concrete(op(x, y)),
            (Robustness::Concrete(x), Robustness::Interval(lo, hi))
            | (Robustness::Interval(lo, hi), Robustness::Concrete(x)) => {
                Robustness::Interval(op(lo, x), op(hi, x))
            }
            (Robustness::Interval(lo1, hi1), Robustness::Interval(lo2, hi2)) => {
                Robustness::Interval(op(lo1, lo2), op(hi1, hi2))
            }
        }
    }
}