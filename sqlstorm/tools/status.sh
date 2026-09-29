#!/bin/bash
# Where the port stands, derived from the files rather than from prose.
# Run this first in a new session.
set -uo pipefail
cd "$(dirname "$0")/.."

DATA=${SQLSTORM_DATA:-/Users/paultalma/projects/sqlstorm_data}
PY="$DATA/venv/bin/python"

ported=$(grep -rhoE '\("[0-9]+",' rust/c[0-9]*/src/*.rs | sort -u | wc -l | tr -d ' ')
blocked=$(awk '/^\| query \| feature/,/^$/' notes/blocked.md | grep -c '^| [0-9]')
invalid=$(awk '/^\| query \| error/,/^$/'   notes/blocked.md | grep -c '^| [0-9]')
rewrites=$(ls rewrites/*.sql 2>/dev/null | wc -l | tr -d ' ')

echo "ported   $ported      blocked $blocked   invalid $invalid   rewrites $rewrites"
echo
echo "crates (100 queries each; add the next batch to the last one until it fills):"
for c in rust/c[0-9]*/; do
    n=$(grep -rhoE '\("[0-9]+",' "$c"src/*.rs | sort -u | wc -l | tr -d ' ')
    last=$(ls "$c"src/b*.rs 2>/dev/null | tail -1 | xargs -I{} basename {} .rs)
    printf "  %-5s %3d/100   last batch %s\n" "$(basename "$c")" "$n" "${last:-none}"
done
echo
echo "next batch (one query per unseen shape):"
./tools/next.py 12 --new 2>&1 | sed 's/^/  /'
echo
echo "git: $(git -C .. rev-parse --abbrev-ref HEAD), $(git -C .. status --porcelain | wc -l | tr -d ' ') paths dirty"
