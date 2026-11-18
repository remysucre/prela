use prela::engine::*;

fn main() {
    // the movie "entity" itself: an identity relation over ids 0..3
    // (D=R=id) — this is what a schema's #[primary_key] field gives you.
    let movie: Universe<usize> = Universe::new(3);

    // movie id -> title
    let title: VecRel<usize, &str> =
        VecRel::from_pairs(3, [(0, "Jaws"), (1, "Alien"), (2, "Tron")]);
    // movie id -> year
    let year: VecRel<usize, i64> =
        VecRel::from_pairs(3, [(0, 1975), (1, 1979), (2, 1982)]);
    // movie id -> zero or more keyword ids (CSR / multi-valued)
    let movie_keyword: MultiRel<usize, usize> =
        MultiRel::from_pairs(3, [(0, 0), (0, 1), (1, 1), (2, 2)]);
    // keyword id -> keyword text
    let keyword: VecRel<usize, &str> =
        VecRel::from_pairs(3, [(0, "shark"), (1, "space"), (2, "future")]);

    // 1. a scan
    println!("-- every movie's title --");
    title.drive(|id, t| println!("  {id}: {t}"));

    // 2. a filter: movie.with(pred) is a semijoin — keep ids where the
    // predicate holds, then select reads title off the survivors.
    println!("\n-- movies after 1978 --");
    movie
        .with(year.gt(1978))
        .select(&title)
        .drive(|id, t| println!("  {id}: {t}"));

    // 3. a join: select is Compose — stream movie_keyword, PROBE keyword.
    println!("\n-- movie -> keyword text (a join) --");
    (&movie_keyword)
        .select(&keyword)
        .drive(|movie_id, kw| println!("  movie {movie_id} has keyword {kw:?}"));

    // 4. zip title next to the join result on movie id
    println!("\n-- title + keywords, zipped on movie id --");
    (&title)
        .and((&movie_keyword).select(&keyword))
        .drive(|id, (t, kw)| println!("  {id}: {t} / {kw}"));

    // 5. materialize explicitly: HashIdx builds a hash-join build side,
    // then you can probe it directly.
    let by_keyword: HashIdx<usize, usize> = (&movie_keyword).inv().collect();
    println!("\n-- which movies have keyword 1 (\"space\")? --");
    by_keyword.probe(1, |movie_id| println!("  movie {movie_id}"));
}
