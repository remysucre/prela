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