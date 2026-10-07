#!/usr/bin/env python3
import re
from pathlib import Path

B = Path(__file__).resolve().parent.parent


def tsv(p):
    if not p.exists():
        return {}
    rows = p.read_text().splitlines()
    head = rows[0].split("\t")
    return {(r.split("\t")[0], r.split("\t")[1]): dict(zip(head, r.split("\t"))) for r in rows[1:]}


def main():
    duck, prela = tsv(B / "timing/duck.tsv"), tsv(B / "timing/prela.tsv")
    out = ["| set | query | SQL lines | CTEs | joins | rows | DuckDB 1 thr (s) | DuckDB 8 thr (s) | prela (s) | status |", "|---|---|---|---|---|---|---|---|---|---|"]
    for q in sorted(B.glob("queries/*/*.sql")):
        s, n = q.parent.name, q.stem
        sql = q.read_text()
        ctes = len(re.findall(r"(?:\bWITH|,|\))\s+\w+\s+AS\s*\(", sql, re.I))
        joins = len(re.findall(r"\bJOIN\b", sql, re.I))
        d, p = duck.get((s, n), {}), prela.get((s, n), {})
        out.append(f"| {s} | {n} | {len(sql.splitlines())} | {ctes} | {joins} | {p.get('rows', '')} | {d.get('duck_t1_s', '')} | {d.get('duck_t8_s', '')} | {p.get('prela_s', '')} | {p.get('status', 'todo')} |")
    print("\n".join(out))


if __name__ == "__main__":
    main()
