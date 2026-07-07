#!/usr/bin/env python3
# TPC-H SF=1 single-threaded — scatter overlay of "idiomatic", "optimized"
# and "idiomatic_optimized" Rust prela vs DuckDB-ST. Idiomatic is the honest
# baseline (no per-query rewriting; just the algebra ports); optimized
# hand-encodes the plans a stats-driven optimizer would pick;
# idiomatic_optimized is a from-scratch idiomatic rewrite of the queries
# optimized touches, kept separate so it can be compared against both.
# Diagonal y=x marks parity.
#
# Reads warm run-2 timings from data/{idiomatic,optimized,idiomatic_optimized}_<suffix>.txt
# (generated via `REPS=N STAT=min|median QS=all ./target/release/prela tpch`, which
# runs all three variants interleaved in a single process to avoid cross-process
# timing noise — see run_tpch_all in src/main.rs) and DuckDB `.timer on` output
# from data/duckdb_st.txt.
#
# Usage: plot_tpch.py [suffix ...]   (default: min10 median10)
# Writes tpch_scatter_<suffix>.png next to this script for each suffix given.

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


def plot_one(suffix):
    ido   = parse_rust(DATA / f"idiomatic_{suffix}.txt")
    opt   = parse_rust(DATA / f"optimized_{suffix}.txt")
    opti  = parse_rust(DATA / f"idiomatic_optimized_{suffix}.txt")
    duck  = parse_duck(DATA / "duckdb_st.txt")

    qs = list(range(1, 23))
    xs  = [duck[q]  for q in qs]
    yr  = [ido[q]   for q in qs]
    yo  = [opt[q]   for q in qs]
    yoi = [opti[q]  for q in qs]

    lo = min(min(xs), min(yr), min(yo), min(yoi)) * 0.5
    hi = max(max(xs), max(yr), max(yo), max(yoi)) * 2.0

    fig, ax = plt.subplots(figsize=(8, 8))
    ax.plot([lo, hi], [lo, hi], color="#888", linestyle="--", linewidth=1,
            label="y = x (parity)")
    ax.scatter(xs, yr, s=40, color="#2BA84A", edgecolor="black",
               linewidth=0.4, alpha=0.85, label="prela (idiomatic)",
               zorder=4)
    ax.scatter(xs, yo, s=40, color="#E07B1C", edgecolor="black",
               linewidth=0.4, alpha=0.85, label="prela (optimized)",
               zorder=5, marker="D")
    ax.scatter(xs, yoi, s=40, color="#1C6FE0", edgecolor="black",
               linewidth=0.4, alpha=0.85, label="prela (idiomatic optimized)",
               zorder=6, marker="^")

    ax.set_xscale("log"); ax.set_yscale("log")
    ax.set_xlim(lo, hi);  ax.set_ylim(lo, hi)
    ax.set_aspect("equal")
    ax.grid(True, which="both", alpha=0.3, linestyle=":")
    ax.set_xlabel("DuckDB-ST time (s, log)")
    ax.set_ylabel("prela time (s, log)")

    tx  = sum(xs)
    tr  = sum(yr);  to  = sum(yo);  toi = sum(yoi)
    ax.set_title(
        f"TPC-H SF=1 — prela vs DuckDB single-threaded ({suffix})\n"
        f"DuckDB {tx:>5.2f}s   idiomatic {tr:>5.2f}s ({tr/tx:.2f}×)   "
        f"optimized {to:>5.2f}s ({to/tx:.2f}×)\n"
        f"idiomatic_optimized {toi:>5.2f}s ({toi/tx:.2f}×)"
    )
    ax.legend(loc="upper left", fontsize=9)

    plt.tight_layout()
    out_path = Path(__file__).resolve().parent / f"tpch_scatter_{suffix}.png"
    plt.savefig(out_path, dpi=130)
    plt.close(fig)
    print(f"saved {out_path}")


def main():
    suffixes = sys.argv[1:] or ["min10", "median10"]
    for suffix in suffixes:
        plot_one(suffix)


if __name__ == "__main__":
    sys.exit(main())
