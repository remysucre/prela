### Running Storage Engine on IMDB dataset 

To run:
`PRELA_ONLY=1a,2a,3a,5c,6a ROUNDS=2 cargo run --release --features job_loaded -- job_loaded`

This requires the imdb dataset to be converted into parquet format.

The queries_loaded.rs file is an analagous file to the queries.rs file which runs the same queries but with slight changes in syntax to run .load() on the queries first and then .drive() them. Also, since some nodes have not yet been implemented, like Diff, those queries are marked as "pending" and not ran. 

I ran several queries and compared the reported runtimes. Note that the reported times are not one-to-one comparisons: in `queries_loaded.rs`, the data is loaded from scratch each time, while in the original `queries.rs`, the data is loaded fully just once and then ran in-memory. To get a fair comparison, you should compare the load time + per-query time of `queries.rs` to the reported query time of `queries_loaded.rs`. 

### Select rule: LHS first

#### In-memory full load once
For most queries, the full execution time for the storage engine is approximately the same as the loading time. The exception is query 6a which takes significantly longer:

```let q6a = move || {db.movie.with(production_year.gt(2010)).select(keyword.eq("marvel-cinematic-universe").and(title).and(cast.select(person).select(person_name).rx(r"Downey.*Robert")))};```

The issue is that the outermost select will first load its LHS which is almost the whole of movie -- restricted only by the year predicate. This will then inform the loading of the RHS which will get pushed down into reading keyword, where the predicate will get applied. But by this point, we've already loaded all of movie, so we've done a much load than if we'd first evaluated the RHS of select so we could benefit loading movies with the keyword's selectivity as well as the selectivity from the year predicate.

But in addition, it's much slower because we are reading the same id column multiple times -- we will need to modify the general load method and split it into a RHS and LHS load which only reads one column at a time.


```load: 2.82s  (movie n=2528312, person n=4167491)
114 queries registered
--- run 1 ---
1a    ok    0.2836s  (A Warner Bros.-First National Picture) (presents) || A Clockwork Orange || 1934
2a    ok    0.5959s  'Doc'
3a    ok    0.9983s  2 Days in New York
5c    ok    0.1538s  11,830,420
6a    ok    0.1515s  marvel-cinematic-universe || Iron Man 3 || Downey Jr., Robert
run 1: 5/114 ok  total 2.18s
--- run 2 ---
1a    ok    0.0238s  (A Warner Bros.-First National Picture) (presents) || A Clockwork Orange || 1934
2a    ok    0.0242s  'Doc'
3a    ok    0.1216s  2 Days in New York
5c    ok    0.0261s  11,830,420
6a    ok    0.0130s  marvel-cinematic-universe || Iron Man 3 || Downey Jr., Robert
run 2: 5/114 ok  total 0.21s


114 queries registered (loaded)
--- run 1 ---
  scan movie_info_idx.parquet     movie_id -> id            kept 1380035 of 1380035 rows
  scan movie_info_idx.parquet           id -> info_type_id  kept 1380035 of 1380035 rows
  scan info_type.parquet           id -> info          kept    1 of  113 rows
  scan title.parquet               id -> id            kept 2528312 of 2528312 rows
  scan movie_companies.parquet           id -> company_type_id kept 2609129 of 2609129 rows
  scan company_type.parquet           id -> kind          kept    1 of    4 rows
  scan movie_companies.parquet     movie_id -> id            kept 2609129 of 2609129 rows
  scan movie_companies.parquet           id -> note          kept 28889 of 2609129 rows
  scan title.parquet               id -> title         kept 2528312 of 2528312 rows
  scan title.parquet               id -> production_year kept 2528312 of 2528312 rows
1a    ok    2.4320s  (A Warner Bros.-First National Picture) (presents) || A Clockwork Orange || 1934
  scan movie_keyword.parquet     movie_id -> keyword_id    kept 4523930 of 4523930 rows
  scan keyword.parquet             id -> keyword       kept    1 of 134170 rows
  scan movie_companies.parquet     movie_id -> id            kept 2609129 of 2609129 rows
  scan movie_companies.parquet           id -> company_id    kept 2609129 of 2609129 rows
  scan company_name.parquet           id -> country_code  kept 9775 of 234997 rows
  scan title.parquet               id -> id            kept 2528312 of 2528312 rows
  scan title.parquet               id -> title         kept 2528312 of 2528312 rows
2a    ok    2.5923s  'Doc'
  scan movie_keyword.parquet     movie_id -> keyword_id    kept 4523930 of 4523930 rows
  scan keyword.parquet             id -> keyword       kept   30 of 134170 rows
  scan movie_info.parquet     movie_id -> id            kept 14835720 of 14835720 rows
  scan movie_info.parquet           id -> info          kept 153897 of 14835720 rows
  scan title.parquet               id -> production_year kept 1012920 of 2528312 rows
  scan title.parquet               id -> id            kept 1015128 of 2528312 rows
  scan title.parquet               id -> title         kept 1015128 of 2528312 rows
3a    ok    5.9131s  2 Days in New York
  scan movie_companies.parquet           id -> company_type_id kept 2609129 of 2609129 rows
  scan company_type.parquet           id -> kind          kept    1 of    4 rows
  scan movie_companies.parquet           id -> note          kept 295628 of 2609129 rows
  scan movie_companies.parquet     movie_id -> id            kept 295726 of 2609129 rows
  scan movie_info.parquet     movie_id -> id            kept 4438190 of 14835720 rows
  scan movie_info.parquet           id -> info          kept 174797 of 14835720 rows
  scan title.parquet               id -> production_year kept 83350 of 2528312 rows
  scan title.parquet               id -> id            kept 83350 of 2528312 rows
  scan title.parquet               id -> title         kept 83355 of 2528312 rows
5c    ok    5.6804s  11,830,420
  scan title.parquet               id -> production_year kept 391666 of 2528312 rows
  scan title.parquet               id -> id            kept 391875 of 2528312 rows
  scan movie_keyword.parquet     movie_id -> keyword_id    kept 299726 of 4523930 rows
  scan keyword.parquet             id -> keyword       kept    1 of 134170 rows
  scan title.parquet               id -> title         kept 36243 of 2528312 rows
  scan cast_info.parquet     movie_id -> id            kept 970856 of 36244344 rows
  scan cast_info.parquet           id -> person_id     kept 970871 of 36244344 rows
  scan name.parquet                id -> name          kept    2 of 4167491 rows
6a    ok   12.8514s  marvel-cinematic-universe || Iron Man 3 || Downey Jr., Robert




--- run 2 ---
  scan movie_info_idx.parquet     movie_id -> id            kept 1380035 of 1380035 rows
  scan movie_info_idx.parquet           id -> info_type_id  kept 1380035 of 1380035 rows
  scan info_type.parquet           id -> info          kept    1 of  113 rows
  scan title.parquet               id -> id            kept 2528312 of 2528312 rows
  scan movie_companies.parquet           id -> company_type_id kept 2609129 of 2609129 rows
  scan company_type.parquet           id -> kind          kept    1 of    4 rows
  scan movie_companies.parquet     movie_id -> id            kept 2609129 of 2609129 rows
  scan movie_companies.parquet           id -> note          kept 28889 of 2609129 rows
  scan title.parquet               id -> title         kept 2528312 of 2528312 rows
  scan title.parquet               id -> production_year kept 2528312 of 2528312 rows
1a    ok    1.8903s  (A Warner Bros.-First National Picture) (presents) || A Clockwork Orange || 1934
  scan movie_keyword.parquet     movie_id -> keyword_id    kept 4523930 of 4523930 rows
  scan keyword.parquet             id -> keyword       kept    1 of 134170 rows
  scan movie_companies.parquet     movie_id -> id            kept 2609129 of 2609129 rows
  scan movie_companies.parquet           id -> company_id    kept 2609129 of 2609129 rows
  scan company_name.parquet           id -> country_code  kept 9775 of 234997 rows
  scan title.parquet               id -> id            kept 2528312 of 2528312 rows
  scan title.parquet               id -> title         kept 2528312 of 2528312 rows
2a    ok    1.7793s  'Doc'
  scan movie_keyword.parquet     movie_id -> keyword_id    kept 4523930 of 4523930 rows
  scan keyword.parquet             id -> keyword       kept   30 of 134170 rows
  scan movie_info.parquet     movie_id -> id            kept 14835720 of 14835720 rows
  scan movie_info.parquet           id -> info          kept 153897 of 14835720 rows
  scan title.parquet               id -> production_year kept 1012920 of 2528312 rows
  scan title.parquet               id -> id            kept 1015128 of 2528312 rows
  scan title.parquet               id -> title         kept 1015128 of 2528312 rows
3a    ok    3.9897s  2 Days in New York
  scan movie_companies.parquet           id -> company_type_id kept 2609129 of 2609129 rows
  scan company_type.parquet           id -> kind          kept    1 of    4 rows
  scan movie_companies.parquet           id -> note          kept 295628 of 2609129 rows
  scan movie_companies.parquet     movie_id -> id            kept 295726 of 2609129 rows
  scan movie_info.parquet     movie_id -> id            kept 4438190 of 14835720 rows
  scan movie_info.parquet           id -> info          kept 174797 of 14835720 rows
  scan title.parquet               id -> production_year kept 83350 of 2528312 rows
  scan title.parquet               id -> id            kept 83350 of 2528312 rows
  scan title.parquet               id -> title         kept 83355 of 2528312 rows
5c    ok    3.7033s  11,830,420
  scan title.parquet               id -> production_year kept 391666 of 2528312 rows
  scan title.parquet               id -> id            kept 391875 of 2528312 rows
  scan movie_keyword.parquet     movie_id -> keyword_id    kept 299726 of 4523930 rows
  scan keyword.parquet             id -> keyword       kept    1 of 134170 rows
  scan title.parquet               id -> title         kept 36243 of 2528312 rows
  scan cast_info.parquet     movie_id -> id            kept 970856 of 36244344 rows
  scan cast_info.parquet           id -> person_id     kept 970871 of 36244344 rows
  scan name.parquet                id -> name          kept    2 of 4167491 rows
6a    ok   10.4544s  marvel-cinematic-universe || Iron Man 3 || Downey Jr., Robert
run 2: 5/114 ok  total 21.82s```


### Changing select to evaluate RHS first 
After changing, I reran query 6a to check whether there was an improvement, which there very much was:

```schema: 0.01s
114 queries registered (loaded)
--- run 1 ---
  scan keyword.parquet             id -> keyword       kept    1 of 134170 rows
  scan movie_keyword.parquet     movie_id -> keyword_id    kept   14 of 4523930 rows
  scan title.parquet               id -> title         kept   14 of 2528312 rows
  scan name.parquet                id -> name          kept    2 of 4167491 rows
  scan cast_info.parquet           id -> person_id     kept  486 of 36244344 rows
  scan cast_info.parquet     movie_id -> id            kept    6 of 36244344 rows
  scan title.parquet               id -> production_year kept 391666 of 2528312 rows
  scan title.parquet               id -> id            kept    4 of 2528312 rows
6a    ok    2.7591s  marvel-cinematic-universe || Iron Man 3 || Downey Jr., Robert
run 1: 1/114 ok  total 2.76s
--- run 2 ---
  scan keyword.parquet             id -> keyword       kept    1 of 134170 rows
  scan movie_keyword.parquet     movie_id -> keyword_id    kept   14 of 4523930 rows
  scan title.parquet               id -> title         kept   14 of 2528312 rows
  scan name.parquet                id -> name          kept    2 of 4167491 rows
  scan cast_info.parquet           id -> person_id     kept  486 of 36244344 rows
  scan cast_info.parquet     movie_id -> id            kept    6 of 36244344 rows
  scan title.parquet               id -> production_year kept 391666 of 2528312 rows
  scan title.parquet               id -> id            kept    4 of 2528312 rows
6a    ok    2.3210s  marvel-cinematic-universe || Iron Man 3 || Downey Jr., Robert
run 2: 1/114 ok  total 2.32s


To compare to the single full load, we see that this is much closer to the full-load time:

load: 2.59s  (movie n=2528312, person n=4167491)
114 queries registered
--- run 1 ---
6a    ok    0.4316s  marvel-cinematic-universe || Iron Man 3 || Downey Jr., Robert
run 1: 1/114 ok  total 0.43s
--- run 2 ---
6a    ok    0.0094s  marvel-cinematic-universe || Iron Man 3 || Downey Jr., Robert
run 2: 1/114 ok  total 0.01s```