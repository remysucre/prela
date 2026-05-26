#!/usr/bin/env python3
# TPC-H SF=10 single-threaded comparison plot.
#
# Reads warm run-2 timings from data/{idiomatic,optimized,ddbcheat}.txt
# (Rust bench captures) and data/duckdb_st.txt (DuckDB `.timer on`
# output). Writes tpch_sf10.png next to this script.
#
# Run from this directory:    python3 plot_tpch.py

import re
import sys
from pathlib import Path
import matplotlib.pyplot as plt

DATA = Path(__file__).resolve().parent / "data"


def parse_rust(path):
    """Pull (qnum, time_s) for run-2 (warm) from a Rust bench dump."""
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
    """Pull (qnum, real_seconds) from a DuckDB `.timer on` log.

    Each query in the log emits one Run Time line for the SELECT-name
    statement and one for the query itself; we take every other line
    starting with the second (the actual query timing).
    """
    out = {}
    with open(path) as f:
        timer_lines = [
            l.strip() for l in f if l.strip().startswith("Run Time")
        ]
    for i, l in enumerate(timer_lines[1::2], 1):
        m = re.search(r"real ([\d.]+)", l)
        if m:
            out[i] = float(m.group(1))
    return out


def main():
    ido = parse_rust(DATA / "idiomatic.txt")
    opt = parse_rust(DATA / "optimized.txt")
    ch  = parse_rust(DATA / "ddbcheat.txt")
    duck = parse_duck(DATA / "duckdb_st.txt")

    # Sort queries by DuckDB time ascending — fastest queries on the left.
    qs = sorted(range(1, 23), key=lambda q: duck.get(q, 0))
    x = list(range(len(qs)))
    labels = [f"Q{q}" for q in qs]

    fig, ax = plt.subplots(figsize=(14, 6))
    ax.plot(x, [ido[q]  for q in qs], marker="o", linewidth=1.5,
            label="idiomatic", color="#888888")
    ax.plot(x, [opt[q]  for q in qs], marker="s", linewidth=1.5,
            label="optimized", color="#4C8BC7")
    ax.plot(x, [duck[q] for q in qs], marker="d", linewidth=2.5,
            label="DuckDB-ST", color="#E66B23")
    ax.plot(x, [ch[q]   for q in qs], marker="^", linewidth=2.5,
            label="ddbcheat", color="#2BA84A")

    ax.set_yscale("log")
    ax.set_xticks(x)
    ax.set_xticklabels(labels, rotation=0, fontsize=9)
    ax.set_ylabel("Query time (seconds, log)")
    ax.set_xlabel("Queries — sorted left-to-right by DuckDB-ST time")
    ax.set_title("TPC-H SF=10 — Rust variants vs DuckDB single-threaded")
    ax.grid(True, which="both", alpha=0.3, linestyle=":")
    ax.legend(loc="upper left", fontsize=10)

    tots = {
        "idiomatic": sum(ido[q]  for q in qs),
        "optimized": sum(opt[q]  for q in qs),
        "DuckDB-ST": sum(duck[q] for q in qs),
        "ddbcheat":  sum(ch[q]   for q in qs),
    }
    ax.text(
        0.99, 0.02,
        "Totals (sum of warm run-2):\n"
        f"  idiomatic:  {tots['idiomatic']:.2f}s\n"
        f"  optimized:  {tots['optimized']:.2f}s\n"
        f"  DuckDB-ST:  {tots['DuckDB-ST']:.2f}s\n"
        f"  ddbcheat:   {tots['ddbcheat']:.2f}s  "
        f"({tots['ddbcheat']/tots['DuckDB-ST']:.0%} of DuckDB)",
        transform=ax.transAxes, ha="right", va="bottom",
        family="monospace", fontsize=9,
        bbox=dict(facecolor="white", edgecolor="#ccc",
                  boxstyle="round,pad=0.4"),
    )

    plt.tight_layout()
    out_path = Path(__file__).resolve().parent / "tpch_sf10.png"
    plt.savefig(out_path, dpi=130)
    print(f"saved {out_path}")
    print("query order (by DuckDB time):", labels)


if __name__ == "__main__":
    sys.exit(main())
