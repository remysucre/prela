#!/usr/bin/env python3
"""Produce the DuckDB oracle for one or more SQLStorm queries.

    python3 oracle.py 10117 10235 ...
    python3 oracle.py --list worklist.txt

Runs each `corpus/queries/<id>.sql` against the loaded StackOverflow
database and writes `oracles/<id>.txt` in the canonical format that
harness/src/fmt.rs mirrors, so a byte-identical result means prela and
DuckDB agree. A query that errors or times out gets `oracles/<id>.err`
instead and is reported.
"""

import argparse
import datetime
import decimal
import sys
import uuid
from pathlib import Path

DATA = Path("/Users/paultalma/projects/sqlstorm_data")


def esc(s):
    return (
        s.replace("\\", "\\\\")
        .replace("\t", "\\t")
        .replace("\n", "\\n")
        .replace("\r", "\\r")
    )


REWRITES = Path(__file__).parent.parent / "rewrites"


def val(v):
    """Format one value by the rules in harness/src/fmt.rs."""
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


def fmt(rows):
    return "\n".join("\t".join(val(v) for v in r) for r in rows)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("ids", nargs="*")
    ap.add_argument("--list", help="file with one query id per line")
    ap.add_argument("--data", default=str(DATA))
    ap.add_argument("--timeout", type=float, default=120.0)
    ap.add_argument("--force", action="store_true", help="redo existing oracles")
    args = ap.parse_args()

    import duckdb

    data = Path(args.data)
    corpus = data / "corpus/queries"
    out = data / "oracles"
    out.mkdir(exist_ok=True)

    ids = list(args.ids)
    if args.list:
        ids += [
            ln.strip()
            for ln in Path(args.list).read_text().splitlines()
            if ln.strip() and not ln.startswith("#")
        ]

    con = duckdb.connect(str(data / "so_dba.duckdb"), read_only=True)
    con.execute("SET threads=4")

    ok = err = skip = 0
    for qid in ids:
        qid = qid.removesuffix(".sql")
        dst = out / f"{qid}.txt"
        if dst.exists() and not args.force:
            skip += 1
            continue
        rewrite = REWRITES / f"{qid}.sql"
        sql = (rewrite if rewrite.exists() else corpus / f"{qid}.sql").read_text()
        try:
            rows = con.execute(sql).fetchall()
        except Exception as e:  # noqa: BLE001 — every failure is a data point
            (out / f"{qid}.err").write_text(f"{type(e).__name__}: {e}\n")
            print(f"{qid:>8} ERR  {type(e).__name__}: {str(e).splitlines()[0][:100]}")
            err += 1
            continue
        dst.write_text(fmt(rows) + "\n")
        (out / f"{qid}.err").unlink(missing_ok=True)
        print(f"{qid:>8} ok   {len(rows)} rows")
        ok += 1
    print(f"\n{ok} written, {err} errored, {skip} already present", file=sys.stderr)


if __name__ == "__main__":
    main()
