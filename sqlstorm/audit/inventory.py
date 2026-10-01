#!/usr/bin/env python3
import json, re, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DATA = Path("/Users/paultalma/projects/sqlstorm_data")


def fns(src):
    out = {}
    for m in re.finditer(r"^(?:pub )?fn ([A-Za-z_0-9]+)", src, re.M):
        i = src.index("{", m.end())
        d, j = 0, i
        while True:
            c = src[j]
            if c == "{":
                d += 1
            elif c == "}":
                d -= 1
                if d == 0:
                    break
            j += 1
        start = src.rfind("\n\n", 0, m.start())
        out[m.group(1)] = (m.start(), j + 1, src[m.start():j + 1])
    return out


def main():
    pg = set((DATA / "pg_worklist.txt").read_text().split())
    rows = []
    for f in sorted((ROOT / "rust").glob("c*/src/b*.rs")):
        crate = f.parent.parent.name
        if int(crate[1:]) > 113:
            continue
        src = f.read_text()
        m = re.search(r"pub (?:const|static) ENTRIES[^=]*=\s*&\[(.*?)\];", src, re.S)
        if not m:
            continue
        for qid, fn in re.findall(r'\("(\d+)",\s*([A-Za-z_0-9]+)\)', m.group(1)):
            rows.append(dict(id=qid, fn=fn, crate=crate, file=str(f.relative_to(ROOT)),
                             oracle=(DATA / "oracles" / f"{qid}.txt").exists(),
                             rewrite=(ROOT / "rewrites" / f"{qid}.sql").exists(),
                             pg=qid in pg))
    json.dump(rows, open(ROOT / "audit/inventory.json", "w"), indent=0)
    ids = [r["id"] for r in rows]
    print("entries", len(rows), "distinct ids", len(set(ids)))
    print("with oracle", sum(r["oracle"] for r in rows), "pg", sum(r["pg"] for r in rows),
          "rewrites", sum(r["rewrite"] for r in rows))
    print("distinct fns", len({(r["file"], r["fn"]) for r in rows}))


if __name__ == "__main__":
    main()
