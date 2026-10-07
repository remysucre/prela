import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from oracle import DATA, lines  # noqa: E402

import duckdb

T = DATA / "tuva"
Q = Path(__file__).resolve().parent.parent / "queries/tuva"
START = re.compile(r"(?:\bwith|\),)\s+__dbt__cte__(\w+) as \(")


def prefixes(sql):
    ms = list(START.finditer(sql))
    for i, m in enumerate(ms):
        end = ms[i + 1].start() + 1 if i + 1 < len(ms) else None
        if end is None:
            depth, j = 0, m.end() - 1
            while True:
                depth += {"(": 1, ")": -1}.get(sql[j], 0)
                if depth == 0:
                    break
                j += 1
            end = j + 1
        yield m.group(1), sql[ms[0].start():end]


def main():
    con = duckdb.connect(str(T / "tuva.duckdb"), read_only=True)
    out = T / "oracles"
    seen = set()
    for f in sys.argv[1:]:
        for name, prefix in prefixes((Q / f).read_text()):
            if name in seen:
                continue
            seen.add(name)
            q = f"{prefix}\nSELECT * FROM __dbt__cte__{name}"
            got = {}
            for t in (4, 1):
                con.execute(f"SET threads={t}")
                got[t] = sorted(lines(con.execute(q).fetchall()))
            cols = [d[0] for d in con.execute(q).description]
            (out / f"c_{name.lstrip('_')}.txt").write_text("\n".join(lines(con.execute(q).fetchall())) + "\n")
            (out / f"c_{name.lstrip('_')}.cols").write_text("\n".join(cols) + "\n")
            print(f"{name:<64} {len(got[4]):>6} rows {len(cols):>3} cols {'stable' if got[4] == got[1] else 'UNSTABLE'}")


if __name__ == "__main__":
    main()
