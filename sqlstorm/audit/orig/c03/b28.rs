use harness::prelude::*;

#[derive(Clone, Copy)]
struct Wide {
    name: Str,
    n: i64,
    score_sum: i64,
    views_n: i64,
    views_sum: i64,
    answers_n: i64,
    answers_sum: i64,
    q_answers: i64,
    q_comments: i64,
    comment_sum: i64,
    fav_n: i64,
    fav_sum: i64,
    last_activity: i64,
    elapsed: i128,
}

fn wide(db: &'static So) -> Vec<Wide> {
    let Post {
        post_type, post_type_id, score, view_count, comment_count, answer_count,
        favorite_count, creation_date, last_activity_date, ..
    } = &db.post;
    let PostType { name, .. } = &db.post_type;

    let init = Wide {
        name: "",
        n: 0,
        score_sum: 0,
        views_n: 0,
        views_sum: 0,
        answers_n: 0,
        answers_sum: 0,
        q_answers: 0,
        q_comments: 0,
        comment_sum: 0,
        fav_n: 0,
        fav_sum: 0,
        last_activity: i64::MIN,
        elapsed: 0,
    };

    let mut out: Vec<Wide> = Vec::new();
    db.post
        .group_by(post_type.select(name))
        .select(
            score
                .and(comment_count)
                .and(creation_date)
                .and(last_activity_date)
                .and(post_type_id)
                .and(view_count.opt())
                .and(answer_count.opt())
                .and(favorite_count.opt()),
        )
        .fold(init, |a, (((((((x, c), cd), lad), ti), v), ac), fc)| Wide {
            n: a.n + 1,
            score_sum: a.score_sum + x,
            comment_sum: a.comment_sum + c,
            last_activity: a.last_activity.max(lad),
            elapsed: a.elapsed + (lad - cd) as i128,
            views_n: a.views_n + v.is_some() as i64,
            views_sum: a.views_sum + v.unwrap_or(0),
            answers_n: a.answers_n + ac.is_some() as i64,
            answers_sum: a.answers_sum + ac.unwrap_or(0),
            fav_n: a.fav_n + fc.is_some() as i64,
            fav_sum: a.fav_sum + fc.unwrap_or(0),
            q_answers: a.q_answers + if ti == 1 { ac.unwrap_or(0) } else { 0 },
            q_comments: a.q_comments + if ti == 1 { c } else { 0 },
            ..a
        })
        .drive(|k, a| out.push(Wide { name: k, ..a }));
    out.sort_by(|a, b| b.n.cmp(&a.n));
    out
}

fn q13903(db: &'static So) -> String {
    rows(wide(db).iter().map(|a| {
        row(vec![
            V::S(a.name),
            V::I(a.n),
            avg(a.score_sum, a.n),
            avg(a.views_sum, a.views_n),
            V::I(a.q_answers),
            V::I(a.q_comments),
        ])
    }))
}

// SELECT pt.Name AS PostType, COUNT(p.Id) AS TotalPosts, AVG(p.Score) AS AverageScore, SUM(p.ViewCount) AS TotalViews, AVG(p.AnswerCount) AS AverageAnswerCount,
//        AVG(p.CommentCount) AS AverageCommentCount, AVG(p.FavoriteCount) AS AverageFavoriteCount, COUNT(DISTINCT p.OwnerUserId) AS UniqueUsers
// FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Name ORDER BY TotalPosts DESC;
fn q12170(db: &'static So) -> String {
    let d = db.post.group_by(ptype_name(db)).select(&db.post.owner_user_id).count_distinct();
    let v = drain((&stats_by_type(db)).and((&d).opt()));
    rows(v.into_iter().map(|(nm, (a, d))| {
        row(vec![V::S(nm), V::I(a[0]), avg(a[1], a[0]), nullable(a[3], a[2]), avg(a[5], a[4]), avg(a[6], a[0]), avg(a[8], a[7]), V::I(d.unwrap_or(0))])
    }))
}

fn q14948(db: &'static So) -> String {
    rows(wide(db).iter().map(|a| {
        row(vec![
            V::S(a.name),
            V::I(a.n),
            avg(a.score_sum, a.n),
            avg(a.views_sum, a.views_n),
            avg(a.answers_sum, a.answers_n),
            V::T(a.last_activity),
            V::F(a.elapsed as f64 / 1e6 / a.n as f64),
        ])
    }))
}

// SELECT pt.Name AS PostType, COUNT(p.Id) AS TotalPosts, COUNT(DISTINCT p.OwnerUserId) AS TotalAuthors, AVG(p.Score) AS AverageScore, AVG(p.ViewCount) AS AverageViews,
//        SUM(p.AnswerCount) AS TotalAnswers, SUM(p.CommentCount) AS TotalComments, SUM(p.FavoriteCount) AS TotalFavorites, MAX(p.CreationDate) AS MostRecentPost,
//        MIN(p.CreationDate) AS OldestPost
// FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id JOIN Users u ON p.OwnerUserId = u.Id GROUP BY pt.Name ORDER BY TotalPosts DESC;
fn q14817(db: &'static So) -> String {
    let Post { score, view_count, comment_count, answer_count, favorite_count, creation_date, owner_user, owner_user_id, .. } = &db.post;
    let g = db
        .post
        .with(owner_user)
        .group_by(ptype_name(db))
        .select(score.and(comment_count).and(creation_date).and(view_count.opt()).and(answer_count.opt()).and(favorite_count.opt()))
        .fold([0, 0, 0, i64::MIN, i64::MAX, 0, 0, 0, 0, 0, 0], |a, (((((x, c), cd), v), n), f)| {
            [a[0] + 1, a[1] + x, a[2] + c, a[3].max(cd), a[4].min(cd), a[5] + v.is_some() as i64, a[6] + v.unwrap_or(0), a[7] + n.is_some() as i64, a[8] + n.unwrap_or(0), a[9] + f.is_some() as i64, a[10] + f.unwrap_or(0)]
        });
    let d = db.post.with(owner_user).group_by(ptype_name(db)).select(owner_user_id).count_distinct();
    let v = drain((&g).and((&d).opt()));
    rows(v.into_iter().map(|(k, (a, d))| {
        row(vec![V::S(k), V::I(a[0]), V::I(d.unwrap_or(0)), avg(a[1], a[0]), avg(a[6], a[5]), nullable(a[8], a[7]), V::I(a[2]), nullable(a[10], a[9]), V::T(a[3]), V::T(a[4])])
    }))
}





// SELECT PT.Name AS PostType, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS PositiveScorePosts,
//        SUM(CASE WHEN P.ViewCount > 0 THEN 1 ELSE 0 END) AS ViewedPosts, AVG(U.Reputation) AS AverageUserReputation, MAX(P.CreationDate) AS MostRecentPost
// FROM Posts P JOIN PostTypes PT ON P.PostTypeId = PT.Id JOIN Users U ON P.OwnerUserId = U.Id GROUP BY PT.Name ORDER BY TotalPosts DESC;
fn q12179(db: &'static So) -> String {
    let Post { score, view_count, creation_date, owner_user, .. } = &db.post;
    let g = db
        .post
        .with(owner_user)
        .group_by(ptype_name(db))
        .select(score.and(creation_date).and(owner_user.select(&db.user.reputation)).and(view_count.opt()))
        .fold([0, 0, 0, i64::MIN, 0], |a, (((x, cd), rep), w)| [a[0] + 1, a[1] + (x > 0) as i64, a[2] + rep, a[3].max(cd), a[4] + w.map_or(false, |w| w > 0) as i64]);
    rows(drain(&g).into_iter().map(|(k, a)| row(vec![V::S(k), V::I(a[0]), V::I(a[1]), V::I(a[4]), avg(a[2], a[0]), V::T(a[3])])))
}

fn q18277(db: &'static So) -> String {
    let Post { owner_user, post_type_id, .. } = &db.post;
    let User { display_name, .. } = &db.user;

    let mut all: Vec<(Str, i64, i64, i64, i64)> = Vec::new();
    db.post
        .with(owner_user)
        .group_by(owner_user.select(display_name))
        .select(post_type_id)
        .fold((0i64, 0i64, 0i64, 0i64), |(n, q, a, w), ty| {
            (
                n + 1,
                q + (ty == 1) as i64,
                a + (ty == 2) as i64,
                w + (ty >= 3 && ty <= 5) as i64,
            )
        })
        .drive(|k, (n, q, a, w)| all.push((k, n, q, a, w)));
    all.sort_by(|a, b| b.1.cmp(&a.1));
    rows(all.iter().take(10).map(|(dn, n, q, a, w)| {
        row(vec![V::S(dn), V::I(*n), V::I(*q), V::I(*a), V::I(*w)])
    }))
}

fn q10696(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, comment_count, answer_count, .. } =
        &db.post;

    let win = || db.post.with(creation_date.ge(date(2023, 1, 1)));

    let (qn, qs, an, asum, cc) = win()
        .with(creation_date.lt(date(2024, 1, 1)))
        .select(post_type_id.and(score).and(comment_count))
        .fold_flat(
            (0i64, 0i64, 0i64, 0i64, 0i64),
            |(qn, qs, an, asum, cc), ((ty, sc), c)| {
                (
                    qn + (ty == 1) as i64,
                    qs + if ty == 1 { sc } else { 0 },
                    an + (ty == 2) as i64,
                    asum + if ty == 2 { sc } else { 0 },
                    cc + c,
                )
            },
        );
    let (vn, vs) = win()
        .with(creation_date.lt(date(2024, 1, 1)))
        .select(view_count)
        .fold_flat((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let (acn, acs) = win()
        .with(creation_date.lt(date(2024, 1, 1)))
        .select(answer_count)
        .fold_flat((0i64, 0i64), |(n, s), x| (n + 1, s + x));

    row(vec![
        V::I(qn),
        V::I(an),
        avg(qs, qn),
        avg(asum, an),
        nullable(vs, vn),
        V::I(cc),
        nullable(acs, acn),
    ])
}

fn q11619(db: &'static So) -> String {
    let Post { title: pt, creation_date: pcd, score, view_count, .. } = &db.post;
    let PostHistory {
        post, post_id, post_history_type_id, creation_date, user, ..
    } = &db.post_history;
    let User { display_name, .. } = &db.user;

    let mut v: Vec<(Id<Post>, i64, i64, i64, Str)> = Vec::new();
    db.post_history
        .with(creation_date.gt(ts(2023, 10, 1, 12, 34, 56)))
        .select(
            post.and(post_id)
                .and(post_history_type_id)
                .and(creation_date)
                .and(user.select(display_name)),
        )
        .drive(|_, ((((p, pi), ty), cd), dn)| v.push((p, pi, ty, cd, dn)));
    v.sort_by(|a, b| b.3.cmp(&a.3));
    rows(v.iter().take(100).map(|(p, pi, ty, cd, dn)| {
        row(vec![
            V::I(*pi),
            ostr(pt.get(*p)),
            V::T(pcd.get(*p).unwrap()),
            V::I(score.get(*p).unwrap()),
            oint(view_count.get(*p)),
            V::I(*ty),
            V::T(*cd),
            V::S(dn),
        ])
    }))
}

fn q14720(db: &'static So) -> String {
    let Post {
        origid, title: pt, creation_date, score, view_count, answer_count, owner_user, ..
    } = &db.post;
    let User {
        origid: uid, display_name, reputation, creation_date: ucd, last_access_date, ..
    } = &db.user;

    let mut v: Vec<(Id<Post>, Id<User>, i64, i64, i64)> = Vec::new();
    db.post
        .with(creation_date.ge(ts(2023, 10, 1, 12, 34, 56)))
        .select(origid.and(creation_date).and(score).and(owner_user))
        .drive(|p, (((id, cd), sc), u)| v.push((p, u, id, cd, sc)));
    rows(v.iter().map(|(p, u, id, cd, sc)| {
        row(vec![
            V::I(*id),
            ostr(pt.get(*p)),
            V::T(*cd),
            V::I(*sc),
            oint(view_count.get(*p)),
            oint(answer_count.get(*p)),
            V::I(uid.get(*u).unwrap()),
            V::S(display_name.get(*u).unwrap()),
            V::I(reputation.get(*u).unwrap()),
            V::T(ucd.get(*u).unwrap()),
            V::T(last_access_date.get(*u).unwrap()),
        ])
    }))
}

fn q13737(db: &'static So) -> String {
    let Post {
        post_type_id, title: pt, creation_date, view_count, score, parent, owner_user, ..
    } = &db.post;
    let User { display_name, .. } = &db.user;

    let mut v: Vec<(Id<Post>, i64, i64, i64, Str)> = Vec::new();
    db.post
        .with(post_type_id.eq(2))
        .with(parent.select(post_type_id).eq(1))
        .select(
            parent
                .and(creation_date)
                .and(score)
                .and(owner_user.select(display_name)),
        )
        .drive(|_, (((q, acd), asc), dn)| v.push((q, acd, asc, 0, dn)));
    rows(v.iter().map(|(q, acd, asc, _, dn)| {
        let qcd = creation_date.get(*q).unwrap();
        row(vec![
            ostr(pt.get(*q)),
            V::T(qcd),
            oint(view_count.get(*q)),
            V::I(score.get(*q).unwrap()),
            V::T(*acd),
            V::F((*acd - qcd) as f64 / 1e6),
            V::I(*asc),
            V::S(dn),
        ])
    }))
}

fn q12493(db: &'static So) -> String {
    let PostHistory { post_history_type_id, post, creation_date, .. } = &db.post_history;
    let Post { title: pt, owner_display_name: odn, .. } = &db.post;

    let cut = ts(2024, 9, 1, 12, 34, 56);
    let mut all: Vec<(i64, Option<Str>, Option<Str>, i64, i64, i64)> = Vec::new();

    db.post_history
        .with(creation_date.ge(cut))
        .group_by(
            post_history_type_id
                .and(post.select(pt).opt())
                .and(post.select(odn).opt()),
        )
        .select(creation_date)
        .fold((0i64, i64::MAX, i64::MIN), |(n, lo, hi), x| {
            (n + 1, lo.min(x), hi.max(x))
        })
        .drive(|((t, ti), dn), (n, lo, hi)| all.push((t, ti, dn, n, lo, hi)));

    all.sort_by(|a, b| b.3.cmp(&a.3));
    rows(all.iter().map(|(t, ti, dn, n, lo, hi)| {
        row(vec![
            V::I(*t),
            ti.map(V::S).unwrap_or(V::Null),
            dn.map(V::S).unwrap_or(V::Null),
            V::I(*n),
            V::T(*lo),
            V::T(*hi),
        ])
    }))
}

fn q10232(db: &'static So) -> String {
    let PostHistory { post_history_type_id, post, creation_date, user, .. } =
        &db.post_history;
    let Post { title: pt, creation_date: pcd, .. } = &db.post;
    let User { display_name, .. } = &db.user;

    let cut = ts(2024, 9, 1, 12, 34, 56);
    let mut all: Vec<(i64, Option<Str>, i64, Str, i64, i64)> = Vec::new();

    db.post_history
        .with(creation_date.ge(cut))
        .group_by(
            post_history_type_id
                .and(post.select(pt))
                .and(post.select(pcd))
                .and(user.select(display_name))
                .and(creation_date),
        )
        .select(creation_date)
        .fold(0i64, |a, _| a + 1)
        .drive(|((((t, ti), pc), dn), hc), n| {
            all.push((t, Some(ti), pc, dn, hc, n))
        });

    db.post_history
        .with(creation_date.ge(cut))
        .minus(post.select(pt))
        .group_by(
            post_history_type_id
                .and(post.select(pcd))
                .and(user.select(display_name))
                .and(creation_date),
        )
        .select(creation_date)
        .fold(0i64, |a, _| a + 1)
        .drive(|(((t, pc), dn), hc), n| all.push((t, None, pc, dn, hc, n)));

    all.sort_by(|a, b| b.5.cmp(&a.5));
    rows(all.iter().map(|(t, ti, pc, dn, hc, n)| {
        row(vec![
            V::I(*t),
            ti.map(V::S).unwrap_or(V::Null),
            V::T(*pc),
            V::S(dn),
            V::T(*hc),
            V::I(*n),
        ])
    }))
}

pub const ENTRIES: &[harness::Entry] = &[
    ("13903", q13903),
    ("11619", q11619),
    ("18277", q18277),
    ("12170", q12170),
    ("12493", q12493),
    ("14948", q14948),
    ("14720", q14720),
    ("12179", q12179),
    ("10696", q10696),
    ("13737", q13737),
    ("10232", q10232),
    ("14817", q14817),
];
