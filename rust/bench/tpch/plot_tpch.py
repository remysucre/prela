#!/usr/bin/env python3
# TPC-H SF=1 single-threaded — scatter overlay of "idiomatic" and
# "optimized" Rust prela vs DuckDB-ST. Idiomatic is the honest baseline
# (no per-query rewriting; just the algebra ports); optimized hand-encodes
# the plans a stats-driven optimizer would pick. Diagonal y=x marks parity.
#
# Reads warm run-2 timings from data/{idiomatic,optimized}.txt
# and DuckDB `.timer on` output from data/duckdb_st.txt.
# Writes tpch_scatter.pdf next to this script.

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
            m = re.match(r"(\d+)\s+(?:ok|DIFF)\s+([\d.]+)s", line.strip())
            if m:
                out[int(m.group(1))] = float(m.group(2))
    return out


def parse_duck(path):
    out = {}
    with open(path) as f:
        timer_lines = [l.strip() for l in f if l.strip().startswith("Run Time")]
    for i, l in enumerate(timer_lines[1::2], 1):
        m = re.search(r"real ([\d.]+)", l)
        if m:
            out[i] = float(m.group(1))
    return out




def main():
    ido   = parse_rust(DATA / "idiomatic.txt")
    opt   = parse_rust(DATA / "optimized.txt")
    duck  = parse_duck(DATA / "duckdb_st.txt")

    qs = list(range(1, 23))
    xs  = [duck[q]  for q in qs]
    yr  = [ido[q]   for q in qs]
    yo  = [opt[q]   for q in qs]

    # Fixed limits shared with plot_job.py so the paper's side-by-side
    # y scales align; widen both if a capture ever falls outside.
    lo, hi = 1e-3, 2.0

    fig, ax = plt.subplots(figsize=(4.5, 4.5))
    ax.plot([lo, hi], [lo, hi], color="#888", linestyle="--", linewidth=1,
            label="y = x (parity)")
    ax.scatter(xs, yr, s=40, color="#2BA84A", edgecolor="black",
               linewidth=0.4, alpha=0.85, label="prela (idiomatic)",
               zorder=4)
    ax.scatter(xs, yo, s=40, color="#E07B1C", edgecolor="black",
               linewidth=0.4, alpha=0.85, label="prela (optimized)",
               zorder=5, marker="D")

    ax.set_xscale("log"); ax.set_yscale("log")
    ax.set_xlim(lo, hi);  ax.set_ylim(lo, hi)
    ax.set_aspect("equal")
    ax.set_xlabel("DuckDB time (s, log)", fontsize=13)
    ax.tick_params(labelsize=11)

    ax.text(0.5, 0.035, "Prela faster", transform=ax.transAxes,
            ha="center", va="bottom", fontsize=12, color="#666")
    ax.text(0.035, 0.65, "DuckDB faster", transform=ax.transAxes,
            ha="left", va="center", fontsize=12, color="#666")

    ax.set_title("TPC-H", fontsize=14)
    ax.legend(loc="upper left", fontsize=11, framealpha=0)

    plt.tight_layout()
    # The paper references rust/bench/tpch_scatter.pdf, so the chart lands in
    # bench/ alongside job_scatter.pdf rather than inside this subdir.
    out_path = Path(__file__).resolve().parent.parent / "tpch_scatter.pdf"
    plt.savefig(out_path)
    print(f"saved {out_path}")


if __name__ == "__main__":
    sys.exit(main())
