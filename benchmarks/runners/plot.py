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

def _series(records, tool, lang, question, benchmark="scalability/length"):
    return {r["size"]: r["timing"]["mean_ms"] for r in records
            if r.get("tool") == tool and r.get("language") == lang
            and r.get("benchmark") == benchmark and r.get("question") == question}

def _fmt_ms(v):
    if v >= 100:
        return f"{v:,.0f}"
    if v >= 1:
        return f"{v:.1f}"
    return f"{v:.3f}".rstrip("0").rstrip(".")

def _human_size(n):
    if n >= 1_000_000:
        return f"{n // 1_000_000}M"
    if n >= 1000:
        return f"{n // 1000}k"
    return str(int(n))

def bar_comparison(specs, sizes, ylabel, title, fname):
    # specs is [(label, {size: ms}, color)]
    fig, ax = plt.subplots(figsize=(8.6, 5.0))
    m = len(specs)
    width = 0.82 / m
    top = 0.0
    for i, (label, data, color) in enumerate(specs):
        xs = [j + i * width for j in range(len(sizes))]
        vals = [data.get(s, 0.0) for s in sizes]
        top = max(top, max(vals))
        rects = ax.bar(xs, vals, width=width, label=label, color=color, zorder=3)
        for rect, v in zip(rects, vals):
            if v > 0:
                ax.text(rect.get_x() + rect.get_width() / 2, v * 1.35, _fmt_ms(v),
                        ha="center", va="bottom", fontsize=7.5, color=style.INK)
    ax.set_yscale("log")
    ax.set_ylim(top=top * 8)
    ax.set_xticks([j + width * (m - 1) / 2 for j in range(len(sizes))])
    ax.set_xticklabels([_human_size(s) for s in sizes])
    ax.set_xlabel("trace length (samples)")
    ax.set_ylabel(ylabel)
    ax.set_title(title)
    ax.legend(loc="upper left", ncol=m)
    save(fig, fname)

def discrete_offline(records):
    s, rt = _series(records, "sentil", "rust", "full_signal"), _series(records, "rtamt", "python", "full_signal")
    ml = _series(records, "moonlight", "java", "full_signal")
    bq = _series(records, "banquo", "python", "full_signal")
    if not s or not rt:
        return
    sizes = sorted(set(s) & set(rt))
    specs = [("SENTIL", s, style.TOOL["sentil"]), ("RTAMT", rt, style.TOOL["rtamt"])]
    if ml:
        specs.append(("MoonLight", ml, style.TOOL["moonlight"]))
    if bq:
        specs.append(("Banquo", bq, style.TOOL["banquo"]))
    names = ", ".join(label for label, _, _ in specs[1:])
    bar_comparison(specs, sizes, "milliseconds to score the whole signal (lower is faster)",
                   f"Offline discrete STL: SENTIL vs {names}", "discrete_offline.png")

def dense_offline(records):
    s = _series(records, "sentil", "rust", "monitoring")
    br = _series(records, "breach", "matlab", "monitoring")
    rt = _series(records, "rtamt", "python", "monitoring")
    if not s or not br:
        return
    sizes = sorted(set(s) & set(br))
    specs = [("SENTIL", s, style.TOOL["sentil"]), ("Breach", br, style.TOOL["breach"])]
    title = "Dense-time monitoring: SENTIL vs Breach"
    if rt:
        specs.append(("RTAMT", rt, style.TOOL["rtamt"]))
        title = "Dense-time monitoring: SENTIL vs Breach and RTAMT"
    bar_comparison(specs, sizes, "milliseconds to answer the monitoring query (lower is faster)",
                   title, "dense_offline.png")

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

_BINDING_NAMES = {"rust": "Rust core", "c": "C", "cpp": "C++", "java": "Java",
                  "julia": "Julia", "matlab": "MATLAB", "python": "Python"}

def scaling_compare(records, lang, label, fname):
    rx, ry = length_series(records, "sentil", "rust", "full_signal")
    bx, by = length_series(records, "sentil", lang, "full_signal")
    if not rx or not bx:
        return
    fig, ax = plt.subplots()
    ax.plot(rx, ry, color=style.TOOL["sentil"], marker="o", label="SENTIL (Rust core)", zorder=6)
    ax.plot(bx, by, color=style.TOOL["sentil"], marker="s", linestyle=(0, (4, 2)),
            label=f"SENTIL ({label})", zorder=5)
    for tool, tlabel, lg in (("rtamt", "RTAMT", "python"), ("moonlight", "MoonLight", "java"),
                             ("banquo", "Banquo", "python")):
        tx, ty = length_series(records, tool, lg, "full_signal")
        if tx:
            ax.plot(tx, ty, color=style.TOOL[tool], marker="o", label=tlabel, zorder=4)
    ax.set_xscale("log"); ax.set_yscale("log")
    ax.set_xlabel("trace length (samples)")
    ax.set_ylabel("time to score the whole signal (ms)")
    ax.set_title(f"Offline cost over length: SENTIL ({label}) and the core vs the baselines")
    ax.legend(loc="upper left")
    save(fig, fname)

def streaming_compare(records, lang, label, fname):
    us = {r["language"]: r["timing"]["mean_ms"] * 1000.0
          for r in records if r.get("benchmark") == "streaming"}
    if "rust" not in us or lang not in us:
        return
    order = sorted(us, key=us.get)
    colors = [style.TOOL["sentil"] if l in ("rust", lang) else style.FAINT for l in order]
    fig, ax = plt.subplots()
    bars = ax.bar([_BINDING_NAMES.get(l, l) for l in order], [us[l] for l in order], color=colors, width=0.66)
    ax.axhline(100.0, color=style.TOOL["rtamt"], linestyle=(0, (4, 3)), linewidth=1.1)
    ax.text(len(order) - 0.5, 108, "10 kHz real-time budget (100 us)", ha="right", va="bottom",
            fontsize=9, color=style.INK)
    ax.set_yscale("log")
    ax.set_ylabel("per-sample update (us)")
    ax.set_title(f"Online streaming cost per sample")
    for b, l in zip(bars, order):
        ax.text(b.get_x() + b.get_width() / 2, us[l] * 1.12, f"{us[l]:.2f}", ha="center", va="bottom", fontsize=8, color=style.INK)
    save(fig, fname)

def memory(records):
    def rss(tool):
        return sorted((r["size"], r["peak_rss_bytes"] / 1e6) for r in records
                      if r.get("benchmark") == "memory/length" and r.get("tool") == tool)
    s = rss("sentil")
    if not s:
        return
    fig, ax = plt.subplots()
    style.hero_line(ax, [p[0] for p in s], [p[1] for p in s], "SENTIL (streaming)")
    for tool, label in (("rtamt", "RTAMT (holds the stream)"), ("banquo", "Banquo (holds the stream)")):
        pts = rss(tool)
        if pts:
            ax.plot([p[0] for p in pts], [p[1] for p in pts], color=style.TOOL[tool],
                    marker="o", label=label, zorder=4)
    ax.set_xscale("log")
    ax.set_yscale("log")
    ax.set_xlabel("samples monitored")
    ax.set_ylabel("peak resident memory (MB)")
    ax.set_title("Monitoring a stream: SENTIL streams, the offline tools hold it all")
    ax.legend(loc="upper left")
    style.footnote(fig, "RTAMT's online monitor and Banquo have no bounded-memory mode for this "
                        "bounded-future formula, so following the stream means keeping all of it.")
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

def smc_model(records, benchmark, title, out):
    def pick(tool):
        return next((r for r in records if r.get("benchmark") == benchmark and r.get("tool") == tool), None)
    s, p = pick("sentil"), pick("prism")
    if not s or not p:
        return
    bars = [("SENTIL", s, style.TOOL["sentil"]), ("PRISM", p, style.TOOL["prism"])]
    for tool, label in (("modest", "Modest"), ("uppaal", "UPPAAL-SMC")):
        r = pick(tool)
        if r:
            bars.append((label, r, style.TOOL[tool]))
    bars.sort(key=lambda b: b[1]["time_ms"])
    fig, ax = plt.subplots(figsize=(5.8, 4.8))
    tools = [label for label, _, _ in bars]
    secs = [r["time_ms"] / 1000.0 for _, r, _ in bars]
    probs = [r["probability"] for _, r, _ in bars]
    rects = ax.bar(tools, secs, color=[c for _, _, c in bars], width=0.5, zorder=3)
    ax.set_ylabel("seconds at about 10,000 samples (lower is faster)")
    ax.set_title(title)
    ax.set_ylim(top=max(secs) * 1.25)
    for rect, sec, pr in zip(rects, secs, probs):
        ax.text(rect.get_x() + rect.get_width() / 2, sec + max(secs) * 0.02,
                f"{sec:.1f} s\nP = {pr:.3f}", ha="center", va="bottom", fontsize=10, color=style.INK)
    save(fig, out)

def smc_circadian(records):
    smc_model(records, "smc/circadian", "Statistical model checking the circadian CTMC", "smc_circadian.png")

def smc_tandem_queue(records):
    smc_model(records, "smc/tandem_queue", "Statistical model checking the tandem queue", "smc_tandem_queue.png")

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
    for figure in (discrete_offline, dense_offline, dense_cost, streaming_online,
                   scaling, memory, smc_throughput, smc_accuracy, smc_circadian,
                   smc_tandem_queue, particles, synthesis):
        figure(records)
    for lang, label in (("c", "C ABI"), ("cpp", "C++"), ("python", "Python"),
                        ("julia", "Julia"), ("java", "Java"), ("matlab", "MATLAB")):
        scaling_compare(records, lang, label, f"scaling_{lang}.png")
        streaming_compare(records, lang, label, f"streaming_{lang}.png")
        memory_compare(records, label, f"memory_{lang}.png")

if __name__ == "__main__":
    main()