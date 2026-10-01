#!/bin/bash
# tools/pg_load.sh: (re)create the Postgres database so_dba from the dba dump,
# with the same schema and copy statements the DuckDB build used. Postgres is
# the oracle for the queries SQLStorm validated on Postgres and Umbra but not
# DuckDB (see tools/pg_oracle.py). C collation and New York time, as the
# DuckDB oracles have.
set -euo pipefail
DATA=${SQLSTORM_DATA:-/Users/paultalma/projects/sqlstorm_data}
DB=${SQLSTORM_PG:-so_dba}
dropdb --if-exists "$DB"
createdb --template=template0 --locale=C --encoding=UTF8 "$DB"
psql -q -d "$DB" -c "alter database $DB set timezone = 'America/New_York'"
psql -q -v ON_ERROR_STOP=1 -d "$DB" -f "$DATA/schema_nofk.sql"
sed -E "s#^copy ([A-Za-z]+) from '[^']*/([A-Za-z]+\.csv)'#\\\\copy \1 from '$DATA/stackoverflow_dba/\2'#" \
    "$DATA/copy_local.sql" | psql -v ON_ERROR_STOP=1 -d "$DB"
psql -q -d "$DB" -c "delete from badges where id = 11"
for ix in posts:owneruserid posts:parentid posts:acceptedanswerid posts:posttypeid comments:postid comments:userid \
          votes:postid votes:userid badges:userid posthistory:postid posthistory:userid postlinks:postid postlinks:relatedpostid; do
    psql -q -d "$DB" -c "create index on ${ix%%:*} (${ix#*:})"
done
psql -q -d "$DB" -c "vacuum analyze"
