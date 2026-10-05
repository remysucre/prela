#!/usr/bin/env python3
import json, os, re, subprocess, sys
from pathlib import Path
ROOT = Path(__file__).resolve().parent.parent
out = ROOT / "audit/prela_times.tsv"
done = {l.split("\t")[0] for l in out.read_text().splitlines()} if out.exists() else set()
inv = json.load(open(ROOT / "audit/inventory.json"))
f = open(out, "a")
for r in inv:
    q, c = r["id"], r["crate"]
    if q in done:
        continue
    env = dict(os.environ, SQLSTORM_ONLY=q)
    try:
        p = subprocess.run([str(ROOT / f"rust/target/release/{c}")], env=env, capture_output=True, text=True, timeout=600)
        m = re.search(rf"^{q}\s+(ok|DIFF)\s+([\d.]+)s", p.stdout, re.M)
        st, t = (m.group(1), m.group(2)) if m else ("err", "-1")
    except subprocess.TimeoutExpired:
        st, t = "timeout", "600"
    f.write(f"{q}\t{c}\t{st}\t{t}\n"); f.flush()
