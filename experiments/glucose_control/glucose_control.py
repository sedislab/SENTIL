"""Artificial-pancreas case study on the UVA/Padova model."""

import json
import os

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np

import sentil

HERE = os.path.dirname(os.path.abspath(__file__))
RESULTS = os.path.join(HERE, "results")

# UVA/Padova average-adult parameters (Dalla Man 2007 / 2014).
BW = 69.7  # kg
VG = 1.88  # dL/kg
K1, K2 = 0.065, 0.079  # 1/min
VI = 0.05  # L/kg
M1, M2, M4 = 0.190, 0.484, 0.194
HEB = 0.6
KMAX, KMIN, KABS, KGRI = 0.0558, 0.0080, 0.057, 0.0558
F, BB, CC = 0.90, 0.82, 0.010
KP1, KP2, KP3, KI = 2.70, 0.0021, 0.009, 0.0079
FCNS, VM0, VMX, KM0, P2U = 1.0, 2.50, 0.047, 225.59, 0.0331
KE1, KE2 = 0.0005, 339.0
KA1, KA2, KD = 0.0018, 0.0182, 0.0164
GB = 120.0  # mg/dL
GPB = GB * VG  # mg/kg
PMOL_PER_UNIT = 6000.0

# (minute, carbohydrates in grams)
MEALS = [(7 * 60, 55.0), (12 * 60 + 30, 75.0), (19 * 60, 65.0)]

ICR = 8.5
CONTROLLERS = {"missed_lunch_bolus": (ICR, {1}), "tuned": (ICR, set())}

DURATION = 24 * 60  # minutes
CGM_SIGMA = 7.0  # mg/dL


def _kempt(qsto, dose):
    """Gastric-emptying rate."""
    if dose <= 0:
        return KMAX
    alpha = 5.0 / (2.0 * dose * (1.0 - BB))
    beta = 5.0 / (2.0 * dose * CC)
    return KMIN + 0.5 * (KMAX - KMIN) * (
        np.tanh(alpha * (qsto - BB * dose)) - np.tanh(beta * (qsto - CC * dose)) + 2.0
    )


def _risk(g):
    """S2013 hypoglycemia amplification of insulin-dependent utilization."""
    if g >= GB:
        return 0.0
    gth = 60.0
    ref = max(g, gth)
    return 10.0 * (np.log(ref) ** 2 - np.log(GB) ** 2) ** 2


def _basal():
    """The basal insulin infusion that holds glucose at GB."""
    gt = GPB * K1 / K2
    for _ in range(300):
        uid = VM0 * gt / (KM0 + gt)
        gt = (K1 * GPB - uid) / K2
    uid = VM0 * gt / (KM0 + gt)
    e = max(KE1 * (GPB - KE2), 0.0)
    egp_needed = FCNS + uid + e
    ib = (KP1 - KP2 * GPB - egp_needed) / KP3  # pmol/L
    ipb = ib * VI
    m3 = HEB * M1 / (1.0 - HEB)
    ilb = M2 * ipb / (M1 + m3)
    basal_iir = (M2 + M4) * ipb - M1 * ilb       # pmol/kg/min
    return dict(ipb=ipb, ilb=ilb, gtb=gt, m3=m3, ib=ib, basal_iir=basal_iir)


def simulate(icr, skip):
    """Integrate the model with a basal-bolus controller."""
    st = _basal()
    gp, gt, ip, il = GPB, st["gtb"], st["ipb"], st["ilb"]
    isc1 = st["basal_iir"] / (KD + KA1)
    isc2 = KD * isc1 / KA2
    qsto1 = qsto2 = qgut = 0.0
    xl = i1 = st["ib"]
    x = 0.0
    m3 = st["m3"]
    dose_at = {int(round(t)): c * 1000.0 / BW for t, c in MEALS}  # g -> mg/kg
    total_dose = sum(c for _, c in MEALS) * 1000.0 / BW
    bolus_at = {}
    for k, (t, carbs) in enumerate(MEALS):
        if k not in skip:
            pmol = (carbs / icr) * PMOL_PER_UNIT / BW  # pmol/kg
            for minute in range(int(t), int(t) + 15):
                bolus_at[minute] = bolus_at.get(minute, 0.0) + pmol / 15.0

    sub = 10
    h = 1.0 / sub
    times, glucose = [], []
    for step in range(DURATION + 1):
        times.append(float(step))
        glucose.append(gp / VG)
        qsto1 += dose_at.get(step, 0.0)
        iir = st["basal_iir"] + bolus_at.get(step, 0.0)
        for _ in range(sub):
            conc_i = ip / VI
            g = gp / VG
            qsto = qsto1 + qsto2
            ke = _kempt(qsto, total_dose)
            ra = F * KABS * qgut
            egp = max(KP1 - KP2 * gp - KP3 * xl, 0.0)
            e = max(KE1 * (gp - KE2), 0.0)
            uid = (VM0 + VMX * x * (1.0 + _risk(g))) * gt / (KM0 + gt)
            rai = KA1 * isc1 + KA2 * isc2

            gp += h * (egp + ra - FCNS - e - K1 * gp + K2 * gt)
            gt += h * (-uid + K1 * gp - K2 * gt)
            qsto1 += h * (-KGRI * qsto1)
            qsto2 += h * (-ke * qsto2 + KGRI * qsto1)
            qgut += h * (-KABS * qgut + ke * qsto2)
            ip += h * (-(M2 + M4) * ip + M1 * il + rai)
            il += h * (-(M1 + m3) * il + M2 * ip)
            isc1 += h * (-(KD + KA1) * isc1 + iir)
            isc2 += h * (KD * isc1 - KA2 * isc2)
            xl += h * (-KI * (xl - i1))
            i1 += h * (-KI * (i1 - conc_i))
            x += h * (-P2U * x + P2U * (conc_i - st["ib"]))
    return times, glucose


def monitor(times, glucose):
    trace = sentil.Trace(times, {"glucose": glucose})

    deterministic = {
        "euglycemia": "always (glucose > 70 and glucose < 180)",
        "no_severe_hypo": "always (glucose > 54)",
        "bounded_hyper": "always (glucose < 250)",
        "hypo_recovers_30min": "always ((glucose < 70) implies eventually[0,30] (glucose > 70))",
    }
    results = {}
    for name, text in deterministic.items():
        phi = sentil.parse(text)
        rob = phi.robustness(trace)
        results[name] = {"formula": text, "robustness": round(rob, 4), "satisfied": rob >= 0.0}

    out_of_range = sentil.parse("glucose > 70 and glucose < 180")
    results["unsafe_intervals"] = [
        (round(v.start, 1), round(v.end, 1)) for v in out_of_range.violations(trace)
    ]

    lifting = sentil.LiftingRegistry()
    lifting.register(
        "glucose", sentil.NoiseModel.gaussian(0.0, CGM_SIGMA), sentil.NoiseInteraction.Additive
    )
    config = sentil.SmcConfig(samples=4000, seed=7)
    prstl = sentil.parse("P>=0.95(always (glucose > 70))")
    smc = prstl.check(trace, lifting, config)
    results["hypo_safety_under_cgm_noise"] = {
        "formula": "P>=0.95(always (glucose > 70))",
        "probability": round(smc.probability, 4),
        "confidence_interval": [round(smc.interval.lower, 4), round(smc.interval.upper, 4)],
        "holds": smc.holds,
        "cgm_sigma": CGM_SIGMA,
    }
    return results


def plot(runs):
    fig, axes = plt.subplots(len(runs), 1, figsize=(11, 8), sharex=True)
    rng = np.random.default_rng(1)
    for ax, (name, times, glucose, results) in zip(axes, runs):
        hours = np.array(times) / 60.0
        glucose = np.array(glucose)
        ax.axhspan(70, 180, color="#bfe3c0", alpha=0.5, label="target range (70-180)")
        ax.axhline(54, color="#c0392b", ls="--", lw=1, label="severe hypo (54)")
        ax.scatter(hours, glucose + rng.normal(0, CGM_SIGMA, glucose.shape), s=3,
                   color="#7f8c8d", alpha=0.3, label="CGM readings")
        ax.plot(hours, glucose, color="#2c3e50", lw=1.8, label="plasma glucose")
        for start, carbs in MEALS:
            ax.axvline(start / 60.0, color="#e67e22", ls=":", lw=1)
        for (s, e) in results.get("unsafe_intervals", []):
            ax.axvspan(s / 60.0, e / 60.0, color="#e74c3c", alpha=0.25)
        ax.set_ylabel("glucose (mg/dL)")
        ax.set_xlim(0, 24)
        ax.set_ylim(40, 300)
        ax.set_title(f"{name} controller (red = SENTIL-flagged violations)")
        ax.legend(loc="upper right", fontsize=7, ncol=2)
    axes[-1].set_xlabel("time (hours)")
    fig.suptitle("Artificial pancreas on the UVA/Padova model: a controller SENTIL flags vs. one it clears", fontsize=12)
    fig.tight_layout()
    path = os.path.join(RESULTS, "glucose.png")
    fig.savefig(path, dpi=130)
    return path


def main():
    os.makedirs(RESULTS, exist_ok=True)
    report = {
        "case_study": "artificial_pancreas_glucose_control",
        "model": "UVA/Padova (Dalla Man 2007/2013) meal-simulation model, average adult, type-1 patient",
        "duration_min": DURATION,
        "meals": [{"minute": m, "carbs_g": c} for m, c in MEALS],
        "cgm_sigma": CGM_SIGMA,
        "controllers": {},
    }
    runs = []
    for name, (icr, skip) in CONTROLLERS.items():
        times, glucose = simulate(icr, skip)
        results = monitor(times, glucose)
        report["controllers"][name] = {
            "insulin_to_carb_ratio": icr,
            "skipped_meal_boluses": sorted(skip),
            "peak_glucose": round(float(max(glucose)), 1),
            "min_glucose": round(float(min(glucose)), 1),
            "specifications": results,
        }
        runs.append((name, times, glucose, results))
    with open(os.path.join(RESULTS, "glucose.json"), "w") as f:
        json.dump(report, f, indent=2)
    plot(runs)
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()