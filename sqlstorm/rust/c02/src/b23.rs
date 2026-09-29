use harness::prelude::*;

// Posts JOIN PostTypes JOIN Users grouped by (pt.Name, u.Reputation): (key, posts, score sum, views present, views sum, PostTypeId).
fn type_rep_avg(db: &'static So) -> Vec<((Str, i64), i64, i64, i64, i64, i64)> {
    let Post { post_type_id, score, view_count, owner_user, .. } = &db.post;
    let g = db
        .post
        .with(owner_user)
        .group_by(ptype_name(db).and(owner_user.select(&db.user.reputation)))
        .select(score.and(view_count.opt()).and(post_type_id))
        .fold([0i64; 5], |a, ((s, w), t)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), t]);
    drain(&g).into_iter().map(|(k, a)| (k, a[0], a[1], a[2], a[3], a[4])).collect()
}

fn q14342(db: &'static So) -> String {
    let mut a = type_rep_avg(db);
    a.sort_by(|x, y| x.0.0.cmp(y.0.0).then(x.0.1.cmp(&y.0.1)));
    rows(a.iter().map(|((nm, rep), n, s, vn, vs, _)| {
        row(vec![V::S(nm), V::I(*rep), V::I(*n), avg(*s, *n), avg(*vs, *vn)])
    }))
}

fn q13713(db: &'static So) -> String {
    let mut a = type_rep_avg(db);
    a.sort_by(|x, y| x.5.cmp(&y.5).then(y.0.1.cmp(&x.0.1)));
    rows(a.iter().map(|((nm, rep), n, s, vn, vs, t)| {
        row(vec![
            V::I(*t),
            V::S(nm),
            V::I(*rep),
            V::I(*n),
            avg(*s, *n),
            avg(*vs, *vn),
        ])
    }))
}

// SELECT pt.Name AS PostType, COUNT(p.Id) AS TotalPosts, AVG(p.Score) AS AverageScore, AVG(p.ViewCount) AS AverageViewCount, u.Reputation AS UserReputation,
//        u.CreationDate AS UserCreationDate
// FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id JOIN Users u ON p.OwnerUserId = u.Id GROUP BY pt.Name, u.Reputation, u.CreationDate ORDER BY TotalPosts DESC;
fn q14740(db: &'static So) -> String {
    let Post { score, view_count, owner_user, .. } = &db.post;
    let g = db
        .post
        .with(owner_user)
        .group_by(ptype_name(db).and(owner_user.select((&db.user.reputation).and(&db.user.creation_date))))
        .select(score.and(view_count.opt()))
        .fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    rows(drain(&g).into_iter().map(|((nm, (rep, ucd)), a)| row(vec![V::S(nm), V::I(a[0]), avg(a[1], a[0]), avg(a[3], a[2]), V::I(rep), V::T(ucd)])))
}

fn q11672(db: &'static So) -> String {
    let Post {
        origid, creation_date, score, view_count, answer_count, comment_count,
        owner_user, ..
    } = &db.post;
    let User { origid: uid, display_name, reputation, .. } = &db.user;

    let mut v: Vec<(Id<Post>, i64, i64, i64, i64, Str, i64)> = Vec::new();
    db.post
        .with(owner_user.select(reputation).gt(0))
        .select(
            origid
                .and(creation_date)
                .and(score)
                .and(owner_user.select(uid.and(display_name).and(reputation))),
        )
        .drive(|pid, (((id, created), sc), ((u, dn), rep))| {
            v.push((pid, id, created, sc, u, dn, rep))
        });
    v.sort_by(|a, b| b.6.cmp(&a.6).then(b.2.cmp(&a.2)));
    rows(v.iter().take(100).map(|(pid, id, created, sc, u, dn, rep)| {
        row(vec![
            V::I(*u),
            V::S(dn),
            V::I(*rep),
            V::I(*id),
            title(db, *pid),
            V::T(*created),
            V::I(*sc),
            oint(view_count.get(*pid)),
            oint(answer_count.get(*pid)),
            V::I(comment_count.get(*pid).unwrap()),
        ])
    }))
}

fn q16317(db: &'static So) -> String {
    let Post { creation_date, owner_user, tags_str, .. } = &db.post;
    let PostHistory { post_history_type_id, post, creation_date: hdate, .. } =
        &db.post_history;
    let User { display_name, .. } = &db.user;

    let mut v: Vec<(Id<Post>, i64, i64, Str)> = Vec::new();
    db.post_history
        .with(post_history_type_id.in_v(vec![4, 5]))
        .select(
            post.and(hdate)
                .and(post.select(creation_date.and(owner_user.select(display_name)))),
        )
        .drive(|_, ((p, hc), (pc, dn))| v.push((p, hc, pc, dn)));
    v.sort_by(|a, b| b.1.cmp(&a.1));
    rows(v.iter().take(100).map(|(p, hc, pc, dn)| {
        row(vec![
            V::S(dn),
            title(db, *p),
            V::T(*pc),
            V::T(*hc),
            ostr(tags_str.get(*p)),
        ])
    }))
}

fn owned_since(db: &'static So, from: i64) -> Vec<(Str, i64, i64, i64, i64, i64)> {
    let Post { creation_date, post_type, score, view_count, owner_user, .. } = &db.post;
    let PostType { name, .. } = &db.post_type;
    let User { reputation, .. } = &db.user;

    let mut out: Vec<(Str, i64, i64, i64, i64, i64)> = Vec::new();
    db.post
        .with(creation_date.ge(from))
        .with(owner_user)
        .group_by(post_type.select(name))
        .select(score.and(owner_user.select(reputation)).and(view_count.opt()))
        .fold(
            (0i64, 0i64, 0i64, 0i64, 0i64),
            |(n, s, r, vn, vs), ((x, rep), v)| {
                (n + 1, s + x, r + rep, vn + v.is_some() as i64, vs + v.unwrap_or(0))
            },
        )
        .drive(|k, (n, s, r, vn, vs)| out.push((k, n, s, r, vn, vs)));
    out.sort_by(|a, b| b.1.cmp(&a.1));
    out
}

fn q10582(db: &'static So) -> String {
    rows(owned_since(db, date(2023, 1, 1)).iter().map(|(k, n, s, r, vn, vs)| {
        row(vec![
            V::S(k),
            V::I(*n),
            avg(*r, *n),
            V::I(*s),
            nullable(*vs, *vn),
        ])
    }))
}

fn q14091(db: &'static So) -> String {
    rows(owned_since(db, date(2023, 1, 1)).iter().map(|(k, n, s, r, vn, vs)| {
        row(vec![
            V::S(k),
            V::I(*n),
            avg(*s, *n),
            avg(*vs, *vn),
            avg(*r, *n),
        ])
    }))
}

// SELECT pt.Name AS PostType, COUNT(p.Id) AS PostCount, AVG(p.Score) AS AverageScore, AVG(p.ViewCount) AS AverageViews, SUM(p.AnswerCount) AS TotalAnswers,
//        SUM(p.CommentCount) AS TotalComments, SUM(p.FavoriteCount) AS TotalFavorites
// FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY pt.Name ORDER BY PostCount DESC;
fn q13628(db: &'static So) -> String {
    rows(drain(&stats_by_type(db)).into_iter().map(|(nm, a)| {
        row(vec![V::S(nm), V::I(a[0]), avg(a[1], a[0]), avg(a[3], a[2]), nullable(a[5], a[4]), V::I(a[6]), nullable(a[8], a[7])])
    }))
}

fn q17104(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, owner_user, .. } = &db.post;
    let Comment { post, text, creation_date: cdate, .. } = &db.comment;
    let User { display_name, .. } = &db.user;

    let mut v: Vec<(Id<Post>, i64, i64, Str, Str, i64)> = Vec::new();
    db.comment
        .with(post.select(post_type_id).eq(1))
        .select(
            post.and(post.select(
                creation_date.and(score).and(owner_user.select(display_name)),
            ))
            .and(text)
            .and(cdate),
        )
        .drive(|_, (((p, ((pc, sc), dn)), txt), cc)| v.push((p, pc, sc, dn, txt, cc)));
    v.sort_by(|a, b| b.5.cmp(&a.5));
    rows(v.iter().take(10).map(|(p, pc, sc, dn, txt, cc)| {
        row(vec![
            V::S(dn),
            title(db, *p),
            V::T(*pc),
            V::I(*sc),
            V::S(txt),
            V::T(*cc),
        ])
    }))
}

fn q19045(db: &'static So) -> String {
    let Post { post_type_id, creation_date, body, owner_user, .. } = &db.post;
    let PostHistory { post, creation_date: hdate, comment, .. } = &db.post_history;
    let User { display_name, .. } = &db.user;

    let mut v: Vec<(Id<PostHistory>, Id<Post>, i64, i64, Str, Str)> = Vec::new();
    db.post_history
        .with(post.select(post_type_id).eq(1))
        .select(
            post.and(hdate).and(post.select(
                creation_date.and(body).and(owner_user.select(display_name)),
            )),
        )
        .drive(|h, ((p, hc), ((pc, bd), dn))| v.push((h, p, hc, pc, bd, dn)));
    v.sort_by(|a, b| b.2.cmp(&a.2));
    rows(v.iter().take(10).map(|(h, p, hc, pc, bd, dn)| {
        row(vec![
            V::S(dn),
            title(db, *p),
            V::T(*pc),
            V::T(*hc),
            V::S(bd),
            ostr(comment.get(*h)),
        ])
    }))
}

fn q13323(db: &'static So) -> String {
    let PostHistory { post_history_type_id, post, creation_date, .. } = &db.post_history;
    let Post { title: ptitle, .. } = &db.post;

    let win = db
        .post_history
        .with(creation_date.ge(date(2023, 1, 1)))
        .with(creation_date.le(date(2023, 10, 31)));

    let mut all: Vec<(i64, Option<Str>, i64, i64, i64)> = Vec::new();
    win.group_by(post_history_type_id.and(post.select(ptitle)))
        .select(creation_date)
        .fold((0i64, i64::MAX, i64::MIN), |(n, lo, hi), x| {
            (n + 1, lo.min(x), hi.max(x))
        })
        .drive(|(t, ti), (n, lo, hi)| all.push((t, Some(ti), n, lo, hi)));

    db.post_history
        .with(creation_date.ge(date(2023, 1, 1)))
        .with(creation_date.le(date(2023, 10, 31)))
        .minus(post.select(ptitle))
        .group_by(post_history_type_id)
        .select(creation_date)
        .fold((0i64, i64::MAX, i64::MIN), |(n, lo, hi), x| {
            (n + 1, lo.min(x), hi.max(x))
        })
        .drive(|t, (n, lo, hi)| all.push((t, None, n, lo, hi)));

    all.sort_by(|a, b| b.2.cmp(&a.2).then(b.3.cmp(&a.3)));
    rows(all.iter().map(|(t, ti, n, lo, hi)| {
        row(vec![
            V::I(*t),
            ti.map(V::S).unwrap_or(V::Null),
            V::I(*n),
            V::T(*lo),
            V::T(*hi),
        ])
    }))
}

fn q18297(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let Vote { post, vote_type, creation_date: vdate, .. } = &db.vote;
    let VoteType { name: vt_name, .. } = &db.vote_type;
    let User { display_name, .. } = &db.user;

    let mut v: Vec<(Id<Post>, i64, i64, Str, Str)> = Vec::new();
    db.vote
        .with(post.select(creation_date).gt(date(2023, 1, 1)))
        .select(
            post.and(post.select(creation_date.and(owner_user.select(display_name))))
                .and(vdate)
                .and(vote_type.select(vt_name)),
        )
        .drive(|_, (((p, (pc, dn)), vc), vt)| v.push((p, pc, vc, dn, vt)));
    v.sort_by(|a, b| b.2.cmp(&a.2));
    rows(v.iter().map(|(p, pc, vc, dn, vt)| {
        row(vec![V::S(dn), title(db, *p), V::T(*pc), V::T(*vc), V::S(vt)])
    }))
}

pub const ENTRIES: &[harness::Entry] = &[
    ("14342", q14342),
    ("11672", q11672),
    ("16317", q16317),
    ("13713", q13713),
    ("10582", q10582),
    ("13628", q13628),
    ("17104", q17104),
    ("14091", q14091),
    ("19045", q19045),
    ("13323", q13323),
    ("18297", q18297),
    ("14740", q14740),
];
