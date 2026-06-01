#!/usr/bin/env python3
# JOB (Join Order Benchmark, IMDB) — scatter of Julia prela query time (y) vs
# DuckDB-ST (x). The diagonal y=x marks parity; points below it are wins.
#
# Reads:
#   data/job_qnames.txt — 113 query names in canonical order
#   data/job_duck.txt   — DuckDB `.timer on` log (cold+warm per query)
#   data/julia_job.txt  — Julia prela bench (name<TAB>seconds)
# Writes job_scatter.png next to this script.

import re
import sys
from pathlib import Path
import matplotlib.pyplot as plt

DATA = Path(__file__).resolve().parent / "data"


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


def parse_julia(path):
    """Parse `name<TAB>seconds` lines from julia/bench.jl output."""
    out = {}
    with open(path) as f:
        for line in f:
            parts = line.strip().split()
            if len(parts) == 2:
                try:
                    out[parts[0]] = float(parts[1])
                except ValueError:
                    pass
    return out


def main():
    with open(DATA / "job_qnames.txt") as f:
        qnames = [l.strip() for l in f if l.strip()]
    duck  = parse_duck(DATA / "job_duck.txt", qnames)
    julia = parse_julia(DATA / "julia_job.txt")
    common = [q for q in qnames if q in duck and q in julia]

    xs = [duck[q]  for q in common]
    yj = [julia[q] for q in common]

    lo = max(min(min(xs), min(yj)) * 0.5, 1e-3)
    hi = max(max(xs), max(yj)) * 2.0

    fig, ax = plt.subplots(figsize=(8, 8))
    ax.plot([lo, hi], [lo, hi], color="#888", linestyle="--", linewidth=1,
            label="y = x (parity)")
    ax.scatter(xs, yj, s=40, color="#9461D9", edgecolor="black",
               linewidth=0.4, alpha=0.75, label="Julia prela", zorder=2)

    ax.set_xscale("log"); ax.set_yscale("log")
    ax.set_xlim(lo, hi); ax.set_ylim(lo, hi)
    ax.set_aspect("equal")
    ax.grid(True, which="both", alpha=0.3, linestyle=":")
    ax.set_xlabel("DuckDB-ST time (s, log)")
    ax.set_ylabel("Julia prela time (s, log)")

    tx = sum(xs); tj = sum(yj)
    wj = sum(1 for x, y in zip(xs, yj) if y < x)
    ax.set_title(
        f"Join Order Benchmark — Julia prela vs DuckDB single-threaded\n"
        f"Julia {tj:>5.2f}s   DuckDB {tx:>5.2f}s   "
        f"({tx/tj:.1f}× speedup, {wj}/{len(common)} wins)"
    )
    ax.legend(loc="upper left", fontsize=10)

    plt.tight_layout()
    out_path = Path(__file__).resolve().parent / "job_scatter.png"
    plt.savefig(out_path, dpi=130)
    print(f"saved {out_path}")


if __name__ == "__main__":
    sys.exit(main())
