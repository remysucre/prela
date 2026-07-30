#!/usr/bin/env python3
# JOB (Join Order Benchmark, IMDB) — scatter plot of prela query time (y) vs
# DuckDB-ST (x).
#
# Reads:
#   data/job_qnames.txt
#   data/job_prela.txt
#   data/job_duck.txt
#
# Both captures must come from the SAME machine to be comparable, so a run
# on other hardware goes to its own files rather than over these:
#   plot_job.py --prela data/job_prela.txt \
#               --duck data/job_duck.txt \
#               --out  job_scatter.pdf

import argparse
import re
import sys
from pathlib import Path
import matplotlib.pyplot as plt

DATA = Path(__file__).resolve().parent / 'data'


def parse_prela(path):
    with open(path) as f:
        parts = f.read().split('--- run 2 ---')
    out = {}
    if len(parts) > 1:
        for line in parts[1].splitlines():
            m = re.match(r'(\S+)\s+ok\s+([\d.]+)s', line.strip())
            if m:
                out[m.group(1)] = float(m.group(2))
    return out


def parse_duck(path, qnames):
    timings = []
    with open(path) as f:
        for line in f:
            m = re.search(r'Run Time \(s\): real ([\d.]+)', line)
            if m:
                t = float(m.group(1))
                if t > 1e-6:
                    timings.append(t)
    out = {}
    for i, q in enumerate(qnames):
        if 2 * i + 1 < len(timings):
            out[q] = timings[2 * i + 1]  # warm of the cold/warm pair
    return out


def main():
    here = Path(__file__).resolve().parent
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument(
        '--prela',
        default=DATA / 'job_prela.txt',
        type=Path,
        help='prela bench log (default data/job_prela.txt)',
    )
    ap.add_argument(
        '--duck',
        default=DATA / 'job_duck.txt',
        type=Path,
        help='DuckDB .timer log (default data/job_duck.txt)',
    )
    ap.add_argument(
        '--out',
        default=here.parent / 'job_scatter.pdf',
        type=Path,
        help='output file (default job_scatter.pdf)',
    )
    args = ap.parse_args()

    with open(DATA / 'job_qnames.txt') as f:
        qnames = [l.strip() for l in f if l.strip()]
    prela = parse_prela(args.prela)
    duck = parse_duck(args.duck, qnames)
    common = [q for q in qnames if q in prela and q in duck]

    xs = [duck[q] for q in common]
    yr = [prela[q] for q in common]

    # Fixed limits shared with plot_tpch.py so the paper's side-by-side
    # y scales align; widen both if a capture ever falls outside.
    lo, hi = 1e-3, 2.0

    fig, ax = plt.subplots(figsize=(4.5, 4.5))
    ax.plot(
        [lo, hi],
        [lo, hi],
        color='#888',
        linestyle='--',
        linewidth=1,
        label='y = x (parity)',
    )
    ax.scatter(
        xs,
        yr,
        s=40,
        color='#2BA84A',
        edgecolor='black',
        linewidth=0.4,
        alpha=0.85,
        label='prela',
        zorder=3,
    )

    ax.set_xscale('log')
    ax.set_yscale('log')
    ax.set_xlim(lo, hi)
    ax.set_ylim(lo, hi)
    ax.set_aspect('equal')
    ax.set_xlabel('DuckDB time (s, log)', fontsize=13)
    ax.set_ylabel('prela time (s, log)', fontsize=13)
    ax.tick_params(labelsize=11)

    ax.text(
        0.5,
        0.035,
        'Prela faster',
        transform=ax.transAxes,
        ha='center',
        va='bottom',
        fontsize=12,
        color='#666',
    )
    ax.text(
        0.035,
        0.65,
        'DuckDB faster',
        transform=ax.transAxes,
        ha='left',
        va='center',
        fontsize=12,
        color='#666',
    )

    ax.set_title('Join Order Benchmark', fontsize=14)
    ax.legend(loc='upper left', fontsize=11, framealpha=0)

    plt.tight_layout()
    plt.savefig(args.out)
    print(f'saved {args.out}')


if __name__ == '__main__':
    sys.exit(main())
