#!/usr/bin/env python3
# TPC-H SF=1 single-threaded — single scatter overlay of "idiomatic" Rust
# and Julia prela vs DuckDB-ST. The idiomatic Rust series is the honest
# baseline (no per-query algorithmic rewriting; just the algebra-port of
# the Julia originals); Julia uses the same algebra in its native form.
# Diagonal y=x marks parity.
#
# Reads warm run-2 timings from data/{idiomatic,optimized,ddbcheat}.txt
# and DuckDB `.timer on` output from data/duckdb_st.txt.
# Writes tpch_scatter.png next to this script.

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
    ido   = parse_rust(DATA / "idiomatic.txt")
    duck  = parse_duck(DATA / "duckdb_st.txt")
    julia = parse_julia(DATA / "julia_tpch.txt")

    qs = list(range(1, 23))
    xs = [duck[q]  for q in qs]
    yr = [ido[q]   for q in qs]
    yj = [julia[q] for q in qs]

    lo = min(min(xs), min(yr), min(yj)) * 0.5
    hi = max(max(xs), max(yr), max(yj)) * 2.0

    fig, ax = plt.subplots(figsize=(8, 8))
    ax.plot([lo, hi], [lo, hi], color="#888", linestyle="--", linewidth=1,
            label="y = x (parity)")
    ax.scatter(xs, yj, s=60, color="#9461D9", edgecolor="black",
               linewidth=0.5, alpha=0.85, label="Julia prela", zorder=2)
    ax.scatter(xs, yr, s=60, color="#888888", edgecolor="black",
               linewidth=0.5, alpha=0.95, label="Rust prela (idiomatic)",
               zorder=3)
    for q, x, y in zip(qs, xs, yr):
        ax.annotate(f"Q{q}", (x, y), xytext=(4, 4),
                    textcoords="offset points", fontsize=8, color="#333")

    ax.set_xscale("log"); ax.set_yscale("log")
    ax.set_xlim(lo, hi);  ax.set_ylim(lo, hi)
    ax.set_aspect("equal")
    ax.grid(True, which="both", alpha=0.3, linestyle=":")
    ax.set_xlabel("DuckDB-ST time (s, log)")
    ax.set_ylabel("prela time (s, log)")

    tx = sum(xs); tr = sum(yr); tj = sum(yj)
    ax.set_title(
        f"TPC-H SF=1 — prela vs DuckDB single-threaded\n"
        f"Rust  {tr:>5.2f}s  ({tr/tx:.2f}× of DuckDB)   "
        f"Julia {tj:>5.2f}s  ({tj/tx:.2f}× of DuckDB)"
    )
    ax.legend(loc="upper left", fontsize=10)

    plt.tight_layout()
    out_path = Path(__file__).resolve().parent / "tpch_scatter.png"
    plt.savefig(out_path, dpi=130)
    print(f"saved {out_path}")


if __name__ == "__main__":
    sys.exit(main())
