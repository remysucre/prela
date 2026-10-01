use harness::prelude::*;

fn q15640(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let PostHistory {
        post_history_type_id, post, creation_date: hdate, comment, user_display_name, ..
    } = &db.post_history;
    let User { display_name, .. } = &db.user;

    let mut v: Vec<(Id<PostHistory>, Id<Post>, i64, i64, Str)> = Vec::new();
    db.post_history
        .with(post_history_type_id.in_v(vec![4, 5]))
        .select(
            post.and(hdate)
                .and(post.select(creation_date.and(owner_user.select(display_name)))),
        )
        .drive(|h, ((p, hc), (pc, dn))| v.push((h, p, pc, hc, dn)));
    v.sort_by(|a, b| b.2.cmp(&a.2));
    rows(v.iter().take(10).map(|(h, p, pc, hc, dn)| {
        row(vec![
            V::S(dn),
            title(db, *p),
            V::T(*pc),
            ostr(user_display_name.get(*h)),
            V::T(*hc),
            ostr(comment.get(*h)),
        ])
    }))
}

// SELECT pt.Name AS PostType, COUNT(p.Id) AS TotalPosts, AVG(p.Score) AS AverageScore, SUM(p.ViewCount) AS TotalViews, u.Reputation AS UserReputation,
//        u.CreationDate AS UserCreationDate
// FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id JOIN Users u ON p.OwnerUserId = u.Id GROUP BY pt.Name, u.Reputation, u.CreationDate ORDER BY TotalPosts DESC;
fn q10695(db: &'static So) -> String {
    let Post { score, view_count, owner_user, .. } = &db.post;
    let g = db
        .post
        .with(owner_user)
        .group_by(ptype_name(db).and(owner_user.select((&db.user.reputation).and(&db.user.creation_date))))
        .select(score.and(view_count.opt()))
        .fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    rows(drain(&g).into_iter().map(|((nm, (rep, uc)), a)| row(vec![V::S(nm), V::I(a[0]), avg(a[1], a[0]), nullable(a[3], a[2]), V::I(rep), V::T(uc)])))
}

fn question_history(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let PostHistory { post, creation_date: hdate, post_history_type, .. } =
        &db.post_history;
    let PostHistoryType { name: htname, .. } = &db.post_history_type;
    let User { display_name, .. } = &db.user;

    let mut v: Vec<(Id<PostHistory>, Id<Post>, i64, i64, Str, Str)> = Vec::new();
    db.post_history
        .with(post.select(post_type_id).eq(1))
        .select(
            post.and(hdate)
                .and(post.select(creation_date.and(owner_user.select(display_name))))
                .and(post_history_type.select(htname)),
        )
        .drive(|h, (((p, hc), (pc, dn)), tn)| v.push((h, p, pc, hc, dn, tn)));
    v.sort_by(|a, b| b.3.cmp(&a.3).then(a.0.cmp(&b.0)));
    rows(v.iter().take(10).map(|(_, p, pc, hc, dn, tn)| {
        row(vec![V::S(dn), title(db, *p), V::T(*pc), V::T(*hc), V::S(tn)])
    }))
}

fn q18659(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let PostHistory { post, creation_date: hdate, post_history_type, .. } =
        &db.post_history;
    let PostHistoryType { name: htname, .. } = &db.post_history_type;
    let User { display_name, reputation, .. } = &db.user;

    let mut v: Vec<(Id<Post>, i64, i64, Str, Str)> = Vec::new();
    db.post_history
        .with(post.select(owner_user.select(reputation)).gt(1000))
        .select(
            post.and(hdate)
                .and(post.select(creation_date.and(owner_user.select(display_name))))
                .and(post_history_type.select(htname)),
        )
        .drive(|_, (((p, hc), (pc, dn)), tn)| v.push((p, pc, hc, dn, tn)));
    v.sort_by(|a, b| b.2.cmp(&a.2));
    rows(v.iter().take(10).map(|(p, pc, hc, dn, tn)| {
        row(vec![V::S(dn), title(db, *p), V::T(*pc), V::T(*hc), V::S(tn)])
    }))
}

fn q12457(db: &'static So) -> String {
    let Post {
        score, view_count, answer_count, comment_count, favorite_count, tags_str, ..
    } = &db.post;
    let PostHistory {
        post_history_type_id, post, post_id, user, user_id, creation_date, comment, text, ..
    } = &db.post_history;
    let User { display_name, .. } = &db.user;

    let mut v: Vec<(Id<PostHistory>, Id<Post>, i64, i64, Str)> = Vec::new();
    db.post_history
        .with(post_history_type_id.in_v(vec![4, 5, 6]))
        .select(post.and(post_id).and(creation_date).and(user.select(display_name)))
        .drive(|h, (((p, pi), cd), dn)| v.push((h, p, pi, cd, dn)));
    v.sort_by(|a, b| b.3.cmp(&a.3));
    rows(v.iter().map(|(h, p, pi, cd, dn)| {
        row(vec![
            V::I(*pi),
            title(db, *p),
            oint(user_id.get(*h)),
            V::S(dn),
            V::T(*cd),
            V::I(score.get(*p).unwrap()),
            oint(view_count.get(*p)),
            oint(answer_count.get(*p)),
            V::I(comment_count.get(*p).unwrap()),
            oint(favorite_count.get(*p)),
            ostr(comment.get(*h)),
            ostr(text.get(*h)),
            ostr(tags_str.get(*p)),
        ])
    }))
}

fn q10922(db: &'static So) -> String {
    let Post {
        title: ptitle, body, score, view_count, answer_count, comment_count,
        favorite_count, creation_date: pcd, last_activity_date, tags_str,
        accepted_answer_id, closed_date, ..
    } = &db.post;
    let PostHistory { post, post_id, user, user_id, creation_date, .. } =
        &db.post_history;
    let User { display_name, .. } = &db.user;

    let mut v: Vec<(Id<PostHistory>, Id<Post>, i64, i64, Str)> = Vec::new();
    db.post_history
        .with(creation_date.ge(date(2023, 1, 1)))
        .with(creation_date.lt(date(2024, 1, 1)))
        .select(post.and(post_id).and(creation_date).and(user.select(display_name)))
        .drive(|h, (((p, pi), cd), dn)| v.push((h, p, pi, cd, dn)));
    v.sort_by(|a, b| b.3.cmp(&a.3));
    rows(v.iter().take(100).map(|(h, p, pi, cd, dn)| {
        row(vec![
            V::I(*pi),
            oint(user_id.get(*h)),
            V::S(dn),
            V::T(*cd),
            ostr(ptitle.get(*p)),
            V::S(body.get(*p).unwrap()),
            V::I(score.get(*p).unwrap()),
            oint(view_count.get(*p)),
            oint(answer_count.get(*p)),
            V::I(comment_count.get(*p).unwrap()),
            oint(favorite_count.get(*p)),
            V::T(pcd.get(*p).unwrap()),
            V::T(last_activity_date.get(*p).unwrap()),
            ostr(tags_str.get(*p)),
            oint(accepted_answer_id.get(*p)),
            ots(closed_date.get(*p)),
        ])
    }))
}

fn q11216(db: &'static So) -> String {
    let PostHistory { post_history_type_id, post, user, creation_date, .. } = &db.post_history;
    let g = db
        .post_history
        .with(creation_date.between(date(2023, 1, 1), date(2023, 12, 31)))
        .with(user)
        .group_by(post_history_type_id.and(post.select(&db.post.title).opt()).and(user.select(&db.user.display_name)))
        .select(creation_date)
        .fold((0i64, i64::MAX, i64::MIN), |(n, lo, hi), x| (n + 1, lo.min(x), hi.max(x)));
    let v = top_n(drain(&g), |&(_, (n, _, _))| std::cmp::Reverse(n), 10);
    rows(v.into_iter().map(|(((t, ti), dn), (n, lo, hi))| row(vec![V::I(t), V::I(n), V::T(lo), V::T(hi), ostr(ti), V::S(dn)])))
}

fn q14482(db: &'static So) -> String {
    let Post {
        post_type_id, origid, creation_date, score, view_count, answer_count,
        comment_count, favorite_count, owner_user, ..
    } = &db.post;
    let User {
        origid: uid, display_name, reputation, creation_date: ucd, last_access_date,
        views, up_votes, down_votes, ..
    } = &db.user;

    let mut v: Vec<(Id<Post>, Id<User>, i64, i64, i64)> = Vec::new();
    db.post
        .with(post_type_id.in_v(vec![1, 2]))
        .select(origid.and(creation_date).and(score).and(owner_user))
        .drive(|p, (((id, cd), sc), u)| v.push((p, u, id, cd, sc)));
    v.sort_by(|a, b| b.3.cmp(&a.3));
    rows(v.iter().take(100).map(|(p, u, id, cd, sc)| {
        row(vec![
            V::I(*id),
            title(db, *p),
            V::T(*cd),
            V::I(*sc),
            oint(view_count.get(*p)),
            oint(answer_count.get(*p)),
            V::I(comment_count.get(*p).unwrap()),
            oint(favorite_count.get(*p)),
            V::I(uid.get(*u).unwrap()),
            V::S(display_name.get(*u).unwrap()),
            V::I(reputation.get(*u).unwrap()),
            V::T(ucd.get(*u).unwrap()),
            V::T(last_access_date.get(*u).unwrap()),
            V::I(views.get(*u).unwrap()),
            V::I(up_votes.get(*u).unwrap()),
            V::I(down_votes.get(*u).unwrap()),
        ])
    }))
}

fn q17357(db: &'static So) -> String {
    let Post { owner_user, origid, .. } = &db.post;
    let User { display_name, reputation, .. } = &db.user;

    let mut all: Vec<(Str, i64)> = Vec::new();
    db.post
        .with(owner_user.select(reputation).gt(1000))
        .group_by(owner_user.select(display_name))
        .select(origid)
        .fold(0i64, |a, _| a + 1)
        .drive(|k, n| all.push((k, n)));
    all.sort_by(|a, b| b.1.cmp(&a.1));
    rows(all.iter().take(10).map(|(dn, n)| row(vec![V::S(dn), V::I(*n)])))
}

fn q18898(db: &'static So) -> String {
    let Post { post_type_id, view_count, .. } = &db.post;

    let (total, q, a) = db
        .post
        .select(post_type_id)
        .fold_flat((0i64, 0i64, 0i64), |(t, q, a), ty| {
            (t + 1, q + (ty == 1) as i64, a + (ty == 2) as i64)
        });
    let (vn, vs) = db
        .post
        .select(view_count)
        .fold_flat((0i64, 0i64), |(n, s), x| (n + 1, s + x));

    row(vec![V::I(total), V::I(q), V::I(a), avg(vs, vn)])
}

// SELECT U.DisplayName, COUNT(P.Id) AS PostCount, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews FROM Users U JOIN Posts P ON U.Id = P.OwnerUserId
// GROUP BY U.DisplayName ORDER BY PostCount DESC LIMIT 10;
fn q18210(db: &'static So) -> String {
    let Post { owner_user, view_count, .. } = &db.post;
    let g = db.post.with(owner_user).group_by(owner_user.select(&db.user.display_name)).select(view_count.opt()).fold([0i64; 2], |a, w| [a[0] + 1, a[1] + w.unwrap_or(0)]);
    let v = top_n(drain(&g), |&(_, a)| std::cmp::Reverse(a[0]), 10);
    rows(v.into_iter().map(|(dn, a)| row(vec![V::S(dn), V::I(a[0]), V::I(a[1])])))
}

pub const ENTRIES: &[harness::Entry] = &[
    ("15640", q15640),
    ("10695", q10695),
    ("17116", question_history),
    ("16100", question_history),
    ("18659", q18659),
    ("12457", q12457),
    ("11216", q11216),
    ("14482", q14482),
    ("10922", q10922),
    ("17357", q17357),
    ("18898", q18898),
    ("18210", q18210),
];
