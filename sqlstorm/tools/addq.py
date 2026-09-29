#!/usr/bin/env python3
"""Append port functions to a batch file and register them in its ENTRIES.

    python3 tools/addq.py rust/cNN/src/bNNN.rs new.rs

new.rs holds one or more `fn qNNNN(db: &'static So) -> String` with their
SQL comments. Prints the ids added, comma-separated, ready for
`SQLSTORM_ONLY=<ids> cargo run -q --release -p cNN`.
"""

import re
import sys

batch, code = sys.argv[1], sys.argv[2]
s = open(batch).read()
add = open(code).read()
ids = re.findall(r"^fn q(\d+)\(", add, re.M)
i = s.index("\npub static ENTRIES")
s = s[:i] + "\n" + add.rstrip() + "\n" + s[i:]
j = s.rindex("];")
s = s[:j] + "".join(f'    ("{q}", q{q}),\n' for q in ids) + s[j:]
open(batch, "w").write(s)
print(",".join(ids))
