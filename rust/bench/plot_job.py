#!/usr/bin/env python3
# JOB (Join Order Benchmark, IMDB) — scatter plot of prela query time (y) vs
# DuckDB-ST (x). The diagonal y=x marks parity; points below it are wins.
#
# Reads:
#   data/job_qnames.txt — 113 query names in canonical order
#   data/job_rust.txt   — Rust prela bench (warm, run 2)
#   data/job_duck.txt   — DuckDB `.timer on` log (cold+warm per query)
# Writes job_scatter.png next to this script.

import re
import sys
from pathlib import Path
import matplotlib.pyplot as plt

DATA = Path(__file__).resolve().parent / "data"


def parse_rust(path):
    with open(path) as f:
        parts = f.read().split("--- run 2 ---")
    out = {}
    if len(parts) > 1:
        for line in parts[1].splitlines():
            m = re.match(r"(\S+)\s+ok\s+([\d.]+)s", line.strip())
            if m:
                out[m.group(1)] = float(m.group(2))
    return out


def parse_duck(path, qnames):
    timings = []
    with open(path) as f:
        for line in f:
            m = re.search(r"Run Time \(s\): real ([\d.]+)", line)
            if m:
                t = float(m.group(1))
                if t > 1e-6:
                    timings.append(t)
    out = {}
    for i, q in enumerate(qnames):
        if 2 * i + 1 < len(timings):
            out[q] = timings[2 * i + 1]  # warm of the cold/warm pair
    return out




def main():
    with open(DATA / "job_qnames.txt") as f:
        qnames = [l.strip() for l in f if l.strip()]
    rust  = parse_rust(DATA / "job_rust.txt")
    duck  = parse_duck(DATA / "job_duck.txt", qnames)
    common = [q for q in qnames if q in rust and q in duck]

    xs = [duck[q]  for q in common]
    yr = [rust[q]  for q in common]

    lo = max(min(min(xs), min(yr)) * 0.5, 1e-3)
    hi = max(max(xs), max(yr)) * 2.0

    fig, ax = plt.subplots(figsize=(8, 8))
    ax.plot([lo, hi], [lo, hi], color="#888", linestyle="--", linewidth=1,
            label="y = x (parity)")
    ax.scatter(xs, yr, s=40, color="#2BA84A", edgecolor="black",
               linewidth=0.4, alpha=0.85, label="prela", zorder=3)

    ax.set_xscale("log"); ax.set_yscale("log")
    ax.set_xlim(lo, hi); ax.set_ylim(lo, hi)
    ax.set_aspect("equal")
    ax.grid(True, which="both", alpha=0.3, linestyle=":")
    ax.set_xlabel("DuckDB-ST time (s, log)")
    ax.set_ylabel("prela time (s, log)")

    tx = sum(xs); tr = sum(yr)
    wr = sum(1 for x, y in zip(xs, yr) if y < x)
    ax.set_title(
        f"Join Order Benchmark — prela vs DuckDB single-threaded\n"
        f"DuckDB {tx:>5.2f}s   prela {tr:>5.2f}s  "
        f"({tx/tr:.1f}× speedup, {wr}/{len(common)} wins)"
    )
    ax.legend(loc="upper left", fontsize=10)

    plt.tight_layout()
    out_path = Path(__file__).resolve().parent / "job_scatter.png"
    plt.savefig(out_path, dpi=130)
    print(f"saved {out_path}")


if __name__ == "__main__":
    sys.exit(main())
