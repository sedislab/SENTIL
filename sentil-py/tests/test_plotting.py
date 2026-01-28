import pytest

matplotlib = pytest.importorskip("matplotlib")
matplotlib.use("Agg")

import sentil
from matplotlib.figure import Figure
from sentil.plotting import plot_robustness, plot_signals, plot_smc_convergence, robustness_figure

def test_robustness_figure_from_a_formula():
    trace = sentil.Trace([0, 1, 2, 3], {"speed": [12, 9, 7, 4]})
    phi = sentil.Formula.parse("always[0, 1] (speed > 5)")
    fig = robustness_figure(phi, trace)
    assert isinstance(fig, Figure)
    matplotlib.pyplot.close(fig)

def test_plot_robustness_array():
    fig = plot_robustness([0, 1, 2], [2.0, -1.0, 0.5])
    assert isinstance(fig, Figure)
    matplotlib.pyplot.close(fig)

def test_plot_signals_indexes_without_times():
    fig = plot_signals({"x": [1, 2, 3], "y": [3, 2, 1]})
    assert isinstance(fig, Figure)
    matplotlib.pyplot.close(fig)

def test_plot_smc_convergence_with_truth():
    fig = plot_smc_convergence(
        [10, 100, 1000],
        [0.40, 0.45, 0.48],
        [(0.30, 0.50), (0.40, 0.50), (0.46, 0.50)],
        true_probability=0.48,
    )
    assert isinstance(fig, Figure)
    matplotlib.pyplot.close(fig)