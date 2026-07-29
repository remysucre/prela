#!/usr/bin/env python3
# Rewrite the canonical JOB queries to Prela's JOB schema, whose ONE
# denormalization vs canonical is Company = movie_companies ⋈ company_name
# (regen inlines cn.name / cn.country_code onto the movie_companies rows;
# company identity is dropped). Everything else in the cache is 1:1 with
# the canonical tables, so the rewrite is: drop the company_name relation
# and its join predicate, and redirect cn.* through the paired mc alias.
#
# Every company_name join in the suite is a clean 1:1 cn↔mc pairing
# (`cnX.id = mcY.company_id`, 74 predicates incl. the q33 cn1/cn2
# self-joins), which the assertions below re-verify on every run.
#
# Emits TWO query sets: <out-canon> (canonical schema) and <out-prela>
# (prela schema). Both get the semantics-neutral alias rename at → att
# (15a-d alias aka_title AS at; AT is a reserved word in duckdb 1.5.3,
# which silently zeroed those timings in an earlier capture).
#
# usage: rewrite_job_prela_schema.py <canonical-query-dir> <out-canon> <out-prela>
import re, sys, glob, os

QDIR, OUTC, OUTP = sys.argv[1], sys.argv[2], sys.argv[3]
os.makedirs(OUTC, exist_ok=True)
os.makedirs(OUTP, exist_ok=True)

rewritten, verbatim = 0, 0
for path in sorted(glob.glob(f"{QDIR}/[0-9]*.sql")):
    name = os.path.basename(path)
    sql = open(path).read()

    # reserved-word alias: `aka_title AS at` → `AS att` (both schemas)
    if re.search(r"\bAS\s+at\b", sql):
        sql = re.sub(r"\bAS\s+at\b", "AS att", sql)
        sql = re.sub(r"\bat\.", "att.", sql)
    open(f"{OUTC}/{name}", "w").write(sql)

    if "company_name" not in sql:
        open(f"{OUTP}/{name}", "w").write(sql)
        verbatim += 1
        continue

    for cn in re.findall(r"company_name\s+AS\s+(\w+)", sql):
        # the paired mc alias, via the join predicate (either direction)
        m = (re.search(rf"(\w+)\.company_id\s*=\s*{cn}\.id\b", sql)
             or re.search(rf"{cn}\.id\s*=\s*(\w+)\.company_id\b", sql))
        assert m, f"{name}: no join predicate for {cn}"
        mc = m.group(1)

        # drop the join predicate (AND-chained; handle WHERE-first too)
        pred = rf"({cn}\.id\s*=\s*{mc}\.company_id|{mc}\.company_id\s*=\s*{cn}\.id)"
        sql2 = re.sub(rf"\s+AND\s+{pred}", "", sql, count=1)
        if sql2 == sql:
            sql2 = re.sub(rf"WHERE\s+{pred}\s+AND\s+", "WHERE ", sql, count=1)
        assert sql2 != sql, f"{name}: join predicate for {cn} not removed"
        sql = sql2

        # drop the FROM item
        sql2 = re.sub(rf",\s*company_name\s+AS\s+{cn}\b", "", sql, count=1)
        if sql2 == sql:
            sql2 = re.sub(rf"company_name\s+AS\s+{cn}\s*,\s*", "", sql, count=1)
        assert sql2 != sql, f"{name}: FROM item for {cn} not removed"
        sql = sql2

        # redirect columns (the suite only ever reads cn.name / cn.country_code)
        sql = re.sub(rf"\b{cn}\.country_code\b", f"{mc}.company_country", sql)
        sql = re.sub(rf"\b{cn}\.name\b", f"{mc}.company_name", sql)
        assert not re.search(rf"\b{cn}\.", sql), f"{name}: leftover {cn}. reference"

    open(f"{OUTP}/{name}", "w").write(sql)
    rewritten += 1

print(f"rewritten={rewritten} verbatim={verbatim} total={rewritten+verbatim}")
