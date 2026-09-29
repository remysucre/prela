#!/usr/bin/env python3
"""Add a query DuckDB refuses to run to the invalid table of notes/blocked.md.

    python3 tools/block.py <id> "BinderException: ..."
"""

import sys
from pathlib import Path

p = Path(__file__).parent.parent / "notes" / "blocked.md"
s = p.read_text()
qid, why = sys.argv[1], sys.argv[2]
i = s.index("## Invalid queries")
i = s.index("| ----- |", i)
i = s.index("\n", i) + 1
while s[i:].startswith("| "):
    i = s.index("\n", i) + 1
s = s[:i] + f"| {qid} | {why} |\n" + s[i:]
p.write_text(s)
