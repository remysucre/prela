#!/usr/bin/env python3
import statistics
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from oracle import QUERIES, connect  # noqa: E402

OUT = Path(__file__).resolve().parent.parent / "timing/duck.tsv"


def main():
    s, names = sys.argv[1], sys.argv[2:]
    names = names or sorted(p.stem for p in (QUERIES / s).glob("*.sql"))
    con = connect(s)
    rows = {}
    if OUT.exists():
        for ln in OUT.read_text().splitlines()[1:]:
            f = ln.split("\t")
            rows[(f[0], f[1])] = f
    for n in names:
        sql = (QUERIES / s / f"{n}.sql").read_text()
        res = [s, n]
        for t in (1, 8):
            con.execute(f"SET threads={t}")
            con.execute(sql).fetchall()
            ts = []
            for _ in range(5):
                t0 = time.perf_counter()
                con.execute(sql).fetchall()
                ts.append(time.perf_counter() - t0)
            res.append(f"{statistics.median(ts):.4f}")
        rows[(s, n)] = res
        print("\t".join(res), flush=True)
    OUT.write_text("set\tquery\tduck_t1_s\tduck_t8_s\n" + "".join("\t".join(r) + "\n" for _, r in sorted(rows.items())))


if __name__ == "__main__":
    main()
