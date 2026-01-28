"""Matplotlib helpers for robustness, signals, and SMC convergence."""

from __future__ import annotations

from typing import Optional, Sequence

try:
    import matplotlib.pyplot as plt
    from matplotlib.figure import Figure
except ModuleNotFoundError as exc:
    raise ModuleNotFoundError(
        "sentil.plotting needs matplotlib; install it with `pip install sentil[plotting]`"
    ) from exc

from ._sentil import Formula, Trace

__all__ = [
    "robustness_figure",
    "plot_robustness",
    "plot_signals",
    "plot_smc_convergence",
]

def robustness_figure(formula: Formula, trace: Trace, **kwargs) -> Figure:
    """Compute the robustness signal of `formula` over `trace` and plot it."""
    return plot_robustness(trace.times, formula.robustness_signal(trace), **kwargs)

def plot_robustness(
    times: Sequence[float],
    robustness: Sequence[float],
    title: str = "robustness over time",
    figsize: tuple[float, float] = (10, 6),
) -> Figure:
    """Plot a robustness signal against time, with the satisfaction line at zero."""
    fig, ax = plt.subplots(figsize=figsize)
    ax.plot(times, robustness, linewidth=2)
    ax.axhline(0.0, color="crimson", linestyle="--", alpha=0.6)
    ax.set_xlabel("time")
    ax.set_ylabel("robustness")
    ax.set_title(title)
    ax.grid(True, alpha=0.3)
    return fig

def plot_signals(
    signals: dict[str, Sequence[float]],
    times: Optional[Sequence[float]] = None,
    title: str = "signals",
    figsize: tuple[float, float] = (10, 6),
) -> Figure:
    """Plot each named signal on shared axes."""
    fig, ax = plt.subplots(figsize=figsize)
    for name, values in signals.items():
        t = range(len(values)) if times is None else times[: len(values)]
        ax.plot(t, values, label=name, linewidth=2)
    ax.set_xlabel("time")
    ax.set_ylabel("value")
    ax.set_title(title)
    ax.legend()
    ax.grid(True, alpha=0.3)
    return fig

def plot_smc_convergence(
    samples: Sequence[int],
    estimates: Sequence[float],
    intervals: Sequence[tuple[float, float]],
    true_probability: Optional[float] = None,
    figsize: tuple[float, float] = (10, 6),
) -> Figure:
    """Plot a satisfaction-probability estimate against the sample count."""
    fig, ax = plt.subplots(figsize=figsize)
    ax.plot(samples, estimates, color="steelblue", linewidth=2, label="estimate")
    lower = [interval[0] for interval in intervals]
    upper = [interval[1] for interval in intervals]
    ax.fill_between(samples, lower, upper, color="steelblue", alpha=0.25, label="confidence interval")
    if true_probability is not None:
        ax.axhline(true_probability, color="crimson", linestyle="--", label="true probability")
    ax.set_xlabel("samples")
    ax.set_ylabel("satisfaction probability")
    ax.set_title("SMC convergence")
    ax.legend()
    ax.grid(True, alpha=0.3)
    return fig