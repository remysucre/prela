#!/usr/bin/env python3
# TPC-H SF=10 single-threaded — three scatter plots (idiomatic / optimized
# / ddbcheat), each charting our query time (y) against DuckDB-ST (x). The
# diagonal y=x marks parity; points below it are Rust wins.
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


def draw_panel(ax, title, our_times, duck_times, color):
    """One scatter panel: per-query (duck, ours) with diagonal reference."""
    qs = list(range(1, 23))
    xs = [duck_times[q] for q in qs]
    ys = [our_times[q]  for q in qs]

    # log-log y=x diagonal — covers the full plotted range
    lo = min(min(xs), min(ys)) * 0.5
    hi = max(max(xs), max(ys)) * 2.0
    ax.plot([lo, hi], [lo, hi], color="#888", linestyle="--", linewidth=1,
            label="y = x (parity)")

    ax.scatter(xs, ys, s=60, color=color, edgecolor="black",
               linewidth=0.5, zorder=3)
    for q, x, y in zip(qs, xs, ys):
        # nudge label slightly NE of each point
        ax.annotate(f"Q{q}", (x, y), xytext=(4, 4), textcoords="offset points",
                    fontsize=8, color="#333")

    ax.set_xscale("log"); ax.set_yscale("log")
    ax.set_xlim(lo, hi);  ax.set_ylim(lo, hi)
    ax.set_aspect("equal")
    ax.grid(True, which="both", alpha=0.3, linestyle=":")
    ax.set_xlabel("DuckDB-ST time (s, log)")
    ax.set_ylabel(f"{title} time (s, log)")

    tot_x = sum(xs); tot_y = sum(ys)
    wins = sum(1 for x, y in zip(xs, ys) if y < x)
    ax.set_title(
        f"{title}   total {tot_y:.2f}s   ({tot_y/tot_x:.2f}× of DuckDB)"
        f"   {wins}/22 wins"
    )
    ax.legend(loc="upper left", fontsize=9)


def main():
    ido  = parse_rust(DATA / "idiomatic.txt")
    opt  = parse_rust(DATA / "optimized.txt")
    ch   = parse_rust(DATA / "ddbcheat.txt")
    duck = parse_duck(DATA / "duckdb_st.txt")

    fig, axes = plt.subplots(1, 3, figsize=(18, 6))
    draw_panel(axes[0], "idiomatic", ido, duck, "#888888")
    draw_panel(axes[1], "optimized", opt, duck, "#4C8BC7")
    draw_panel(axes[2], "ddbcheat",  ch,  duck, "#2BA84A")

    fig.suptitle("TPC-H SF=10 — Rust variants vs DuckDB single-threaded "
                 "(below diagonal = Rust wins)", fontsize=13)
    plt.tight_layout(rect=[0, 0, 1, 0.96])
    out_path = Path(__file__).resolve().parent / "tpch_scatter.png"
    plt.savefig(out_path, dpi=130)
    print(f"saved {out_path}")


if __name__ == "__main__":
    sys.exit(main())
