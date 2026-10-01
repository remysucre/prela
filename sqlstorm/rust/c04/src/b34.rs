use harness::prelude::*;

// ---- Users LEFT JOIN Posts ----------------------------------------------

struct UAgg {
    id: i64,
    display_name: Str,
    reputation: i64,
    n: i64,
    score_sum: i64,
}

fn user_aggs(db: &'static So) -> Vec<UAgg> {
    let Post { owner_user, score, .. } = &db.post;
    let User { origid, display_name, reputation, .. } = &db.user;

    let sc = owner_user
        .inv()
        .select(score)
        .dense_fold_outer(db.user.id.n, (0i64, 0i64), |(c, s), x| (c + 1, s + x));

    let mut v = Vec::new();
    db.user
        .select(origid.and(display_name).and(reputation).and(sc))
        .drive(|_, (((id, dn), rep), (n, score_sum))| {
            v.push(UAgg { id, display_name: dn, reputation: rep, n, score_sum })
        });
    v
}

fn q10921(db: &'static So) -> String {
    let mut v = user_aggs(db);
    v.sort_by(|a, b| b.reputation.cmp(&a.reputation));
    rows(v.iter().take(10).map(|r| {
        row(vec![
            V::I(r.id),
            V::S(r.display_name),
            V::I(r.reputation),
            V::I(r.n),
            avg(r.score_sum, r.n),
        ])
    }))
}

/// UserId, DisplayName, Reputation, PostCount, AveragePostScore
fn q11773(db: &'static So) -> String {
    let v = user_aggs(db);
    rows(v.iter().map(|r| {
        row(vec![
            V::I(r.id),
            V::S(r.display_name),
            V::I(r.reputation),
            V::I(r.n),
            avg(r.score_sum, r.n),
        ])
    }))
}

/// The same aggregate with Reputation last.
fn q12987(db: &'static So) -> String {
    let v = user_aggs(db);
    rows(v.iter().map(|r| {
        row(vec![
            V::I(r.id),
            V::S(r.display_name),
            V::I(r.n),
            avg(r.score_sum, r.n),
            V::I(r.reputation),
        ])
    }))
}

// ---- ... LEFT JOIN Votes ON p.Id = v.PostId AND v.VoteTypeId = 8 --------

/// Per user: joined row count (each post multiplied by its bounty-start
/// votes, or kept once), and the bounty sum with its own non-null count.
fn bounty_by_user(db: &'static So) -> DenseFold<Id<User>, (i64, i64, i64)> {
    let Post { owner_user, .. } = &db.post;
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    owner_user.inv().select(votes_of(db).with(vote_type_id.eq(8)).select(bounty_amount.opt()).opt()).dense_fold_outer(
        db.user.id.n,
        (0i64, 0i64, 0i64),
        |(c, bn, bs), b| {
            let b = b.flatten();
            (c + 1, bn + b.is_some() as i64, bs + b.unwrap_or(0))
        },
    )
}

fn q16628(db: &'static So) -> String {
    let User { display_name, .. } = &db.user;
    let per_user = bounty_by_user(db);

    let mut v: Vec<(Str, i64, i64, i64)> = Vec::new();
    db.user
        .group_by(display_name)
        .select(&per_user)
        .fold((0i64, 0i64, 0i64), |(c, bn, bs), (x, y, z)| (c + x, bn + y, bs + z))
        .drive(|dn, (c, bn, bs)| v.push((dn, c, bn, bs)));

    v.sort_by(|a, b| b.1.cmp(&a.1));
    rows(v.iter().take(10).map(|(dn, c, bn, bs)| {
        row(vec![V::S(dn), V::I(*c), nullable(*bs, *bn)])
    }))
}

fn q16071(db: &'static So) -> String {
    let User { display_name, reputation, .. } = &db.user;
    let per_user = bounty_by_user(db);

    let mut v: Vec<(Str, i64, i64, i64)> = Vec::new();
    db.user
        .with(reputation.gt(1000))
        .group_by(display_name)
        .select(&per_user)
        .fold((0i64, 0i64, 0i64), |(c, bn, bs), (x, y, z)| (c + x, bn + y, bs + z))
        .drive(|dn, (c, bn, bs)| v.push((dn, c, bn, bs)));

    let v = top_n(v, |&(_, c, bn, bs)| (std::cmp::Reverse(c), bn == 0, std::cmp::Reverse(bs)), 10);
    rows(v.iter().map(|(dn, c, bn, bs)| {
        row(vec![V::S(dn), V::I(*c), nullable(*bs, *bn)])
    }))
}

// ---- Posts LEFT JOIN PostTypes, then LEFT JOIN Votes / Comments ---------
//
// The second join multiplies each post by its votes (or comments), so
// COUNT(p.Id) and AVG(p.Score) are over the JOINED rows.

/// Per post type: joined rows, score sum over them, and how many rows came
/// from a real child row.
fn type_rollup<C: Probe<D = Id<Post>>>(db: &'static So, children: C) -> Vec<(Str, i64, i64, i64)> {
    let Post { post_type, score, .. } = &db.post;
    let PostType { name, .. } = &db.post_type;

    let mut v = Vec::new();
    db.post
        .group_by(post_type.select(name))
        .select(score.and(children.opt()))
        .fold((0i64, 0i64, 0i64), |(n, ss, ch), (sc, c)| (n + 1, ss + sc, ch + c.is_some() as i64))
        .drive(|k, (n, ss, ch)| v.push((k, n, ss, ch)));
    v
}

fn comments_per_post(db: &'static So) -> DenseFold<Id<Post>, i64> {
    (&db.comment.post).inv().dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1)
}

fn q13553(db: &'static So) -> String {
    let v = type_rollup(db, votes_of(db));
    rows(v.iter().map(|(k, n, ss, ch)| {
        row(vec![V::S(k), V::I(*n), avg(*ss, *n), V::I(*ch)])
    }))
}

fn q10818(db: &'static So) -> String {
    let v = type_rollup(db, comments_of(db));
    rows(v.iter().map(|(k, n, ss, ch)| {
        row(vec![V::S(k), V::I(*n), avg(*ss, *n), V::I(*ch)])
    }))
}

fn q12067(db: &'static So) -> String {
    let Post { post_type, score, view_count, .. } = &db.post;
    let PostType { name, .. } = &db.post_type;

    let mut v: Vec<(Str, i64, i64, i64, i64)> = Vec::new();
    db.post
        .group_by(post_type.select(name))
        .select(score.and(view_count.opt()).and(votes_of(db).opt()))
        .fold((0i64, 0i64, 0i64, 0i64), |(n, ss, wn, ws), ((sc, w), _)| {
            (n + 1, ss + sc, wn + w.is_some() as i64, ws + w.unwrap_or(0))
        })
        .drive(|k, (n, ss, wn, ws)| v.push((k, n, ss, wn, ws)));

    rows(v.iter().map(|(k, n, ss, wn, ws)| {
        row(vec![V::S(k), V::I(*n), avg(*ss, *n), nullable(*ws, *wn)])
    }))
}

struct QRow {
    pid: Id<Post>,
    id: i64,
    created: i64,
    score: i64,
    comments: i64,
}

/// `owned` is the `JOIN Users` — without it a question with no owner stays.
fn question_comment_rows(db: &'static So, owned: bool) -> Vec<QRow> {
    let Post { post_type_id, origid, creation_date, score, owner_user, .. } = &db.post;

    let cc = comments_per_post(db);
    let mut v = Vec::new();
    let proj = origid.and(creation_date).and(score).and(cc);
    let mut push = |pid: Id<Post>, (((id, created), score), comments)| {
        v.push(QRow { pid, id, created, score, comments })
    };
    if owned {
        db.post.with(post_type_id.eq(1)).with(owner_user).select(proj).drive(&mut push);
    } else {
        db.post.with(post_type_id.eq(1)).select(proj).drive(&mut push);
    }
    v
}

fn q16510(db: &'static So) -> String {
    let mut v = question_comment_rows(db, false);
    v.sort_by(|a, b| b.created.cmp(&a.created));
    rows(v.iter().take(10).map(|r| {
        row(vec![
            V::I(r.id),
            title(db, r.pid),
            V::T(r.created),
            V::I(r.score),
            V::I(r.comments),
        ])
    }))
}

fn q15284(db: &'static So) -> String {
    let owner_user = &db.post.owner_user;
    let display_name = &db.user.display_name;

    let mut v = question_comment_rows(db, true);
    v.sort_by(|a, b| b.score.cmp(&a.score));
    rows(v.iter().take(10).map(|r| {
        row(vec![
            V::I(r.id),
            title(db, r.pid),
            V::T(r.created),
            ostr((&owner_user.select(display_name)).get(r.pid)),
            V::I(r.score),
            V::I(r.comments),
        ])
    }))
}

// SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS UserName, t.TagName FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
// LEFT JOIN Tags t ON t.ExcerptPostId = p.Id WHERE p.PostTypeId = 1 ORDER BY p.CreationDate DESC LIMIT 10;
fn q15743(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let ti_idx: HashIdx<Id<Post>, Str> = (&db.tag.excerpt_post).inv().select(&db.tag.tag_name).collect();
    let v = drain(db.post.with(post_type_id.eq(1)).with(owner_user).select(creation_date.and((&ti_idx).opt())));
    let v = top_n(v, |&(_, (d, _))| std::cmp::Reverse(d), 10);
    rows(v.into_iter().map(|(p, (_, t))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.push(ostr(t));
        row(f)
    }))
}

fn q18045(db: &'static So) -> String {
    let Post { creation_date, title: pt, owner_user, .. } = &db.post;
    let User { display_name, .. } = &db.user;
    let g = db
        .post
        .with(creation_date.ge(date(2023, 1, 1)))
        .group_by(owner_user.select(display_name).and(pt.opt()))
        .select(comments_of(db).opt())
        .fold(0i64, |a, c| a + c.is_some() as i64);
    let v = top_n(drain(&g), |&(_, n)| std::cmp::Reverse(n), 10);
    rows(v.into_iter().map(|((dn, ti), n)| row(vec![V::S(dn), ostr(ti), V::I(n)])))
}

pub const ENTRIES: &[harness::Entry] = &[
    ("10921", q10921),
    ("13553", q13553),
    ("10818", q10818),
    ("11773", q11773),
    ("16628", q16628),
    ("12067", q12067),
    ("16510", q16510),
    ("15743", q15743),
    ("12987", q12987),
    ("16071", q16071),
    ("15284", q15284),
    ("18045", q18045),
];
