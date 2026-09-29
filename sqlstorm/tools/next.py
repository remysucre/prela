#!/usr/bin/env python3
"""Print the next N unported query ids, in worklist order.

    python3 next.py [N] [--new]

"Ported" means registered in some batch module's ENTRIES, which is the only
place a query id appears in the Rust, so the list cannot drift from what
actually runs. Ids listed in notes/blocked.md count as done too, so a
query that needs a missing operator is not offered again. Queries sharing a shape with an already-ported one are
listed first: they are the same plan with a different projection.

With --new the preference is inverted and capped at one id per shape, so a
batch of ten covers ten distinct plans. Use it to find prela gaps; use the
default to sweep up the spellings of a shape that already works.
"""

import json
import re
import sys
from pathlib import Path

DATA = Path("/Users/paultalma/projects/sqlstorm_data")
RUST = Path(__file__).parent.parent / "rust"
BLOCKED = Path(__file__).parent.parent / "notes" / "blocked.md"


def ported():
    done = set()
    for m in RUST.glob("c*/src/*.rs"):
        done |= set(re.findall(r'\("(\d+)",', m.read_text()))
    done |= set(re.findall(r"^\| (\d+) \|", BLOCKED.read_text(), re.M))
    return done


def main():
    args = sys.argv[1:]
    new = "--new" in args
    args = [a for a in args if a != "--new"]
    n = int(args[0]) if args else 10
    done = ported()
    index = {e["id"]: e for e in json.loads((DATA / "index.json").read_text())}
    order = (DATA / "worklist.txt").read_text().split()

    done_shapes = {index[q]["shape"] for q in done if q in index}
    todo = [q for q in order if q not in done]
    remaining = len(todo)
    pos = {q: i for i, q in enumerate(order)}
    if new:
        seen = set(done_shapes)
        fresh = []
        for q in todo:
            sh = index[q]["shape"]
            if sh not in seen:
                seen.add(sh)
                fresh.append(q)
        todo = fresh
    else:
        todo.sort(key=lambda q: (index[q]["shape"] not in done_shapes, pos[q]))

    print(f"# {len(done)} ported, {len(order) - remaining} of the worklist done, "
          f"{len(todo)} candidates", file=sys.stderr)
    for q in todo[:n]:
        e = index[q]
        mark = " (known shape)" if e["shape"] in done_shapes else ""
        # The engines disagreeing means the query has more than one
        # defensible answer; check for a tie at the LIMIT before porting,
        # and add a rewrites/<id>.sql if the tied rows differ in a projected
        # column. "duckdb alone" additionally means duckdb may refuse it.
        if e.get("split"):
            mark += " [SPLIT]" if e.get("agree", 0) > 1 else " [SPLIT, duckdb alone]"
        print(f"{q}\t{e['score']}\t{','.join(e['features'])}{mark}")


if __name__ == "__main__":
    main()
