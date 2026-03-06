import matplotlib
import matplotlib.pyplot as plt

_OKABE_ITO = {
    "blue": "#0072B2",
    "vermillion": "#D55E00",
    "orange": "#E69F00",
    "green": "#009E73",
    "sky": "#56B4E9",
    "purple": "#CC79A7",
    "yellow": "#F0E442",
    "black": "#222222",
}

TOOL = {
    "sentil": _OKABE_ITO["blue"],
    "rtamt": _OKABE_ITO["vermillion"],
    "breach": _OKABE_ITO["orange"],
    "uppaal": _OKABE_ITO["purple"],
    "prism": _OKABE_ITO["green"],
    "moonlight": _OKABE_ITO["sky"],
    "banquo": _OKABE_ITO["purple"],
}

CYCLE = [_OKABE_ITO[k] for k in ("blue", "vermillion", "green", "orange", "purple", "sky")]

INK = "#222222"
FAINT = "#8a8a8a"
GRID = "#e8e8e8"

def apply():
    matplotlib.rcParams.update({
        "figure.figsize": (7.2, 4.6),
        "figure.dpi": 200,
        "savefig.dpi": 200,
        "savefig.bbox": "tight",
        "font.family": "DejaVu Sans",
        "font.size": 11,
        "axes.titlesize": 13,
        "axes.titleweight": "semibold",
        "axes.titlepad": 12,
        "axes.labelsize": 11,
        "axes.labelcolor": INK,
        "axes.edgecolor": FAINT,
        "axes.linewidth": 0.8,
        "axes.spines.top": False,
        "axes.spines.right": False,
        "axes.grid": True,
        "axes.grid.axis": "y",
        "axes.axisbelow": True,
        "axes.prop_cycle": plt.cycler(color=CYCLE),
        "grid.color": GRID,
        "grid.linewidth": 0.9,
        "xtick.color": INK,
        "ytick.color": INK,
        "xtick.labelsize": 9.5,
        "ytick.labelsize": 9.5,
        "xtick.direction": "out",
        "ytick.direction": "out",
        "lines.linewidth": 2.3,
        "lines.markersize": 6,
        "lines.markeredgewidth": 0,
        "legend.frameon": False,
        "legend.fontsize": 10,
        "text.color": INK,
    })

def hero_line(ax, xs, ys, label):
    ax.plot(xs, ys, color=TOOL["sentil"], marker="o", label=label, zorder=5)

def footnote(fig, text):
    fig.text(0.5, -0.02, text, ha="center", va="top", fontsize=8.5, color=FAINT)