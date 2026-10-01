#!/usr/bin/env python3
"""Build Postgres oracles for the queries DuckDB could not answer.

    $PY tools/pg_oracle.py --list            # write pg_worklist.txt, print it
    $PY tools/pg_oracle.py [--nosql] [--timeout SECS] <ids...>

SQLStorm validated 86 representatives on Postgres and Umbra (they agreed) but
not on DuckDB, so they have no DuckDB oracle. This is prep.py against the
so_dba database `tools/pg_load.sh` builds: it prints the SQL, runs it with
parallel workers off and on, reports whether the sorted results agree, and
writes `oracles/<id>.txt` through oracle.py's formatter, so the Rust harness
compares against it like any other oracle. json values are kept as Postgres
prints them rather than parsed.
"""

import sys

sys.path.pop(0)
import csv
import importlib.util
import json
import re
from pathlib import Path

import psycopg
from psycopg.types.string import TextLoader

_spec = importlib.util.spec_from_file_location("oracle", Path(__file__).parent / "oracle.py")
oracle = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(oracle)

DATA = Path("/Users/paultalma/projects/sqlstorm_data")
REW = Path(__file__).parent.parent / "rewrites"
LIST = DATA / "pg_worklist.txt"


def pg_only():
    reps = {e["id"] for e in json.loads((DATA / "index.json").read_text()) if not e["duckdb"]}
    out = []
    with open(DATA / "corpus/valid_queries.csv") as f:
        for r in csv.DictReader(f):
            q = r["query"].removesuffix(".sql")
            gs = json.loads(r["systems"])
            if q in reps and any("postgres" in g for g in gs):
                out.append(q)
    return sorted(out, key=int)


args = sys.argv[1:]
if "--list" in args:
    ids = pg_only()
    LIST.write_text("\n".join(ids) + "\n")
    print("\n".join(ids))
    print(f"# {len(ids)} ids -> {LIST}", file=sys.stderr)
    sys.exit(0)

show_sql = "--nosql" not in args
args = [a for a in args if a != "--nosql"]
timeout = 90.0
if "--timeout" in args:
    i = args.index("--timeout")
    timeout = float(args[i + 1])
    del args[i : i + 2]

con = psycopg.connect("dbname=so_dba", autocommit=True)
for t in ("json", "jsonb"):
    con.adapters.register_loader(t, TextLoader)
con.execute(f"SET statement_timeout = {int(timeout * 1000)}")


def run(sql, workers):
    con.execute(f"SET max_parallel_workers_per_gather = {workers}")
    return con.execute(sql).fetchall()


for q in args:
    rw = REW / f"{q}.sql"
    sql = (rw if rw.exists() else DATA / f"corpus/queries/{q}.sql").read_text()
    print(f"=================== {q}" + (" (REWRITE)" if rw.exists() else ""), flush=True)
    if show_sql:
        lines = [l.rstrip() for l in sql.splitlines() if l.strip()]
        print("\n".join(re.sub(r"^\s+", " ", l) for l in lines))
    try:
        serial = run(sql, 0)
        par = run(sql, 4)
    except Exception as e:
        msg = str(e).splitlines()[0][:200]
        print(f"ERR {type(e).__name__}: {msg}")
        (DATA / f"oracles/{q}.err").write_text(f"postgres {type(e).__name__}: {e}\n")
        continue
    stable = sorted(oracle.fmt([r]) for r in serial) == sorted(oracle.fmt([r]) for r in par)
    (DATA / f"oracles/{q}.txt").write_text(oracle.fmt(serial) + "\n")
    (DATA / f"oracles/{q}.err").unlink(missing_ok=True)
    print(f"-- {len(serial)} rows; {'stable' if stable else 'UNSTABLE across workers'}")
    for r in oracle.fmt(serial).splitlines()[:3]:
        print("   " + r[:300])
