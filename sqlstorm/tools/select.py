#!/usr/bin/env python3
"""Order the SQLStorm corpus easiest-first and emit the worklist.

    python3 select.py --out worklist.txt

The end goal is all 18,251 queries, so this is an ordering, not a filter.
It does three things:

  * drops queries SQLStorm itself could not run on DuckDB
    (`valid_queries.csv` is the systems each query validated on);
  * collapses exact duplicates after whitespace/case normalization — the
    corpus is LLM-generated and repeats itself (12558 and 13123 are the
    same `SELECT COUNT(*) FROM Posts`), keeping the lowest id as the
    representative and recording the rest as aliases;
  * sorts by a feature-difficulty score, so a batch is a coherent set of
    queries rather than a random walk across the whole feature space.

Difficulty is the count of prela-relevant SQL features present, weighted:
window functions and lateral/unnest cost most, plain GROUP BY / ORDER BY
nothing at all (both are host-Rust work after the plan).
"""

import argparse
import csv
import json
import re
from collections import defaultdict
from pathlib import Path

DATA = Path("/Users/paultalma/projects/sqlstorm_data")

# feature -> (regex, weight). Weight 0 = free in prela today.
FEATURES = {
    "order_by": (r"\bORDER\s+BY\b", 0),
    "group_by": (r"\bGROUP\s+BY\b", 0),
    "limit": (r"\bLIMIT\b", 0),
    "having": (r"\bHAVING\b", 0),
    "like": (r"\bLIKE\b", 0),
    "cast": (r"\bCAST\s*\(", 0),
    "case": (r"\bCASE\b", 1),
    "coalesce": (r"\b(COALESCE|NULLIF|IFNULL)\b", 1),
    "is_null": (r"\bIS\s+(NOT\s+)?NULL\b", 1),
    "distinct": (r"\bDISTINCT\b", 1),
    "interval": (r"\bINTERVAL\b|\bDATE_TRUNC\b|\bEXTRACT\b", 1),
    "outer_join": (r"\b(LEFT|RIGHT|FULL)\s+(OUTER\s+)?JOIN\b", 2),
    "cte": (r"\bWITH\b", 2),
    "subquery": (r"\bSELECT\b[\s\S]*\(\s*SELECT\b", 3),
    "setop": (r"\b(UNION|EXCEPT|INTERSECT)\b", 3),
    "string_agg": (r"\b(STRING_AGG|ARRAY_AGG|LISTAGG)\b", 4),
    "lateral": (r"\b(LATERAL|UNNEST)\b", 5),
    "window": (r"\bOVER\s*\(", 5),
}


def features(sql):
    up = sql.upper()
    return {k for k, (p, _) in FEATURES.items() if re.search(p, up)}


def score(fs):
    return sum(FEATURES[f][1] for f in fs)


def normalize(sql):
    s = re.sub(r"--[^\n]*", " ", sql)
    s = re.sub(r"/\*[\s\S]*?\*/", " ", s)
    return re.sub(r"\s+", " ", s).strip().rstrip(";").upper()


# ----- structural shape --------------------------------------------------
#
# Text dedup barely dents this corpus: positions 11..40 of the worklist are
# thirty spellings of one query, differing only in alias case, join
# direction, and the order of the projection. So a second key is taken from
# DuckDB's own parse tree with everything cosmetic removed — table aliases,
# output aliases, source positions, the qualifier on a column reference —
# and the projection list sorted. Queries sharing a shape key are ONE
# relational plan with several formatters, which is how they get ported.

DROP_KEYS = {"query_location", "alias", "column_name_alias", "catalog_name", "schema_name"}


def canon(o):
    if isinstance(o, dict):
        d = {k: canon(v) for k, v in o.items() if k not in DROP_KEYS}
        cn = d.get("column_names")
        if isinstance(cn, list) and cn:
            d["column_names"] = cn[-1:]  # drop the alias qualifier
        sl = d.get("select_list")
        if isinstance(sl, list):
            d["select_list"] = sorted(sl, key=lambda x: json.dumps(x, sort_keys=True))
        return d
    if isinstance(o, list):
        return [canon(x) for x in o]
    return o


def shape_key(con, sql):
    """A hash of the query's shape, or None if DuckDB cannot parse it."""
    import hashlib

    try:
        js = con.execute("select json_serialize_sql(?)", [sql.strip()]).fetchone()[0]
        ast = json.loads(js)
        if ast.get("error"):
            return None
    except Exception:  # noqa: BLE001
        return None
    blob = json.dumps(canon(ast), sort_keys=True)
    return hashlib.sha1(blob.encode()).hexdigest()[:16]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--data", default=str(DATA))
    ap.add_argument("--out", default=None)
    ap.add_argument("--json", default=None, help="also write the full index here")
    args = ap.parse_args()

    import duckdb

    data = Path(args.data)
    qdir = data / "corpus/queries"

    # valid_queries.csv's `systems` column is NOT a flat list of engines that
    # ran the query. It is a PARTITION of the engines into groups that agreed
    # on the result: [["umbra","duckdb","postgres"]] means all three agreed,
    # [["postgres","umbra"],["duckdb"]] means duckdb disagreed with the other
    # two — which includes the case where duckdb refuses the query outright.
    # Testing `"duckdb" in r["systems"]` as a substring silently accepts both.
    duckdb_ok = set()
    agreed = {}
    with open(data / "corpus/valid_queries.csv") as f:
        for r in csv.DictReader(f):
            qid = r["query"].removesuffix(".sql")
            gs = json.loads(r["systems"])
            for g in gs:
                if "duckdb" in g:
                    duckdb_ok.add(qid)
                    # How many engines duckdb agreed with, and whether the
                    # engines split at all. A split means the query has more
                    # than one defensible answer — nearly always an ORDER BY
                    # that is not a total order under a LIMIT.
                    agreed[qid] = (len(g), len(gs))

    seen = {}
    aliases = defaultdict(list)
    index = []
    for p in sorted(qdir.glob("*.sql"), key=lambda p: int(p.stem)):
        qid = p.stem
        sql = p.read_text(encoding="utf8", errors="replace")
        key = normalize(sql)
        if key in seen:
            aliases[seen[key]].append(qid)
            continue
        seen[key] = qid
        fs = features(sql)
        index.append(
            dict(
                id=qid,
                duckdb=qid in duckdb_ok,
                # engines that matched duckdb (itself included), and how many
                # distinct answers the engines produced between them
                agree=agreed.get(qid, (0, 0))[0],
                split=agreed.get(qid, (0, 1))[1] > 1,
                score=score(fs),
                features=sorted(fs),
                chars=len(sql),
            )
        )

    for e in index:
        e["aliases"] = aliases.get(e["id"], [])

    con = duckdb.connect()
    shapes = defaultdict(list)
    for e in index:
        e["shape"] = shape_key(con, (qdir / f"{e['id']}.sql").read_text())
        if e["shape"]:
            shapes[e["shape"]].append(e["id"])

    runnable = [e for e in index if e["duckdb"]]
    # Group by shape: all the spellings of one query land together, ordered
    # by the difficulty of the shape's easiest member.
    rank = {}
    for e in runnable:
        k = e["shape"] or f"x{e['id']}"
        r = (e["score"], e["chars"], int(e["id"]))
        rank[k] = min(rank.get(k, r), r)
    runnable.sort(key=lambda e: (rank[e["shape"] or f"x{e['id']}"], int(e["id"])))

    print(f"{len(index)} distinct of {len(list(qdir.glob('*.sql')))} queries")
    print(f"{len(runnable)} validated on duckdb")
    print(f"{sum(1 for e in runnable if e['split'])} where the engines disagree "
          f"({sum(1 for e in runnable if e['agree'] == 1)} where duckdb stands alone)")
    print(f"{len({e['shape'] for e in runnable if e['shape']})} distinct shapes among them")
    unparsed = sum(1 for e in index if not e["shape"])
    print(f"{unparsed} did not parse (no shape key)")
    hist = defaultdict(int)
    for e in runnable:
        hist[e["score"]] += 1
    print("difficulty score histogram:")
    for s in sorted(hist):
        print(f"  {s:>3}: {hist[s]}")

    if args.out:
        Path(args.out).write_text("\n".join(e["id"] for e in runnable) + "\n")
        print(f"wrote {args.out}")
    if args.json:
        Path(args.json).write_text(json.dumps(index, indent=1))
        print(f"wrote {args.json}")


if __name__ == "__main__":
    main()
