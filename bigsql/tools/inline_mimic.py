#!/usr/bin/env python3

import re
import sys
from pathlib import Path

SRC = Path("/Users/paultalma/projects/bigsql_data/src/mimic-code/mimic-iv/concepts_duckdb")
OUT = Path(__file__).resolve().parent.parent / "queries/mimic"

HEAD = re.compile(
    r"^--[^\n]*\n\s*DROP TABLE IF EXISTS mimiciv_derived\.\w+;\s*CREATE TABLE mimiciv_derived\.\w+ AS\s*",
    re.S,
)
REF = re.compile(r"mimiciv_derived\.(\w+)")
CTE = re.compile(r"(?:\bWITH|,)\s*(\w+)\s+AS\s*\(", re.I)


def files():
    return {p.stem: p for p in SRC.glob("*/*.sql")}


def body(path):
    s = path.read_text()
    m = HEAD.match(s)
    assert m, path
    return s[m.end():].rstrip().rstrip(";").rstrip()


def order(target, fs):
    seen, out = set(), []

    def visit(k):
        for d in sorted(set(REF.findall(body(fs[k]))) - {k}):
            if d not in seen:
                seen.add(d)
                visit(d)
                out.append(d)

    visit(target)
    return out


def indent(s, n=4):
    return "\n".join((" " * n + ln) if ln.strip() else ln for ln in s.splitlines())


def inline(target):
    fs = files()
    deps = order(target, fs)
    inner = {}
    for k in deps + [target]:
        for name in CTE.findall(body(fs[k])):
            inner.setdefault(name.lower(), set()).add(k)
    clash = {d for d in deps if d.lower() in inner and inner[d.lower()] - {d}}
    name = {d: (f"{d}_" if d in clash else d) for d in deps}

    def sub(s):
        return REF.sub(lambda m: name[m.group(1)], s)

    parts = [f"-- mimic-code concepts_duckdb: {target}, with its {len(deps)} upstream concepts inlined as CTEs"]
    ctes = [f"{name[d]} AS (\n    -- {fs[d].relative_to(SRC)}\n{indent(sub(body(fs[d])))}\n)" for d in deps]
    parts.append("WITH " + ",\n".join(ctes))
    parts.append(f"-- {fs[target].relative_to(SRC)}")
    b = sub(body(fs[target]))
    if re.match(r"\s*WITH\b", b, re.I):
        b = f"SELECT * FROM (\n{indent(b)}\n) AS {target}_"
    parts.append(b)
    return "\n".join(parts) + "\n", deps


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    for t in sys.argv[1:]:
        sql, deps = inline(t)
        (OUT / f"{t}.sql").write_text(sql)
        print(f"{t}: {len(sql.splitlines())} lines, deps={deps}")


if __name__ == "__main__":
    main()
