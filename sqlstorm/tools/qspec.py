"""Print the skeleton of a SQLStorm query, one line per clause.

    python3 tools/qspec.py 18895 13617 ...

Aliases are folded onto the table they name (P, U, C, V, B, PH, PT), so two
queries that are the same shape under different aliases print the same
skeleton. That is what says which of them can share one implementation and
which have to be written out. Reading only -- it emits no code.

It parses one flat SELECT. A query written with CTEs comes back garbled; read
those by eye (they are all a projection over a single GROUP BY).
"""

import re, sys
from pathlib import Path
D = Path('/Users/paultalma/projects/sqlstorm_data/corpus/queries')
SHORT = {'id':'id','title':'title','creationdate':'created','score':'score','viewcount':'views',
         'displayname':'owner','body':'body','answercount':'answers','commentcount':'comments',
         'reputation':'rep','posttypeid':'type_id','tags':'tags','lastactivitydate':'activity',
         'acceptedanswerid':'accepted','favoritecount':'favorites','name':'ptype'}
for q in sys.argv[1:]:
    s = re.sub(r'\s+',' ', D.joinpath(q+'.sql').read_text()).strip().rstrip(';')
    s2 = re.sub(r'\bINNER JOIN\b','JOIN',s,flags=re.I)
    for tbl,tag in (('Posts','P'),('Users','U'),('PostTypes','PT'),('Comments','C'),('Votes','V'),('Badges','B'),('PostHistory','PH')):
        for m in re.finditer(rf'\b(?:FROM|JOIN)\s+{tbl}\s+(?:AS\s+)?(\w+)', s2, re.I):
            a=m.group(1)
            if a.upper() in ('ON','WHERE','GROUP','ORDER','LEFT','JOIN','LIMIT','FETCH'): continue
            s2 = re.sub(rf'\b{a}\.', tag+'.', s2)
        s2 = re.sub(rf'\b{tbl}\b(?!\.)', tag, s2, flags=re.I)
    def gr(pat, nxt):
        m = re.search(pat + r'(.*?)(?:' + nxt + r'|$)', s2, re.I)
        return re.sub(r'\s+',' ',m.group(1)).strip() if m else ''
    NXT = r'\bFROM\b|\bWHERE\b|\bGROUP BY\b|\bORDER BY\b|\bHAVING\b|\bLIMIT\b|\bFETCH\b'
    sel = gr(r'^SELECT\b', r'\bFROM\b')
    frm = gr(r'\bFROM\b', NXT)
    whr = gr(r'\bWHERE\b', r'\bGROUP BY\b|\bORDER BY\b|\bLIMIT\b|\bFETCH\b')
    grp = gr(r'\bGROUP BY\b', r'\bORDER BY\b|\bHAVING\b|\bLIMIT\b|\bFETCH\b')
    orr = gr(r'\bORDER BY\b', r'\bLIMIT\b|\bFETCH\b')
    lim = re.search(r'\bLIMIT\s+(\d+)|\bFETCH FIRST (\d+)', s2, re.I)
    def items(x):
        out,depth,cur=[],0,''
        for ch in x:
            if ch=='(':depth+=1
            elif ch==')':depth-=1
            if ch==',' and depth==0: out.append(cur);cur=''
            else: cur+=ch
        if cur.strip(): out.append(cur)
        return [y.strip() for y in out]
    def short(it):
        it = re.sub(r'\s+AS\s+\w+$','',it,flags=re.I).strip()
        m = re.match(r'^(?:P|U|PT)\.(\w+)$', it)
        if m: return SHORT.get(m.group(1).lower(), m.group(1))
        it = re.sub(r'\s+',' ',it)
        return {'COUNT(C.Id)':'#c','COUNT(DISTINCT C.Id)':'#dc','COUNT(V.Id)':'#v',
                'COUNT(DISTINCT V.Id)':'#dv','COUNT(P.Id)':'#p','COUNT(DISTINCT P.Id)':'#dp',
                'SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END)':'#up',
                'SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END)':'#down'}.get(it, it)
    print(f"{q}| G[{','.join(short(x) for x in items(grp))}]"
          f" S[{','.join(short(x) for x in items(sel))}]"
          f" O[{orr}] L[{(lim.group(1) or lim.group(2)) if lim else ''}]"
          f" W[{whr}] F[{frm}]")
