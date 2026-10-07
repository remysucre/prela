#!/usr/bin/env python3
import os
import subprocess
import sys
from pathlib import Path

B = Path(__file__).resolve().parent.parent
OUT = B / "timing/prela.tsv"


def main():
    sets = sys.argv[1:] or ["mimic", "ohdsi"]
    rows = {}
    if OUT.exists():
        for ln in OUT.read_text().splitlines()[1:]:
            f = ln.split("\t")
            rows[(f[0], f[1])] = f
    for s in sets:
        subprocess.run(["cargo", "build", "-q", "--release", "-p", s, "--bin", s], cwd=B / "rust", check=True)
        env = dict(os.environ, BIGSQL_REPS="5")
        r = subprocess.run([str(B / "rust/target/release" / s)], cwd=B / "rust", env=env, capture_output=True, text=True)
        for ln in r.stdout.splitlines():
            f = ln.split()
            if len(f) >= 4 and not f[0].startswith("c_"):
                rows[(s, f[0])] = [s, f[0], f[1], f[2].rstrip("s"), f[3]]
                print("\t".join(rows[(s, f[0])]))
    OUT.write_text("set\tquery\tstatus\tprela_s\trows\n" + "".join("\t".join(r) + "\n" for _, r in sorted(rows.items())))


if __name__ == "__main__":
    main()
