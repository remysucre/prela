#!/usr/bin/env python3
"""Build the prela binary cache for the SQLStorm StackOverflow dataset.

    python3 build_cache.py [--data DIR]

Reads the DuckDB database loaded from the SQLStorm CSV dump and writes one
`<Entity>_<field>.bin` per column into DIR/cache, in the format specified by
rust/src/format.rs. Also emits:

    DIR/cache/columns.json   what each column became, and why
    sqlstorm/rust/harness/src/schema.rs   the prela schema, generated

Everything load-time happens here, the way `regen` does it for JOB/TPC-H:
ids are remapped dense, foreign keys are resolved to the target's dense id
(dangling and NULL alike become an absent CSR entry), timestamps are stored
as epoch microseconds, and Posts.Tags is exploded into a real CSR edge into
Tags.
"""

import argparse
import json
import struct
import sys
from array import array
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import spec  # noqa: E402

MAGIC = b"prela2\0\0"
HEADER_LEN = 32
KIND_DENSE_I64 = 0
KIND_DENSE_F64 = 1
KIND_DENSE_STR = 2
KIND_CSR_WORDS = 3
KIND_CSR_STR = 4


def align8(x):
    return (x + 7) & ~7


def header(kind, n, m):
    return MAGIC + struct.pack("<IIQQ", kind, 0, n, m)


def write_bin(path, kind, n, m, payload):
    with open(path, "wb") as f:
        f.write(header(kind, n, m))
        f.write(payload)


# ----- writers, one per physical kind ------------------------------------


def w_dense_i64(path, vals):
    write_bin(path, KIND_DENSE_I64, len(vals), 0, array("q", vals).tobytes())


def w_dense_f64(path, vals):
    write_bin(path, KIND_DENSE_F64, len(vals), 0, array("d", vals).tobytes())


def w_dense_str(path, vals):
    offs = array("I", [0])
    buf = bytearray()
    for v in vals:
        buf += v.encode("utf8")
        offs.append(len(buf))
    write_bin(path, KIND_DENSE_STR, len(vals), len(buf), offs.tobytes() + bytes(buf))


def w_csr_words(path, rows):
    """rows: list (len n) of lists of ints."""
    offs = array("I", [0])
    vals = array("q")
    for r in rows:
        for v in r:
            vals.append(v)
        offs.append(len(vals))
    n = len(rows)
    pad = align8(HEADER_LEN + (n + 1) * 4) - (HEADER_LEN + (n + 1) * 4)
    write_bin(
        path, KIND_CSR_WORDS, n, len(vals), offs.tobytes() + bytes(pad) + vals.tobytes()
    )


def w_csr_str(path, rows):
    """rows: list (len n) of lists of str."""
    row_off = array("I", [0])
    str_off = array("I", [0])
    buf = bytearray()
    total = 0
    for r in rows:
        for s in r:
            buf += s.encode("utf8")
            str_off.append(len(buf))
            total += 1
        row_off.append(total)
    write_bin(
        path,
        KIND_CSR_STR,
        len(rows),
        total,
        row_off.tobytes() + str_off.tobytes() + bytes(buf),
    )


# ----- the build ---------------------------------------------------------


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--data", default="/Users/paultalma/projects/sqlstorm_data")
    ap.add_argument("--only", default=None, help="comma-separated entity names")
    args = ap.parse_args()

    import duckdb

    data = Path(args.data)
    cache = data / "cache"
    cache.mkdir(exist_ok=True)

    con = duckdb.connect()
    con.execute(f"ATTACH '{data / 'so_dba.duckdb'}' AS so (READ_ONLY)")
    con.execute("PRAGMA memory_limit='8GB'")

    # column order/type straight from the loaded database
    cols = {}
    for tbl, col, ty in con.execute(
        "select table_name, column_name, data_type from duckdb_columns() "
        "where database_name='so' order by table_name, column_index"
    ).fetchall():
        cols.setdefault(tbl, []).append((col, ty))

    # 1. dense id maps
    for ent, tbl in spec.ENTITIES.items():
        con.execute(
            f"CREATE TABLE map_{tbl} AS SELECT Id AS orig, "
            f"CAST(row_number() OVER (ORDER BY Id) - 1 AS BIGINT) AS k FROM so.{tbl}"
        )
        con.execute(f"CREATE UNIQUE INDEX ix_{tbl} ON map_{tbl}(orig)")

    manifest = []

    for ent, tbl in spec.ENTITIES.items():
        if args.only and ent not in args.only.split(","):
            continue
        print(f"--- {ent} ({tbl})", flush=True)

        # 2. one denormalized table per entity: dense key + resolved columns
        sel = ["m.k AS k"]
        joins = [f"so.{tbl} t JOIN map_{tbl} m ON t.Id = m.orig"]
        fields = []  # (rust field, sql expr alias, payload type)
        for i, (col, ty) in enumerate(cols[tbl]):
            fld = spec.field_name(tbl, col)
            if (tbl, col) in spec.FKS:
                target = spec.FKS[(tbl, col)]
                a = f"f{i}"
                joins.append(f"LEFT JOIN map_{target} {a} ON t.\"{col}\" = {a}.orig")
                sel.append(f'{a}.k AS "{fld}"')
                fields.append((fld, "i64", spec.SQL_TO_ENTITY[target]))
                # The raw SQL id alongside the resolved edge. A FK can be
                # NOT NULL yet dangle (Votes.PostId points outside the dba
                # subset for 113k rows), and then the edge is absent while
                # SQL still has a value to project and to count.
                sel.append(f'CAST(t."{col}" AS BIGINT) AS "{fld}_id"')
                fields.append((f"{fld}_id", "i64", None))
            else:
                pt = spec.payload(ty)
                if spec.is_timestamp(ty):
                    e = f'epoch_us(t."{col}")'
                elif pt == "i64":
                    e = f'CAST(t."{col}" AS BIGINT)'
                elif pt == "f64":
                    e = f'CAST(t."{col}" AS DOUBLE)'
                else:
                    e = f't."{col}"'
                sel.append(f'{e} AS "{fld}"')
                fields.append((fld, pt, None))

        con.execute(
            f"CREATE TABLE d_{tbl} AS SELECT {', '.join(sel)} FROM "
            + " ".join(joins)
        )
        n = con.execute(f"SELECT count(*) FROM d_{tbl}").fetchone()[0]

        # 3. one file per column; dense if the data has no nulls, else CSR
        for fld, pt, fk in fields:
            rows = con.execute(
                f'SELECT "{fld}" FROM d_{tbl} ORDER BY k'
            ).fetchall()
            vals = [r[0] for r in rows]
            assert len(vals) == n, (ent, fld, len(vals), n)
            nulls = sum(1 for v in vals if v is None)
            name = f"{ent}_{fld}"
            path = cache / f"{name}.bin"
            if nulls == 0:
                if pt == "Str":
                    w_dense_str(path, vals)
                    kind = KIND_DENSE_STR
                elif pt == "f64":
                    w_dense_f64(path, vals)
                    kind = KIND_DENSE_F64
                else:
                    w_dense_i64(path, vals)
                    kind = KIND_DENSE_I64
                shape = "dense"
            else:
                rs = [[] if v is None else [v] for v in vals]
                if pt == "Str":
                    w_csr_str(path, rs)
                    kind = KIND_CSR_STR
                else:
                    w_csr_words(path, rs)
                    kind = KIND_CSR_WORDS
                shape = "set"
            manifest.append(
                dict(entity=ent, field=fld, table=tbl, file=name, kind=kind,
                     shape=shape, payload=pt, fk=fk, n=n, nulls=nulls)
            )
            print(f"    {name:38s} {shape:5s} {pt:3s} nulls={nulls}", flush=True)

        # 4. Posts.Tags -> a real CSR edge into Tags
        if tbl == "Posts":
            pairs = con.execute(
                """
                WITH ex AS (
                    SELECT p.k AS pk,
                           unnest(string_split(trim(sp.Tags, '<>'), '><')) AS tag
                    FROM d_Posts p
                    JOIN so.Posts sp ON sp.Id = p.origid
                    WHERE sp.Tags IS NOT NULL AND sp.Tags <> ''
                )
                SELECT ex.pk, mt.k AS tk
                FROM ex
                JOIN so.Tags st ON st.TagName = ex.tag
                JOIN map_Tags mt ON mt.orig = st.Id
                ORDER BY ex.pk
                """
            ).fetchall()
            rs = [[] for _ in range(n)]
            for pk, tk in pairs:
                rs[pk].append(tk)
            w_csr_words(cache / "Post_tags.bin", rs)
            manifest.append(
                dict(entity="Post", field="tags", table="Posts", file="Post_tags",
                     kind=KIND_CSR_WORDS, shape="set", payload="i64", fk="Tag",
                     n=n, nulls=sum(1 for r in rs if not r))
            )
            print(f"    Post_tags                              set   fk  pairs={len(pairs)}",
                  flush=True)

    (cache / "columns.json").write_text(json.dumps(manifest, indent=1))
    print(f"\nwrote {len(manifest)} columns to {cache}")
    emit_schema(manifest, Path(__file__).parent.parent / "rust/harness/src/schema.rs")


# ----- generated prela schema -------------------------------------------


def emit_schema(manifest, out):
    out.parent.mkdir(parents=True, exist_ok=True)
    by_ent = {}
    for m in manifest:
        by_ent.setdefault(m["entity"], []).append(m)

    L = []
    L.append("// GENERATED by sqlstorm/tools/build_cache.py — do not edit by hand.")
    L.append("//")
    L.append("// The SQLStorm StackOverflow schema as prela entities. A column that is")
    L.append("// NULL anywhere in the data is a `Set` (CSR, zero-or-one entry per key):")
    L.append("// absence IS the null, so `.with(col)` is IS NOT NULL, `.minus(col)` is")
    L.append("// IS NULL, and `.select(col)` drops null rows by itself. Columns with no")
    L.append("// nulls stay dense `Col`s. Timestamps are epoch MICROSECONDS (see")
    L.append("// harness::time). `origid` is the original SQL `Id`.")
    L.append("")
    L.append("#![allow(dead_code)]")
    L.append("")
    L.append("use prela::engine::{Id, IntoQuery};")
    L.append("use prela::loader::{Col, Key, Loader, Set, Str};")
    L.append("use std::path::Path;")
    L.append("")

    for ent, ms in by_ent.items():
        L.append("#[derive(IntoQuery)]")
        L.append(f"pub struct {ent} {{")
        L.append("    #[primary_key]")
        L.append("    pub id: Key<Self>,")
        for m in ms:
            ty = f"Id<{m['fk']}>" if m["fk"] else m["payload"]
            wrap = "Col" if m["shape"] == "dense" else "Set"
            L.append(f"    pub {m['field']}: {wrap}<Self, {ty}>,")
        L.append("}")
        L.append("")

    L.append("pub struct So {")
    for ent in by_ent:
        L.append(f"    pub {spec.snake(ent)}: {ent},")
    L.append("}")
    L.append("")
    L.append("fn build(l: &mut Loader) -> So {")
    L.append("    So {")
    for ent, ms in by_ent.items():
        L.append(f"        {spec.snake(ent)}: {ent} {{")
        L.append(f'            id: l.key("{ent}_origid"),')
        for m in ms:
            call = {
                ("dense", "i64", False): "i64s",
                ("dense", "f64", False): "f64s",
                ("dense", "Str", False): "strs",
                ("dense", "i64", True): "ids",
                ("set", "i64", False): "multi_i64",
                ("set", "Str", False): "multi_strs",
                ("set", "i64", True): "multi_ids",
            }[(m["shape"], m["payload"], bool(m["fk"]))]
            L.append(f'            {m["field"]}: l.{call}("{m["file"]}"),')
        L.append("        },")
    L.append("    }")
    L.append("}")
    L.append("")
    L.append("pub fn load(dir: &Path) -> So {")
    L.append("    build(&mut Loader::new(dir))")
    L.append("}")
    L.append("")
    L.append("pub fn manifest() -> Vec<(String, u32)> {")
    L.append("    let mut l = Loader::probing();")
    L.append("    let _ = build(&mut l);")
    L.append("    l.manifest()")
    L.append("}")
    out.write_text("\n".join(L) + "\n")
    print(f"wrote {out}")


if __name__ == "__main__":
    main()
