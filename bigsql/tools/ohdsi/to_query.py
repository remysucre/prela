import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from survey import O, run_script, session  # noqa: E402

OUT = Path(__file__).resolve().parents[2] / "queries/ohdsi"


def strip_comments(s):
    return "\n".join(l for l in s.splitlines() if not l.strip().startswith("--")).strip()


def coldefs(body):
    cols = []
    for part in re.split(r",(?![^()]*\))", body):
        toks = part.split()
        cols.append((toks[0], " ".join(t for t in toks[1:] if t.upper() not in ("NOT", "NULL"))))
    return cols


def convert(sql):
    stmts = [strip_comments(x) for x in sql.split(";")]
    stmts = [s for s in stmts if s]
    ctes, decl, final = [], {}, None
    for st in stmts:
        if re.match(r"(ANALYZE|DELETE|DROP|TRUNCATE)\b", st, re.I):
            continue
        m = re.match(r"CREATE TEMP TABLE (\w+)\s+AS\s+(.*)$", st, re.S | re.I)
        if m:
            ctes.append((m.group(1), m.group(2), None))
            continue
        m = re.match(r"CREATE TEMP TABLE (\w+)\s*\((.*)\)$", st, re.S | re.I)
        if m:
            decl[m.group(1).lower()] = (m.group(1), coldefs(m.group(2)), [])
            ctes.append((m.group(1), None, m.group(1).lower()))
            continue
        m = re.match(r"INSERT INTO main\.cohort_out\s*\(([^)]*)\)\s*(.*)$", st, re.S | re.I)
        if m:
            final = m.group(2)
            break
        m = re.match(r"INSERT INTO (\w+)\s*\(([^)]*)\)\s*(.*)$", st, re.S | re.I)
        if m and m.group(1).lower() in decl:
            decl[m.group(1).lower()][2].append(m.group(3))
            continue
        raise SystemExit(f"unhandled statement: {st[:80]!r}")
    assert final is not None
    parts = []
    for name, body, key in ctes:
        if body is not None:
            parts.append(f"{name} AS (\n{body}\n)")
            continue
        _, cols, inserts = decl[key]
        names = ", ".join(c for c, _ in cols)
        casts = ", ".join(f"CAST({c} AS {t}) AS {c}" for c, t in cols)
        if inserts:
            src = "\nUNION ALL\n".join(f"SELECT * FROM (\n{i}\n)" for i in inserts)
            parts.append(f"{name} AS (\nSELECT {casts} FROM (\n{src}\n) AS _t({names})\n)")
        else:
            nulls = ", ".join(f"CAST(NULL AS {t}) AS {c}" for c, t in cols)
            parts.append(f"{name} AS (\nSELECT {nulls} WHERE FALSE\n)")
    final = (
        "SELECT CAST(cohort_definition_id AS BIGINT) AS cohort_definition_id, CAST(subject_id AS BIGINT) AS subject_id, "
        "CAST(cohort_start_date AS DATE) AS cohort_start_date, CAST(cohort_end_date AS DATE) AS cohort_end_date FROM (\n"
        + final
        + "\n) AS _f(cohort_definition_id, subject_id, cohort_start_date, cohort_end_date)"
    )
    return "WITH " + ",\n".join(parts) + "\n" + final + "\n"


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    for cid in sys.argv[1:]:
        script = (O / "rendered" / f"{cid}.sql").read_text()
        q = convert(script)
        con = session()
        con.execute(script)
        want = sorted(con.execute("SELECT * FROM cohort_out").fetchall())
        got = sorted(session().execute(q).fetchall())
        (OUT / f"c{cid}.sql").write_text(f"-- OHDSI PhenotypeLibrary cohort {cid}, rendered by SqlRender for duckdb, temp tables folded into CTEs\n" + q)
        print(f"c{cid}: {len(q.splitlines())} lines, {len(re.findall(r'JOIN', q, re.I))} joins, {len(got)} rows, {'matches script' if got == want else 'DIFFERS FROM SCRIPT'}")


if __name__ == "__main__":
    main()
