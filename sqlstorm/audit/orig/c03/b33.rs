use harness::prelude::*;

// ---- Users LEFT JOIN Posts ----------------------------------------------

struct UAgg {
    id: i64,
    display_name: Str,
    reputation: i64,
    n: i64,
    score_sum: i64,
    last_created: i64,
    views_n: i64,
    views_sum: i64,
}

fn user_aggs(db: &'static So) -> Vec<UAgg> {
    let Post { owner_user, score, creation_date, view_count, .. } = &db.post;
    let User { origid, display_name, reputation, .. } = &db.user;
    let nu = db.user.id.n;

    let sc = owner_user
        .inv()
        .select(score.and(creation_date))
        .dense_fold_outer(nu, (0i64, 0i64, i64::MIN), |(c, s, m), (x, d)| {
            (c + 1, s + x, m.max(d))
        });
    let vc = owner_user
        .inv()
        .select(view_count)
        .dense_fold_outer(nu, (0i64, 0i64), |(c, s), x| (c + 1, s + x));

    let mut v = Vec::new();
    db.user
        .select(origid.and(display_name).and(reputation).and(sc).and(vc))
        .drive(
            |_, ((((id, dn), rep), (n, score_sum, last_created)), (views_n, views_sum))| {
                v.push(UAgg {
                    id,
                    display_name: dn,
                    reputation: rep,
                    n,
                    score_sum,
                    last_created,
                    views_n,
                    views_sum,
                })
            },
        );
    v
}

fn q14865(db: &'static So) -> String {
    let mut v = user_aggs(db);
    v.sort_by(|a, b| b.reputation.cmp(&a.reputation));
    rows(v.iter().take(10).map(|r| {
        row(vec![
            V::I(r.id),
            V::S(r.display_name),
            V::I(r.reputation),
            V::I(r.n),
            nullable(r.score_sum, r.n),
        ])
    }))
}

fn q14763(db: &'static So) -> String {
    let mut v = user_aggs(db);
    v.sort_by(|a, b| b.reputation.cmp(&a.reputation).then(b.n.cmp(&a.n)));
    rows(v.iter().take(10).map(|r| {
        row(vec![
            V::S(r.display_name),
            V::I(r.reputation),
            V::I(r.n),
            avg(r.score_sum, r.n),
        ])
    }))
}

fn q13446(db: &'static So) -> String {
    let mut v = user_aggs(db);
    v.sort_by(|a, b| b.n.cmp(&a.n));
    rows(v.iter().take(10).map(|r| {
        row(vec![
            V::I(r.id),
            V::S(r.display_name),
            V::I(r.n),
            avg(r.score_sum, r.n),
            V::I(r.reputation),
        ])
    }))
}

fn q11541(db: &'static So) -> String {
    let mut v = user_aggs(db);
    v.sort_by(|a, b| b.n.cmp(&a.n));
    rows(v.iter().take(100).map(|r| {
        row(vec![
            V::I(r.id),
            V::S(r.display_name),
            V::I(r.n),
            avg(r.score_sum, r.n),
            nullable(r.views_sum, r.views_n),
        ])
    }))
}

fn q13761(db: &'static So) -> String {
    let v = user_aggs(db);
    rows(v.iter().map(|r| {
        row(vec![
            V::I(r.id),
            V::S(r.display_name),
            V::I(r.n),
            avg(r.views_sum, r.views_n),
            avg(r.score_sum, r.n),
        ])
    }))
}

fn q12891(db: &'static So) -> String {
    let v = user_aggs(db);
    rows(v.iter().map(|r| {
        row(vec![
            V::I(r.id),
            V::S(r.display_name),
            V::I(r.n),
            avg(r.score_sum, r.n),
            if r.n == 0 { V::Null } else { V::T(r.last_created) },
        ])
    }))
}

// ---- Questions LEFT JOIN Comments, GROUP BY p.Id ------------------------

struct QRow {
    pid: Id<Post>,
    id: i64,
    display_name: Str,
    created: i64,
    comments: i64,
}

fn question_comment_rows(db: &'static So) -> Vec<QRow> {
    let Post { post_type_id, origid, creation_date, owner_user, .. } = &db.post;
    let User { display_name, .. } = &db.user;

    let cc = (&db.comment.post)
        .inv()
        .dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1);

    let mut v = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .select(origid.and(creation_date).and(owner_user.select(display_name)).and(cc))
        .drive(|pid, (((id, created), dn), comments)| {
            v.push(QRow { pid, id, display_name: dn, created, comments })
        });
    v
}

fn by_created(db: &'static So) -> Vec<QRow> {
    let mut v = question_comment_rows(db);
    v.sort_by(|a, b| b.created.cmp(&a.created));
    v.truncate(10);
    v
}

/// PostId, Title, CreationDate, DisplayName, CommentCount
fn q17140(db: &'static So) -> String {
    let v = question_comment_rows(db);
    rows(v.iter().map(|r| {
        row(vec![
            V::I(r.id),
            title(db, r.pid),
            V::T(r.created),
            V::S(r.display_name),
            V::I(r.comments),
        ])
    }))
}

fn q15027(db: &'static So) -> String {
    let v = by_created(db);
    rows(v.iter().map(|r| {
        row(vec![
            V::I(r.id),
            title(db, r.pid),
            V::T(r.created),
            V::S(r.display_name),
            V::I(r.comments),
        ])
    }))
}

/// PostId, Title, DisplayName, CreationDate, CommentCount
fn owner_before_date(db: &'static So) -> String {
    let v = by_created(db);
    rows(v.iter().map(|r| {
        row(vec![
            V::I(r.id),
            title(db, r.pid),
            V::S(r.display_name),
            V::T(r.created),
            V::I(r.comments),
        ])
    }))
}

/// `Posts JOIN PostTypes LEFT JOIN Votes` — AVG(p.ViewCount) is averaged
/// over the JOINED rows, and skips the posts whose ViewCount is null, so it
/// needs its own pass with its own divisor.
fn q13091(db: &'static So) -> String {
    let Post { post_type, view_count, .. } = &db.post;
    let PostType { name, .. } = &db.post_type;

    let mut tot: Vec<(Str, i64, i64, i64, i64, i64)> = Vec::new();
    db.post
        .group_by(post_type.select(name))
        .select(view_count.opt().and(votes_of(db).select(&db.vote.origid).opt()))
        .fold((0i64, 0i64, 0i64, 0i64, 0i64), |(n, wn, ws, vc, vs), (w, id)| {
            (n + 1, wn + w.is_some() as i64, ws + w.unwrap_or(0), vc + id.is_some() as i64, vs + id.unwrap_or(0))
        })
        .drive(|k, (n, wn, ws, vc, vs)| tot.push((k, n, wn, ws, vc, vs)));

    rows(tot.iter().map(|(k, n, wn, ws, vc, vs)| {
        row(vec![V::S(k), V::I(*n), avg(*ws, *wn), nullable(*vs, *vc)])
    }))
}

// SELECT P.Title, P.CreationDate, U.DisplayName AS OwnerDisplayName, C.Score AS CommentScore FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id
// LEFT JOIN Comments C ON P.Id = C.PostId WHERE P.PostTypeId = 1 ORDER BY P.CreationDate DESC LIMIT 10;
fn q17445(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let ci: HashIdx<Id<Post>, i64> = (&db.comment.post).inv().select(&db.comment.score).collect();
    let v = drain(db.post.with(post_type_id.eq(1)).with(owner_user).select(creation_date.and((&ci).opt())));
    let v = top_n(v, |&(_, (d, _))| std::cmp::Reverse(d), 10);
    rows(v.into_iter().map(|(p, (_, s))| {
        let mut f = post_fields(db, p, &["title", "created", "owner"]);
        f.push(oint(s));
        row(f)
    }))
}

pub const ENTRIES: &[harness::Entry] = &[
    ("14865", q14865),
    ("17140", q17140),
    ("13761", q13761),
    ("14763", q14763),
    ("13446", q13446),
    ("12891", q12891),
    ("15014", owner_before_date),
    ("15551", owner_before_date),
    ("15027", q15027),
    ("17445", q17445),
    ("13091", q13091),
    ("11541", q11541),
];
