use harness::prelude::*;

fn year_ago() -> i64 {
    add_years(ts(2024, 10, 1, 12, 34, 56), -1)
}

fn q6659(db: &'static So) -> String {
    let Post { owner_user, score, view_count, creation_date, .. } = &db.post;
    let User { display_name, reputation, .. } = &db.user;
    let stats = db.post.with(score.gt(0)).select(owner_user).inv().select(view_count.opt().and(score).and(creation_date)).fold(
        (0i64, 0i64, 0i64, 0i64, i64::MIN),
        |(n, vn, vs, s, last), ((v, sc), cd)| (n + 1, vn + v.is_some() as i64, vs + v.unwrap_or(0), s + sc, last.max(cd)),
    );
    let mut v = Vec::new();
    db.user.select(display_name.and(reputation).and(&stats)).filt(|(_, a)| a.0 > 1).drive(|_, x| v.push(x));
    let views = |a: &(i64, i64, i64, i64, i64)| (a.1 > 0).then_some(a.2);
    v.sort_by(|a, b| (b.0.1, views(&b.1)).cmp(&(a.0.1, views(&a.1))));
    rows(v.iter().take(10).map(|&((dn, rep), (n, vn, vs, s, last))| {
        row(vec![V::S(dn), V::I(rep), V::I(n), nullable(vs, vn), avg(s, n), V::T(last)])
    }))
}

fn q28192(db: &'static So) -> String {
    let Post { owner_user_id, creation_date, origid, view_count, score, .. } = &db.post;
    let base = db.post.with(creation_date.ge(year_ago()));
    let filtered: MatSet<Id<Post>> = (&base)
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, cd)| cd, desc)
        .filt(|(_, n)| n <= 5)
        .map(|((p, _), _)| p)
        .collect();
    let user_by_id: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let as_user = origid.select(&user_by_id);
    let tu = (&filtered)
        .select(&as_user)
        .inv()
        .select(view_count.opt().and(score))
        .fold((0i64, 0i64, 0i64, 0i64), |(n, vn, vs, s), (v, sc)| (n + 1, vn + v.is_some() as i64, vs + v.unwrap_or(0), s + sc));
    type Key = ((i64, Option<i64>), Id<User>);
    let top: MatSet<Id<User>> = whole(&tu)
        .select(Ident::<User>::new().and(&tu))
        .window(
            row_number,
            |(u, a)| ((a.0, (a.1 > 0).then_some(a.2)), u),
            |x: &Key, y: &Key| y.0.cmp(&x.0).then(x.1.cmp(&y.1)),
        )
        .filt(|(_, n)| n <= 10)
        .map(|((u, _), _)| u)
        .collect();
    let mut out = Vec::new();
    (&filtered).select((&as_user).and((&as_user).select((&tu).and(&top)))).drive(|p, (u, ((n, vn, vs, s), _))| {
        let mut f = vec![V::I(db.user.origid.get(u).unwrap()), V::S(db.user.display_name.get(u).unwrap())];
        f.extend([V::I(n), nullable(vs, vn), V::I(s)]);
        f.extend(post_fields(db, p, &["title", "views", "score", "created"]));
        out.push(row(f))
    });
    rows(out)
}

fn q8660(db: &'static So) -> String {
    let Post { post_type, creation_date, score, .. } = &db.post;
    let Vote { post, user, .. } = &db.vote;
    let base = db.post.with(creation_date.ge(add_years(current_date(), -1)));
    let top: MatSet<Id<Post>> = (&base)
        .group_by(post_type)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((_, s), cd)| (s, cd), desc)
        .filt(|(_, n)| n <= 5)
        .map(|(((p, _), _), _)| p)
        .collect();
    let votes = user.inv().fold(0i64, |a, _| a + 1);
    let mut out = Vec::new();
    db.vote.with(post.with(&top)).select(post.and(user.select((&votes).gt(10)).and(user))).drive(|_, (p, (n, u))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend([V::S(db.user.display_name.get(u).unwrap()), V::I(n)]);
        out.push(row(f))
    });
    rows(out)
}

fn q25895(db: &'static So) -> String {
    let Post { creation_date, parent, post_type_id, last_activity_date, .. } = &db.post;
    let cc = comments_per_post(db);
    let up = votes_of_type(db, 2);
    let down = votes_of_type(db, 3);
    let ac = parent.inv().dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1);
    let edits = db
        .post_history
        .with((&db.post_history.post_history_type_id).is_in([10, 11, 12]))
        .select(&db.post_history.post)
        .inv()
        .dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1);
    let base = owned(db).with(creation_date.ge(date(2020, 1, 1))).with((&cc).gt(0));
    let ranked = whole(&base)
        .select(
            Ident::<Post>::new()
                .and(last_activity_date)
                .and(&cc)
                .and(&up)
                .and(&down)
                .and(&ac)
                .and(post_type_id)
                .and(&edits),
        )
        .window(rank, |(((((((_, la), _), _), _), _), _), _)| la, desc)
        .window(row_number, |((((((((_, la), _), _), _), _), _), _), _)| la, desc);
    let mut out = Vec::new();
    ranked.filt(|(_, n)| n <= 100).drive(
        |_, (((((((((p, _), c), u), d), a), t), e), r), _)| {
            let mut f = post_fields(db, p, &["id", "title", "owner", "created", "activity"]);
            f.extend([V::I(c), V::I(if t == 1 { a } else { 0 }), V::I(u), V::I(d)]);
            f.extend(post_fields(db, p, &["tags"]));
            f.push(V::S(match u - d {
                x if x > 0 => "Positive",
                x if x < 0 => "Negative",
                _ => "Neutral",
            }));
            f.extend([V::I(r), V::I(e)]);
            out.push(row(f))
        },
    );
    rows(out)
}

fn q25518(db: &'static So) -> String {
    let Post { post_type_id, creation_date, tags_str, answer_count, comment_count, score, .. } = &db.post;
    let base = owned(db).with(post_type_id.eq(1)).with(creation_date.ge(year_ago()));
    let stats = (&base)
        .group_by(tags_str)
        .select(answer_count.opt().and(comment_count).and(score))
        .fold((0i64, 0i64, 0i64, 0i64, 0i64), |(n, an, asum, c, s), ((a, cm), sc)| {
            (n + 1, an + a.is_some() as i64, asum + a.unwrap_or(0), c + cm, s + sc)
        });
    let rn = (&base)
        .group_by(tags_str.opt())
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, cd)| cd, desc);
    let mut v = Vec::new();
    (&rn)
        .filt(|(_, n)| n <= 5)
        .map(|((p, _), _)| p)
        .select(Ident::<Post>::new().and(tags_str.and(tags_str.select(&stats)).and(creation_date)))
        .drive(|_, (p, x)| v.push((p, x)));
    let key = |x: &((Str, (i64, i64, i64, i64, i64)), i64)| (x.0.1.4 as f64 / x.0.1.0 as f64, x.1);
    v.sort_by(|a, b| {
        let (ka, kb) = (key(&a.1), key(&b.1));
        kb.0.partial_cmp(&ka.0).unwrap().then(kb.1.cmp(&ka.1))
    });
    rows(v.iter().take(10).map(|&(p, ((t, (n, an, asum, c, s)), _))| {
        let mut f = post_fields(db, p, &["title", "created"]);
        f.extend([V::S(t), V::I(n), nullable(asum, an), V::I(c), avg(s, n)]);
        f.extend(post_fields(db, p, &["owner"]));
        row(f)
    }))
}

fn q8110(db: &'static So) -> String {
    let Post { post_type, creation_date, score, view_count, answer_count, comment_count, owner_user, .. } = &db.post;
    let up = votes_of_type(db, 2);
    let down = votes_of_type(db, 3);
    let base = owned(db).with(creation_date.ge(date(2024, 9, 1)));
    let top: MatSet<Id<Post>> = (&base)
        .group_by(post_type)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(rank, |((_, s), v)| (s, v), desc)
        .filt(|(_, r)| r <= 5)
        .map(|(((p, _), _), _)| p)
        .collect();
    let (tu, td) = (&top).select((&up).and(&down)).fold_flat((0i64, 0i64), |(a, b), (u, d)| (a + u, b + d));
    let agg = (&top)
        .group_by(owner_user.select(&db.user.display_name))
        .select(score.and(view_count.opt()).and(answer_count.opt()).and(comment_count))
        .fold((0i64, 0i64, 0i64, 0i64, 0i64, 0i64, 0i64), |(n, s, vn, vs, an, asum, c), (((sc, v), a), cm)| {
            (n + 1, s + sc, vn + v.is_some() as i64, vs + v.unwrap_or(0), an + a.is_some() as i64, asum + a.unwrap_or(0), c + cm)
        });
    let mut out = Vec::new();
    agg.drive(|dn, (n, s, vn, vs, an, asum, c)| {
        out.push(row(vec![V::S(dn), V::I(n), V::I(s), nullable(vs, vn), nullable(asum, an), V::I(c), V::I(tu), V::I(td)]))
    });
    rows(out)
}

fn q25021(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, parent, .. } = &db.post;
    let cc = comments_per_post(db);
    let ac = parent.inv().dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1);
    let base = owned(db).with(post_type_id.eq(1));
    let rn = (&base)
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, cd)| cd, desc);
    let votes = (&rn)
        .filt(|(_, n)| n <= 5)
        .map(|((p, _), _)| p)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(children_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold((0i64, 0i64), |(u, d), (_, vt)| (u + (vt == Some(2)) as i64, d + (vt == Some(3)) as i64));
    let mut out = Vec::new();
    (&votes).and(&cc).and(&ac).drive(|p, (((u, d), c), a)| {
        let mut f = post_fields(db, p, &["owner", "title", "created"]);
        f.extend([V::I(c), V::I(a), V::I(u), V::I(d)]);
        f.push(V::S(if c > 0 {
            "Active"
        } else if a > 0 {
            "Responded"
        } else {
            "Unanswered"
        }));
        f.push(V::S(match u - d {
            x if x > 0 => "Positive Feedback",
            x if x < 0 => "Negative Feedback",
            _ => "Neutral Feedback",
        }));
        out.push(row(f))
    });
    rows(out)
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, p.CreationDate, p.Score, COALESCE((SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id), 0) AS CommentCount,
//        COALESCE((SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2), 0) AS UpvoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY p.CreationDate DESC) AS RowNum FROM Posts p WHERE p.PostTypeId = 1),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.Body, rp.Tags, rp.CreationDate, rp.Score, rp.CommentCount, rp.UpvoteCount FROM RankedPosts rp WHERE rp.RowNum <= 5),
// PostAggregates AS (SELECT f.Tags, COUNT(f.PostId) AS PostCount, SUM(f.UpvoteCount) AS TotalUpvotes, AVG(f.Score) AS AverageScore FROM FilteredPosts f GROUP BY f.Tags),
// TagDetails AS (SELECT CONCAT('<', t.TagName, '>') AS FormattedTagName, ta.PostCount, ta.TotalUpvotes, ta.AverageScore FROM PostAggregates ta
//     JOIN Tags t ON t.TagName = ANY(string_to_array(ta.Tags, '>')))
// SELECT td.FormattedTagName, td.PostCount, td.TotalUpvotes, td.AverageScore, (SELECT COUNT(*) FROM Users u WHERE u.Reputation > 1000) AS ActiveUsersWithHighReputation
// FROM TagDetails td ORDER BY td.TotalUpvotes DESC;
//
// string_to_array(Tags, '>') splits `<a><b>` into `<a`, `<b`, ``: only a piece that is exactly a tag name joins, which the `<` prefix rules out
// for every piece but the empty last one.
fn q29079(db: &'static So) -> String {
    let Post { post_type_id, creation_date, tags_str, score, .. } = &db.post;
    let up = votes_of_type(db, 2);
    let base = db.post.with(post_type_id.eq(1));
    let rn = (&base)
        .group_by(tags_str.opt())
        .select(creation_date.and(&up).and(score))
        .window(row_number, |((cd, _), _)| cd, desc);
    let agg = (&rn)
        .filt(|(_, n)| n <= 5)
        .fold((0i64, 0i64, 0i64), |(n, u, s), (((_, x), sc), _)| (n + 1, u + x, s + sc));
    let by_name: HashIdx<Str, Id<Tag>> = (&db.tag.tag_name).inv().collect();
    let high = db.user.with((&db.user.reputation).gt(1000)).fold_flat(0i64, |a, _| a + 1);
    let td = (&agg).and(Same::<Option<Str>>::new().flat_map(|tags: Option<Str>| tags.into_iter().flat_map(|t| t.split('>'))).select(&by_name));
    let mut out = Vec::new();
    td.drive(|_, ((n, u, s), t)| {
        out.push(row(vec![V::Owned(format!("<{}>", db.tag.tag_name.get(t).unwrap())), V::I(n), V::I(u), avg(s, n), V::I(high)]))
    });
    rows(out)
}

fn q2520(db: &'static So) -> String {
    let Post { owner_user, post_type_id, score, .. } = &db.post;
    let User { display_name, reputation, .. } = &db.user;
    let Vote { user, vote_type_id, .. } = &db.vote;
    let pc = owner_user.inv().select(post_type_id.and(score)).fold((0i64, 0i64, 0i64, 0i64), |(n, q, a, qs), (t, s)| {
        (n + 1, q + (t == 1) as i64, a + (t == 2) as i64, qs + if t == 1 { s } else { 0 })
    });
    let ups = db.vote.with(vote_type_id.eq(2)).select(user).inv().dense_fold_outer(db.user.id.n, 0i64, |a, _| a + 1);
    let downs = db.vote.with(vote_type_id.eq(3)).select(user).inv().dense_fold_outer(db.user.id.n, 0i64, |a, _| a + 1);
    let users = db.user.with(reputation.gt(5000)).with(&pc);
    let rn = whole(&users)
        .select(display_name.and(reputation).and(&pc).and(&ups).and(&downs))
        .window(row_number, |((((_, _), pc), _), _)| pc.0, desc);
    let mut out = Vec::new();
    (&rn).filt(|(_, n)| n <= 10).drive(
        |_, (((((dn, rep), (n, q, a, qs)), u), d), _)| {
            out.push(row(vec![
                V::S(dn),
                V::I(rep),
                V::I(n),
                V::I(q),
                V::I(a),
                V::I(u),
                V::I(d),
                if q > 0 { V::F(qs as f64 / q as f64) } else { V::Null },
            ]))
        },
    );
    rows(out)
}

fn q25446(db: &'static So) -> String {
    let Post { owner_user, post_type_id, view_count, score, .. } = &db.post;
    let usage = db.tag.group_by(Ident::<Tag>::new()).fold(0i64, |a, _| a + 1);
    let activity = owner_user.inv().select(post_type_id.and(view_count.opt()).and(score)).fold(
        (0i64, 0i64, 0i64, 0i64, 0i64),
        |(n, q, a, pop, s), ((t, v), sc)| {
            (n + 1, q + (t == 1) as i64, a + (t == 2) as i64, pop + (v.unwrap_or(0) > 100) as i64, s + sc)
        },
    );
    let mentions = tag_mentions(db);
    let owner_of = (&mentions).map(|(p, _)| p).select(owner_user);
    let tag_of = (&mentions).map(|(_, t)| t);
    let pairs: MatSet<(Id<User>, Id<Tag>)> =
        (&mentions).with((&tag_of).select((&usage).ge(10))).select((&owner_of).and(&tag_of)).collect();
    let mut out = Vec::new();
    (&pairs)
        .select((&pairs).map(|(u, _)| u).select(&activity).and((&pairs).map(|(_, t)| t).select(&usage)))
        .filt(|((n, q, _, _, s), _)| n > 0 && (q == 0 || (s as f64) / (n as f64) < 1.0))
        .drive(|(u, t), ((n, q, a, pop, _), c)| {
            out.push(row(vec![
                V::S(db.user.display_name.get(u).unwrap()),
                V::I(n),
                V::I(q),
                V::I(a),
                V::I(pop),
                V::S(db.tag.tag_name.get(t).unwrap()),
                V::I(c),
            ]))
        });
    rows(out)
}

fn q28706(db: &'static So) -> String {
    let Post { owner_user, post_type_id, creation_date, score, tags_str, .. } = &db.post;
    let User { display_name, reputation, .. } = &db.user;
    let mentions = tag_mentions(db);
    let name_of = (&mentions).map(|(_, t)| t).select(&db.tag.tag_name);
    let tag_stats = (&mentions)
        .group_by(&name_of)
        .select((&mentions).map(|(p, _)| p).select(score))
        .fold((0i64, 0i64), |(n, pos), s| (n + 1, pos + (s > 0) as i64));
    let per_user = owner_user.inv().select(score).fold((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let tu = db
        .user
        .group_by(display_name.and(reputation))
        .select(display_name.and(&per_user))
        .fold(("", 0i64, 0i64), |(_, n, s), (dn, (pn, ps))| (dn, n + pn, s + ps));
    let big = (&tu).filt(|a| a.1 > 5);
    type Key = (i64, (Str, i64));
    let tops: MatSet<(Str, i64, i64)> = whole(&big)
        .select(Same::new().and(&tu))
        .window(row_number, |(k, a)| (a.2, k), |x: &Key, y: &Key| y.0.cmp(&x.0).then(x.1.cmp(&y.1)))
        .filt(|(_, n)| n <= 10)
        .map(|((_, a), _)| a)
        .collect();
    let tops_by_name: HashIdx<Str, (Str, i64, i64)> = (&tops).map(|(dn, _, _)| dn).inv().collect();
    let post_names: MatSet<(Id<Post>, Str)> = (&mentions).map(|(p, _)| p).and(&name_of).collect();
    let names_by_post: HashIdx<Id<Post>, Str> = (&post_names).map(|(p, _)| p).inv().select((&post_names).map(|(_, n)| n)).collect();

    let base = owned(db).with(post_type_id.eq(1));
    let rn = (&base)
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, cd)| cd, desc);
    let mut out = Vec::new();
    (&rn)
        .filt(|(_, n)| n == 1)
        .map(|((p, _), _)| p)
        .select(Ident::<Post>::new().and((&names_by_post).select(Same::<Str>::new().and(&tag_stats))).and(owner_user.select(display_name).select(&tops_by_name)))
        .drive(|_, ((p, (t, (tn, _))), (dn, n, s))| {
            let mut f = post_fields(db, p, &["id", "title", "body", "created", "owner"]);
            f.push(oint(tags_str.get(p).map(|x| x.matches('>').count() as i64)));
            f.extend([V::S(t), V::I(tn)]);
            f.extend([V::S(dn), V::I(n), V::I(s)]);
            out.push(row(f))
        });
    rows(out)
}

fn q7784(db: &'static So) -> String {
    let Post { post_type, creation_date, score, view_count, origid, owner_display_name, .. } = &db.post;
    let PostHistory { post, creation_date: hd, comment, text, .. } = &db.post_history;
    let base = owned(db).with(creation_date.ge(year_ago()));
    let rn = (&base)
        .group_by(post_type)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((_, s), v)| (s, v), desc);
    let type_by_id: HashIdx<i64, Str> = (&db.post_type.origid).inv().select(&db.post_type.name).collect();
    let stats = (&rn)
        .filt(|(_, n)| n <= 10)
        .map(|(((p, _), _), _)| p)
        .group_by(origid.select(&type_by_id))
        .select(score.and(view_count.opt()))
        .fold((0i64, 0i64, 0i64, 0i64), |(n, s, vn, vs), (sc, v)| (n + 1, s + sc, vn + v.is_some() as i64, vs + v.unwrap_or(0)));
    let history = db.post_history.with(hd.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -6)));
    let mut out = Vec::new();
    (&stats).filt(|a| a.0 > 0).cross((&history).select(post)).drive(|(name, h), ((n, s, vn, vs), p)| {
        let mut f = vec![V::S(name), V::I(n), avg(s, n), avg(vs, vn)];
        f.extend(post_fields(db, p, &["id", "title", "created"]));
        f.push(V::T(hd.get(h).unwrap()));
        f.push(ostr(owner_display_name.get(p)));
        f.extend(post_fields(db, p, &["score"]));
        f.extend([ostr(comment.get(h)), ostr(text.get(h))]);
        out.push(row(f))
    });
    rows(out)
}

pub const ENTRIES: &[harness::Entry] = &[
    ("6659", q6659),
    ("28192", q28192),
    ("8660", q8660),
    ("25895", q25895),
    ("25518", q25518),
    ("8110", q8110),
    ("25021", q25021),
    ("29079", q29079),
    ("2520", q2520),
    ("25446", q25446),
    ("28706", q28706),
    ("7784", q7784),
];
