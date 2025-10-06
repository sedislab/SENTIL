"""A small DSL for writing specifications in Python."""

from typing import Optional

from ._sentil import Expr, Formula

def var(name: str) -> Expr:
    """A reference to a trace variable."""
    return Expr.var(name)

def lit(value: float) -> Expr:
    """A constant expression."""
    return Expr.constant(value)

def parse(text: str) -> Formula:
    """Parse a formula from its textual syntax."""
    return Formula.parse(text)

def always(formula: Formula, lower: float = 0.0, upper: Optional[float] = None) -> Formula:
    """Require `formula` at every point of [lower, upper]."""
    return formula.always(lower, upper)

def eventually(formula: Formula, lower: float = 0.0, upper: Optional[float] = None) -> Formula:
    """Require `formula` at some point of [lower, upper]."""
    return formula.eventually(lower, upper)

def historically(formula: Formula, lower: float = 0.0, upper: Optional[float] = None) -> Formula:
    """Require `formula` at every past point of [lower, upper]."""
    return formula.historically(lower, upper)

def once(formula: Formula, lower: float = 0.0, upper: Optional[float] = None) -> Formula:
    """Require `formula` at some past point of [lower, upper]."""
    return formula.once(lower, upper)

def nxt(formula: Formula) -> Formula:
    """The formula at the next sample."""
    return formula.next()

def until(
    left: Formula, right: Formula, lower: float = 0.0, upper: Optional[float] = None
) -> Formula:
    """Require `left` until `right` becomes true within [lower, upper]."""
    return left.until(right, lower, upper)

def since(
    left: Formula, right: Formula, lower: float = 0.0, upper: Optional[float] = None
) -> Formula:
    """Require `left` since `right` was true within the past [lower, upper]."""
    return left.since(right, lower, upper)