use harness::prelude::*;

fn leak_join(parts: impl IntoIterator<Item = Str>, sep: &str) -> Str {
    Box::leak(parts.into_iter().collect::<Vec<_>>().join(sep).into_boxed_str())
}

fn now() -> i64 {
    ts(2024, 10, 1, 12, 34, 56)
}

fn q11653(db: &'static So) -> String {
    let Post { creation_date, owner_user, tags_str, .. } = &db.post;
    let elems = tags_str.flat_map(|t: Str| t.split(','));
    let mut v = Vec::new();
    db.post
        .with(owner_user)
        .group_by(Ident::<Post>::new().and(creation_date).and(elems.opt()))
        .select(comments_of(db).opt().and(votes_of(db).opt()))
        .fold((0i64, 0i64), |(c, n), (ci, vi)| (c + ci.is_some() as i64, n + vi.is_some() as i64))
        .drive(|((p, cd), t), (c, n)| v.push((cd, p, t, c, n)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(100).map(|&(_, p, t, c, n)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c), V::I(n)]);
        f.extend(post_fields(db, p, &["owner", "rep"]));
        f.push(ostr(t));
        row(f)
    }))
}

fn q11554(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, tags_str, .. } = &db.post;
    let elems = tags_str.flat_map(|t: Str| t.split(','));
    let mut v = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .with(owner_user)
        .group_by(Ident::<Post>::new().and(creation_date).and(elems.opt()))
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold((0i64, 0i64, 0i64), |(c, up, down), (ci, vt)| {
            (c + ci.is_some() as i64, up + (vt == Some(2)) as i64, down + (vt == Some(3)) as i64)
        })
        .drive(|((p, cd), t), (c, up, down)| v.push((cd, p, t, c, up, down)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(100).map(|&(_, p, t, c, up, down)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::I(c), V::I(up), V::I(down)]);
        f.extend(post_fields(db, p, &["views", "score"]));
        f.push(ostr(t));
        row(f)
    }))
}

fn q5603(db: &'static So) -> String {
    let PostHistory { post, post_history_type_id, creation_date, .. } = &db.post_history;
    let key = post
        .select(&db.post.owner_user)
        .select(&db.user.display_name)
        .and(post.select((&db.post.title).opt()))
        .and(post_history_type_id);
    let mut out = Vec::new();
    db.post_history
        .with(creation_date.ge(add_years(now(), -1)))
        .group_by(key)
        .select(creation_date.and(post_history_type_id))
        .fold((0i64, 0i64, 0i64, 0i64), |(n, us, tb, cr), (cd, t)| {
            (
                n + 1,
                us + (now() - cd),
                tb + matches!(t, 4 | 5 | 6) as i64,
                cr + matches!(t, 10 | 11 | 12) as i64,
            )
        })
        .filt(|(n, _, _, _)| n > 5)
        .drive(|((dn, ti), ty), (n, us, tb, cr)| {
            out.push(row(vec![V::S(dn), ostr(ti), V::I(ty), V::I(n), V::I(tb), V::I(cr), V::F(us as f64 / n as f64 / 1e6)]))
        });
    rows(out)
}

// COUNT(DISTINCT ...) alongside ordinary aggregates on the same group: two
// folds over the same key, joined by probing one from the other.
fn q13456(db: &'static So) -> String {
    let Post { post_type, score, owner_user_id, creation_date, .. } = &db.post;
    let cutoff = add_years(now(), -1);
    let main = db
        .post
        .with(creation_date.ge(cutoff))
        .group_by(post_type.select(&db.post_type.name))
        .select(score)
        .fold((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let uniq = db
        .post
        .with(creation_date.ge(cutoff))
        .group_by(post_type.select(&db.post_type.name))
        .select(owner_user_id)
        .count_distinct();
    let mut out = Vec::new();
    main.and((&uniq).opt()).drive(|k, ((n, s), u)| {
        out.push(row(vec![V::S(k), V::I(n), avg(s, n), V::I(u.unwrap_or(0))]))
    });
    rows(out)
}

fn q12038(db: &'static So) -> String {
    let Post { post_type, score, view_count, answer_count, creation_date, owner_user_id, .. } =
        &db.post;
    let main = db
        .post
        .group_by(post_type.select(&db.post_type.name))
        .select(
            score
                .and(view_count.opt())
                .and(answer_count.opt())
                .and(creation_date),
        )
        .fold(
            (0i64, 0i64, 0i64, 0i64, 0i64, 0i64, i64::MIN, i64::MAX),
            |(n, pos, vn, vs, an, asum, hi, lo), (((s, v), a), cd)| {
                (
                    n + 1,
                    pos + (s > 0) as i64,
                    vn + v.is_some() as i64,
                    vs + v.unwrap_or(0),
                    an + a.is_some() as i64,
                    asum + a.unwrap_or(0),
                    hi.max(cd),
                    lo.min(cd),
                )
            },
        );
    let uniq = db
        .post
        .group_by(post_type.select(&db.post_type.name))
        .select(owner_user_id)
        .count_distinct();
    let mut out = Vec::new();
    main.and((&uniq).opt()).drive(
        |k, ((n, pos, vn, vs, an, asum, hi, lo), u)| {
            out.push(row(vec![
                V::S(k),
                V::I(n),
                V::I(pos),
                avg(vs, vn),
                avg(asum, an),
                V::T(hi),
                V::T(lo),
                V::I(u.unwrap_or(0)),
            ]))
        },
    );
    rows(out)
}

fn q10616(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, owner_user, .. } = &db.post;
    let main = owner_user
        .inv()
        .select(
            score
                .and(view_count.opt())
                .and(post_type_id)
                .and(comments_of(db).opt()),
        )
        .fold(
            (0i64, 0i64, 0i64, 0i64, 0i64, 0i64, 0i64),
            |(n, q, a, vn, vs, ss, _), (((s, v), t), _c)| {
                (
                    n + 1,
                    q + (t == 1) as i64,
                    a + (t == 2) as i64,
                    vn + v.is_some() as i64,
                    vs + v.unwrap_or(0),
                    ss + s,
                    0,
                )
            },
        );
    let cdist = owner_user.inv().select(comments_of(db)).count_distinct();
    let mut out = Vec::new();
    main.and((&cdist).opt()).drive(|u, ((n, q, a, vn, vs, ss, _), cd)| {
        out.push(row(vec![
            V::S(db.user.display_name.get(u).unwrap()),
            V::I(n),
            V::I(q),
            V::I(a),
            nullable(vs, vn),
            nullable(ss, n),
            avg(vs, vn),
            avg(ss, n),
            V::I(cd.unwrap_or(0)),
        ]))
    });
    rows(out)
}

// Three LEFT JOINs off one user: (posts x their votes) x badges. `.and` of two
// probeable children IS that cross product, so COUNT(p.Id) counts posts once
// per vote AND once per badge, which is what DuckDB does.
fn q7289(db: &'static So) -> String {
    let User { display_name, reputation, creation_date, .. } = &db.user;
    let pv = posts_of(db).select(
        ptype_name(db)
            .and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt()),
    );
    let nb = badges_per_user(db);
    let agg = db
        .user
        .with(creation_date.gt(date(2020, 1, 1)))
        .select(pv.opt().and(badges_of(db).opt()))
        .fold((0i64, 0i64, 0i64, 0i64, 0i64), |(n, q, a, bs, bn), (p, _b)| {
            let bounty = p.and_then(|(_, v)| v).flatten();
            (
                n + p.is_some() as i64,
                q + p.map_or(false, |(t, _)| t == "Question") as i64,
                a + p.map_or(false, |(t, _)| t == "Answer") as i64,
                bs + bounty.unwrap_or(0),
                bn + bounty.is_some() as i64,
            )
        });
    let mut v = Vec::new();
    (&agg).filt(|(n, _, _, _, _)| n > 10).and(display_name.and(reputation).and(&nb))
        .drive(|_, ((n, q, a, bs, bn), ((dn, rep), badges))| v.push((rep, dn, n, q, a, badges, bs, bn)));
    v.sort_by(|x, y| y.0.cmp(&x.0));
    rows(v.iter().take(50).map(|&(rep, dn, n, q, a, badges, bs, bn)| {
        row(vec![
            V::S(dn),
            V::I(n),
            V::I(q),
            V::I(a),
            V::F(rep as f64),
            V::I(badges),
            nullable(bs, bn),
        ])
    }))
}

fn q19122(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let excerpt_tags = (&db.tag.excerpt_post)
        .inv()
        .select(&db.tag.tag_name)
        .buf_fold(|names| leak_join(names, ", "));
    let mut v = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .with(owner_user)
        .select(creation_date.and((&excerpt_tags).opt()))
        .drive(|p, (cd, t)| v.push((cd, p, t)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(10).map(|&(_, p, t)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner", "score", "views"]);
        f.push(ostr(t));
        row(f)
    }))
}

// WITH PopularQuestions AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, U.Reputation AS OwnerReputation, T.TagName
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id JOIN Tags T ON T.ExcerptPostId = P.Id WHERE P.PostTypeId = 1 ORDER BY P.Score DESC LIMIT 100)
// SELECT PQ.PostId, PQ.Title, PQ.CreationDate, PQ.Score, PQ.ViewCount, PQ.OwnerReputation, string_agg(PQ.TagName, ', ') AS Tags
// FROM PopularQuestions PQ GROUP BY PQ.PostId, PQ.Title, PQ.CreationDate, PQ.Score, PQ.ViewCount, PQ.OwnerReputation ORDER BY PQ.Score DESC;
//
// No question is a tag's excerpt post, so the result is empty.
fn q11576(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, .. } = &db.post;
    let excerpts: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    let v = drain(db.post.with(post_type_id.eq(1)).with(owner_user).select(score.and(&excerpts)));
    let v = top_n(v, |&(p, (s, t))| (std::cmp::Reverse(s), p, t), 100);
    type R = (Id<Post>, (i64, Id<Tag>));
    let g = rel(v).group_by(Same::<R>::new().map(|(p, _): R| p)).select(Same::<R>::new().map(|(_, (_, t)): R| db.tag.tag_name.get(t).unwrap())).buf_fold(|ns| leak_join(ns.iter().copied(), ", "));
    rows(drain(&g).into_iter().map(|(p, tags)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "rep"]);
        f.push(V::S(tags));
        row(f)
    }))
}

fn q11125(db: &'static So) -> String {
    let PostHistory { post, post_history_type_id, creation_date, user, .. } = &db.post_history;
    let mut out = Vec::new();
    db.post_history
        .with(user)
        .group_by(post)
        .select(
            creation_date
                .and(post_history_type_id)
                .and(user.select(&db.user.reputation))
                .and(post.select((&db.post.tags_str).opt())),
        )
        .fold(
            (0i64, i64::MAX, i64::MIN, 0i64, 0i64, None::<Str>),
            |(n, lo, hi, cr, rs, tags), (((cd, t), rep), tg)| {
                (
                    n + 1,
                    lo.min(cd),
                    hi.max(cd),
                    cr + matches!(t, 10 | 11) as i64,
                    rs + rep,
                    tags.or(tg),
                )
            },
        )
        .drive(|p, (n, lo, hi, cr, rs, tags)| {
            out.push(row(vec![
                V::I(db.post.origid.get(p).unwrap()),
                V::I(n),
                V::T(lo),
                V::T(hi),
                ostr(tags),
                V::I(cr),
                avg(rs, n),
            ]))
        });
    rows(out)
}

// RANK() OVER (ORDER BY BadgeCount DESC, TotalViewCount DESC) with the rank
// itself filtered. TotalViewCount is a SUM that can be NULL, and DuckDB sorts
// NULLs last in DESC, which `Option`'s own order does not — hence the cmp.
fn q5531(db: &'static So) -> String {
    let Post { post_type_id, view_count, owner_user, .. } = &db.post;
    let User { display_name, .. } = &db.user;
    let badge_count = badges_per_user(db);
    let stats = owner_user
        .inv()
        .select(post_type_id.and(view_count.opt()))
        .fold((0i64, 0i64, 0i64, 0i64, 0i64), |(n, q, a, vn, vs), (t, v)| {
            (
                n + 1,
                q + (t == 1) as i64,
                a + (t == 2) as i64,
                vn + v.is_some() as i64,
                vs + v.unwrap_or(0),
            )
        });
    let base = db.user.with(&stats);
    let cmp = |a: &(i64, Option<i64>), b: &(i64, Option<i64>)| {
        b.0.cmp(&a.0).then_with(|| match (a.1, b.1) {
            (Some(x), Some(y)) => y.cmp(&x),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        })
    };
    let rk = whole(&base)
        .select(display_name.and(&badge_count).and(&stats))
        .window(rank, |((_, bc), s)| (bc, if s.3 == 0 { None } else { Some(s.4) }), cmp);
    let mut out = Vec::new();
    (&rk)
        .filt(|(_, r)| r <= 10)
        .drive(|_, (((dn, bc), st), r)| {
            out.push(row(vec![
                V::S(dn),
                V::I(bc),
                V::I(st.0),
                V::I(st.1),
                V::I(st.2),
                nullable(st.4, st.3),
                V::I(r),
            ]))
        });
    rows(out)
}

pub static ENTRIES: &[harness::Entry] = &[
    ("5531", q5531),
    ("5603", q5603),
    ("7289", q7289),
    ("10616", q10616),
    ("11125", q11125),
    ("11554", q11554),
    ("11576", q11576),
    ("11653", q11653),
    ("12038", q12038),
    ("13456", q13456),
    ("19122", q19122),
];
