1. Separation of the load and execution plans could be really good because it can help to put the burden of evaluating expensive predicates to in-memory and leaving easier and more selectively computable predicates to loading work; but when populating Sinks more care might need to be taken (e.g. loading multiple columns twice)

2. We could have more hints about which version of select to use (a or b) first because it isn't obvious/ there isn't a single answer 

3. Case study:
| 6c | R2 | 2.6961 | 0.0062 | 0.0044 | **0.71x** |
| 6d | R2 | 3.0666 | 0.0062 | 0.8244 | 132.97x |

The .with() predicate is selective in different ways:
year > 2014: 438 movies
year > 2000: 1,381,453 movies

```
    let q6c = move || {
        db.movie.with(production_year.gt(2014))
                .select(keyword.eq("marvel-cinematic-universe")
                        .and(title)
                        .and(cast.select(person).select(person_name).rx(r"Downey.*Robert")))
    };```

```let q6d = move || {
        db.movie.with(production_year.gt(2000))
                .select(keyword.is_in(kw8())
                        .and(title)
                        .and(cast.select(person).select(person_name).rx(r"Downey.*Robert")))
    };```



