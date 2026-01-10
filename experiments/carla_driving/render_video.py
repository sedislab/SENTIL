"""Render the CARLA drive to an mp4 with the sentil_ros verdicts overlaid."""

import argparse
import bisect
import json
import os

import imageio.v2 as imageio
import numpy as np
from PIL import Image, ImageDraw, ImageFont

SPECS = [
    ("speed_limit", "speed < 12 m/s"),
    ("following_distance", "front gap > 6 m"),
    ("pedestrian_clearance", "ped clearance > 5 m"),
]
PROB_SPEC = ("collision_risk", "P[ped > 4 m] >= 0.95", 0.95)
GREEN = (60, 200, 90)
RED = (230, 70, 60)
AMBER = (235, 170, 40)
WHITE = (235, 235, 235)


def font(size):
    for path in ("/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
                 "/usr/share/fonts/dejavu/DejaVuSans-Bold.ttf"):
        if os.path.exists(path):
            return ImageFont.truetype(path, size)
    return ImageFont.load_default()


def timeline(records, fid, kind, key):
    """Sorted (t, value, concrete) triples for one formula and message kind."""
    pts = [(r["t"], r[key], r.get("concrete", True)) for r in records
           if r["id"] == fid and r["kind"] == kind]
    pts.sort()
    return [p[0] for p in pts], pts


def latest(times, pts, t):
    if not times:
        return None
    i = bisect.bisect_right(times, t) - 1
    return pts[i] if i >= 0 else None


def draw_hud(img, t, speed, verdicts, prob):
    s = img.width / 800.0
    draw = ImageDraw.Draw(img, "RGBA")
    f_title, f_row, f_small = font(int(22 * s)), font(int(19 * s)), font(int(15 * s))
    row_h = int(30 * s)
    m = int(10 * s)
    w, h = int(360 * s), int(70 * s) + (len(SPECS) + 1) * row_h
    draw.rectangle([m, m, m + w, m + h], fill=(15, 18, 24, 205))
    draw.text((int(24 * s), int(22 * s)), "SENTIL  online monitor", font=f_title, fill=WHITE)
    draw.text((int(24 * s), int(48 * s)),
              "t = {:5.1f} s    speed = {:4.1f} m/s".format(t, speed),
              font=f_small, fill=(170, 180, 195))
    y = int(78 * s)

    def row(color, label, detail):
        draw.ellipse([int(24 * s), y + int(6 * s), int(38 * s), y + int(20 * s)], fill=color)
        draw.text((int(48 * s), y), label, font=f_row, fill=WHITE)
        draw.text((int(48 * s), y + int(18 * s)), detail, font=f_small, fill=color)

    for (fid, label) in SPECS:
        v = verdicts.get(fid)
        if v is None:
            color, mark, val = (120, 120, 120), "--", ""
        elif not v[2]:
            color, mark, val = AMBER, "UNRESOLVED", "window filling"
        else:
            sat = v[1] >= 0.0
            color = GREEN if sat else RED
            mark = "OK" if sat else "VIOLATED"
            val = "rho={:+.1f}".format(v[1]) if abs(v[1]) < 1e6 else ""
        row(color, label, "{}  {}".format(mark, val))
        y += row_h
    thr = PROB_SPEC[2]
    if prob is None:
        color, txt = (120, 120, 120), "P = --"
    elif not prob[1]:
        color, txt = AMBER, "P = {:.2f}  window filling".format(prob[0])
    else:
        color = GREEN if prob[0] >= thr else (AMBER if prob[0] >= thr - 0.1 else RED)
        txt = "P = {:.2f}  (>= {:.2f})".format(prob[0], thr)
    row(color, PROB_SPEC[1], txt)
    return img


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--capture", default="capture")
    ap.add_argument("--verdicts", default="results/verdicts_timeline.json")
    ap.add_argument("--out", default="results/carla_drive.mp4")
    ap.add_argument("--fps", type=int, default=20)
    ap.add_argument("--crf", type=int, default=26, help="H.264 quality, lower is sharper and larger")
    args = ap.parse_args()

    cap = json.load(open(os.path.join(args.capture, "capture.json")))
    records = json.load(open(args.verdicts))
    dt = cap["dt"]
    frames_dir = os.path.join(args.capture, "frames")

    rob_tl = {fid: timeline(records, fid, "robustness", "value") for fid, _ in SPECS}
    prob_times, prob_pts = timeline(records, PROB_SPEC[0], "probability", "estimate")

    all_t = [r["t"] for r in records]
    offset = min(all_t) if all_t else 0.0

    os.makedirs(os.path.dirname(args.out), exist_ok=True)
    writer = imageio.get_writer(args.out, fps=args.fps, codec="libx264", macro_block_size=16,
                               pixelformat="yuv420p",
                               ffmpeg_params=["-crf", str(args.crf), "-preset", "slow"])
    n = cap["frames"]
    for i in range(n):
        path = os.path.join(frames_dir, "%06d.jpg" % i)
        if not os.path.exists(path):
            continue
        t = i * dt
        img = Image.open(path).convert("RGB")
        verdicts = {}
        for fid, _ in SPECS:
            v = latest(rob_tl[fid][0], rob_tl[fid][1], t + offset)
            verdicts[fid] = v
        p = latest(prob_times, prob_pts, t + offset)
        prob = (p[1], p[2]) if p else None
        draw_hud(img, t, cap["records"][i]["speed"], verdicts, prob)
        writer.append_data(np.asarray(img))
    writer.close()
    print("wrote", args.out)


if __name__ == "__main__":
    main()