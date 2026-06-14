#!/usr/bin/env python3
# TPC-H SF=10 — prela (parallel, chili) vs DuckDB (multi-threaded). Scatter:
# prela-MT time (y) vs DuckDB-MT (x), log-log, parity diagonal.
import re, sys
from pathlib import Path
import matplotlib.pyplot as plt
DATA = Path(__file__).resolve().parent / "data"

def parse_rust(path):
    # run-2 section; lines "<q> ok|DIFF <t>s"  (SF=10 mismatches SF=1 oracles,
    # so timing comes from DIFF lines too — correctness was checked at SF=1).
    parts = open(path).read().split("--- run 2 ---")
    out = {}
    if len(parts) > 1:
        for line in parts[1].splitlines():
            m = re.match(r"(\d+)\s+(?:ok|DIFF)\s+([\d.]+)s", line.strip())
            if m:
                out[m.group(1)] = float(m.group(2))
    return out

def parse_duck(path):
    # lines "<q> <warm_t>", one per query.
    out = {}
    for line in open(path):
        p = line.split()
        if len(p) == 2:
            out[p[0]] = float(p[1])
    return out

def main():
    rust = parse_rust(DATA / "tpch10_rust_mt.txt")
    duck = parse_duck(DATA / "tpch10_duck_mt.txt")
    common = sorted(set(rust) & set(duck), key=int)
    xs = [duck[q] for q in common]; ys = [rust[q] for q in common]
    lo = max(min(min(xs), min(ys)) * 0.5, 1e-3); hi = max(max(xs), max(ys)) * 2.0
    fig, ax = plt.subplots(figsize=(8, 8))
    ax.plot([lo, hi], [lo, hi], color="#888", ls="--", lw=1, label="y = x (parity)")
    ax.scatter(xs, ys, s=55, color="#2BA84A", edgecolor="black", lw=0.4, alpha=0.85,
               label="prela (parallel)", zorder=3)
    for q, x, y in zip(common, xs, ys):
        ax.annotate(q, (x, y), fontsize=7, ha="center", va="center")
    ax.set_xscale("log"); ax.set_yscale("log")
    ax.set_xlim(lo, hi); ax.set_ylim(lo, hi); ax.set_aspect("equal")
    ax.grid(True, which="both", alpha=0.3, ls=":")
    ax.set_xlabel("DuckDB multi-threaded time (s, log)")
    ax.set_ylabel("prela parallel time (s, log)")
    tx, ty = sum(xs), sum(ys)
    wins = sum(1 for x, y in zip(xs, ys) if y < x)
    ax.set_title(f"TPC-H SF=10 — prela (chili) vs DuckDB, both multi-threaded (10 cores)\n"
                 f"DuckDB {tx:.2f}s   prela {ty:.2f}s   ({tx/ty:.2f}x, {wins}/{len(common)} wins)")
    ax.legend(loc="upper left", fontsize=10)
    plt.tight_layout()
    out = Path(__file__).resolve().parent / "tpch10_scatter_mt.png"
    plt.savefig(out, dpi=130)
    print(f"saved {out}")
    print(f"DuckDB-MT {tx:.2f}s  prela-MT {ty:.2f}s  speedup {tx/ty:.2f}x  wins {wins}/{len(common)}")

if __name__ == "__main__":
    sys.exit(main())
