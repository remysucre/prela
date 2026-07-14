#!/usr/bin/env python3
"""Compare TPC-H query timings across the three query variants:

  base                — the baseline algebraic ports (QS=idiomatic, common.rs)
  optimized           — hand-encoded optimizer plans (optimized.rs)
  idiomatic_optimized — idiomatic-style rewrites of the same optimizations
                        (idiomatic_optimized.rs)

Builds the release binary, runs each variant once with ROUNDS=<n> timed
rounds over the loaded data (so the cache is loaded once per variant), and
reports the per-query median time plus the optimized / idiomatic_optimized
ratio.

Usage: python3 bench/compare_idiomatic_optimized.py [--runs N]
Run from the rust/ directory (or anywhere; paths are script-relative).
"""

import argparse
import os
import re
import statistics
import subprocess
import sys
from pathlib import Path

RUST_DIR = Path(__file__).resolve().parent.parent
LINE_RE = re.compile(r"^(\S+)\s+(ok|DIFF)\s+([\d.]+)s\s*$")

VARIANTS = ["idiomatic", "optimized", "idiomatic_optimized"]


def build():
    subprocess.run(
        ["cargo", "build", "--release", "--bin", "prela"],
        cwd=RUST_DIR, check=True,
    )


def run_variant(variant: str, runs: int) -> dict[str, float]:
    """Run one variant for `runs` rounds; return {query: median_seconds}."""
    env = {"QS": variant, "ROUNDS": str(runs)}
    proc = subprocess.run(
        [str(RUST_DIR / "target/release/prela"), "tpch"],
        cwd=RUST_DIR, capture_output=True, text=True,
        env={**os.environ, **env},
    )
    if proc.returncode != 0:
        sys.stderr.write(proc.stderr)
        raise SystemExit(f"{variant}: prela exited with {proc.returncode}")

    times: dict[str, list[float]] = {}
    for line in proc.stdout.splitlines():
        m = LINE_RE.match(line)
        if not m:
            continue
        name, status, dt = m.group(1), m.group(2), float(m.group(3))
        if status == "DIFF":
            print(f"WARNING: {variant} Q{name} DIFFs from its oracle", file=sys.stderr)
        times.setdefault(name, []).append(dt)

    medians = {}
    for name, ts in times.items():
        if len(ts) != runs:
            print(f"WARNING: {variant} Q{name}: expected {runs} samples, got {len(ts)}",
                  file=sys.stderr)
        medians[name] = statistics.median(ts)
    return medians


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--runs", type=int, default=10, help="timed rounds per variant (default 10)")
    args = ap.parse_args()

    build()

    results = {}
    for v in VARIANTS:
        print(f"running {v} ({args.runs} rounds)...", file=sys.stderr)
        results[v] = run_variant(v, args.runs)

    base, opt, idio = results["idiomatic"], results["optimized"], results["idiomatic_optimized"]
    queries = sorted(base, key=lambda q: int(q))

    hdr = f"{'query':>5}  {'base (s)':>10}  {'optimized (s)':>14}  {'idio-opt (s)':>13}  {'opt/idio-opt':>12}"
    print(hdr)
    print("-" * len(hdr))
    for q in queries:
        b, o, i = base.get(q), opt.get(q), idio.get(q)
        if b is None or o is None or i is None:
            print(f"{q:>5}  (missing sample)")
            continue
        print(f"{q:>5}  {b:>10.4f}  {o:>14.4f}  {i:>13.4f}  {o / i:>12.3f}")


if __name__ == "__main__":
    main()
