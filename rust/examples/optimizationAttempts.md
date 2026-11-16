So far, I have added an additional select_left/select_right distinction (but not a keyload one)

I have also for now changed so that everything that is passed around is an exact bitset rather than a Bloom filter. 


My attempts now are to figure out whether we can get a more reasonable runtime with this more finetuned/compositional control which the query loading plan gives us.

Here is documenting my approaches on two case studies. 


Query 6c:

`let q6c = move || {
        db.movie.with(production_year.gt(2014))
                .select(keyword.eq("marvel-cinematic-universe")
                        .and(title)
                        .and(cast.select(person).select(person_name).rx(r"Downey.*Robert")))
    };`

What we know about it:
- the regex is very selective
- the production year predicate is very selective 

6c_myplan:
1. Given that the production year is selective, we should first load movies filtered on it before we load the keyword -- hence the first select should be select_left 
2. It's expensive to evaluate a regex as we load on every piece in the file so we can leave that for in-memory execution 
3. becomes there's no predicates at the end I keep everything as a select left

Comparison between my plan and the original plan obtained directly from the q6c job query structure -- this shows improvement over finer control:

--- run 2 ---
6c    load   1.2341s  exec   0.0055s
6c    ok    1.2396s  marvel-cinematic-universe || The Avengers 2 || Downey Jr., Robert
6c_myplan load   0.8980s  exec   0.0006s
6c_myplan ok    0.8986s  marvel-cinematic-universe || The Avengers 2 || Downey Jr., Robert

Without any loading plan (loading full dataset)
load: 3.89s  (movie n=2528312, person n=4167491)
114 queries registered
--- run 1 ---
6c    ok    0.1717s  marvel-cinematic-universe || The Avengers 2 || Downey Jr., Robert
run 1: 1/114 ok  total 0.17s
--- run 2 ---
6c    ok    0.0042s  marvel-cinematic-universe || The Avengers 2 || Downey Jr., Robert
run 2: 1/114 ok  total 0.00s


Goal: reason through all the queries I previously ran to get a sense between the optimised and not ones 1a through 5a 


1a
`let q1a = move || {
        db.movie.with(data.select(data_ty).eq("top 250 rank"))
                .select_left(company.with(company_ty.eq("production companies"))
                               .select(company_note.nrx(r"\(as Metro-Goldwyn-Mayer Pictures\)")
                                                   .rx(r"\(co-production\)|\(presents\)"))
                        .and(title)
                        .and(production_year))
    }`

What we know:
- the top 250 rank predicate is very selective -> so we should use select_left 
- I omitted the regex checks from the loading plan and only kept them to be checked at memory time; I omitted the preds that are already checked during load time
- whenever thee's no "end predicates" I opt to use select_left

Comparing my good plan to the unoptimised query structure plan:

--- run 1 ---
1a    load   0.5246s  exec   0.0066s
1a    ok    0.5313s  (A Warner Bros.-First National Picture) (presents) || A Clockwork Orange || 1934
1a_mine load   0.2419s  exec   0.0006s
run 1: 2/122 ok  total 0.77s
--- run 2 ---
1a    load   0.3744s  exec   0.0064s
1a    ok    0.3808s  (A Warner Bros.-First National Picture) (presents) || A Clockwork Orange || 1934
1a_mine load   0.2561s  exec   0.0006s
1a_mine ok    0.2568s  (A Warner Bros.-First National Picture) (presents) || A Clockwork Orange || 1934
run 2: 2/122 ok  total 0.64s


Comparison with loading the whole dataset:

load: 3.39s  (movie n=2528312, person n=4167491)
114 queries registered
--- run 1 ---
1a    ok    0.3124s  (A Warner Bros.-First National Picture) (presents) || A Clockwork Orange || 1934
run 1: 1/114 ok  total 0.31s
--- run 2 ---
1a    ok    0.0086s  (A Warner Bros.-First National Picture) (presents) || A Clockwork Orange || 1934
run 2: 1/114 ok  total 0.01s
(base) kat@Kats-MacBook-Pro rust % 





2a
3a
4a
5a

5a    load   2.0981s  exec   0.0003s
5a    ok    2.0984s  (empty)
5a_mine load   1.4752s  exec   0.0017s
5a_mine ok    1.4770s  (empty)
run 1: 2/124 ok  total 3.58s
--- run 2 ---
5a    load   2.1769s  exec   0.0000s
5a    ok    2.1769s  (empty)
5a_mine load   1.4821s  exec   0.0019s
5a_mine ok    1.4840s  (empty)
run 2: 2/124 ok  total 3.66s


6a


9d -- this one is hard to optimize I guess -- need to do a manual hop/keys maybe

schema: 0.01s
123 queries registered (loaded, optimized)
--- run 1 ---
9d    load   8.8879s  exec   0.1632s
9d    ok    9.0514s  !!!, Toy || Aaron, Caroline || "Cockamamie's" Salesgirl || $15,000.00 Error
9d_mine load   4.0125s  exec   0.0393s
9d_mine ok    4.0519s  !!!, Toy || Aaron, Caroline || "Cockamamie's" Salesgirl || $15,000.00 Error
run 1: 2/123 ok  total 13.10s
--- run 2 ---
9d    load   9.2591s  exec   0.2225s
9d    ok    9.4817s  !!!, Toy || Aaron, Caroline || "Cockamamie's" Salesgirl || $15,000.00 Error
9d_mine load   3.9302s  exec   0.0480s
9d_mine ok    3.9783s  !!!, Toy || Aaron, Caroline || "Cockamamie's" Salesgirl || $15,000.00 Error
run 2: 2/123 ok  total 13.46s


