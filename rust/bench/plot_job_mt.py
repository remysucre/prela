#!/usr/bin/env python3
# JOB — prela (parallel, chili) vs DuckDB (multi-threaded). Scatter: prela-MT
# time (y) vs DuckDB-MT (x), log-log, parity diagonal. Points below y=x win.
import re, sys
from pathlib import Path
import matplotlib.pyplot as plt
DATA = Path(__file__).resolve().parent / "data"

def parse_rust(path):
    parts = open(path).read().split("--- run 2 ---")
    out = {}
    if len(parts) > 1:
        for line in parts[1].splitlines():
            m = re.match(r"(\S+)\s+ok\s+([\d.]+)s", line.strip())
            if m: out[m.group(1)] = float(m.group(2))
    return out

def parse_duck(path, qnames):
    ts = []
    for line in open(path):
        m = re.search(r"Run Time \(s\): real ([\d.]+)", line)
        if m:
            t = float(m.group(1))
            ts.append(t if t > 1e-6 else 1e-6)
        elif "real NA" in line:
            ts.append(None)
    out = {}
    for i, q in enumerate(qnames):
        if 2*i+1 < len(ts) and ts[2*i+1] is not None:
            out[q] = ts[2*i+1]
    return out

def main():
    qnames = [l.strip() for l in open(DATA/"job_qnames.txt") if l.strip()]
    rust = parse_rust(DATA/"job_rust_mt.txt")
    duck = parse_duck(DATA/"job_duck_mt.txt", qnames)
    common = [q for q in qnames if q in rust and q in duck]
    xs = [duck[q] for q in common]; ys = [rust[q] for q in common]
    lo = max(min(min(xs), min(ys))*0.5, 1e-4); hi = max(max(xs), max(ys))*2.0
    fig, ax = plt.subplots(figsize=(8,8))
    ax.plot([lo,hi],[lo,hi], color="#888", ls="--", lw=1, label="y = x (parity)")
    ax.scatter(xs, ys, s=40, color="#2BA84A", edgecolor="black", lw=0.4, alpha=0.85, label="prela (parallel)", zorder=3)
    ax.set_xscale("log"); ax.set_yscale("log")
    ax.set_xlim(lo,hi); ax.set_ylim(lo,hi); ax.set_aspect("equal")
    ax.grid(True, which="both", alpha=0.3, ls=":")
    ax.set_xlabel("DuckDB multi-threaded time (s, log)")
    ax.set_ylabel("prela parallel time (s, log)")
    tx, ty = sum(xs), sum(ys)
    wins = sum(1 for x,y in zip(xs,ys) if y < x)
    ax.set_title(f"Join Order Benchmark — prela (chili) vs DuckDB, both multi-threaded ({10} cores)\n"
                 f"DuckDB {tx:.2f}s   prela {ty:.2f}s   ({tx/ty:.2f}x, {wins}/{len(common)} wins)")
    ax.legend(loc="upper left", fontsize=10)
    plt.tight_layout()
    out = Path(__file__).resolve().parent/"job_scatter_mt.png"
    plt.savefig(out, dpi=130)
    print(f"saved {out}")
    print(f"DuckDB-MT total {tx:.2f}s  prela-MT total {ty:.2f}s  speedup {tx/ty:.2f}x  wins {wins}/{len(common)}")

if __name__ == "__main__": sys.exit(main())
