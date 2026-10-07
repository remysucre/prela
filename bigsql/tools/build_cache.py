#!/usr/bin/env python3
import argparse
import importlib
import json
import struct
import sys
from array import array
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
DATA = Path("/Users/paultalma/projects/bigsql_data")

MAGIC = b"prela2\0\0"
HEADER_LEN = 32
KIND_DENSE_I64, KIND_DENSE_F64, KIND_DENSE_STR, KIND_CSR_WORDS, KIND_CSR_STR = 0, 1, 2, 3, 4
RESERVED = {"type", "ref", "match", "move", "box", "self", "crate", "as", "in", "fn", "mod", "id", "use", "where", "loop", "struct", "enum", "impl", "trait", "static", "const", "let", "mut", "pub", "super", "return", "true", "false", "if", "else", "for", "while", "break", "continue", "dyn", "async", "await", "extern", "unsafe", "override", "final", "abstract", "yield", "macro", "priv", "typeof", "virtual", "do", "become", "try", "gen"}


def align8(x):
    return (x + 7) & ~7


def write_bin(path, kind, n, m, payload):
    with open(path, "wb") as f:
        f.write(MAGIC + struct.pack("<IIQQ", kind, 0, n, m))
        f.write(payload)


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
    offs = array("I", [0])
    vals = array("q")
    for r in rows:
        vals.extend(r)
        offs.append(len(vals))
    n = len(rows)
    pad = align8(HEADER_LEN + (n + 1) * 4) - (HEADER_LEN + (n + 1) * 4)
    write_bin(path, KIND_CSR_WORDS, n, len(vals), offs.tobytes() + bytes(pad) + vals.tobytes())


def w_csr_f64(path, rows):
    w_csr_words(path, [[struct.unpack("<q", struct.pack("<d", v))[0] for v in r] for r in rows])


def w_csr_str(path, rows):
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
    write_bin(path, KIND_CSR_STR, len(rows), total, row_off.tobytes() + str_off.tobytes() + bytes(buf))


def snake(name):
    if not any(c.islower() for c in name):
        s = name.lower()
        return s + "_" if s in RESERVED else s
    out = []
    for i, ch in enumerate(name):
        if ch.isupper() and i and not name[i - 1].isupper():
            out.append("_")
        out.append(ch.lower())
    s = "".join(out)
    return s + "_" if s in RESERVED else s


def payload(ty):
    t = ty.upper()
    if t.startswith(("VARCHAR", "TEXT", "CHAR", "UUID")):
        return "Str", "str"
    if t.startswith(("TIMESTAMP",)):
        return "i64", "Ts"
    if t.startswith(("DATE",)):
        return "i64", "Date"
    if t.startswith(("INT", "BIGINT", "SMALLINT", "TINYINT", "HUGEINT", "UINT", "UBIGINT", "USMALLINT", "UTINYINT")):
        return "i64", "i64"
    if t.startswith("BOOL"):
        return "i64", "i64"
    if t.startswith(("DOUBLE", "FLOAT", "DECIMAL", "REAL", "NUMERIC")):
        return "f64", "f64"
    raise SystemExit(f"unmapped SQL type {ty!r}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("set")
    args = ap.parse_args()
    spec = importlib.import_module(f"specs.{args.set}")

    import duckdb

    cache = DATA / args.set / "cache"
    cache.mkdir(parents=True, exist_ok=True)
    con = duckdb.connect()
    con.execute(f"ATTACH '{DATA / args.set / (args.set + '.duckdb')}' AS src (READ_ONLY)")

    by_table = {tbl: ent for ent, (tbl, _) in spec.ENTITIES.items()}
    fk_cols = {k.lower(): v for k, v in getattr(spec, "FK_COLUMNS", {}).items()}
    fks = dict(getattr(spec, "FKS", {}))

    def mapname(tbl):
        return "map_" + tbl.replace(".", "_")

    for ent, (tbl, pk) in spec.ENTITIES.items():
        order = pk if pk else "rowid"
        con.execute(
            f"CREATE TABLE {mapname(tbl)} AS SELECT {'t.' + pk if pk else 'NULL'} AS orig, t.rowid AS rid, "
            f"CAST(row_number() OVER (ORDER BY t.{order}) - 1 AS BIGINT) AS k FROM src.{tbl} t"
        )

    manifest = []
    for ent, (tbl, pk) in spec.ENTITIES.items():
        schema_, name = tbl.split(".") if "." in tbl else ("main", tbl)
        cols = con.execute(
            "SELECT column_name, data_type FROM duckdb_columns() WHERE database_name='src' "
            "AND schema_name=? AND table_name=? ORDER BY column_index",
            [schema_, name],
        ).fetchall()
        print(f"--- {ent} ({tbl})", flush=True)
        sel = ["m.k AS k"]
        joins = [f"src.{tbl} t JOIN {mapname(tbl)} m ON t.rowid = m.rid"]
        fields = []
        for i, (col, ty) in enumerate(cols):
            fld = snake(col)
            lc = col.lower()
            target = None
            if pk is None or lc != pk.lower():
                target = fks.get((tbl, col)) or fk_cols.get(lc)
                if target is None:
                    target = next((t for suf, t in getattr(spec, "FK_SUFFIX", {}).items() if lc.endswith(suf)), None)
            if target and target != tbl and target in by_table:
                a = f"f{i}"
                joins.append(f'LEFT JOIN {mapname(target)} {a} ON t."{col}" = {a}.orig')
                edge = fld[:-3] if fld.endswith("_id") else fld + "_ref"
                sel.append(f'{a}.k AS "{edge}"')
                fields.append(dict(field=edge, payload="i64", rust=None, fk=by_table[target], sql=col, sqltype="EDGE"))
            pt, rust = payload(ty)
            if rust in ("Ts", "Date"):
                e = f'epoch_us(t."{col}")'
            elif pt == "i64":
                e = f'CAST(t."{col}" AS BIGINT)'
            elif pt == "f64":
                e = f'CAST(t."{col}" AS DOUBLE)'
            else:
                e = f'CAST(t."{col}" AS VARCHAR)'
            sel.append(f'{e} AS "{fld}"')
            fields.append(dict(field=fld, payload=pt, rust=rust, fk=None, sql=col, sqltype=ty))

        con.execute(f"CREATE OR REPLACE TABLE d AS SELECT {', '.join(sel)} FROM " + " ".join(joins))
        n = con.execute("SELECT count(*) FROM d").fetchone()[0]
        w_dense_i64(cache / f"{ent}_row.bin", list(range(n)))
        for f in fields:
            vals = [r[0] for r in con.execute(f'SELECT "{f["field"]}" FROM d ORDER BY k').fetchall()]
            nulls = sum(v is None for v in vals)
            path = cache / f"{ent}_{f['field']}.bin"
            if nulls == 0:
                {"Str": w_dense_str, "f64": w_dense_f64, "i64": w_dense_i64}[f["payload"]](path, vals)
                shape = "dense"
            else:
                rs = [[] if v is None else [v] for v in vals]
                {"Str": w_csr_str, "f64": w_csr_f64, "i64": w_csr_words}[f["payload"]](path, rs)
                shape = "set"
            manifest.append(dict(entity=ent, table=tbl, file=f"{ent}_{f['field']}", shape=shape, n=n, nulls=nulls, **f))
            print(f"    {f['field']:32s} {shape:5s} {f['rust'] or 'Id<' + f['fk'] + '>':10s} nulls={nulls}", flush=True)

    (cache / "columns.json").write_text(json.dumps(manifest, indent=1))
    emit_schema(args.set, manifest, HERE.parent / "rust" / args.set / "src/schema.rs")


def emit_schema(set_, manifest, out):
    out.parent.mkdir(parents=True, exist_ok=True)
    by_ent = {}
    for m in manifest:
        by_ent.setdefault(m["entity"], []).append(m)
    L = [
        "#![allow(dead_code)]",
        "",
        "use harness::time::{Date, Ts};",
        "use prela::engine::{Id, IntoQuery};",
        "use prela::loader::{Col, Key, Loader, Set, Str};",
        "use std::path::Path;",
        "",
    ]
    for ent, ms in by_ent.items():
        L += ["#[derive(IntoQuery)]", f"pub struct {ent} {{", "    #[primary_key]", "    pub id: Key<Self>,"]
        for m in ms:
            ty = f"Id<{m['fk']}>" if m["fk"] else {"str": "Str"}.get(m["rust"], m["rust"])
            wrap = "Col" if m["shape"] == "dense" else "Set"
            L.append(f"    pub {m['field']}: {wrap}<Self, {ty}>,")
        L += ["}", ""]
    L.append("pub struct Db {")
    L += [f"    pub {snake(ent)}: {ent}," for ent in by_ent]
    L += ["}", "", "fn build(l: &mut Loader) -> Db {", "    Db {"]
    for ent, ms in by_ent.items():
        L.append(f"        {snake(ent)}: {ent} {{")
        L.append(f'            id: l.key("{ent}_row"),')
        for m in ms:
            if m["fk"]:
                call = "ids" if m["shape"] == "dense" else "multi_ids"
            else:
                call = {
                    ("dense", "i64"): "i64s",
                    ("dense", "f64"): "f64s",
                    ("dense", "Str"): "strs",
                    ("set", "i64"): "multi_i64",
                    ("set", "f64"): "multi_f64",
                    ("set", "Str"): "multi_strs",
                }[(m["shape"], m["payload"])]
            if call == "multi_f64":
                L.append(f'            {m["field"]}: harness::cols::multi_f64(l, "{m["file"]}"),')
            else:
                L.append(f'            {m["field"]}: l.{call}("{m["file"]}"),')
        L.append("        },")
    L += ["    }", "}", "", "pub fn load(dir: &Path) -> Db {", "    build(&mut Loader::new(dir))", "}", ""]
    out.write_text("\n".join(L) + "\n")
    print(f"wrote {out}")


if __name__ == "__main__":
    main()
