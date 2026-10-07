import re
import sys
from pathlib import Path

import duckdb

O = Path("/Users/paultalma/projects/bigsql_data/ohdsi")


def session():
    con = duckdb.connect()
    con.execute(f"ATTACH '{O / 'ohdsi.duckdb'}' AS src (READ_ONLY)")
    for (t,) in con.execute("SELECT table_name FROM duckdb_tables() WHERE database_name='src'").fetchall():
        con.execute(f"CREATE VIEW main.{t} AS SELECT * FROM src.{t}")
    con.execute((Path(__file__).parent / "setup.sql").read_text())
    return con


def run_script(con, sql):
    con.execute(sql)
    return sorted(con.execute("SELECT subject_id, cohort_start_date, cohort_end_date FROM cohort_out ORDER BY ALL").fetchall())


def main():
    out = []
    for p in sorted((p for p in (O / "rendered").glob("*.sql") if p.stem.isdigit()), key=lambda p: int(p.stem)):
        sql = p.read_text()
        try:
            con = session()
            rows = run_script(con, sql)
            n = len(rows)
        except Exception as e:
            n = f"ERR {type(e).__name__}"
        joins = len(re.findall(r"\bJOIN\b", sql, re.I))
        out.append((p.stem, len(sql.splitlines()), joins, n))
        print(*out[-1], sep="\t", flush=True)


if __name__ == "__main__":
    main()
