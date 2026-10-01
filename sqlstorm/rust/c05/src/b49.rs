use harness::prelude::*;

fn tagged_questions(db: &'static So, key: fn(&'static So, Id<Post>) -> i64, cols: &[&str]) -> String {
    let Post { post_type_id, origid, .. } = &db.post;
    let mentions = tag_mentions(db);
    let tags_by_post: HashIdx<Id<Post>, Id<Tag>> = (&mentions).map(|(p, _)| p).inv().select((&mentions).map(|(_, t)| t)).collect();
    let mut v = Vec::new();
    owned(db).with(post_type_id.eq(1)).select(origid.and(&tags_by_post)).drive(|p, (id, t)| {
        v.push((key(db, p), id, db.tag.tag_name.get(t).unwrap(), p))
    });
    v.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)).then(a.2.cmp(b.2)));
    rows(v.iter().take(10).map(|&(_, _, name, p)| {
        let mut f = post_fields(db, p, cols);
        f.push(V::S(name));
        row(f)
    }))
}

fn by_score(db: &'static So, p: Id<Post>) -> i64 {
    db.post.score.get(p).unwrap()
}

fn by_created(db: &'static So, p: Id<Post>) -> i64 {
    db.post.creation_date.get(p).unwrap()
}

fn q19637(db: &'static So) -> String {
    tagged_questions(db, by_score, &["title", "created", "owner"])
}

fn q15835(db: &'static So) -> String {
    tagged_questions(db, by_created, &["title", "created", "owner"])
}

fn q16507(db: &'static So) -> String {
    tagged_questions(db, by_created, &["title", "created", "owner"])
}

fn q19034(db: &'static So) -> String {
    tagged_questions(db, by_created, &["id", "title", "created", "owner"])
}

fn q15017(db: &'static So) -> String {
    tagged_questions(db, by_created, &["id", "title", "created", "owner"])
}

fn q19692(db: &'static So) -> String {
    tagged_questions(db, by_created, &["title", "owner", "created", "score", "views"])
}

fn q15136(db: &'static So) -> String {
    tagged_questions(db, by_created, &["title", "created", "owner", "score", "views"])
}

fn q15838(db: &'static So) -> String {
    tagged_questions(db, by_created, &["owner", "title", "created", "score"])
}

fn q16290(db: &'static So) -> String {
    tagged_questions(db, by_created, &["id", "title", "created", "owner", "score", "views"])
}

fn q19002(db: &'static So) -> String {
    tagged_questions(db, by_created, &["id", "title", "created", "owner", "score", "views"])
}

fn q19176(db: &'static So) -> String {
    tagged_questions(db, by_created, &["id", "title", "owner", "created", "score", "views", "answers", "comments"])
}

fn q19100(db: &'static So) -> String {
    tagged_questions(db, by_created, &["id", "title", "created", "owner", "score", "views", "answers", "comments"])
}

fn q12701(db: &'static So) -> String {
    let Post { post_type_id, tags_str, .. } = &db.post;
    let pieces: MatSet<(Id<Post>, Str)> = db.post.select(Ident::<Post>::new().and(tags_str.flat_map(|t: Str| t.split("<>")))).collect();
    let pieces_of: HashIdx<Id<Post>, Str> = (&pieces).map(|(p, _)| p).inv().select((&pieces).map(|(_, s)| s)).collect();
    let by_name: HashIdx<Str, Id<Tag>> = (&db.tag.tag_name).inv().collect();
    let agg = db
        .post
        .with(post_type_id.eq(1))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()).and((&pieces_of).select((&by_name).opt()).opt()))
        .buf_fold(|rs| {
            let names: Vec<Str> = rs.iter().filter_map(|&(_, t)| t.flatten()).map(|t| db.tag.tag_name.get(t).unwrap()).collect();
            let joined: Option<Str> = (!names.is_empty()).then(|| &*Box::leak(names.join(", ").into_boxed_str()));
            (rs.iter().filter(|((c, _), _)| c.is_some()).count() as i64, rs.iter().filter(|((_, v), _)| v.is_some()).count() as i64, joined)
        });
    let mut out = Vec::new();
    (&agg).drive(|p, (c, v, tags)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "rep"]);
        f.extend([V::I(c), V::I(v), ostr(tags)]);
        out.push(row(f))
    });
    rows(out)
}

pub const ENTRIES: &[harness::Entry] = &[
    ("19637", q19637),
    ("15835", q15835),
    ("16507", q16507),
    ("19034", q19034),
    ("15017", q15017),
    ("19692", q19692),
    ("15136", q15136),
    ("15838", q15838),
    ("16290", q16290),
    ("19002", q19002),
    ("19176", q19176),
    ("19100", q19100),
    ("12701", q12701),
];
