"""Histogram the FROM/WHERE/GROUP/ORDER skeleton of a set of queries."""
import re, sys
from collections import Counter
from pathlib import Path
D = Path('/Users/paultalma/projects/sqlstorm_data/corpus/queries')
def canon(s):
    s = re.sub(r'\s+', ' ', s).strip().rstrip(';')
    s = re.sub(r'\bINNER JOIN\b', 'JOIN', s, flags=re.I)
    s = re.sub(r'\b(?:pt|PT|PostTypes)\.', 'PT.', s)
    s = re.sub(r'\b(?:p|Posts)\.', 'P.', s)
    s = re.sub(r'\b(?:u|Users)\.', 'U.', s)
    s = re.sub(r'\b(?:c|Comments)\.', 'C.', s)
    s = re.sub(r'\b(?:v|Votes)\.', 'V.', s)
    s = re.sub(r'\b(?:t|Tags)\.', 'T.', s)
    s = re.sub(r'\b(?:b|Badges)\.', 'B.', s)
    s = re.sub(r'\s+AS\s+\w+', '', s, flags=re.I)
    return s
c = Counter(); ex = {}
for q in sys.argv[1:]:
    s = canon(D.joinpath(q + '.sql').read_text())
    m = re.search(r'\bFROM\b(.*)$', s, re.I)
    tail = m.group(1).strip() if m else '???'
    tail = re.sub(r"'[^']*'", "'?'", tail)
    tail = re.sub(r'\b\d+\b', '#', tail)
    c[tail] += 1
    ex.setdefault(tail, q)
for tail, n in c.most_common(22):
    print(f"{n:4d}  [{ex[tail]}]  {tail[:150]}")
print("distinct skeletons:", len(c))
