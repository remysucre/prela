#!/usr/bin/env python3
# TPC-H SF=1 single-threaded — scatter of Julia prela query time (y) vs
# DuckDB-ST (x), two series: the "idiomatic" algebra (a direct port) and the
# "optimized" algebra (hand-tuned plans). Diagonal y=x marks parity; points
# below it are wins.
#
# Reads DuckDB `.timer on` output from data/duckdb_st.txt and Julia bench
# (name<TAB>seconds) from data/julia_tpch_{idiomatic,optimized}.txt.
# Writes tpch_scatter.png next to this script.

import re
import sys
from pathlib import Path
import matplotlib.pyplot as plt

DATA = Path(__file__).resolve().parent / "data"


def parse_duck(path):
    out = {}
    with open(path) as f:
        timer_lines = [l.strip() for l in f if l.strip().startswith("Run Time")]
    for i, l in enumerate(timer_lines[1::2], 1):
        m = re.search(r"real ([\d.]+)", l)
        if m:
            out[i] = float(m.group(1))
    return out


def parse_julia(path):
    """Parse `name<TAB>seconds` lines from julia/bench.jl output. Query
    names look like 'q1', 'q2', etc.; we map to ints for joint plotting."""
    out = {}
    with open(path) as f:
        for line in f:
            parts = line.strip().split()
            if len(parts) != 2:
                continue
            name = parts[0].lstrip("qQ")
            try:
                out[int(name)] = float(parts[1])
            except ValueError:
                pass
    return out


def main():
    duck  = parse_duck(DATA / "duckdb_st.txt")
    j_ido = parse_julia(DATA / "julia_tpch_idiomatic.txt")
    j_opt = parse_julia(DATA / "julia_tpch_optimized.txt")

    qs = list(range(1, 23))
    xs  = [duck[q]  for q in qs]
    yji = [j_ido[q] for q in qs]
    yjo = [j_opt[q] for q in qs]

    lo = min(min(xs), min(yji), min(yjo)) * 0.5
    hi = max(max(xs), max(yji), max(yjo)) * 2.0

    fig, ax = plt.subplots(figsize=(8, 8))
    ax.plot([lo, hi], [lo, hi], color="#888", linestyle="--", linewidth=1,
            label="y = x (parity)")
    ax.scatter(xs, yji, s=40, color="#9461D9", edgecolor="black",
               linewidth=0.4, alpha=0.7, label="Julia prela (idiomatic)",
               zorder=2)
    ax.scatter(xs, yjo, s=40, color="#5B2DB0", edgecolor="black",
               linewidth=0.4, alpha=0.85, label="Julia prela (optimized)",
               zorder=3, marker="D")

    ax.set_xscale("log"); ax.set_yscale("log")
    ax.set_xlim(lo, hi);  ax.set_ylim(lo, hi)
    ax.set_aspect("equal")
    ax.grid(True, which="both", alpha=0.3, linestyle=":")
    ax.set_xlabel("DuckDB-ST time (s, log)")
    ax.set_ylabel("Julia prela time (s, log)")

    tx  = sum(xs); tji = sum(yji); tjo = sum(yjo)
    wji = sum(1 for x, y in zip(xs, yji) if y < x)
    wjo = sum(1 for x, y in zip(xs, yjo) if y < x)
    ax.set_title(
        f"TPC-H SF=1 — Julia prela vs DuckDB single-threaded\n"
        f"idiomatic {tji:>5.2f}s ({tx/tji:.1f}× speedup, {wji}/22 wins)   "
        f"optimized {tjo:>5.2f}s ({tx/tjo:.1f}× speedup, {wjo}/22 wins)"
    )
    ax.legend(loc="upper left", fontsize=10)

    plt.tight_layout()
    out_path = Path(__file__).resolve().parent / "tpch_scatter.png"
    plt.savefig(out_path, dpi=130)
    print(f"saved {out_path}")


if __name__ == "__main__":
    sys.exit(main())
