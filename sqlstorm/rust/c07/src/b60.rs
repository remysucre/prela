use harness::prelude::*;

pub type Counts = DenseFold<Id<User>, (i64, i64, i64, i64, i64, i64, i64)>;

pub fn user_posts(db: &'static So) -> Counts {
    let Post { owner_user, post_type_id, score, view_count, .. } = &db.post;
    owner_user.inv().select(post_type_id.and(score).and(view_count.opt())).dense_fold_outer(
        db.user.id.n,
        (0i64, 0i64, 0i64, 0i64, 0i64, 0i64, 0i64),
        |(n, q, a, s, vn, vs, w), ((t, sc), v)| {
            (n + 1, q + (t == 1) as i64, a + (t == 2) as i64, s + sc, vn + v.is_some() as i64, vs + v.unwrap_or(0), w + (t == 3) as i64)
        },
    )
}

pub type Totals = (i64, i64, i64, i64, i64, i64, i64);
/// Every user with their reputation and post totals, ranked: () -> (((user,
/// rep), totals), rank). The reputation rides in the value so that a ranking
/// by reputation is a function of the value like any other.
pub type Ranked = Window<(), ((Id<User>, i64), Totals), i64>;

pub fn ranked_by<K, O, F>(db: &'static So, t: &Counts, order: O, f: F) -> Ranked
where
    K: Copy + Ord,
    O: Fn(((Id<User>, i64), Totals)) -> K,
    F: Fn(&[(K, ((Id<User>, i64), Totals))], &mut Vec<i64>),
{
    whole(&db.user.id).select(Ident::<User>::new().and(&db.user.reputation).and(t)).window(f, order, desc)
}

pub fn top_by_posts<F>(db: &'static So, t: &Counts, f: F) -> Ranked
where
    F: Fn(&[(i64, ((Id<User>, i64), Totals))], &mut Vec<i64>),
{
    ranked_by(db, t, |(_, a)| a.0, f)
}

pub fn counts_row(db: &'static So, rank: &Ranked, cols: &[&str]) -> String {
    let mut out = Vec::new();
    rank.filt(|(_, r)| r <= 10).drive(|_, (((u, rep), (n, q, a, s, vn, vs, w)), r)| {
        let f = cols
            .iter()
            .map(|c| match *c {
                "id" => V::I(db.user.origid.get(u).unwrap()),
                "name" => V::S(db.user.display_name.get(u).unwrap()),
                "rep" => V::I(rep),
                "views_user" => V::I(db.user.views.get(u).unwrap()),
                "n" => V::I(n),
                "q" => V::I(q),
                "a" => V::I(a),
                "wiki" => V::I(w),
                "score_sum" => nullable(s, n),
                "views_sum" => nullable(vs, vn),
                "score0" => V::I(s),
                "views0" => V::I(vs),
                "avg_score" => avg(s, n),
                "avg_views" => avg(vs, vn),
                "rank" => V::I(r),
                _ => panic!("{c}"),
            })
            .collect();
        out.push(row(f))
    });
    rows(out)
}

fn q11094(db: &'static So) -> String {
    let t = user_posts(db);
    counts_row(db, &top_by_posts(db, &t, row_number), &["name", "rep", "n", "q", "a"])
}

fn q14910(db: &'static So) -> String {
    let t = user_posts(db);
    counts_row(db, &top_by_posts(db, &t, row_number), &["name", "n", "q", "a"])
}

fn q13545(db: &'static So) -> String {
    let t = user_posts(db);
    counts_row(db, &top_by_posts(db, &t, row_number), &["name", "rep", "n", "q", "a"])
}

fn q14967(db: &'static So) -> String {
    let t = user_posts(db);
    counts_row(db, &top_by_posts(db, &t, row_number), &["id", "name", "n", "q", "a"])
}

fn q14508(db: &'static So) -> String {
    let t = user_posts(db);
    counts_row(db, &top_by_posts(db, &t, rank), &["name", "rank", "n", "q", "a"])
}

fn q13580(db: &'static So) -> String {
    let t = user_posts(db);
    counts_row(db, &top_by_posts(db, &t, rank), &["id", "name", "n", "q", "a", "rank"])
}

fn q13842(db: &'static So) -> String {
    let t = user_posts(db);
    counts_row(db, &top_by_posts(db, &t, dense_rank), &["name", "rep", "n", "q", "a"])
}

fn q14696(db: &'static So) -> String {
    let t = user_posts(db);
    counts_row(db, &top_by_posts(db, &t, rank), &["name", "rep", "n", "q", "a"])
}

fn q11549(db: &'static So) -> String {
    let t = user_posts(db);
    counts_row(db, &top_by_posts(db, &t, row_number), &["name", "n", "q", "a"])
}

fn q11752(db: &'static So) -> String {
    let t = user_posts(db);
    counts_row(db, &top_by_posts(db, &t, rank), &["id", "name", "n", "q", "a", "avg_score", "avg_views"])
}

fn q12449(db: &'static So) -> String {
    let posts = (&db.post.owner_user).inv().dense_fold_outer(db.user.id.n, 0i64, |a, _| a + 1);
    let comments = (&db.comment.user).inv().dense_fold_outer(db.user.id.n, 0i64, |a, _| a + 1);
    let rn = whole(&db.user.id)
        .select((&db.user.display_name).and(&db.user.reputation).and(&posts).and(&comments))
        .window(row_number, |(((_, rep), _), _)| rep, desc);
    let mut out = Vec::new();
    (&rn).filt(|(_, n)| n <= 10).drive(
        |_, ((((dn, rep), p), c), _)| out.push(row(vec![V::S(dn), V::I(rep), V::I(p), V::I(c)])),
    );
    rows(out)
}

fn q11674(db: &'static So) -> String {
    let t = user_posts(db);
    let ranked = whole(&db.user.id)
        .select((&db.user.display_name).and(&db.user.reputation).and(&t))
        .window(rank, |(_, t)| t.3, desc)
        .window(rank, |((_, t), _)| t.5, desc);
    let mut out = Vec::new();
    ranked.drive(
        |_, ((((dn, rep), (n, _, _, s, _, vs, _)), a), b)| {
            out.push(row(vec![V::S(dn), V::I(rep), V::I(n), V::I(vs), V::I(s), V::I(a), V::I(b)]))
        },
    );
    rows(out)
}

fn q10650(db: &'static So) -> String {
    let posts = (&db.post.owner_user).inv().dense_fold_outer(db.user.id.n, 0i64, |a, _| a + 1);
    let bc = badges_per_user(db);
    let rk = whole(&db.user.id)
        .select((&db.user.origid).and(&db.user.display_name).and(&db.user.reputation).and(&posts).and(&bc))
        .window(rank, |((((_, _), rep), _), _)| rep, desc);
    let mut out = Vec::new();
    (&rk)
        .filt(|(_, r)| r <= 10)
        .drive(|_, (((((id, dn), rep), p), b), _)| out.push(row(vec![V::I(id), V::S(dn), V::I(rep), V::I(p), V::I(b)])));
    rows(out)
}

fn q10044(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let t = db
        .user
        .select(posts_of(db).select(score.and(view_count.opt())).opt())
        .dense_fold_outer(db.user.id.n, (0i64, 0i64, 0i64, 0i64), |(rows_, n, s, vs), p| {
            (rows_ + 1, n + p.is_some() as i64, s + p.map_or(0, |(sc, _)| sc), vs + p.and_then(|(_, v)| v).unwrap_or(0))
        });
    let rn = whole(&db.user.id)
        .select((&db.user.origid).and(&db.user.display_name).and(&t))
        .window(row_number, |(_, a)| a.2, desc);
    let mut out = Vec::new();
    (&rn).filt(|(_, r)| r <= 10).drive(
        |_, (((id, dn), (rows_, n, s, vs)), _)| {
            out.push(row(vec![V::I(id), V::S(dn), V::I(n), V::I(s), V::I(vs), V::F(s as f64 / rows_ as f64)]))
        },
    );
    rows(out)
}

pub const ENTRIES: &[harness::Entry] = &[
    ("11094", q11094),
    ("12449", q12449),
    ("14910", q14910),
    ("11674", q11674),
    ("13545", q13545),
    ("14967", q14967),
    ("14508", q14508),
    ("13580", q13580),
    ("13842", q13842),
    ("14696", q14696),
    ("10650", q10650),
    ("11549", q11549),
    ("11752", q11752),
    ("10044", q10044),
];
