#!/usr/bin/env python3
import json, sys, time, threading
from pathlib import Path
import duckdb

ROOT = Path(__file__).resolve().parent.parent
DATA = Path("/Users/paultalma/projects/sqlstorm_data")
threads = int(sys.argv[1])
out = ROOT / f"audit/duck_t{threads}.tsv"
ids = [l.strip() for l in open(sys.argv[2])] if len(sys.argv) > 2 else [r["id"] for r in json.load(open(ROOT / "audit/inventory.json"))]
done = set()
if out.exists():
    done = {l.split("\t")[0] for l in out.read_text().splitlines()}
con = duckdb.connect(str(DATA / "so_dba.duckdb"), read_only=True)
con.execute(f"SET threads={threads}")
TIMEOUT = 60.0
f = open(out, "a")
for qid in ids:
    if qid in done:
        continue
    rw = ROOT / "rewrites" / f"{qid}.sql"
    sql = (rw if rw.exists() else DATA / "corpus/queries" / f"{qid}.sql").read_text()
    timer = threading.Timer(TIMEOUT, con.interrupt)
    t = time.perf_counter()
    timer.start()
    try:
        n = len(con.execute(sql).fetchall())
        dt = time.perf_counter() - t
        status = "ok"
    except Exception as e:
        dt = time.perf_counter() - t
        n = -1
        status = "timeout" if dt >= TIMEOUT - 0.5 else "err"
    timer.cancel()
    f.write(f"{qid}\t{status}\t{dt:.4f}\t{n}\n")
    f.flush()
