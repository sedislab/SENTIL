import json
import os

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt

import style

HERE = os.path.dirname(os.path.abspath(__file__))
RESULTS = os.path.normpath(os.path.join(HERE, "..", "results"))

def load():
    records = []
    for name in sorted(os.listdir(RESULTS)):
        if name.endswith(".jsonl"):
            with open(os.path.join(RESULTS, name), encoding="utf-8") as handle:
                records.extend(json.loads(line) for line in handle if line.strip())
    return records

def save(fig, name):
    out = os.path.join(RESULTS, name)
    fig.savefig(out)
    plt.close(fig)
    print("wrote", out)

def length_series(records, tool, lang, question, benchmark="scalability/length"):
    rows = [r for r in records if r.get("tool") == tool and r.get("language") == lang
            and r.get("benchmark") == benchmark and r.get("question") == question]
    pts = sorted((r["size"], r["timing"]["mean_ms"]) for r in rows)
    return [p[0] for p in pts], [p[1] for p in pts]

def discrete_offline(records):
    sx, sy = length_series(records, "sentil", "full_signal")
    rx, ry = length_series(records, "rtamt", "full_signal")
    mx, my = length_series(records, "moonlight", "full_signal")
    if not sx or not rx:
        return
    fig, ax = plt.subplots()
    ax.plot(rx, ry, color=style.TOOL["rtamt"], marker="s", label="RTAMT")
    if mx:
        ax.plot(mx, my, color=style.TOOL["moonlight"], marker="^", label="MoonLight")
    style.hero_line(ax, sx, sy, "SENTIL")
    shared = sorted(set(sx) & set(rx))[-1]
    style.annotate_speedup(ax, shared, dict(zip(sx, sy))[shared], dict(zip(rx, ry))[shared],
                           f"{dict(zip(rx, ry))[shared] / dict(zip(sx, sy))[shared]:.0f}x")
    ax.set_xscale("log"); ax.set_yscale("log")
    ax.set_xlabel("trace length (samples)")
    ax.set_ylabel("time for the whole robustness signal (ms)")
    ax.set_title("Offline discrete STL: SENTIL vs RTAMT and MoonLight")
    ax.legend(loc="upper left")
    save(fig, "discrete_offline.png")

def dense_offline(records):
    sx, sy = length_series(records, "sentil", "monitoring")
    bx, by = length_series(records, "breach", "monitoring")
    if not sx or not bx:
        return
    fig, ax = plt.subplots()
    ax.plot(bx, by, color=style.TOOL["breach"], marker="s", label="Breach (dense-time)")
    style.hero_line(ax, sx, sy, "SENTIL")
    shared = sorted(set(sx) & set(bx))[-1]
    style.annotate_speedup(ax, shared, dict(zip(sx, sy))[shared], dict(zip(bx, by))[shared],
                           f"{dict(zip(bx, by))[shared] / dict(zip(sx, sy))[shared]:.0f}x")
    ax.set_xscale("log"); ax.set_yscale("log")
    ax.set_xlabel("trace length (samples)")
    ax.set_ylabel("time to answer the monitoring query (ms)")
    ax.set_title("Dense-time monitoring: SENTIL vs Breach")
    ax.legend(loc="center left")
    save(fig, "dense_offline.png")

def dense_cost(records):
    dense = sorted((r["size"], r["timing"]["mean_ms"]) for r in records if r.get("benchmark") == "dense/length")
    disc = length_series(records, "sentil", "rust", "full_signal")
    if not dense or not disc[0]:
        return
    fig, ax = plt.subplots()
    ax.plot([p[0] for p in dense], [p[1] for p in dense], color=style.TOOL["sentil"], marker="o", label="dense time")
    ax.plot(disc[0], disc[1], color=style.FAINT, marker="o", label="discrete grid")
    ax.set_xscale("log"); ax.set_yscale("log")
    ax.set_xlabel("trace length (samples)")
    ax.set_ylabel("time for the whole robustness signal (ms)")
    ax.set_title("Cost of dense over discrete time, both linear")
    ax.legend(loc="upper left")
    save(fig, "dense_cost.png")

def streaming_online(records):
    rows = [r for r in records if r.get("benchmark") == "streaming"]
    if not rows:
        return
    us = {r["language"]: r["timing"]["mean_ms"] * 1000.0 for r in rows}
    order = sorted(us, key=us.get)
    colors = [style.TOOL["sentil"] if lang in ("rust", "c") else style.FAINT for lang in order]
    fig, ax = plt.subplots()
    bars = ax.bar(order, [us[l] for l in order], color=colors, width=0.66)
    ax.axhline(100.0, color=style.TOOL["rtamt"], linestyle=(0, (4, 3)), linewidth=1.1)
    ax.text(len(order) - 0.5, 108, "10 kHz real-time budget (100 us)", ha="right", va="bottom",
            fontsize=9, color=style.INK)
    ax.set_yscale("log")
    ax.set_ylabel("per-sample update (us)")
    ax.set_title("Online streaming cost per sample, every binding")
    for b, l in zip(bars, order):
        ax.text(b.get_x() + b.get_width() / 2, us[l] * 1.12, f"{us[l]:.2f}", ha="center", va="bottom", fontsize=8.5, color=style.INK)
    save(fig, "streaming_online.png")

def scaling(records):
    fx, fy = length_series(records, "sentil", "rust", "full_signal")
    mx, my = length_series(records, "sentil", "rust", "monitoring")
    if not fx or not mx:
        return
    fig, ax = plt.subplots()
    ax.plot(fx, fy, color=style.FAINT, marker="o", label="whole signal (linear)")
    style.hero_line(ax, mx, my, "monitoring query (flat)")
    ax.set_xscale("log"); ax.set_yscale("log")
    ax.set_xlabel("trace length (samples)")
    ax.set_ylabel("time per evaluation (ms)")
    ax.legend(loc="center left")
    save(fig, "scaling.png")

def memory(records):
    rows = sorted((r["size"], r["peak_rss_bytes"] / 1e6) for r in records if r.get("benchmark") == "memory/length")
    if not rows:
        return
    fig, ax = plt.subplots()
    style.hero_line(ax, [p[0] for p in rows], [p[1] for p in rows], "resident memory")
    ax.set_xscale("log")
    ax.set_ylim(0, max(p[1] for p in rows) * 2.2)
    ax.set_xlabel("samples streamed")
    ax.set_ylabel("peak resident memory (MB)")
    ax.set_title("Memory is set by the window, not the stream length")
    save(fig, "memory.png")

THROUGHPUT_MODELS = ("single_step", "always_ten", "eventually_ten")

def smc_throughput(records):
    rows = [r for r in records if "device" in r and r.get("model") in THROUGHPUT_MODELS and "throughput_per_s" in r]
    if not rows:
        return
    series = {}
    for r in rows:
        series.setdefault(r["device"], []).append((r["samples"], r["throughput_per_s"] / 1e6))
    fig, ax = plt.subplots()
    color = {"gpu": style.TOOL["sentil"], "cpu": style.FAINT}
    for device, pts in sorted(series.items()):
        agg = {}
        for n, t in pts:
            agg.setdefault(n, []).append(t)
        xs = sorted(agg)
        ys = [max(agg[n]) for n in xs]
        ax.plot(xs, ys, color=color.get(device, style.INK), marker="o", label=device.upper())
    ax.set_xscale("log"); ax.set_yscale("log")
    ax.set_xlabel("samples")
    ax.set_ylabel("throughput (million realizations/s)")
    ax.set_title("Statistical model checking throughput, GPU vs CPU")
    ax.legend(loc="lower right")
    save(fig, "smc_throughput.png")

def smc_accuracy(records):
    rows = [r for r in records if "device" in r and r.get("ground_truth") is not None and r["samples"] <= 100_000]
    if not rows:
        return
    fig, ax = plt.subplots(figsize=(5.4, 5.4))
    ax.plot([0, 1], [0, 1], color=style.FAINT, linewidth=1.0, zorder=1)
    ax.scatter([r["ground_truth"] for r in rows], [r["probability"] for r in rows],
               s=34, color=style.TOOL["sentil"], edgecolor="white", linewidth=0.5, zorder=3)
    ax.set_xlabel("known probability")
    ax.set_ylabel("SENTIL estimate")
    ax.set_title("Estimate against closed-form truth")
    save(fig, "smc_accuracy.png")

def particles(records):
    rows = [r for r in records if r.get("benchmark") == "rare_event/particles"]
    if not rows:
        return
    events = {}
    for r in rows:
        events.setdefault(r["event"], {}).setdefault(r["particles"], []).append(r["rel_error"])
    fig, ax = plt.subplots()
    for i, (event, by_count) in enumerate(sorted(events.items())):
        counts = sorted(by_count)
        errs = [sum(by_count[c]) / len(by_count[c]) for c in counts]
        ax.plot(counts, errs, color=style.CYCLE[i], marker="o", label=event)
    ax.set_xscale("log"); ax.set_yscale("log")
    ax.set_xlabel("particles")
    ax.set_ylabel("relative error vs Monte Carlo truth")
    ax.set_title("Rare-event estimate sharpens with particles")
    ax.legend(loc="upper right")
    save(fig, "particles.png")

def smc_circadian(records):
    s = next((r for r in records if r.get("benchmark") == "smc/circadian" and r.get("tool") == "sentil"), None)
    p = next((r for r in records if r.get("benchmark") == "smc/circadian" and r.get("tool") == "prism"), None)
    if not s or not p:
        return
    fig, ax = plt.subplots(figsize=(5.6, 4.6))
    tools, times, probs = ["PRISM", "SENTIL"], [p["time_ms"], s["time_ms"]], [p["probability"], s["probability"]]
    bars = ax.bar(tools, times, color=[style.TOOL["prism"], style.TOOL["sentil"]], width=0.52)
    ax.set_yscale("log")
    ax.set_ylabel("time for 10,000 samples (ms)")
    ax.set_title("SMC on the circadian CTMC, same model and property")
    for b, t, pr in zip(bars, times, probs):
        ax.text(b.get_x() + b.get_width() / 2, t * 1.15, f"P = {pr:.3f}\n{t:.0f} ms", ha="center", va="bottom", fontsize=9.5, color=style.INK)
    ax.set_ylim(top=max(times) * 3)
    save(fig, "smc_circadian.png")

def comparison_summary(records):
    # One picture of SENTIL against every baseline that runs, each at its largest common
    # size. The comparisons answer different questions, so each bar names its own.
    def series(tool, q, bench="scalability/length"):
        return {r["size"]: r["timing"]["mean_ms"] for r in records
                if r.get("tool") == tool and r.get("benchmark") == bench and r.get("question") == q}
    s_full, s_mon = series("sentil", "full_signal"), series("sentil", "monitoring")
    bars = []
    for tool, base, label in [
        ("rtamt", series("rtamt", "full_signal"), "RTAMT\ndiscrete, whole signal"),
        ("moonlight", series("moonlight", "full_signal"), "MoonLight\ndiscrete, whole signal"),
        ("breach", series("breach", "monitoring"), "Breach\ndense monitoring"),
    ]:
        shared = sorted(set(s_full if "signal" in label else s_mon) & set(base))
        if not shared:
            continue
        n = shared[-1]
        ref = s_full[n] if "signal" in label else s_mon[n]
        bars.append((label, base[n] / ref, style.TOOL[tool]))
    sc = next((r for r in records if r.get("benchmark") == "smc/circadian" and r.get("tool") == "sentil"), None)
    pr = next((r for r in records if r.get("benchmark") == "smc/circadian" and r.get("tool") == "prism"), None)
    if sc and pr:
        bars.append(("PRISM\nstatistical model checking", pr["time_ms"] / sc["time_ms"], style.TOOL["prism"]))
    if not bars:
        return
    bars.sort(key=lambda b: b[1])
    fig, ax = plt.subplots(figsize=(7.6, 4.8))
    ys = range(len(bars))
    ax.barh(list(ys), [b[1] for b in bars], color=[b[2] for b in bars], height=0.62)
    ax.set_yticks(list(ys))
    ax.set_yticklabels([b[0] for b in bars])
    ax.set_xscale("log")
    ax.set_xlabel("times faster than the baseline (log scale)")
    ax.set_title("SENTIL against every baseline that runs")
    for y, b in zip(ys, bars):
        ax.text(b[1] * 1.1, y, f"{b[1]:.0f}x", va="center", ha="left", fontsize=10, fontweight="semibold", color=style.INK)
    ax.set_xlim(right=max(b[1] for b in bars) * 2.2)
    save(fig, "comparison_summary.png")

def synthesis(records):
    rows = [r for r in records if "mode" in r]
    if not rows:
        return
    rows.sort(key=lambda r: (r["mode"], r["case"]))
    fig, ax = plt.subplots(figsize=(8.0, 4.6))
    colors = [style.TOOL["sentil"] if r["mode"] == "open_loop" else style.CYCLE[2] for r in rows]
    ax.bar(range(len(rows)), [r["timing"]["mean_ms"] for r in rows], color=colors, width=0.64)
    ax.set_xticks(range(len(rows)))
    ax.set_xticklabels([r["case"] for r in rows], rotation=25, ha="right")
    ax.set_ylabel("synthesis time (ms)")
    ax.set_title("Open-loop synthesis and one online planning step")
    save(fig, "synthesis.png")

def main():
    style.apply()
    records = load()
    if not records:
        print("no results found under", RESULTS)
        return
    for figure in (discrete_offline, dense_offline, dense_cost, streaming_online, scaling,
                   memory, smc_throughput, smc_accuracy, smc_circadian, comparison_summary,
                   particles, synthesis):
        figure(records)

if __name__ == "__main__":
    main()