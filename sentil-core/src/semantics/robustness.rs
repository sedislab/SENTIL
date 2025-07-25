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
    ///
    /// While streaming, a temporal formula can return an unresolved
    /// [`Interval`](Self::Interval), and its midpoint is only provisional (it can
    /// even be infinite). Use [`concrete`](Self::concrete) when you need the
    /// settled answer rather than a representative one.
    pub fn value(&self) -> f64 {
        match *self {
            Robustness::Concrete(r) => r,
            Robustness::Interval(lower, upper) => f64::midpoint(lower, upper),
        }
    }

    /// Whether the verdict has settled to a single value.
    pub fn is_resolved(&self) -> bool {
        matches!(self, Robustness::Concrete(_))
    }

    /// The settled margin, or `None` while the verdict is still an interval.
    pub fn concrete(&self) -> Option<f64> {
        match *self {
            Robustness::Concrete(r) => Some(r),
            Robustness::Interval(..) => None,
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
    /// that a predicate holds on its boundary. On an unresolved
    /// [`Interval`](Self::Interval) this decides from the midpoint and is
    /// therefore provisional; check [`is_resolved`](Self::is_resolved) first if a
    /// final verdict is required.
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

#[cfg(test)]
mod tests {
    #![allow(
        clippy::float_cmp,
        reason = "these robustness values are exact integer-valued f64 results"
    )]

    use super::*;

    #[test]
    fn concrete_settles_only_when_resolved() {
        let settled = Robustness::Concrete(2.0);
        assert!(settled.is_resolved());
        assert_eq!(settled.concrete(), Some(2.0));

        let pending = Robustness::Interval(-1.0, 3.0);
        assert!(!pending.is_resolved());
        assert_eq!(pending.concrete(), None);
    }

    #[test]
    fn conjunction_takes_the_minimum_and_disjunction_the_maximum() {
        let a = Robustness::Concrete(3.0);
        let b = Robustness::Concrete(-1.0);
        assert_eq!(a.min(b), Robustness::Concrete(-1.0));
        assert_eq!(a.max(b), Robustness::Concrete(3.0));
    }

    #[test]
    fn negation_flips_sign_and_mirrors_intervals() {
        assert_eq!(
            Robustness::Concrete(2.0).negate(),
            Robustness::Concrete(-2.0)
        );
        assert_eq!(
            Robustness::Interval(-1.0, 3.0).negate(),
            Robustness::Interval(-3.0, 1.0)
        );
    }

    #[test]
    fn implication_is_max_of_negated_left_and_right() {
        let antecedent = Robustness::Concrete(5.0);
        let consequent = Robustness::Concrete(3.0);
        assert_eq!(antecedent.implies(consequent), Robustness::Concrete(3.0));
    }

    #[test]
    fn combining_a_concrete_with_an_interval_widens() {
        let concrete = Robustness::Concrete(2.0);
        let interval = Robustness::Interval(0.0, 4.0);
        assert_eq!(concrete.min(interval), Robustness::Interval(0.0, 2.0));
        assert_eq!(concrete.max(interval), Robustness::Interval(2.0, 4.0));
    }

    #[test]
    fn value_and_satisfaction() {
        assert_eq!(Robustness::Interval(1.0, 3.0).value(), 2.0);
        assert_eq!(Robustness::Interval(1.0, 3.0).lower(), 1.0);
        assert_eq!(Robustness::Interval(1.0, 3.0).upper(), 3.0);
        assert!(Robustness::Concrete(0.0).is_satisfied());
        assert!(!Robustness::Concrete(-0.1).is_satisfied());
        assert!(Robustness::TRUE.is_satisfied());
        assert!(!Robustness::FALSE.is_satisfied());
    }
}