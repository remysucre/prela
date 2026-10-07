#!/usr/bin/env python3
import argparse
import datetime
import decimal
import uuid
from pathlib import Path

DATA = Path("/Users/paultalma/projects/bigsql_data")
QUERIES = Path(__file__).resolve().parent.parent / "queries"


def esc(s):
    return s.replace("\\", "\\\\").replace("\t", "\\t").replace("\n", "\\n").replace("\r", "\\r")


def val(v):
    if v is None:
        return "\\N"
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, int):
        return str(v)
    if isinstance(v, (float, decimal.Decimal)):
        return f"{float(v):.6f}"
    if isinstance(v, datetime.datetime):
        return v.strftime("%Y-%m-%d %H:%M:%S.") + f"{v.microsecond:06d}"
    if isinstance(v, datetime.date):
        return v.strftime("%Y-%m-%d")
    if isinstance(v, datetime.timedelta):
        return str(v)
    if isinstance(v, (list, tuple)):
        return "[" + ", ".join(val(x) for x in v) + "]"
    if isinstance(v, dict):
        return "{" + ", ".join(f"{k}: {val(x)}" for k, x in v.items()) + "}"
    if isinstance(v, (bytes, bytearray)):
        return esc(v.decode("utf8", "replace"))
    if isinstance(v, uuid.UUID):
        return str(v)
    return esc(str(v))


def lines(rows):
    return [("\t".join(val(v) for v in r)) for r in rows]


def connect(s):
    import duckdb

    return duckdb.connect(str(DATA / s / f"{s}.duckdb"), read_only=True)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("set")
    ap.add_argument("names", nargs="*")
    ap.add_argument("--check", default=None)
    ap.add_argument("--derived", default=None)
    args = ap.parse_args()

    names = args.names if args.names or args.derived else sorted(p.stem for p in (QUERIES / args.set).glob("*.sql"))
    out = DATA / args.set / "oracles"
    out.mkdir(parents=True, exist_ok=True)
    con = connect(args.set)
    for n in names:
        sql = f"SELECT * FROM {args.derived}.{n}" if args.derived else (QUERIES / args.set / f"{n}.sql").read_text()
        got = {}
        for t in (4, 1, 2):
            con.execute(f"SET threads={t}")
            got[t] = sorted(lines(con.execute(sql).fetchall()))
        stable = got[4] == got[1] == got[2]
        note = ""
        if args.check:
            ref = sorted(lines(con.execute(f"SELECT * FROM {args.check.format(name=n)}").fetchall()))
            note = "  matches DAG" if ref == got[4] else "  DIFFERS FROM DAG"
        cols = len(con.execute(sql).description)
        (out / f"{'c_' if args.derived else ''}{n}.txt").write_text("\n".join(lines(con.execute(sql).fetchall())) + "\n")
        print(f"{n:<24} {len(got[4]):>7} rows {cols:>3} cols  {'stable' if stable else 'UNSTABLE'}{note}")


if __name__ == "__main__":
    main()
