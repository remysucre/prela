use harness::prelude::*;

struct Agg {
    name: Str,
    n: i64,
    score_sum: i64,
    score_max: i64,
    pos: i64,
    nonpos: i64,
    comment_sum: i64,
    views_n: i64,
    views_sum: i64,
    answers_n: i64,
    answers_sum: i64,
    created_max: i64,
}

fn per_type_since(db: &'static So, from: i64) -> Vec<Agg> {
    let Post {
        post_type, creation_date, score, view_count, comment_count, answer_count, ..
    } = &db.post;
    let PostType { name, .. } = &db.post_type;

    let mut out: Vec<Agg> = Vec::new();
    db.post
        .with(creation_date.ge(from))
        .group_by(post_type.select(name))
        .select(
            score
                .and(comment_count)
                .and(creation_date)
                .and(view_count.opt())
                .and(answer_count.opt()),
        )
        .fold(
            (0i64, 0i64, i64::MIN, 0i64, 0i64, 0i64, i64::MIN, 0i64, 0i64, 0i64, 0i64),
            |(n, s, hi, pos, nonpos, cc, cd, vn, vs, an, asum), ((((x, c), d), v), a)| {
                (
                    n + 1,
                    s + x,
                    hi.max(x),
                    pos + (x > 0) as i64,
                    nonpos + (x <= 0) as i64,
                    cc + c,
                    cd.max(d),
                    vn + v.is_some() as i64,
                    vs + v.unwrap_or(0),
                    an + a.is_some() as i64,
                    asum + a.unwrap_or(0),
                )
            },
        )
        .drive(|k, (n, s, hi, pos, nonpos, cc, cd, vn, vs, an, asum)| {
            out.push(Agg {
                name: k,
                n,
                score_sum: s,
                score_max: hi,
                pos,
                nonpos,
                comment_sum: cc,
                views_n: vn,
                views_sum: vs,
                answers_n: an,
                answers_sum: asum,
                created_max: cd,
            })
        });
    out.sort_by(|a, b| b.n.cmp(&a.n));
    out
}

struct OAgg {
    name: Str,
    n: i64,
    score_sum: i64,
    rep_sum: i64,
    views_n: i64,
    views_sum: i64,
    users: i64,
}

// Posts (created since `from`, owner reputation > `min_rep`) JOIN PostTypes JOIN Users grouped by pt.Name, with COUNT(DISTINCT p.OwnerUserId) joined on the type name.
fn owned_since(db: &'static So, from: i64, min_rep: i64) -> Vec<OAgg> {
    let Post { creation_date, score, view_count, owner_user, owner_user_id, .. } = &db.post;
    let reputation = &db.user.reputation;
    let base = || db.post.with(creation_date.ge(from)).with(owner_user.select(reputation).gt(min_rep));
    let sc = base()
        .group_by(ptype_name(db))
        .select(score.and(owner_user.select(reputation)).and(view_count.opt()))
        .fold([0i64; 5], |a, ((x, rep), v)| [a[0] + 1, a[1] + x, a[2] + rep, a[3] + v.is_some() as i64, a[4] + v.unwrap_or(0)]);
    let du = base().group_by(ptype_name(db)).select(owner_user_id).count_distinct();
    let mut out: Vec<OAgg> = drain((&sc).and((&du).opt()))
        .into_iter()
        .map(|(k, (a, u))| OAgg { name: k, n: a[0], score_sum: a[1], rep_sum: a[2], views_n: a[3], views_sum: a[4], users: u.unwrap_or(0) })
        .collect();
    out.sort_by(|a, b| b.n.cmp(&a.n));
    out
}

const YEAR_TS: fn() -> i64 = || ts(2023, 10, 1, 12, 34, 56);
const YEAR_D: fn() -> i64 = || date(2023, 10, 1);

fn q13502(db: &'static So) -> String {
    rows(per_type_since(db, i64::MIN).iter().map(|a| {
        row(vec![
            V::S(a.name),
            V::I(a.n),
            avg(a.views_sum, a.views_n),
            V::I(a.pos),
            V::I(a.nonpos),
        ])
    }))
}

fn q13058(db: &'static So) -> String {
    rows(per_type_since(db, i64::MIN).iter().map(|a| {
        row(vec![
            V::S(a.name),
            V::I(a.n),
            V::I(a.pos),
            nullable(a.views_sum, a.views_n),
            avg(a.comment_sum, a.n),
            avg(a.answers_sum, a.answers_n),
        ])
    }))
}

fn q14048(db: &'static So) -> String {
    rows(per_type_since(db, YEAR_D()).iter().map(|a| {
        row(vec![
            V::S(a.name),
            V::I(a.n),
            avg(a.score_sum, a.n),
            nullable(a.views_sum, a.views_n),
            V::T(a.created_max),
        ])
    }))
}

fn q13935(db: &'static So) -> String {
    let mut a = per_type_since(db, YEAR_TS());
    a.sort_by(|x, y| {
        let f = |t: &Agg| t.score_sum as f64 / t.n as f64;
        f(y).partial_cmp(&f(x)).unwrap()
    });
    rows(a.iter().map(|a| {
        row(vec![
            V::S(a.name),
            avg(a.score_sum, a.n),
            avg(a.comment_sum, a.n),
            avg(a.views_sum, a.views_n),
            V::I(a.n),
        ])
    }))
}

fn q12934(db: &'static So) -> String {
    let mut a = per_type_since(db, YEAR_TS());
    a.sort_by_key(|t| t.name);
    rows(a.iter().map(|a| {
        row(vec![
            V::S(a.name),
            V::I(a.n),
            avg(a.views_sum, a.views_n),
            avg(a.score_sum, a.n),
            avg(a.comment_sum, a.n),
            V::I(a.score_max),
        ])
    }))
}

fn q10075(db: &'static So) -> String {
    rows(owned_since(db, date(2023, 1, 1), i64::MIN).iter().map(|a| {
        row(vec![
            V::S(a.name),
            V::I(a.n),
            avg(a.score_sum, a.n),
            nullable(a.views_sum, a.views_n),
            V::I(a.users),
        ])
    }))
}

fn q14855(db: &'static So) -> String {
    rows(owned_since(db, i64::MIN, 0).iter().map(|a| {
        row(vec![
            V::S(a.name),
            V::I(a.n),
            avg(a.score_sum, a.n),
            avg(a.views_sum, a.views_n),
            V::I(a.users),
        ])
    }))
}

fn q13092(db: &'static So) -> String {
    rows(owned_since(db, i64::MIN, i64::MIN).iter().map(|a| {
        row(vec![
            V::S(a.name),
            V::I(a.n),
            avg(a.score_sum, a.n),
            avg(a.views_sum, a.views_n),
            V::I(a.users),
            avg(a.rep_sum, a.n),
        ])
    }))
}

fn q12785(db: &'static So) -> String {
    let mut a = owned_since(db, YEAR_TS(), i64::MIN);
    a.sort_by(|x, y| {
        let f = |t: &OAgg| t.score_sum as f64 / t.n as f64;
        f(y).partial_cmp(&f(x)).unwrap()
    });
    rows(a.iter().map(|a| {
        row(vec![
            V::S(a.name),
            avg(a.score_sum, a.n),
            avg(a.rep_sum, a.n),
            V::I(a.n),
        ])
    }))
}

fn q10268(db: &'static So) -> String {
    let PostHistory { post_history_type_id, post, creation_date, .. } = &db.post_history;
    let Post { creation_date: pcd, .. } = &db.post;

    let mut all: Vec<(i64, i64, i64, i64)> = Vec::new();
    db.post_history
        .with(post.select(pcd).ge(YEAR_TS()))
        .group_by(post_history_type_id)
        .select(creation_date)
        .fold((0i64, i64::MAX, i64::MIN), |(n, lo, hi), x| {
            (n + 1, lo.min(x), hi.max(x))
        })
        .drive(|k, (n, lo, hi)| all.push((k, n, lo, hi)));
    all.sort_by(|a, b| b.1.cmp(&a.1));
    rows(all.iter().map(|(k, n, lo, hi)| {
        row(vec![V::I(*k), V::I(*n), V::T(*lo), V::T(*hi)])
    }))
}

fn q16267(db: &'static So) -> String {
    let Post { owner_user, post_type_id, .. } = &db.post;
    let User { display_name, reputation, .. } = &db.user;

    let mut all: Vec<(Str, i64, i64, i64, i64)> = Vec::new();
    db.post
        .with(owner_user)
        .group_by(owner_user.select(display_name))
        .select(post_type_id.and(owner_user.select(reputation)))
        .fold((0i64, 0i64, 0i64, 0i64), |(n, q, a, r), (ty, rep)| {
            (n + 1, q + (ty == 1) as i64, a + (ty == 2) as i64, r + rep)
        })
        .drive(|k, (n, q, a, r)| all.push((k, n, q, a, r)));
    all.sort_by(|a, b| b.1.cmp(&a.1));
    rows(all.iter().take(10).map(|(dn, n, q, a, r)| {
        row(vec![V::S(dn), V::I(*n), V::I(*q), V::I(*a), avg(*r, *n)])
    }))
}

fn q13342(db: &'static So) -> String {
    let Post { title: ptitle, creation_date: pcd, .. } = &db.post;
    let PostHistory {
        post, post_id, post_history_type_id, creation_date, user, comment, text, ..
    } = &db.post_history;
    let User { display_name, .. } = &db.user;

    let mut v: Vec<(Id<PostHistory>, Id<Post>, i64, i64, i64, Str)> = Vec::new();
    db.post_history
        .with(creation_date.ge(ts(2024, 9, 1, 12, 34, 56)))
        .select(
            post.and(post_id)
                .and(post_history_type_id)
                .and(creation_date)
                .and(user.select(display_name)),
        )
        .drive(|h, ((((p, pi), ty), cd), dn)| v.push((h, p, pi, ty, cd, dn)));
    v.sort_by(|a, b| b.4.cmp(&a.4));
    rows(v.iter().map(|(h, p, pi, ty, cd, dn)| {
        row(vec![
            V::I(*pi),
            ostr(ptitle.get(*p)),
            V::I(*ty),
            V::T(pcd.get(*p).unwrap()),
            V::T(*cd),
            V::S(dn),
            ostr(comment.get(*h)),
            ostr(text.get(*h)),
        ])
    }))
}

pub const ENTRIES: &[harness::Entry] = &[
    ("13502", q13502),
    ("14048", q14048),
    ("10268", q10268),
    ("10075", q10075),
    ("13058", q13058),
    ("14855", q14855),
    ("13092", q13092),
    ("16267", q16267),
    ("13935", q13935),
    ("12785", q12785),
    ("13342", q13342),
    ("12934", q12934),
];
