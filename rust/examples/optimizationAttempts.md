So far, I have added an additional select_left/select_right distinction (but not a keyload one), and and `and_right` method. I have also for now changed so that everything that is passed around is an exact bitset rather than a Bloom filter. 


My attempts now are to figure out whether we can get a more reasonable runtime with this more finetuned/compositional control which the query loading plan gives us.

Here is documenting my approaches on two case studies. 

**Note: Nominal Loading of full dataset takes 3.35s seconds.**



Query 6c:

```rust
let q6c = move || {
    db.movie.with(production_year.gt(2014))
        .select(keyword.eq("marvel-cinematic-universe")
            .and(title)
            .and(cast.select(person).select(person_name).rx(r"Downey.*Robert")))
};
```
#### Facts to take advantage of
- the regex is very selective but is expensive to apply -- I will apply it in-memory
- the production year predicate is also very selective and keeps very little movies - this indicates we should do select_left to load the reduced movie table before loading the keyword column and reducing it
- since there's no end predicates on the `and`s then the LHS should be loaded first
- therefore in my in-memory plan, I don't need to apply any of the predicates I've already checked (because I am using an exact bitset everywhere including the Eq), so only the regex check is needed


```rust
fn plan_6c_myplan(db: &'static JobFiles) {
    use KeyFilter::All;
    let (m, c, p) = (&db.movie, &db.cast, &db.person);
    m.with(m.production_year.gt(2014))
    .select_left((&m.keyword).eq("marvel-cinematic-universe")
    .and(m.title)
    .and(m.cast.select_left(c.person).select_left(p.name)))
    .load(All,All,Vec::new());
    
}

fn run_6c_myplan(mem: &'static MemJob) -> String {
    let MemMovie { title, production_year, keyword, cast, .. } = &mem.movie;
    let MemCast { person, .. } = &mem.cast;
    let MemPerson { name: person_name, .. } = &mem.person;
    min_row(mem.movie.select(keyword).and(title).and(cast.select(person).select(person_name).rx(r"Downey.*Robert")))

}
```
#### Nominal query plan VS fine-tuned plan

--- run 2 ---
6c    load   1.2341s  exec   0.0055s
6c    ok    1.2396s  marvel-cinematic-universe || The Avengers 2 || Downey Jr., Robert
6c_myplan load   0.8980s  exec   0.0006s
6c_myplan ok    0.8986s  marvel-cinematic-universe || The Avengers 2 || Downey Jr., Robert

**Loading time improvement: x1.37**
**In-memory improvement: x9.17**


#### Without any loading plan (loading full dataset)
load: 3.89s  (movie n=2528312, person n=4167491)
114 queries registered
--- run 1 ---
6c    ok    0.1717s  marvel-cinematic-universe || The Avengers 2 || Downey Jr., Robert
run 1: 1/114 ok  total 0.17s
--- run 2 ---
6c    ok    0.0042s  marvel-cinematic-universe || The Avengers 2 || Downey Jr., Robert
run 2: 1/114 ok  total 0.00s

**Loading time improvement: x4.3**
**In-memory improvement: x7**



### Query 1a

```rust
let q1a = move || {
        db.movie.with(data.select(data_ty).eq("top 250 rank"))
                .select_left(company.with(company_ty.eq("production companies"))
                               .select(company_note.nrx(r"\(as Metro-Goldwyn-Mayer Pictures\)")
                                                   .rx(r"\(co-production\)|\(presents\)"))
                        .and(title)
                        .and(production_year))
    }
```

#### Facts to Exploit:
- the top 250 rank predicate is very selective -> so we should use select_left 
- I omitted the regex checks from the loading plan and only kept them to be checked at memory time; I omitted the preds that are already checked during load time
- whenever there's no "end predicates" I opt to use select_left

```rust
fn myplan_1a(db: &'static JobFiles) {
    use KeyFilter::All;
    let (m, d, co) = (&db.movie, &db.data, &db.company);
    m.with(m.data.select(&(d.ty)).eq("top 250 rank"))
    .select_left(m.company.with((&co.ty).eq("production companies"))
    .select_left(co.note))
    .and(m.title)
    .and(m.production_year)
    .load(All,All, Vec::new());

}

fn run_myplan_1a(mem: &'static MemJob) -> String {
    let MemMovie { title, production_year, company, .. } = &mem.movie;
    let MemCompany { note: company_note, .. } = &mem.company;

    min_row(mem.movie.select(company.select(company_note.nrx(r"\(as Metro-Goldwyn-Mayer Pictures\)")
                                                   .rx(r"\(co-production\)|\(presents\)"))
                        .and(title)
                        .and(production_year)))

}
```

### Nominal Plan vs Specialised Plan
--- run 2 ---
1a    load   0.3744s  exec   0.0064s
1a    ok    0.3808s  (A Warner Bros.-First National Picture) (presents) || A Clockwork Orange || 1934
1a_mine load   0.2561s  exec   0.0006s
1a_mine ok    0.2568s  (A Warner Bros.-First National Picture) (presents) || A Clockwork Orange || 1934
run 2: 2/122 ok  total 0.64s

**Loading time: x1.46**
**Exec time: x10.67**


## Loading Whole dataset

load: 3.39s  (movie n=2528312, person n=4167491)
114 queries registered
--- run 1 ---
1a    ok    0.3124s  (A Warner Bros.-First National Picture) (presents) || A Clockwork Orange || 1934
run 1: 1/114 ok  total 0.31s
--- run 2 ---
1a    ok    0.0086s  (A Warner Bros.-First National Picture) (presents) || A Clockwork Orange || 1934
run 2: 1/114 ok  total 0.01s
(base) kat@Kats-MacBook-Pro rust % 

**Loading time: x13.23**
**Exec time: x14.33**





## Query 9d

```rust
let q9d = move || {
        db.movie.with(company.select(country).eq("[us]"))
                .select(cast.with(cast_note.is_in(voice4())
                                  .and(role.eq("actress")))
                            .select(person.with(gender.eq("f")).select(alias.and(person_name))
                                    .and(character))
                        .and(title))
    }
```
#### Facts to Exploit:
- the US country requirement is not very selective (20%)
- the cast_note requirement is very selective (2.4%) 
- the role requirement is not very selective (20%)
- the overall with clause is very selective 
- gender= f is not selective (99%) -> not worth applying to full scan (?)
- had to slightly modify the order of the query to make sure everything still gets loaded even if we don't apply the predicates

```rust
fn myplan_9d(db: &'static JobFiles) {
    use KeyFilter::All;
    let (m, c, p, co) = (&db.movie, &db.cast, &db.person, &db.company);
    m.with(m.company.select(&co.country).eq("[us]"))                          
     .select_left(m.cast.with(c.note.is_in(voice4()).and((&c.role).eq("actress")))  
                     .select_left(c.person.select_left(p.alias.and(p.name).and(p.gender)) 
                                  .and(&c.character))                    
                  .and(m.title))                                            
     .load(All, All, Vec::new());
}

fn run_myplan_9d(mem: &'static MemJob) -> String {
    let MemMovie { title, cast, .. } = &mem.movie;
    let MemCast { person, character, .. } = &mem.cast;
    let MemPerson { name: person_name, alias, gender, .. } = &mem.person;
    min_row(mem.movie.select(cast.select(person.with(gender.eq("f")).select(alias.and(person_name))
                                         .and(character))
                             .and(title)))
}
```

#### Nominal vs Specialised Comparison
--- run 2 ---
9d    load   9.2591s  exec   0.2225s
9d    ok    9.4817s  !!!, Toy || Aaron, Caroline || "Cockamamie's" Salesgirl || $15,000.00 Error
9d_mine load   3.9302s  exec   0.0480s
9d_mine ok    3.9783s  !!!, Toy || Aaron, Caroline || "Cockamamie's" Salesgirl || $15,000.00 Error
run 2: 2/123 ok  total 13.46s

***Load improvement: x2.36**
**Exec time: x4.63**

**after changing the order of filtering**
--- run 2 ---
9d    load   8.9748s  exec   0.1403s
9d    ok    9.1152s  !!!, Toy || Aaron, Caroline || "Cockamamie's" Salesgirl || $15,000.00 Error
9d_mine load   3.1452s  exec   0.0211s
9d_mine ok    3.1663s  !!!, Toy || Aaron, Caroline || "Cockamamie's" Salesgirl || $15,000.00 Error
run 2: 2/124 ok  total 12.28s


### Comparison with full dataset loaded 
load: 3.88s  (movie n=2528312, person n=4167491)
114 queries registered
--- run 1 ---
9d    ok    7.7958s  !!!, Toy || Aaron, Caroline || "Cockamamie's" Salesgirl || $15,000.00 Error
run 1: 1/114 ok  total 7.80s
--- run 2 ---
9d    ok    0.6637s  !!!, Toy || Aaron, Caroline || "Cockamamie's" Salesgirl || $15,000.00 Error
run 2: 1/114 ok  total 0.66s

**Load time: x0.98**
**Exec time: 13.83**

## Takeaways
- this method fundamentaly has to re-read a lot of the same columns (the id column): on one hand, it makes it extra selective for the value we read in and we still have to read every entry (we're just reading double the columns). Is there a way to load all IDs once?
- There are additional Parquet optimisations we are not yet utilising -- the dataset doesn't have an index; row-group pruning, optimising which filters are evaluated first (because the order in which we give the filters to Parquet matters)
- Does this method have to re-read columns it's already read? 




### Other data
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



--------
The physical ID columns should never be loaded. We need to tell Parquet to just filter on the row_ids of the value column and then we construct MultiRels on those saved row_ids as well. 
Furthermore, the only delicacy is when we load an ID column cos then it goes the otehr way, we read in the values (physical IDs and the row IDs) BUT we should just construct it in the opposite way -- so maybe add a specific COlumnRef inheritance or smth where we can check if it's of Type ID ColumnRef -- maybe define a new struct and have it like that -- then we don't need to prebuild it we actually just load it regularly and it'll work!. 
-> So look into how the Row IDs are fetched from Parquet and so on. 



1. Define new ColumnRef structure as PrimaryColRef ForeignColRef and define loading instructions 
2. Define a new version of scan which will take in a filter on just VALUES and then also the KeyFilter and construct it 




Now that I have implemented my new design, your task is to optimise the writing of queries q1-q16c (including a,b,c, so on) in the queries_loaded_optimized.rs file. 

Code structure:
The structure should have a nice readable separation between every queries loading plan and in-memory query. This is because, as you have seen, with this optimisation, we can optimise for choosing which parts of the original query can be checked at loading versus just in-memory. Thus, it is best to copy the style of "entries", twice in the sense that we will have a list of query loading plans, and then the in-memory plans. The same oracle should be used to run the diffing though to ensure consistency. You should not overengineer the code and try to keep it as readable and minimal as possible with just the bare .to_memory helper or something that will convert between query loading plan and execution plan. You may only modify the queries_loaded_optimized.rs file, because the API/functionality externaly should behave identically. You should certainly keep the same timings for loading and exec as to be consistent with the queries_loaded.rs file. 


How to optimise:
First, you should do a general exploratory data analysis of the IMDB dtaset in the data file, and write yourself a "cheat sheet" about general selectivity/ stats about the files, which you will refer to to inform your choice about query rewriting. 

Next, as you consider writing each query's loading and execution plan, consider the following:
- is it more beneficial to use select_left/right or and_left/right based on the selectivity of the reduction? 
- should I keep this predicate checked during loading or apply in-memory?
- if I apply the .with(), then I can remove this check in the in-memory execution because of exactness
- any other principles you might think of to help you

What you should not do:
You should not need to define any intermediate queries. I would still like all the queries to be single trees, rather than 
q1 = (something).load()
q2 = q1.(something...).load() 
etc. 

The goal of this exercise is first for you to pretend to be a user with a decent knowledge of dataset who wants to have fast per-query runtime (load+execution).

After you have rewritten, I will run the script (on the queries that are optimised) and will compare the reported performance to the nominal plans in queries_loaded.rs.




# performance results 

# Hand-Optimised
Q=1a,1b,1c,1d,2a,2b,2c,2d,3a,3b,3c,4a,4b,4c,5a,5b,5c,6a,6b,6c,6d,6e,6f,7a,7b,7c,8a,8b,8c,8d,9a,9b,9c,9d,10a,10b,10c,11a,11b,11c,11d,12a,12b,12c,13a,13b,13c,13d,14a,14b,14c,15a,15b,15c,15d,16a,16b,16c
PRELA_ONLY=$Q ROUNDS=2 cargo run --release --features job_loaded_optim -- job_loaded_optim
schema: 0.01s
114 queries registered (loaded, optimized)
--- run 1 ---
1a    load   0.2094s  exec   0.0004s
1b    load   0.1175s  exec   0.0001s
1c    load   0.1296s  exec   0.0002s
1d    load   0.1296s  exec   0.0001s
2a    load   0.2613s  exec   0.0006s
2b    load   0.2128s  exec   0.0004s
2c    load   0.0830s  exec   0.0000s
2d    load   0.3647s  exec   0.0009s
3a    load   1.4935s  exec   0.0026s
3a    ok    1.4962s  2 Days in New York
3b    load   1.2061s  exec   0.0013s
3b    ok    1.2073s  300: Rise of an Empire
3c    load   1.3836s  exec   0.0040s
3c    ok    1.3877s  & Teller 2
4a    load   0.4093s  exec   0.0010s
4b    load   0.2118s  exec   0.0005s
4c    load   0.2389s  exec   0.0016s
5a    load   0.1516s  exec   0.0000s
5b    load   0.1523s  exec   0.0000s
5c    load   1.2319s  exec   0.0043s
5c    ok    1.2363s  11,830,420
6a    load   1.6995s  exec   0.0011s
6a    ok    1.7006s  marvel-cinematic-universe || Iron Man 3 || Downey Jr., Robert
6b    load   1.1121s  exec   0.0006s
6b    ok    1.1128s  based-on-comic || The Avengers 2 || Downey Jr., Robert
6c    load   0.7896s  exec   0.0003s
6c    ok    0.7899s  marvel-cinematic-universe || The Avengers 2 || Downey Jr., Robert
6d    load   0.8718s  exec   0.0001s
6d    ok    0.8719s  based-on-comic || 2008 MTV Movie Awards || Downey Jr., Robert
6e    load   1.4137s  exec   0.0012s
6e    ok    1.4150s  marvel-cinematic-universe || Iron Man 3 || Downey Jr., Robert
6f    load   1.7916s  exec   0.1257s
6f    ok    1.9174s  based-on-comic || & Teller 2 || "Steff", Stefanie Oxmann Mcgaha
7a    load   1.4804s  exec   0.0214s
7a    ok    1.5018s  Antonioni, Michelangelo || Dressed to Kill
7b    load   1.4652s  exec   0.0102s
7b    ok    1.4754s  De Palma, Brian || Dressed to Kill
7c    load   1.6692s  exec   0.0578s
7c    ok    1.7271s  50 Cent || "Boo" Arnold was born Earl Arnold in Hattiesburg, Mississippi in 1966. His father gave him the nickname 'Boo' early in life and it stuck through grade school, high school, and college. He is still known as "Boo" to family and friends.  Raised in central Texas, Arnold played baseball at Texas Tech University where he graduated with a BA in Advertising and Marketing. While at Texas Tech he was also a member of the Texas Epsilon chapter of Phi Delta Theta fraternity. After college he worked with Young Life, an outreach to high school students, in San Antonio, Texas.  While with Young Life Arnold began taking extension courses through Fuller Theological Seminary and ultimately went full-time to Gordon-Conwell Theological Seminary in Boston, Massachusetts. At Gordon-Conwell he completed a Master's Degree in Divinity studying Theology, Philosophy, Church History, Biblical Languages (Hebrew & Greek), and Exegetical Methods. Following seminary he was involved with reconciliation efforts in the former Yugoslavia shortly after the war ended there in1995.  Arnold started acting in his early thirties in Texas. After an encouraging visit to Los Angeles where he spent time with childhood friend George Eads (of CSI Las Vegas) he decided to move to Los Angeles in 2001 to pursue acting full-time. While in Los Angeles he has studied acting with Judith Weston at Judith Weston Studio for Actors and Directors.  Arnold's acting career has been one of steady development, booking co-star and guest-star roles in nighttime television. He guest-starred opposite of Jane Seymour on the night time television drama Justice. He played the lead, Michael Hollister, in the film The Seer, written and directed by Patrick Masset (Friday Night Lights).  He was nominated Best Actor in the168 Film Festival for the role of Phil Stevens in the short-film Useless. In Useless he played a US Marshal who must choose between mercy and justice as he confronts the man who murdered his father. Arnold's performance in Useless confirmed his ability to carry lead roles, and he continues to work toward solidifying himself as a male lead in film and television.  Arnold married fellow Texan Stacy Rudd of San Antonio in 2003 and they are now raising their three children in the Los Angeles area.
8a    load   0.9964s  exec   0.0020s
8a    ok    0.9984s  Chambers, Linda || .hack//Quantum
8b    load   1.2627s  exec   0.0014s
8b    ok    1.2642s  Chambers, Linda || Dragon Ball Z: Shin Budokai
8c    load   1.7099s  exec   0.0732s
8c    ok    1.7831s  "A.J." || #1 Cheerleader Camp
8d    load   1.3269s  exec   0.0079s
8d    ok    1.3349s  "Jenny from the Block" || #1 Cheerleader Camp
9a    load   1.8435s  exec   0.0254s
9a    ok    1.8690s  AJ || Airport Announcer || Blue Harvest
9b    load   1.6358s  exec   0.0233s
9b    ok    1.6591s  AJ || Bassett, Angela || Airport Announcer || Blue Harvest
9c    load   1.5301s  exec   0.0335s
9c    ok    1.5636s  'Annette' || Alborg, Ana Esther || 2nd Balladeer || (1975-01-20)
9d    load   1.5975s  exec   0.0437s
9d    ok    1.6414s  !!!, Toy || Aaron, Caroline || "Cockamamie's" Salesgirl || $15,000.00 Error
10a   load   2.3624s  exec   0.0684s
10a   ok    2.4308s  Actor || 12 Rounds
10b   load   0.9417s  exec   0.0076s
10b   ok    0.9493s  (empty)
10c   load   1.3570s  exec   0.0033s
10c   ok    1.3604s  Himself || Evil Eyes: Behind the Scenes
11a   load   0.1876s  exec   0.0000s
11b   load   0.1572s  exec   0.0000s
11c   load   0.1303s  exec   0.0000s
11d   load   0.1102s  exec   0.0001s
12a   load   0.6252s  exec   0.0021s
12a   ok    0.6273s  10th Grade Reunion Films || 8.1 || 3:20
12b   load   0.4611s  exec   0.0001s
12c   load   0.6244s  exec   0.0039s
12c   ok    0.6283s  "Oh That Gus!" || 7.1 || $1.11
13a   load   0.4867s  exec   0.0045s
13b   load   1.1657s  exec   0.0038s
13b   ok    1.1695s  5 Time Champion || 501audio || 1.8
13c   load   0.7416s  exec   0.0013s
13c   ok    0.7428s  Champion || DL Sites || 1.8
13d   load   0.9058s  exec   0.0327s
13d   ok    0.9386s  "O" Films || 1.0 || #54 Meets #47
14a   load   0.6895s  exec   0.0116s
14a   ok    0.7012s  1.0 || $lowdown
14b   load   0.6181s  exec   0.0006s
14b   ok    0.6188s  6.4 || Of Dolls and Murder
14c   load   0.7986s  exec   0.0298s
14c   ok    0.8284s  1.0 || $lowdown
15a   load   0.9997s  exec   0.0147s
15a   ok    1.0144s  USA:1 June 2007 || Battlestar Galactica: The Resistance
15b   load   0.6702s  exec   0.0027s
15b   ok    0.6729s  USA:27 April 2007 || RoboCop vs Terminator
15c   load   0.9892s  exec   0.0082s
15c   ok    0.9974s  USA:1 April 2003 || 24: Day Six - Debrief
15d   load   0.5560s  exec   0.0005s
15d   ok    0.5566s  (Not So) Instant Photo || 06/05
16a   load   2.0626s  exec   0.0036s
16a   ok    2.0663s  Adams, Stan || Carol Burnett vs. Anthony Perkins
16b   load   1.9876s  exec   0.1167s
16b   ok    2.1043s  !!!, Toy || & Teller
16c   load   1.5726s  exec   0.0210s
16c   ok    1.5936s  "Brooklyn" Tony Danza || (#1.5)
run 1: 58/114 ok  total 54.17s
--- run 2 ---
1a    load   0.1908s  exec   0.0010s
1a    ok    0.1918s  (A Warner Bros.-First National Picture) (presents) || A Clockwork Orange || 1934
1b    load   0.1781s  exec   0.0001s
1b    ok    0.1782s  (Set Decoration Rentals) (uncredited) || Disaster Movie || 2008
1c    load   0.1891s  exec   0.0002s
1c    ok    0.1894s  (co-production) || Intouchables || 2011
1d    load   0.1526s  exec   0.0001s
1d    ok    0.1527s  (Set Decoration Rentals) (uncredited) || Disaster Movie || 2004
2a    load   0.2103s  exec   0.0009s
2a    ok    0.2112s  'Doc'
2b    load   0.2042s  exec   0.0006s
2b    ok    0.2049s  'Doc'
2c    load   0.0733s  exec   0.0000s
2c    ok    0.0733s  (empty)
2d    load   0.2453s  exec   0.0009s
2d    ok    0.2463s  & Teller
3a    load   1.4088s  exec   0.0017s
3a    ok    1.4106s  2 Days in New York
3b    load   1.0713s  exec   0.0023s
3b    ok    1.0737s  300: Rise of an Empire
3c    load   1.3443s  exec   0.0066s
3c    ok    1.3509s  & Teller 2
4a    load   0.3310s  exec   0.0008s
4a    ok    0.3318s  5.1 || & Teller 2
4b    load   0.2229s  exec   0.0004s
4b    ok    0.2233s  9.1 || Batman: Arkham City
4c    load   0.2807s  exec   0.0016s
4c    ok    0.2823s  2.1 || & Teller 2
5a    load   0.2484s  exec   0.0000s
5a    ok    0.2484s  (empty)
5b    load   0.2081s  exec   0.0000s
5b    ok    0.2082s  (empty)
5c    load   1.2303s  exec   0.0028s
5c    ok    1.2332s  11,830,420
6a    load   1.5620s  exec   0.0013s
6a    ok    1.5633s  marvel-cinematic-universe || Iron Man 3 || Downey Jr., Robert
6b    load   1.2017s  exec   0.0008s
6b    ok    1.2025s  based-on-comic || The Avengers 2 || Downey Jr., Robert
6c    load   1.2499s  exec   0.0003s
6c    ok    1.2503s  marvel-cinematic-universe || The Avengers 2 || Downey Jr., Robert
6d    load   1.2911s  exec   0.0001s
6d    ok    1.2913s  based-on-comic || 2008 MTV Movie Awards || Downey Jr., Robert
6e    load   1.4993s  exec   0.0012s
6e    ok    1.5004s  marvel-cinematic-universe || Iron Man 3 || Downey Jr., Robert
6f    load   1.8874s  exec   0.1247s
6f    ok    2.0121s  based-on-comic || & Teller 2 || "Steff", Stefanie Oxmann Mcgaha
7a    load   1.3827s  exec   0.0839s
7a    ok    1.4667s  Antonioni, Michelangelo || Dressed to Kill
7b    load   1.7628s  exec   0.0922s
7b    ok    1.8550s  De Palma, Brian || Dressed to Kill
7c    load   2.6070s  exec   0.2138s
7c    ok    2.8208s  50 Cent || "Boo" Arnold was born Earl Arnold in Hattiesburg, Mississippi in 1966. His father gave him the nickname 'Boo' early in life and it stuck through grade school, high school, and college. He is still known as "Boo" to family and friends.  Raised in central Texas, Arnold played baseball at Texas Tech University where he graduated with a BA in Advertising and Marketing. While at Texas Tech he was also a member of the Texas Epsilon chapter of Phi Delta Theta fraternity. After college he worked with Young Life, an outreach to high school students, in San Antonio, Texas.  While with Young Life Arnold began taking extension courses through Fuller Theological Seminary and ultimately went full-time to Gordon-Conwell Theological Seminary in Boston, Massachusetts. At Gordon-Conwell he completed a Master's Degree in Divinity studying Theology, Philosophy, Church History, Biblical Languages (Hebrew & Greek), and Exegetical Methods. Following seminary he was involved with reconciliation efforts in the former Yugoslavia shortly after the war ended there in1995.  Arnold started acting in his early thirties in Texas. After an encouraging visit to Los Angeles where he spent time with childhood friend George Eads (of CSI Las Vegas) he decided to move to Los Angeles in 2001 to pursue acting full-time. While in Los Angeles he has studied acting with Judith Weston at Judith Weston Studio for Actors and Directors.  Arnold's acting career has been one of steady development, booking co-star and guest-star roles in nighttime television. He guest-starred opposite of Jane Seymour on the night time television drama Justice. He played the lead, Michael Hollister, in the film The Seer, written and directed by Patrick Masset (Friday Night Lights).  He was nominated Best Actor in the168 Film Festival for the role of Phil Stevens in the short-film Useless. In Useless he played a US Marshal who must choose between mercy and justice as he confronts the man who murdered his father. Arnold's performance in Useless confirmed his ability to carry lead roles, and he continues to work toward solidifying himself as a male lead in film and television.  Arnold married fellow Texan Stacy Rudd of San Antonio in 2003 and they are now raising their three children in the Los Angeles area.
8a    load   1.3693s  exec   0.0135s
8a    ok    1.3829s  Chambers, Linda || .hack//Quantum
8b    load   1.6162s  exec   0.0016s
8b    ok    1.6179s  Chambers, Linda || Dragon Ball Z: Shin Budokai
8c    load   2.2698s  exec   0.1191s
8c    ok    2.3889s  "A.J." || #1 Cheerleader Camp
8d    load   1.3762s  exec   0.0082s
8d    ok    1.3844s  "Jenny from the Block" || #1 Cheerleader Camp
9a    load   1.8910s  exec   0.0105s
9a    ok    1.9015s  AJ || Airport Announcer || Blue Harvest
9b    load   1.5537s  exec   0.0056s
9b    ok    1.5593s  AJ || Bassett, Angela || Airport Announcer || Blue Harvest
9c    load   1.4454s  exec   0.0168s
9c    ok    1.4622s  'Annette' || Alborg, Ana Esther || 2nd Balladeer || (1975-01-20)
9d    load   1.5724s  exec   0.0201s
9d    ok    1.5925s  !!!, Toy || Aaron, Caroline || "Cockamamie's" Salesgirl || $15,000.00 Error
10a   load   1.5135s  exec   0.0112s
10a   ok    1.5247s  Actor || 12 Rounds
10b   load   1.0979s  exec   0.0061s
10b   ok    1.1040s  (empty)
10c   load   1.7824s  exec   0.0147s
10c   ok    1.7973s  Himself || Evil Eyes: Behind the Scenes
11a   load   0.4342s  exec   0.0001s
11a   ok    0.4343s  Churchill Films || followed by || Batman Beyond
11b   load   0.3034s  exec   0.0000s
11b   ok    0.3034s  Filmlance International AB || follows || The Money Man
11c   load   0.1580s  exec   0.0000s
11c   ok    0.1581s  20th Century Fox Home Entertainment || (1997-2002) (worldwide) (all media) || 24
11d   load   0.1527s  exec   0.0001s
11d   ok    0.1528s  13th Street || (1954) (UK) (TV) || ...denn sie wissen nicht, was sie tun
12a   load   1.0066s  exec   0.0027s
12a   ok    1.0093s  10th Grade Reunion Films || 8.1 || 3:20
12b   load   0.7573s  exec   0.0001s
12b   ok    0.7574s  $10,000 || Birdemic: Shock and Terror
12c   load   0.7482s  exec   0.0120s
12c   ok    0.7602s  "Oh That Gus!" || 7.1 || $1.11
13a   load   0.6396s  exec   0.0040s
13a   ok    0.6436s  Afghanistan:24 June 2012 || 1.0 || &Me
13b   load   0.7223s  exec   0.0025s
13b   ok    0.7248s  5 Time Champion || 501audio || 1.8
13c   load   0.9100s  exec   0.0121s
13c   ok    0.9221s  Champion || DL Sites || 1.8
13d   load   1.0837s  exec   0.0413s
13d   ok    1.1251s  "O" Films || 1.0 || #54 Meets #47
14a   load   0.9002s  exec   0.0018s
14a   ok    0.9021s  1.0 || $lowdown
14b   load   0.6375s  exec   0.0017s
14b   ok    0.6392s  6.4 || Of Dolls and Murder
14c   load   0.8448s  exec   0.0266s
14c   ok    0.8714s  1.0 || $lowdown
15a   load   1.0600s  exec   0.0135s
15a   ok    1.0735s  USA:1 June 2007 || Battlestar Galactica: The Resistance
15b   load   0.7071s  exec   0.0002s
15b   ok    0.7074s  USA:27 April 2007 || RoboCop vs Terminator
15c   load   0.8784s  exec   0.0018s
15c   ok    0.8802s  USA:1 April 2003 || 24: Day Six - Debrief
15d   load   0.6173s  exec   0.0007s
15d   ok    0.6181s  (Not So) Instant Photo || 06/05
16a   load   1.9464s  exec   0.0004s
16a   ok    1.9468s  Adams, Stan || Carol Burnett vs. Anthony Perkins
16b   load   1.8045s  exec   0.1015s
16b   ok    1.9060s  !!!, Toy || & Teller
16c   load   1.6232s  exec   0.0415s
16c   ok    1.6647s  "Brooklyn" Tony Danza || (#1.5)
run 2: 58/114 ok  total 58.69s
(base) kat@Kats-MacBook-Pro rust % 


# Idiomatic/unoptimized
Q=1a,1b,1c,1d,2a,2b,2c,2d,3a,3b,3c,4a,4b,4c,5a,5b,5c,6a,6b,6c,6d,6e,6f,7a,7b,7c,8a,8b,8c,8d,9a,9b,9c,9d,10a,10b,10c,11a,11b,11c,11d,12a,12b,12c,13a,13b,13c,13d,14a,14b,14c,15a,15b,15c,15d,16a,16b,16c
PRELA_ONLY=$Q ROUNDS=2 cargo run --release --features job_loaded -- job_loaded

schema: 0.01s
114 queries registered (loaded)
--- run 1 ---
1a    load   0.3456s  exec   0.0077s
1b    load   0.6550s  exec   0.0045s
1b    ok    0.6595s  (Set Decoration Rentals) (uncredited) || Disaster Movie || 2008
1c    load   0.3634s  exec   0.0044s
1d    load   0.7342s  exec   0.0062s
1d    ok    0.7404s  (Set Decoration Rentals) (uncredited) || Disaster Movie || 2004
2a    load   0.8859s  exec   0.0328s
2a    ok    0.9187s  'Doc'
2b    load   0.7838s  exec   0.0164s
2b    ok    0.8003s  'Doc'
2c    load   0.6402s  exec   0.0000s
2c    ok    0.6402s  (empty)
2d    load   0.7786s  exec   0.0288s
2d    ok    0.8074s  & Teller
3a    load   1.8999s  exec   0.0075s
3a    ok    1.9075s  2 Days in New York
3b    load   1.7719s  exec   0.0059s
3b    ok    1.7779s  300: Rise of an Empire
3c    load   1.8354s  exec   0.0244s
3c    ok    1.8598s  & Teller 2
4a    load   0.0000s  exec   0.0000s
4a    DIFF   0.0000s  (pending: rx_dict)
        oracle: 5.1 || & Teller 2
4b    load   0.0000s  exec   0.0000s
4b    DIFF   0.0000s  (pending: rx_dict)
        oracle: 9.1 || Batman: Arkham City
4c    load   0.0000s  exec   0.0000s
4c    DIFF   0.0000s  (pending: rx_dict)
        oracle: 2.1 || & Teller 2
5a    load   2.0608s  exec   0.0000s
5a    ok    2.0609s  (empty)
5b    load   2.0008s  exec   0.0000s
5b    ok    2.0009s  (empty)
5c    load   2.0542s  exec   0.0152s
5c    ok    2.0694s  11,830,420
6a    load   1.7065s  exec   0.0054s
6a    ok    1.7119s  marvel-cinematic-universe || Iron Man 3 || Downey Jr., Robert
6b    load   1.3666s  exec   0.0050s
6b    ok    1.3717s  based-on-comic || The Avengers 2 || Downey Jr., Robert
6c    load   1.2081s  exec   0.0049s
6c    ok    1.2130s  marvel-cinematic-universe || The Avengers 2 || Downey Jr., Robert
6d    load   1.5958s  exec   0.0056s
6d    ok    1.6014s  based-on-comic || 2008 MTV Movie Awards || Downey Jr., Robert
6e    load   1.7044s  exec   0.0056s
6e    ok    1.7101s  marvel-cinematic-universe || Iron Man 3 || Downey Jr., Robert
6f    load  10.9301s  exec   0.3913s
6f    ok   11.3215s  based-on-comic || & Teller 2 || "Steff", Stefanie Oxmann Mcgaha
7a    load   0.0000s  exec   0.0000s
7a    DIFF   0.0000s  (pending: Disj)
        oracle: Antonioni, Michelangelo || Dressed to Kill
7b    load   2.5053s  exec   0.0051s
7b    ok    2.5105s  De Palma, Brian || Dressed to Kill
7c    load   0.0000s  exec   0.0000s
7c    DIFF   0.0000s  (pending: Disj)
        oracle: 50 Cent || "Boo" Arnold was born Earl Arnold in Hattiesburg, Mississippi in 1966. His father gave him the nickname 'Boo' early in life and it stuck through grade school, high school, and college. He is still known as "Boo" to family and friends.  Raised in central Texas, Arnold played baseball at Texas Tech University where he graduated with a BA in Advertising and Marketing. While at Texas Tech he was also a member of the Texas Epsilon chapter of Phi Delta Theta fraternity. After college he worked with Young Life, an outreach to high school students, in San Antonio, Texas.  While with Young Life Arnold began taking extension courses through Fuller Theological Seminary and ultimately went full-time to Gordon-Conwell Theological Seminary in Boston, Massachusetts. At Gordon-Conwell he completed a Master's Degree in Divinity studying Theology, Philosophy, Church History, Biblical Languages (Hebrew & Greek), and Exegetical Methods. Following seminary he was involved with reconciliation efforts in the former Yugoslavia shortly after the war ended there in1995.  Arnold started acting in his early thirties in Texas. After an encouraging visit to Los Angeles where he spent time with childhood friend George Eads (of CSI Las Vegas) he decided to move to Los Angeles in 2001 to pursue acting full-time. While in Los Angeles he has studied acting with Judith Weston at Judith Weston Studio for Actors and Directors.  Arnold's acting career has been one of steady development, booking co-star and guest-star roles in nighttime television. He guest-starred opposite of Jane Seymour on the night time television drama Justice. He played the lead, Michael Hollister, in the film The Seer, written and directed by Patrick Masset (Friday Night Lights).  He was nominated Best Actor in the168 Film Festival for the role of Phil Stevens in the short-film Useless. In Useless he played a US Marshal who must choose between mercy and justice as he confronts the man who murdered his father. Arnold's performance in Useless confirmed his ability to carry lead roles, and he continues to work toward solidifying himself as a male lead in film and television.  Arnold married fellow Texan Stacy Rudd of San Antonio in 2003 and they are now raising their three children in the Los Angeles area.
8a    load   3.1074s  exec   0.0066s
8a    ok    3.1140s  Chambers, Linda || .hack//Quantum
8b    load   2.8038s  exec   0.0053s
8b    ok    2.8091s  Chambers, Linda || Dragon Ball Z: Shin Budokai
8c    load   9.3069s  exec   0.1278s
8c    ok    9.4347s  "A.J." || #1 Cheerleader Camp
8d    load   8.1917s  exec   0.0623s
8d    ok    8.2541s  "Jenny from the Block" || #1 Cheerleader Camp
9a    load   3.5041s  exec   0.0118s
9a    ok    3.5159s  AJ || Airport Announcer || Blue Harvest
9b    load   3.7428s  exec   0.0074s
9b    ok    3.7502s  AJ || Bassett, Angela || Airport Announcer || Blue Harvest
9c    load   4.0268s  exec   0.0205s
9c    ok    4.0473s  'Annette' || Alborg, Ana Esther || 2nd Balladeer || (1975-01-20)
9d    load   6.9452s  exec   0.2032s
9d    ok    7.1484s  !!!, Toy || Aaron, Caroline || "Cockamamie's" Salesgirl || $15,000.00 Error
10a   load   6.7230s  exec   0.0110s
10a   ok    6.7341s  Actor || 12 Rounds
10b   load   5.4660s  exec   0.0001s
10b   ok    5.4661s  (empty)
10c   load   5.1389s  exec   0.0103s
10c   ok    5.1492s  Himself || Evil Eyes: Behind the Scenes
11a   load   0.0000s  exec   0.0000s
11a   DIFF   0.0000s  (pending: Diff)
        oracle: Churchill Films || followed by || Batman Beyond
11b   load   0.0000s  exec   0.0000s
11b   DIFF   0.0000s  (pending: Diff)
        oracle: Filmlance International AB || follows || The Money Man
11c   load   1.0355s  exec   0.0005s
11c   ok    1.0360s  20th Century Fox Home Entertainment || (1997-2002) (worldwide) (all media) || 24
11d   load   1.0947s  exec   0.0011s
11d   ok    1.0958s  13th Street || (1954) (UK) (TV) || ...denn sie wissen nicht, was sie tun
12a   load   1.0221s  exec   0.0087s
12a   ok    1.0308s  10th Grade Reunion Films || 8.1 || 3:20
12b   load   5.3984s  exec   0.0041s
12b   ok    5.4026s  $10,000 || Birdemic: Shock and Terror
12c   load   1.7866s  exec   0.0399s
12c   ok    1.8265s  "Oh That Gus!" || 7.1 || $1.11
13a   load   5.8553s  exec   0.0391s
13a   ok    5.8945s  Afghanistan:24 June 2012 || 1.0 || &Me
13b   load   1.3001s  exec   0.0054s
13b   ok    1.3055s  5 Time Champion || 501audio || 1.8
13c   load   1.1181s  exec   0.0046s
13c   ok    1.1228s  Champion || DL Sites || 1.8
13d   load   1.2068s  exec   0.0210s
13d   ok    1.2278s  "O" Films || 1.0 || #54 Meets #47
14a   load   0.9144s  exec   0.0061s
14a   ok    0.9205s  1.0 || $lowdown
14b   load   0.6584s  exec   0.0047s
14b   ok    0.6631s  6.4 || Of Dolls and Murder
14c   load   0.9733s  exec   0.0112s
14c   ok    0.9845s  1.0 || $lowdown
15a   load   1.9546s  exec   0.0063s
15a   ok    1.9610s  USA:1 June 2007 || Battlestar Galactica: The Resistance
15b   load   1.7084s  exec   0.0059s
15b   ok    1.7143s  USA:27 April 2007 || RoboCop vs Terminator
15c   load   1.6821s  exec   0.0055s
15c   ok    1.6877s  USA:1 April 2003 || 24: Day Six - Debrief
15d   load   0.9870s  exec   0.0097s
15d   ok    0.9968s  (Not So) Instant Photo || 06/05
16a   load  11.0907s  exec   0.0068s
16a   ok   11.0976s  Adams, Stan || Carol Burnett vs. Anthony Perkins
16b   load   8.6105s  exec   0.3748s
16b   ok    8.9853s  !!!, Toy || & Teller
16c   load   9.2982s  exec   0.1885s
16c   ok    9.4867s  "Brooklyn" Tony Danza || (#1.5)
run 1: 51/114 ok  total 156.27s
--- run 2 ---
1a    load   0.3531s  exec   0.0068s
1a    ok    0.3599s  (A Warner Bros.-First National Picture) (presents) || A Clockwork Orange || 1934
1b    load   0.5164s  exec   0.0042s
1b    ok    0.5207s  (Set Decoration Rentals) (uncredited) || Disaster Movie || 2008
1c    load   0.2602s  exec   0.0042s
1c    ok    0.2644s  (co-production) || Intouchables || 2011
1d    load   0.5781s  exec   0.0056s
1d    ok    0.5838s  (Set Decoration Rentals) (uncredited) || Disaster Movie || 2004
2a    load   0.5987s  exec   0.0083s
2a    ok    0.6070s  'Doc'
2b    load   0.9322s  exec   0.0103s
2b    ok    0.9426s  'Doc'
2c    load   0.5815s  exec   0.0000s
2c    ok    0.5815s  (empty)
2d    load   0.6339s  exec   0.0122s
2d    ok    0.6462s  & Teller
3a    load   1.7552s  exec   0.0061s
3a    ok    1.7614s  2 Days in New York
3b    load   1.6246s  exec   0.0052s
3b    ok    1.6299s  300: Rise of an Empire
3c    load   1.6609s  exec   0.0135s
3c    ok    1.6744s  & Teller 2
4a    load   0.0000s  exec   0.0000s
4a    DIFF   0.0000s  (pending: rx_dict)
        oracle: 5.1 || & Teller 2
4b    load   0.0000s  exec   0.0000s
4b    DIFF   0.0000s  (pending: rx_dict)
        oracle: 9.1 || Batman: Arkham City
4c    load   0.0000s  exec   0.0000s
4c    DIFF   0.0000s  (pending: rx_dict)
        oracle: 2.1 || & Teller 2
5a    load   1.5584s  exec   0.0000s
5a    ok    1.5591s  (empty)
5b    load   1.6171s  exec   0.0000s
5b    ok    1.6171s  (empty)
5c    load   1.7737s  exec   0.0180s
5c    ok    1.7917s  11,830,420
6a    load   1.4545s  exec   0.0054s
6a    ok    1.4598s  marvel-cinematic-universe || Iron Man 3 || Downey Jr., Robert
6b    load   1.4059s  exec   0.0052s
6b    ok    1.4111s  based-on-comic || The Avengers 2 || Downey Jr., Robert
6c    load   1.2724s  exec   0.0051s
6c    ok    1.2776s  marvel-cinematic-universe || The Avengers 2 || Downey Jr., Robert
6d    load   1.4448s  exec   0.0062s
6d    ok    1.4511s  based-on-comic || 2008 MTV Movie Awards || Downey Jr., Robert
6e    load   1.2327s  exec   0.0055s
6e    ok    1.2382s  marvel-cinematic-universe || Iron Man 3 || Downey Jr., Robert
6f    load   9.7169s  exec   0.4848s
6f    ok   10.2018s  based-on-comic || & Teller 2 || "Steff", Stefanie Oxmann Mcgaha
7a    load   0.0000s  exec   0.0000s
7a    DIFF   0.0000s  (pending: Disj)
        oracle: Antonioni, Michelangelo || Dressed to Kill
7b    load   2.3342s  exec   0.0051s
7b    ok    2.3394s  De Palma, Brian || Dressed to Kill
7c    load   0.0000s  exec   0.0000s
7c    DIFF   0.0000s  (pending: Disj)
        oracle: 50 Cent || "Boo" Arnold was born Earl Arnold in Hattiesburg, Mississippi in 1966. His father gave him the nickname 'Boo' early in life and it stuck through grade school, high school, and college. He is still known as "Boo" to family and friends.  Raised in central Texas, Arnold played baseball at Texas Tech University where he graduated with a BA in Advertising and Marketing. While at Texas Tech he was also a member of the Texas Epsilon chapter of Phi Delta Theta fraternity. After college he worked with Young Life, an outreach to high school students, in San Antonio, Texas.  While with Young Life Arnold began taking extension courses through Fuller Theological Seminary and ultimately went full-time to Gordon-Conwell Theological Seminary in Boston, Massachusetts. At Gordon-Conwell he completed a Master's Degree in Divinity studying Theology, Philosophy, Church History, Biblical Languages (Hebrew & Greek), and Exegetical Methods. Following seminary he was involved with reconciliation efforts in the former Yugoslavia shortly after the war ended there in1995.  Arnold started acting in his early thirties in Texas. After an encouraging visit to Los Angeles where he spent time with childhood friend George Eads (of CSI Las Vegas) he decided to move to Los Angeles in 2001 to pursue acting full-time. While in Los Angeles he has studied acting with Judith Weston at Judith Weston Studio for Actors and Directors.  Arnold's acting career has been one of steady development, booking co-star and guest-star roles in nighttime television. He guest-starred opposite of Jane Seymour on the night time television drama Justice. He played the lead, Michael Hollister, in the film The Seer, written and directed by Patrick Masset (Friday Night Lights).  He was nominated Best Actor in the168 Film Festival for the role of Phil Stevens in the short-film Useless. In Useless he played a US Marshal who must choose between mercy and justice as he confronts the man who murdered his father. Arnold's performance in Useless confirmed his ability to carry lead roles, and he continues to work toward solidifying himself as a male lead in film and television.  Arnold married fellow Texan Stacy Rudd of San Antonio in 2003 and they are now raising their three children in the Los Angeles area.
8a    load   2.2114s  exec   0.0059s
8a    ok    2.2173s  Chambers, Linda || .hack//Quantum
8b    load   3.1210s  exec   0.0047s
8b    ok    3.1258s  Chambers, Linda || Dragon Ball Z: Shin Budokai
8c    load   7.5598s  exec   0.1219s
8c    ok    7.6819s  "A.J." || #1 Cheerleader Camp
8d    load   6.8769s  exec   0.0208s
8d    ok    6.8978s  "Jenny from the Block" || #1 Cheerleader Camp
9a    load   2.5718s  exec   0.0066s
9a    ok    2.5784s  AJ || Airport Announcer || Blue Harvest
9b    load   2.5265s  exec   0.0064s
9b    ok    2.5329s  AJ || Bassett, Angela || Airport Announcer || Blue Harvest
9c    load   2.5503s  exec   0.0116s
9c    ok    2.5620s  'Annette' || Alborg, Ana Esther || 2nd Balladeer || (1975-01-20)
9d    load   4.7569s  exec   0.0727s
9d    ok    4.8296s  !!!, Toy || Aaron, Caroline || "Cockamamie's" Salesgirl || $15,000.00 Error
10a   load   4.1165s  exec   0.0060s
10a   ok    4.1225s  Actor || 12 Rounds
10b   load   4.1732s  exec   0.0001s
10b   ok    4.1733s  (empty)
10c   load   4.4346s  exec   0.0113s
10c   ok    4.4459s  Himself || Evil Eyes: Behind the Scenes
11a   load   0.0000s  exec   0.0000s
11a   DIFF   0.0000s  (pending: Diff)
        oracle: Churchill Films || followed by || Batman Beyond
11b   load   0.0000s  exec   0.0000s
11b   DIFF   0.0000s  (pending: Diff)
        oracle: Filmlance International AB || follows || The Money Man
11c   load   0.9757s  exec   0.0005s
11c   ok    0.9762s  20th Century Fox Home Entertainment || (1997-2002) (worldwide) (all media) || 24
11d   load   1.0577s  exec   0.0007s
11d   ok    1.0585s  13th Street || (1954) (UK) (TV) || ...denn sie wissen nicht, was sie tun
12a   load   0.9762s  exec   0.0084s
12a   ok    0.9847s  10th Grade Reunion Films || 8.1 || 3:20
12b   load   4.7942s  exec   0.0039s
12b   ok    4.7982s  $10,000 || Birdemic: Shock and Terror
12c   load   1.0450s  exec   0.0141s
12c   ok    1.0592s  "Oh That Gus!" || 7.1 || $1.11
13a   load   5.1701s  exec   0.0431s
13a   ok    5.2133s  Afghanistan:24 June 2012 || 1.0 || &Me
13b   load   1.1617s  exec   0.0054s
13b   ok    1.1672s  5 Time Champion || 501audio || 1.8
13c   load   1.2206s  exec   0.0045s
13c   ok    1.2251s  Champion || DL Sites || 1.8
13d   load   1.1207s  exec   0.0198s
13d   ok    1.1405s  "O" Films || 1.0 || #54 Meets #47
14a   load   1.3470s  exec   0.0070s
14a   ok    1.3541s  1.0 || $lowdown
14b   load   0.6321s  exec   0.0045s
14b   ok    0.6366s  6.4 || Of Dolls and Murder
14c   load   0.8165s  exec   0.0078s
14c   ok    0.8244s  1.0 || $lowdown
15a   load   1.9613s  exec   0.0057s
15a   ok    1.9670s  USA:1 June 2007 || Battlestar Galactica: The Resistance
15b   load   1.7190s  exec   0.0054s
15b   ok    1.7244s  USA:27 April 2007 || RoboCop vs Terminator
15c   load   1.7831s  exec   0.0065s
15c   ok    1.7897s  USA:1 April 2003 || 24: Day Six - Debrief
15d   load   1.0156s  exec   0.0105s
15d   ok    1.0262s  (Not So) Instant Photo || 06/05
16a   load  10.9858s  exec   0.0056s
16a   ok   10.9915s  Adams, Stan || Carol Burnett vs. Anthony Perkins
16b   load  10.6252s  exec   0.6689s
16b   ok   11.2942s  !!!, Toy || & Teller
16c   load  11.2500s  exec   0.2203s
16c   ok   11.4704s  "Brooklyn" Tony Danza || (#1.5)
run 2: 51/114 ok  total 139.79s
(base) kat@Kats-MacBook-Pro rust % 


# Nominally loading full dataset 

Q=1a,1b,1c,1d,2a,2b,2c,2d,3a,3b,3c,4a,4b,4c,5a,5b,5c,6a,6b,6c,6d,6e,6f,7a,7b,7c,8a,8b,8c,8d,9a,9b,9c,9d,10a,10b,10c,11a,11b,11c,11d,12a,12b,12c,13a,13b,13c,13d,14a,14b,14c,15a,15b,15c,15d,16a,16b,16c
PRELA_ONLY=$Q ROUNDS=2 cargo run --release -- job    
load: 4.02s  (movie n=2528312, person n=4167491)
114 queries registered
--- run 1 ---
1a    ok    0.2496s  (A Warner Bros.-First National Picture) (presents) || A Clockwork Orange || 1934
1b    ok    0.0174s  (Set Decoration Rentals) (uncredited) || Disaster Movie || 2008
1c    ok    0.0075s  (co-production) || Intouchables || 2011
1d    ok    0.0077s  (Set Decoration Rentals) (uncredited) || Disaster Movie || 2004
2a    ok    1.0174s  'Doc'
2b    ok    0.0630s  'Doc'
2c    ok    0.0274s  (empty)
2d    ok    0.0745s  & Teller
3a    ok    1.2316s  2 Days in New York
3b    ok    0.2296s  300: Rise of an Empire
3c    ok    0.1667s  & Teller 2
4a    ok    0.0475s  5.1 || & Teller 2
4b    ok    0.0149s  9.1 || Batman: Arkham City
4c    ok    0.0329s  2.1 || & Teller 2
5a    ok    0.0659s  (empty)
5b    ok    0.0236s  (empty)
5c    ok    0.0888s  11,830,420
6a    ok    0.1837s  marvel-cinematic-universe || Iron Man 3 || Downey Jr., Robert
6b    ok    0.0247s  based-on-comic || The Avengers 2 || Downey Jr., Robert
6c    ok    0.0044s  marvel-cinematic-universe || The Avengers 2 || Downey Jr., Robert
6d    ok    1.4286s  based-on-comic || 2008 MTV Movie Awards || Downey Jr., Robert
6e    ok    0.0203s  marvel-cinematic-universe || Iron Man 3 || Downey Jr., Robert
6f    ok    0.1406s  based-on-comic || & Teller 2 || "Steff", Stefanie Oxmann Mcgaha
7a    ok    1.0302s  Antonioni, Michelangelo || Dressed to Kill
7b    ok    0.0097s  De Palma, Brian || Dressed to Kill
7c    ok    0.7655s  50 Cent || "Boo" Arnold was born Earl Arnold in Hattiesburg, Mississippi in 1966. His father gave him the nickname 'Boo' early in life and it stuck through grade school, high school, and college. He is still known as "Boo" to family and friends.  Raised in central Texas, Arnold played baseball at Texas Tech University where he graduated with a BA in Advertising and Marketing. While at Texas Tech he was also a member of the Texas Epsilon chapter of Phi Delta Theta fraternity. After college he worked with Young Life, an outreach to high school students, in San Antonio, Texas.  While with Young Life Arnold began taking extension courses through Fuller Theological Seminary and ultimately went full-time to Gordon-Conwell Theological Seminary in Boston, Massachusetts. At Gordon-Conwell he completed a Master's Degree in Divinity studying Theology, Philosophy, Church History, Biblical Languages (Hebrew & Greek), and Exegetical Methods. Following seminary he was involved with reconciliation efforts in the former Yugoslavia shortly after the war ended there in1995.  Arnold started acting in his early thirties in Texas. After an encouraging visit to Los Angeles where he spent time with childhood friend George Eads (of CSI Las Vegas) he decided to move to Los Angeles in 2001 to pursue acting full-time. While in Los Angeles he has studied acting with Judith Weston at Judith Weston Studio for Actors and Directors.  Arnold's acting career has been one of steady development, booking co-star and guest-star roles in nighttime television. He guest-starred opposite of Jane Seymour on the night time television drama Justice. He played the lead, Michael Hollister, in the film The Seer, written and directed by Patrick Masset (Friday Night Lights).  He was nominated Best Actor in the168 Film Festival for the role of Phil Stevens in the short-film Useless. In Useless he played a US Marshal who must choose between mercy and justice as he confronts the man who murdered his father. Arnold's performance in Useless confirmed his ability to carry lead roles, and he continues to work toward solidifying himself as a male lead in film and television.  Arnold married fellow Texan Stacy Rudd of San Antonio in 2003 and they are now raising their three children in the Los Angeles area.
8a    ok    3.1586s  Chambers, Linda || .hack//Quantum
8b    ok    0.0677s  Chambers, Linda || Dragon Ball Z: Shin Budokai
8c    ok    1.3411s  "A.J." || #1 Cheerleader Camp
8d    ok    0.1991s  "Jenny from the Block" || #1 Cheerleader Camp
9a    ok    1.4228s  AJ || Airport Announcer || Blue Harvest
9b    ok    0.1034s  AJ || Bassett, Angela || Airport Announcer || Blue Harvest
9c    ok    0.5498s  'Annette' || Alborg, Ana Esther || 2nd Balladeer || (1975-01-20)
9d    ok    1.4249s  !!!, Toy || Aaron, Caroline || "Cockamamie's" Salesgirl || $15,000.00 Error
10a   ok    0.0836s  Actor || 12 Rounds
10b   ok    0.0413s  (empty)
10c   ok    0.7837s  Himself || Evil Eyes: Behind the Scenes
11a   ok    0.1419s  Churchill Films || followed by || Batman Beyond
11b   ok    0.0043s  Filmlance International AB || follows || The Money Man
11c   ok    0.0126s  20th Century Fox Home Entertainment || (1997-2002) (worldwide) (all media) || 24
11d   ok    0.0081s  13th Street || (1954) (UK) (TV) || ...denn sie wissen nicht, was sie tun
12a   ok    0.7542s  10th Grade Reunion Films || 8.1 || 3:20
12b   ok    0.0480s  $10,000 || Birdemic: Shock and Terror
12c   ok    0.1708s  "Oh That Gus!" || 7.1 || $1.11
13a   ok    0.1629s  Afghanistan:24 June 2012 || 1.0 || &Me
13b   ok    0.0341s  5 Time Champion || 501audio || 1.8
13c   ok    0.0295s  Champion || DL Sites || 1.8
13d   ok    0.0540s  "O" Films || 1.0 || #54 Meets #47
14a   ok    0.0795s  1.0 || $lowdown
14b   ok    0.0508s  6.4 || Of Dolls and Murder
14c   ok    0.0953s  1.0 || $lowdown
15a   ok    0.4011s  USA:1 June 2007 || Battlestar Galactica: The Resistance
15b   ok    0.1308s  USA:27 April 2007 || RoboCop vs Terminator
15c   ok    0.0859s  USA:1 April 2003 || 24: Day Six - Debrief
15d   ok    0.0787s  (Not So) Instant Photo || 06/05
16a   ok    0.0908s  Adams, Stan || Carol Burnett vs. Anthony Perkins
16b   ok    0.4646s  !!!, Toy || & Teller
16c   ok    0.0519s  "Brooklyn" Tony Danza || (#1.5)
run 1: 58/114 ok  total 19.10s
--- run 2 ---
1a    ok    0.0236s  (A Warner Bros.-First National Picture) (presents) || A Clockwork Orange || 1934
1b    ok    0.0073s  (Set Decoration Rentals) (uncredited) || Disaster Movie || 2008
1c    ok    0.0066s  (co-production) || Intouchables || 2011
1d    ok    0.0076s  (Set Decoration Rentals) (uncredited) || Disaster Movie || 2004
2a    ok    0.0234s  'Doc'
2b    ok    0.0216s  'Doc'
2c    ok    0.0204s  (empty)
2d    ok    0.0205s  & Teller
3a    ok    0.6598s  2 Days in New York
3b    ok    0.1915s  300: Rise of an Empire
3c    ok    0.1343s  & Teller 2
4a    ok    0.0176s  5.1 || & Teller 2
4b    ok    0.0144s  9.1 || Batman: Arkham City
4c    ok    0.0246s  2.1 || & Teller 2
5a    ok    0.0334s  (empty)
5b    ok    0.0258s  (empty)
5c    ok    0.0639s  11,830,420
6a    ok    0.1212s  marvel-cinematic-universe || Iron Man 3 || Downey Jr., Robert
6b    ok    0.0196s  based-on-comic || The Avengers 2 || Downey Jr., Robert
6c    ok    0.0045s  marvel-cinematic-universe || The Avengers 2 || Downey Jr., Robert
6d    ok    0.4488s  based-on-comic || 2008 MTV Movie Awards || Downey Jr., Robert
6e    ok    0.0186s  marvel-cinematic-universe || Iron Man 3 || Downey Jr., Robert
6f    ok    0.1049s  based-on-comic || & Teller 2 || "Steff", Stefanie Oxmann Mcgaha
7a    ok    0.4344s  Antonioni, Michelangelo || Dressed to Kill
7b    ok    0.0121s  De Palma, Brian || Dressed to Kill
7c    ok    0.7218s  50 Cent || "Boo" Arnold was born Earl Arnold in Hattiesburg, Mississippi in 1966. His father gave him the nickname 'Boo' early in life and it stuck through grade school, high school, and college. He is still known as "Boo" to family and friends.  Raised in central Texas, Arnold played baseball at Texas Tech University where he graduated with a BA in Advertising and Marketing. While at Texas Tech he was also a member of the Texas Epsilon chapter of Phi Delta Theta fraternity. After college he worked with Young Life, an outreach to high school students, in San Antonio, Texas.  While with Young Life Arnold began taking extension courses through Fuller Theological Seminary and ultimately went full-time to Gordon-Conwell Theological Seminary in Boston, Massachusetts. At Gordon-Conwell he completed a Master's Degree in Divinity studying Theology, Philosophy, Church History, Biblical Languages (Hebrew & Greek), and Exegetical Methods. Following seminary he was involved with reconciliation efforts in the former Yugoslavia shortly after the war ended there in1995.  Arnold started acting in his early thirties in Texas. After an encouraging visit to Los Angeles where he spent time with childhood friend George Eads (of CSI Las Vegas) he decided to move to Los Angeles in 2001 to pursue acting full-time. While in Los Angeles he has studied acting with Judith Weston at Judith Weston Studio for Actors and Directors.  Arnold's acting career has been one of steady development, booking co-star and guest-star roles in nighttime television. He guest-starred opposite of Jane Seymour on the night time television drama Justice. He played the lead, Michael Hollister, in the film The Seer, written and directed by Patrick Masset (Friday Night Lights).  He was nominated Best Actor in the168 Film Festival for the role of Phil Stevens in the short-film Useless. In Useless he played a US Marshal who must choose between mercy and justice as he confronts the man who murdered his father. Arnold's performance in Useless confirmed his ability to carry lead roles, and he continues to work toward solidifying himself as a male lead in film and television.  Arnold married fellow Texan Stacy Rudd of San Antonio in 2003 and they are now raising their three children in the Los Angeles area.
8a    ok    0.4395s  Chambers, Linda || .hack//Quantum
8b    ok    0.0314s  Chambers, Linda || Dragon Ball Z: Shin Budokai
8c    ok    0.6815s  "A.J." || #1 Cheerleader Camp
8d    ok    0.3040s  "Jenny from the Block" || #1 Cheerleader Camp
9a    ok    1.9699s  AJ || Airport Announcer || Blue Harvest
9b    ok    0.1294s  AJ || Bassett, Angela || Airport Announcer || Blue Harvest
9c    ok    0.5508s  'Annette' || Alborg, Ana Esther || 2nd Balladeer || (1975-01-20)
9d    ok    1.4261s  !!!, Toy || Aaron, Caroline || "Cockamamie's" Salesgirl || $15,000.00 Error
10a   ok    0.0825s  Actor || 12 Rounds
10b   ok    0.0417s  (empty)
10c   ok    0.7717s  Himself || Evil Eyes: Behind the Scenes
11a   ok    0.1413s  Churchill Films || followed by || Batman Beyond
11b   ok    0.0057s  Filmlance International AB || follows || The Money Man
11c   ok    0.0181s  20th Century Fox Home Entertainment || (1997-2002) (worldwide) (all media) || 24
11d   ok    0.0036s  13th Street || (1954) (UK) (TV) || ...denn sie wissen nicht, was sie tun
12a   ok    0.9855s  10th Grade Reunion Films || 8.1 || 3:20
12b   ok    0.0577s  $10,000 || Birdemic: Shock and Terror
12c   ok    0.1735s  "Oh That Gus!" || 7.1 || $1.11
13a   ok    0.1537s  Afghanistan:24 June 2012 || 1.0 || &Me
13b   ok    0.0438s  5 Time Champion || 501audio || 1.8
13c   ok    0.0260s  Champion || DL Sites || 1.8
13d   ok    0.0474s  "O" Films || 1.0 || #54 Meets #47
14a   ok    0.0734s  1.0 || $lowdown
14b   ok    0.0451s  6.4 || Of Dolls and Murder
14c   ok    0.0977s  1.0 || $lowdown
15a   ok    0.1892s  USA:1 June 2007 || Battlestar Galactica: The Resistance
15b   ok    0.1123s  USA:27 April 2007 || RoboCop vs Terminator
15c   ok    0.0912s  USA:1 April 2003 || 24: Day Six - Debrief
15d   ok    0.0665s  (Not So) Instant Photo || 06/05
16a   ok    0.0842s  Adams, Stan || Carol Burnett vs. Anthony Perkins
16b   ok    0.3210s  !!!, Toy || & Teller
16c   ok    0.0466s  "Brooklyn" Tony Danza || (#1.5)
run 2: 58/114 ok  total 12.40s
(base) kat@Kats-MacBook-Pro rust % 


# Per-query raw timings and speedups (run 2, hand-optimised as baseline)

All times are in seconds. Speedup = other approach's time ÷ hand-optimised time. Above 1 means hand-optimised is faster; below 1 (⚠) means slower. "n/a" means a time of 0.0000s, so the ratio is undefined. Sub-millisecond exec ratios are noisy.

| Query | Hand Load | Hand Exec | Idiom Load | Idiom Exec | Nominal Load | Nominal Exec | Load speedup (hand vs idiom) | Exec speedup (hand vs idiom) | Load speedup (hand vs nominal) | Exec speedup (hand vs nominal) |
|---|---|---|---|---|---|---|---|---|---|---|
| 1a | 0.1908 | 0.0010 | 0.3531 | 0.0068 | 4.02 | 0.0236 | 1.85× | 6.8× | 21.1× | 23.6× |
| 1b | 0.1781 | 0.0001 | 0.5164 | 0.0042 | 4.02 | 0.0073 | 2.90× | 42× | 22.6× | 73× |
| 1c | 0.1891 | 0.0002 | 0.2602 | 0.0042 | 4.02 | 0.0066 | 1.38× | 21× | 21.3× | 33× |
| 1d | 0.1526 | 0.0001 | 0.5781 | 0.0056 | 4.02 | 0.0076 | 3.79× | 56× | 26.3× | 76× |
| 2a | 0.2103 | 0.0009 | 0.5987 | 0.0083 | 4.02 | 0.0234 | 2.85× | 9.2× | 19.1× | 26.0× |
| 2b | 0.2042 | 0.0006 | 0.9322 | 0.0103 | 4.02 | 0.0216 | 4.57× | 17.2× | 19.7× | 36.0× |
| 2c | 0.0733 | 0.0000 | 0.5815 | 0.0000 | 4.02 | 0.0204 | 7.93× | n/a | 54.8× | n/a |
| 2d | 0.2453 | 0.0009 | 0.6339 | 0.0122 | 4.02 | 0.0205 | 2.58× | 13.6× | 16.4× | 22.8× |
| 3a | 1.4088 | 0.0017 | 1.7552 | 0.0061 | 4.02 | 0.6598 | 1.25× | 3.6× | 2.85× | 388× |
| 3b | 1.0713 | 0.0023 | 1.6246 | 0.0052 | 4.02 | 0.1915 | 1.52× | 2.3× | 3.75× | 83× |
| 3c | 1.3443 | 0.0066 | 1.6609 | 0.0135 | 4.02 | 0.1343 | 1.24× | 2.0× | 2.99× | 20.3× |
| 5a | 0.2484 | 0.0000 | 1.5584 | 0.0000 | 4.02 | 0.0334 | 6.27× | n/a | 16.2× | n/a |
| 5b | 0.2081 | 0.0000 | 1.6171 | 0.0000 | 4.02 | 0.0258 | 7.77× | n/a | 19.3× | n/a |
| 5c | 1.2303 | 0.0028 | 1.7737 | 0.0180 | 4.02 | 0.0639 | 1.44× | 6.4× | 3.27× | 22.8× |
| 6a | 1.5620 | 0.0013 | 1.4545 | 0.0054 | 4.02 | 0.1212 | 0.93× ⚠ | 4.2× | 2.57× | 93× |
| 6b | 1.2017 | 0.0008 | 1.4059 | 0.0052 | 4.02 | 0.0196 | 1.17× | 6.5× | 3.35× | 24.5× |
| 6c | 1.2499 | 0.0003 | 1.2724 | 0.0051 | 4.02 | 0.0045 | 1.02× | 17× | 3.22× | 15.0× |
| 6d | 1.2911 | 0.0001 | 1.4448 | 0.0062 | 4.02 | 0.4488 | 1.12× | 62× | 3.11× | 4488× |
| 6e | 1.4993 | 0.0012 | 1.2327 | 0.0055 | 4.02 | 0.0186 | 0.82× ⚠ | 4.6× | 2.68× | 15.5× |
| 6f | 1.8874 | 0.1247 | 9.7169 | 0.4848 | 4.02 | 0.1049 | 5.15× | 3.9× | 2.13× | 0.84× ⚠ |
| 7b | 1.7628 | 0.0922 | 2.3342 | 0.0051 | 4.02 | 0.0121 | 1.32× | 0.06× ⚠ | 2.28× | 0.13× ⚠ |
| 8a | 1.3693 | 0.0135 | 2.2114 | 0.0059 | 4.02 | 0.4395 | 1.62× | 0.44× ⚠ | 2.94× | 32.6× |
| 8b | 1.6162 | 0.0016 | 3.1210 | 0.0047 | 4.02 | 0.0314 | 1.93× | 2.9× | 2.49× | 19.6× |
| 8c | 2.2698 | 0.1191 | 7.5598 | 0.1219 | 4.02 | 0.6815 | 3.33× | 1.02× | 1.77× | 5.7× |
| 8d | 1.3762 | 0.0082 | 6.8769 | 0.0208 | 4.02 | 0.3040 | 5.00× | 2.5× | 2.92× | 37× |
| 9a | 1.8910 | 0.0105 | 2.5718 | 0.0066 | 4.02 | 1.9699 | 1.36× | 0.63× ⚠ | 2.13× | 188× |
| 9b | 1.5537 | 0.0056 | 2.5265 | 0.0064 | 4.02 | 0.1294 | 1.63× | 1.14× | 2.59× | 23.1× |
| 9c | 1.4454 | 0.0168 | 2.5503 | 0.0116 | 4.02 | 0.5508 | 1.76× | 0.69× ⚠ | 2.78× | 32.8× |
| 9d | 1.5724 | 0.0201 | 4.7569 | 0.0727 | 4.02 | 1.4261 | 3.03× | 3.6× | 2.56× | 71× |
| 10a | 1.5135 | 0.0112 | 4.1165 | 0.0060 | 4.02 | 0.0825 | 2.72× | 0.54× ⚠ | 2.66× | 7.4× |
| 10b | 1.0979 | 0.0061 | 4.1732 | 0.0001 | 4.02 | 0.0417 | 3.80× | 0.02× ⚠ | 3.66× | 6.8× |
| 10c | 1.7824 | 0.0147 | 4.4346 | 0.0113 | 4.02 | 0.7717 | 2.49× | 0.77× ⚠ | 2.26× | 52× |
| 11c | 0.1580 | 0.0000 | 0.9757 | 0.0005 | 4.02 | 0.0181 | 6.18× | n/a | 25.4× | n/a |
| 11d | 0.1527 | 0.0001 | 1.0577 | 0.0007 | 4.02 | 0.0036 | 6.93× | 7.0× | 26.3× | 36× |
| 12a | 1.0066 | 0.0027 | 0.9762 | 0.0084 | 4.02 | 0.9855 | 0.97× ⚠ | 3.1× | 3.99× | 365× |
| 12b | 0.7573 | 0.0001 | 4.7942 | 0.0039 | 4.02 | 0.0577 | 6.33× | 39× | 5.31× | 577× |
| 12c | 0.7482 | 0.0120 | 1.0450 | 0.0141 | 4.02 | 0.1735 | 1.40× | 1.18× | 5.37× | 14.5× |
| 13a | 0.6396 | 0.0040 | 5.1701 | 0.0431 | 4.02 | 0.1537 | 8.08× | 10.8× | 6.29× | 38× |
| 13b | 0.7223 | 0.0025 | 1.1617 | 0.0054 | 4.02 | 0.0438 | 1.61× | 2.2× | 5.57× | 17.5× |
| 13c | 0.9100 | 0.0121 | 1.2206 | 0.0045 | 4.02 | 0.0260 | 1.34× | 0.37× ⚠ | 4.42× | 2.15× |
| 13d | 1.0837 | 0.0413 | 1.1207 | 0.0198 | 4.02 | 0.0474 | 1.03× | 0.48× ⚠ | 3.71× | 1.15× |
| 14a | 0.9002 | 0.0018 | 1.3470 | 0.0070 | 4.02 | 0.0734 | 1.50× | 3.9× | 4.47× | 41× |
| 14b | 0.6375 | 0.0017 | 0.6321 | 0.0045 | 4.02 | 0.0451 | 0.99× ⚠ | 2.6× | 6.31× | 26.5× |
| 14c | 0.8448 | 0.0266 | 0.8165 | 0.0078 | 4.02 | 0.0977 | 0.97× ⚠ | 0.29× ⚠ | 4.76× | 3.7× |
| 15a | 1.0600 | 0.0135 | 1.9613 | 0.0057 | 4.02 | 0.1892 | 1.85× | 0.42× ⚠ | 3.79× | 14.0× |
| 15b | 0.7071 | 0.0002 | 1.7190 | 0.0054 | 4.02 | 0.1123 | 2.43× | 27× | 5.69× | 562× |
| 15c | 0.8784 | 0.0018 | 1.7831 | 0.0065 | 4.02 | 0.0912 | 2.03× | 3.6× | 4.58× | 50.7× |
| 15d | 0.6173 | 0.0007 | 1.0156 | 0.0105 | 4.02 | 0.0665 | 1.65× | 15× | 6.51× | 95× |
| 16a | 1.9464 | 0.0004 | 10.9858 | 0.0056 | 4.02 | 0.0842 | 5.64× | 14× | 2.07× | 210× |
| 16b | 1.8045 | 0.1015 | 10.6252 | 0.6689 | 4.02 | 0.3210 | 5.89× | 6.6× | 2.23× | 3.2× |
| 16c | 1.6232 | 0.0415 | 11.2500 | 0.2203 | 4.02 | 0.0466 | 6.93× | 5.3× | 2.48× | 1.12× |
| **Total (51 queries)** | **52.09** | **0.730** | **137.86** | **1.922** | **4.02 (once)** | **11.03** | **2.65×** | **2.63×** | **n/a** | **15.1×** |

## Notes

- The nominal build loads the full dataset once (4.02s), then each query is exec-only. The per-query Nominal Load column repeats that one-time cost, and Load speedup (hand vs nominal) = 4.02 ÷ hand load.
- The Total row's nominal load is counted once, so a load speedup against the summed hand load is not meaningful (n/a). End-to-end, nominal takes about 4.02 + 11.03 ≈ 15.05s versus 52.8s for hand-optimised.
- The 51 queries are those that are not DIFF in the idiomatic build. 4a, 4b, 4c, 7a, 7c, 11a and 11b are excluded.
- Total-row speedups are ratios of summed raw timings, not averages of the per-query ratios.