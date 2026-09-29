#!/usr/bin/env python3
"""Print each query's SQL, build its oracle, and say whether it is stable.

    $PY tools/prep.py [--nosql] [--timeout SECS] <ids...>

Like oracle.py (and it honours rewrites/<id>.sql the same way), but also
prints the SQL with indentation stripped, runs the query at 4, 1 and 2
threads and reports whether the sorted results agree, and shows the first
three rows. "stable" does not rule out a tie at a LIMIT: the cargo run
against the oracle is what catches that. Each id runs in its own process,
killed after the timeout (default 90s; DuckDB's own interrupt does not stop
every query), and is then reported as TIMEOUT with no oracle.
"""

import sys

sys.path.pop(0)  # tools/select.py would shadow the stdlib module
import importlib.util
import re
import subprocess
from pathlib import Path

import duckdb

_spec = importlib.util.spec_from_file_location("oracle", Path(__file__).parent / "oracle.py")
oracle = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(oracle)

DATA = Path("/Users/paultalma/projects/sqlstorm_data")
REW = Path(__file__).parent.parent / "rewrites"

args = sys.argv[1:]
show_sql = "--nosql" not in args
args = [a for a in args if a != "--nosql"]
timeout = 90.0
if "--timeout" in args:
    i = args.index("--timeout")
    timeout = float(args[i + 1])
    del args[i : i + 2]

one = "--one" in args
args = [a for a in args if a != "--one"]
if not one:
    for q in args:
        cmd = [sys.executable, "-u", __file__, "--one"] + (["--nosql"] if not show_sql else []) + [q]
        try:
            subprocess.run(cmd, timeout=timeout)
        except subprocess.TimeoutExpired:
            print(f"TIMEOUT after {timeout:.0f}s", flush=True)
    sys.exit(0)

con = duckdb.connect(str(DATA / "so_dba.duckdb"), read_only=True)


def run(sql):
    return con.execute(sql).fetchall()


for q in args:
    rw = REW / f"{q}.sql"
    sql = (rw if rw.exists() else DATA / f"corpus/queries/{q}.sql").read_text()
    print(f"=================== {q}" + (" (REWRITE)" if rw.exists() else ""))
    if show_sql:
        lines = [l.rstrip() for l in sql.splitlines() if l.strip()]
        print("\n".join(re.sub(r"^\s+", " ", l) for l in lines))
    res = {}
    try:
        for t in (4, 1, 2):
            con.execute(f"SET threads={t}")
            res[t] = sorted(oracle.fmt([r]) for r in run(sql))
    except Exception as e:
        msg = str(e).splitlines()[0][:200]
        print(f"ERR {type(e).__name__}: {msg}")
        (DATA / f"oracles/{q}.err").write_text(f"{type(e).__name__}: {e}\n")
        continue
    stable = res[4] == res[1] == res[2]
    con.execute("SET threads=4")
    rows = run(sql)
    (DATA / f"oracles/{q}.txt").write_text(oracle.fmt(rows) + "\n")
    print(f"-- {len(rows)} rows; {'stable' if stable else 'UNSTABLE across threads'}")
    for r in oracle.fmt(rows).splitlines()[:3]:
        print("   " + r[:300])
