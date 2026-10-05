use harness::prelude::*;
use std::cmp::Reverse;

fn leak(s: String) -> Str {
    Box::leak(s.into_boxed_str())
}

fn agg_distinct(mut v: Vec<&'static str>, sep: &str) -> Option<Str> {
    v.sort();
    v.dedup();
    if v.is_empty() { None } else { Some(leak(v.join(sep))) }
}

fn by_first<A: Copy + Eq + std::hash::Hash, B: Copy + Eq + std::hash::Hash>(m: &MatSet<(A, B)>) -> HashIdx<A, B> {
    m.map(|x: (A, B)| x.0).inv().select(m.map(|x: (A, B)| x.1)).collect()
}

fn close_reasons(db: &'static So) -> HashIdx<i64, Id<CloseReasonType>> {
    (&db.close_reason_type.origid).inv().collect()
}

fn users_by_origid(db: &'static So) -> HashIdx<i64, Id<User>> {
    (&db.user.origid).inv().collect()
}

fn like(s: &str, pat: &str) -> bool {
    let (s, p): (Vec<char>, Vec<char>) = (s.chars().collect(), pat.chars().collect());
    let (mut i, mut j, mut star, mut mark) = (0, 0, usize::MAX, 0);
    while i < s.len() {
        if j < p.len() && (p[j] == '_' || (p[j] != '%' && p[j] == s[i])) {
            i += 1;
            j += 1;
        } else if j < p.len() && p[j] == '%' {
            star = j;
            mark = i;
            j += 1;
        } else if star != usize::MAX {
            j = star + 1;
            mark += 1;
            i = mark;
        } else {
            return false;
        }
    }
    while j < p.len() && p[j] == '%' {
        j += 1;
    }
    j == p.len()
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, u.DisplayName AS OwnerDisplayName, RANK() OVER (PARTITION BY p.PostTypeId
//        ORDER BY p.Score DESC, p.CreationDate DESC) AS RankScore FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' -
//        INTERVAL '6 months' AND p.PostTypeId = 1), TaggedPosts AS (SELECT rp.Id, rp.Title, STRING_AGG(t.TagName, ', ' ORDER BY t.TagName) AS Tags, rp.OwnerDisplayName,
//        rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.RankScore FROM RankedPosts rp JOIN LATERAL (SELECT unnest(string_to_array(rp.Title, ' ')) AS TagName) t ON
//        t.TagName IS NOT NULL GROUP BY rp.Id, rp.Title, rp.OwnerDisplayName, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.RankScore) SELECT tp.Title,
//        tp.OwnerDisplayName, tp.CreationDate, tp.Score, tp.ViewCount, tp.AnswerCount, tp.Tags FROM TaggedPosts tp WHERE tp.RankScore <= 10 ORDER BY tp.Score DESC, tp.CreationDate
//        DESC;
//
// Rewritten (rewrites/6634.sql): the STRING_AGG gets ORDER BY its own argument.
fn q6634(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, score, title, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -6)))
        .with(post_type_id.eq(1))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(rank, |((_, s), d)| (Reverse(s), Reverse(d)), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 10).map(|(((p, _), _), _)| p).collect();
    let v = drain((&tp).select(Ident::<Post>::new().and(owner_user).and(title)));
    rows(v.into_iter().map(|(_, ((p, u), t))| {
        let mut w: Vec<&str> = t.split(' ').collect();
        w.sort();
        let mut f = post_fields(db, p, &["title"]);
        f.push(user_col(db, u, "name"));
        f.extend(post_fields(db, p, &["created", "score", "views", "answers"]));
        f.push(V::Owned(w.join(", ")));
        row(f)
    }))
}

// WITH TagFilter AS (SELECT DISTINCT unnest(string_to_array(substring(Tags, 2, length(Tags)-2), '><')) AS TagName, p.Id AS PostId FROM Posts p WHERE PostTypeId = 1),
//        PostMetrics AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, COALESCE(COUNT(DISTINCT c.Id), 0) AS CommentCount, COALESCE(COUNT(DISTINCT a.Id), 0) AS AnswerCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes FROM Posts p LEFT
//        JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2 LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >=
//        cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.ViewCount), RankedPosts AS (SELECT pm.*, ROW_NUMBER() OVER (ORDER BY pm.UpVotes -
//        pm.DownVotes DESC, pm.ViewCount DESC) AS RankPosition FROM PostMetrics pm) SELECT p.Title, p.ViewCount, p.CommentCount, p.AnswerCount, p.UpVotes, p.DownVotes, tg.TagName
//        FROM RankedPosts p JOIN TagFilter tg ON p.PostId = tg.PostId WHERE p.RankPosition <= 10 ORDER BY p.RankPosition, tg.TagName;
fn q25028(db: &'static So) -> String {
    let Post { creation_date, post_type_id, tags_str, view_count, .. } = &db.post;
    let rp = || db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).group_by(Ident::<Post>::new());
    let ud = rp()
        .select(comments_of(db).opt().and(answers_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let nc = rp().select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let na = rp().select(answers_of(db).opt()).fold(0i64, |n, a| n + a.is_some() as i64);
    let pm = (&nc).and(&na).and(&ud).map(|((c, a), u): ((i64, i64), [i64; 2])| [c, a, u[0], u[1]]);
    let top = top_n(drain(&pm), |&(p, a)| {
        let w = view_count.get(p);
        (Reverse(a[2] - a[3]), w.is_none(), Reverse(w))
    }, 10);
    let tf: MatSet<(Id<Post>, Str)> = db.post.with(post_type_id.eq(1)).select(Ident::<Post>::new().and(tags_str.flat_map(tag_list))).collect();
    type T = (Id<Post>, Str);
    let tfi: HashIdx<Id<Post>, T> = (&tf).map(|x: T| x.0).inv().select(&tf).collect();
    type R = (Id<Post>, [i64; 4]);
    let tv = rel(top);
    let v = drain((&tv).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0).select(&tfi))));
    rows(v.into_iter().map(|(_, ((p, a), (_, t)))| {
        let mut f = post_fields(db, p, &["title", "views"]);
        f.extend(a.map(V::I));
        f.push(V::S(t));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, p.OwnerUserId, u.DisplayName AS OwnerDisplayName, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY
//        p.Tags ORDER BY p.CreationDate DESC) AS TagRank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01
//        12:34:56' - INTERVAL '1 year'), PopularTags AS (SELECT UNNEST(string_to_array(Tags, '><')) AS Tag FROM RankedPosts WHERE TagRank <= 3), TagPopularity AS (SELECT Tag,
//        COUNT(*) AS TagCount FROM PopularTags GROUP BY Tag ORDER BY TagCount DESC LIMIT 10) SELECT tp.Tag, tp.TagCount, COUNT(DISTINCT p.Id) AS QuestionCount,
//        COALESCE(SUM(c.Score), 0) AS TotalComments, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes FROM TagPopularity tp JOIN Posts p ON p.Tags
//        LIKE '%' || tp.Tag || '%' LEFT JOIN Comments c ON c.PostId = p.Id LEFT JOIN Votes v ON v.PostId = p.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01
//        12:34:56' - INTERVAL '1 year' GROUP BY tp.Tag, tp.TagCount ORDER BY tp.TagCount DESC;
fn q29520(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, tags_str, .. } = &db.post;
    let cut = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    let q = || Ident::<Post>::new().with(post_type_id.eq(1)).with(creation_date.ge(cut));
    let w = db.post.select(q()).with(owner_user).group_by(tags_str.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tc = (&w)
        .filt(|(_, k)| k <= 3)
        .map(|((p, _), _)| p)
        .select(tags_str.flat_map(|t: Str| t.split("><")))
        .group_by(Same::<Str>::new())
        .fold(0i64, |a, _| a + 1);
    let tt = top_n(drain(&tc), |&(t, n)| (Reverse(n), t), 10);
    let by_tags: HashIdx<Str, Id<Post>> = db.post.select(q()).select(tags_str).inv().collect();
    type T = (Str, i64);
    let ttv = rel(tt);
    let m = || (&ttv).group_by(Same::<T>::new()).select(Same::<T>::new().map(|x: T| x.0).select_where(&by_tags, |t: Str, s: Str| like(s, &format!("%{t}%"))));
    let qn = m().count_distinct();
    let ag = m()
        .select(comments_of(db).select(&db.comment.score).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (c, t)| [a[0] + c.unwrap_or(0), a[1] + (t == Some(2)) as i64]);
    let g = (&qn).and(&ag).map(|(n, a): (i64, [i64; 2])| [n, a[0], a[1]]);
    rows(drain(&g).into_iter().map(|((t, n), a)| row(vec![V::S(t), V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2])])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, pt.Name AS PostTypeName, ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY
//        p.Score DESC) AS RankByScore, ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.CreationDate DESC) AS RankByDate FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id
//        WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year') SELECT u.DisplayName AS UserDisplayName, u.Reputation, r.PostId, r.Title, r.CreationDate, r.ViewCount, r.Score,
//        r.PostTypeName, COALESCE(comments.CommentCount, 0) AS TotalComments, (SELECT COUNT(*) FROM Votes v WHERE v.PostId = r.PostId AND v.VoteTypeId IN (2, 3)) AS TotalVotes,
//        (SELECT STRING_AGG(DISTINCT b.Name, ', ') FROM Badges b WHERE b.UserId = u.Id) AS UserBadges FROM RankedPosts r LEFT JOIN Users u ON r.PostId = u.Id LEFT JOIN (SELECT
//        PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) comments ON r.PostId = comments.PostId WHERE r.RankByScore <= 5 AND u.Reputation IS NOT NULL ORDER BY CASE
//        WHEN r.PostTypeName = 'Question' THEN 1 WHEN r.PostTypeName = 'Answer' THEN 2 ELSE 3 END, r.Score DESC, r.CreationDate DESC;
fn q21130(db: &'static So) -> String {
    let Post { creation_date, score, origid, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(current_date(), -1))).group_by(ptype_name(db)).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|((p, _), _)| p).collect();
    let uo = users_by_origid(db);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |a, _| a + 1);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([2, 3])))).fold(0i64, |a, _| a + 1);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.name)).buf_fold(|v| agg_distinct(v.to_vec(), ", ").unwrap());
    let v = drain((&tp).select(Ident::<Post>::new().and(origid.select(&uo).select(Ident::<User>::new().and((&ub).opt()))).and((&cc).opt()).and((&vc).opt())));
    let key = |n: Str| match n {
        "Question" => 1,
        "Answer" => 2,
        _ => 3,
    };
    let v = top_n(v, |&(p, _)| (key(ptype_name(db).get(p).unwrap()), Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())), 0);
    rows(v.into_iter().map(|(_, (((p, (u, b)), c), n))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["id", "title", "created", "views", "score", "type"]));
        f.extend([V::I(c.unwrap_or(0)), V::I(n.unwrap_or(0)), ostr(b)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, p.CreationDate, p.Score, u.DisplayName AS OwnerDisplayName, COUNT(a.Id) AS AnswerCount, ROW_NUMBER()
//        OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Posts a ON p.Id = a.ParentId WHERE p.PostTypeId =
//        1 GROUP BY p.Id, p.Title, p.Body, p.Tags, p.CreationDate, p.Score, u.DisplayName), PostTagCounts AS (SELECT p.Id AS PostId, UNNEST(string_to_array(substring(p.Tags, 2,
//        LENGTH(p.Tags) - 2), '><')) AS Tag FROM Posts p WHERE p.PostTypeId = 1), TagStats AS (SELECT Tag, COUNT(*) AS TagUsageCount FROM PostTagCounts GROUP BY Tag), TopTags AS
//        (SELECT Tag FROM TagStats ORDER BY TagUsageCount DESC LIMIT 10) SELECT rp.PostId, rp.Title, rp.Body, rp.CreationDate, rp.Score, rp.OwnerDisplayName, rp.AnswerCount,
//        (SELECT STRING_AGG(tt.Tag, ', ' ORDER BY tt.Tag) FROM TopTags tt JOIN PostTagCounts pt ON tt.Tag = pt.Tag WHERE pt.PostId = rp.PostId) AS PopularTags FROM RankedPosts rp
//        WHERE rp.rn = 1 ORDER BY rp.CreationDate DESC LIMIT 20;
//
// Rewritten (rewrites/29639.sql): the STRING_AGG gets ORDER BY its own argument.
fn q29639(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, tags_str, .. } = &db.post;
    let top = top_n(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 20);
    let tp: MatSet<Id<Post>> = rel(top).map(|x: (Id<Post>, Id<User>)| x.0).collect();
    let tc = db.post.with(post_type_id.eq(1)).select(tags_str.flat_map(tag_list)).group_by(Same::<Str>::new()).fold(0i64, |a, _| a + 1);
    let tt: MatSet<Str> = rel(top_n(drain(&tc), |&(t, n)| (Reverse(n), t), 10)).map(|x: (Str, i64)| x.0).collect();
    let ac = (&tp).group_by(Ident::<Post>::new()).select(children_of(db).opt()).fold(0i64, |a, c| a + c.is_some() as i64);
    let pt = (&tp).group_by(Ident::<Post>::new()).select(tags_str.flat_map(tag_list).with(&tt)).buf_fold(|v| {
        let mut v = v.to_vec();
        v.sort();
        leak(v.join(", "))
    });
    let v = drain((&tp).select(Ident::<Post>::new().and(&ac).and((&pt).opt())));
    rows(v.into_iter().map(|(_, ((p, a), t))| {
        let mut f = post_fields(db, p, &["id", "title", "body", "created", "score", "owner"]);
        f.extend([V::I(a), ostr(t)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.Body, P.CreationDate, U.DisplayName AS OwnerDisplayName, P.ViewCount, P.Score, ROW_NUMBER() OVER (PARTITION BY
//        P.OwnerUserId ORDER BY P.ViewCount DESC, P.CreationDate DESC) AS rn FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.PostTypeId = 1 AND P.Score > 10), PostTags
//        AS (SELECT P.Id AS PostId, STRING_AGG(TRIM(TAG.TagName), ', ' ORDER BY TRIM(TAG.TagName)) AS TagList FROM Posts P CROSS JOIN LATERAL (SELECT
//        UNNEST(string_to_array(SUBSTRING(P.Tags, 2, LENGTH(P.Tags) - 2), '><')) AS TagName) AS TAG GROUP BY P.Id), ClosedPosts AS (SELECT PH.PostId, PH.CreationDate AS
//        ClosedDate, C.Name AS CloseReason FROM PostHistory PH JOIN CloseReasonTypes C ON CAST(PH.Comment AS INT) = C.Id WHERE PH.PostHistoryTypeId = 10) SELECT RP.PostId,
//        RP.Title, RP.Body, RP.CreationDate AS QuestionDate, RP.OwnerDisplayName, RP.ViewCount, RP.Score, PT.TagList, CP.ClosedDate, CP.CloseReason FROM RankedPosts RP LEFT JOIN
//        PostTags PT ON RP.PostId = PT.PostId LEFT JOIN ClosedPosts CP ON RP.PostId = CP.PostId WHERE RP.rn = 1 ORDER BY RP.Score DESC, RP.ViewCount DESC;
//
// Rewritten (rewrites/28634.sql): the STRING_AGG gets ORDER BY its own argument.
fn q28634(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, score, view_count, tags_str, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1))
        .with(score.gt(10))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(view_count.opt()).and(creation_date))
        .window(row_number, |((p, w), d)| (w.is_none(), Reverse(w), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k == 1).map(|(((p, _), _), _)| p).collect();
    let pt = (&tp).group_by(Ident::<Post>::new()).select(tags_str.flat_map(tag_list).map(|t: Str| t.trim())).buf_fold(|v| {
        let mut v = v.to_vec();
        v.sort();
        leak(v.join(", "))
    });
    let cr = close_reasons(db);
    let PostHistory { post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let cp = history_of(db).select(
        Ident::<PostHistory>::new()
            .with(post_history_type_id.eq(10))
            .select(hd.and(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&cr).select(&db.close_reason_type.name))),
    );
    let v = drain((&tp).select(Ident::<Post>::new().and((&pt).opt()).and(cp.opt())));
    rows(v.into_iter().map(|(_, ((p, t), c))| {
        let mut f = post_fields(db, p, &["id", "title", "body", "created", "owner", "views", "score"]);
        f.push(ostr(t));
        f.extend(match c {
            Some((d, n)) => [V::T(d), V::S(n)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//        FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.Score IS NOT NULL), UserVotes AS (SELECT v.PostId, COUNT(CASE WHEN vt.Name
//        = 'UpMod' THEN 1 END) AS Upvotes, COUNT(CASE WHEN vt.Name = 'DownMod' THEN 1 END) AS Downvotes FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY v.PostId),
//        CloseReasons AS (SELECT ph.PostId, STRING_AGG(DISTINCT cr.Name, ', ') AS CloseReasonNames FROM PostHistory ph JOIN CloseReasonTypes cr ON CAST(ph.Comment AS INT) = cr.Id
//        WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId) SELECT rp.Title AS PostTitle, CASE WHEN rp.Rank <= 3 THEN 'Top Post' ELSE 'Other Post' END AS PostCategory,
//        COALESCE(uv.Upvotes, 0) AS Upvotes, COALESCE(uv.Downvotes, 0) AS Downvotes, COALESCE(cr.CloseReasonNames, 'No close reasons') AS CloseReasons FROM RankedPosts rp LEFT
//        JOIN UserVotes uv ON rp.PostId = uv.PostId LEFT JOIN CloseReasons cr ON rp.PostId = cr.PostId WHERE rp.Rank <= 10 ORDER BY rp.Score DESC, rp.Title;
//
// A score tie inside the ROW_NUMBER goes to the smaller post id (the SQL leaves it open).
fn q22259(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, title, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    type R = (Id<Post>, i64);
    let rv: MatSet<R> = (&w).filt(|(_, k)| k <= 10).map(|((p, _), k)| (p, k)).collect();
    let tp: MatSet<Id<Post>> = (&rv).map(|x: R| x.0).collect();
    let uv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(vtype_name(db))).fold([0i64; 2], |a, n| [a[0] + (n == "UpMod") as i64, a[1] + (n == "DownMod") as i64]);
    let cr = close_reasons(db);
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let crn = db
        .post_history
        .with(post_history_type_id.is_in([10, 11]))
        .group_by(post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&cr).select(&db.close_reason_type.name))
        .buf_fold(|v| agg_distinct(v.to_vec(), ", ").unwrap());
    let k = || Same::<R>::new().map(|x: R| x.0);
    let v = drain((&rv).select(Same::<R>::new().and(k().select(title.opt())).and(k().select(&uv).opt()).and(k().select(&crn).opt())));
    rows(v.into_iter().map(|(_, ((((_, n), t), a), c))| {
        let a = a.unwrap_or([0; 2]);
        row(vec![ostr(t), V::S(if n <= 3 { "Top Post" } else { "Other Post" }), V::I(a[0]), V::I(a[1]), V::S(c.unwrap_or("No close reasons"))])
    }))
}


// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC,
//        p.CreationDate ASC) AS Rank FROM Posts p WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' AND p.Score > 0), UserActivity AS (SELECT
//        u.Id AS UserId, COUNT(DISTINCT p.Id) AS PostsCount, SUM(v.BountyAmount) AS TotalBounty, AVG(p.ViewCount) AS AverageViews FROM Users u LEFT JOIN Posts p ON u.Id =
//        p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (2, 3) GROUP BY u.Id) SELECT r.PostId, r.Title, r.ViewCount, ua.UserId, ua.PostsCount,
//        ua.TotalBounty, CASE WHEN r.Rank <= 5 THEN 'Top Post' ELSE 'Regular Post' END AS PostType, CASE WHEN ua.PostsCount IS NULL THEN 'Inactive' ELSE 'Active' END AS
//        UserStatus, COALESCE((SELECT STRING_AGG(CONCAT(b.Name, ' - ', CAST(b.Class AS TEXT)), ', ') FROM Badges b WHERE b.UserId = ua.UserId), 'No Badges') AS UserBadges FROM
//        RankedPosts r JOIN UserActivity ua ON r.PostId = ua.UserId LEFT JOIN Users u ON r.PostId = u.Id WHERE r.Rank <= 10 ORDER BY r.Score DESC, r.ViewCount DESC;
//
// `r.PostId = ua.UserId` joins a post id to a user id, as written. The badge STRING_AGG order is left open by the SQL; the port joins in badge id order.
fn q3681(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, origid, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(score.gt(0))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), d, p), asc);
    type R = (Id<Post>, i64);
    let rv: MatSet<R> = (&w).filt(|(_, k)| k <= 10).map(|(((p, _), _), k)| (p, k)).collect();
    let uo = users_by_origid(db);
    let k = || Same::<R>::new().map(|x: R| x.0);
    let mu: MatSet<Id<User>> = (&rv).select(k().select(origid).select(&uo)).collect();
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let ua = (&mu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.is_in([2, 3]))).select(bounty_amount.opt()).opt()).opt())
        .fold((0i64, 0i64), |(s, n), b| match b.flatten().flatten() {
            Some(x) => (s + x, n + 1),
            None => (s, n),
        });
    let pc = (&mu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let ub = (&mu)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).select((&db.badge.name).and(&db.badge.class)))
        .buf_fold(|v| leak(v.iter().map(|(n, c)| format!("{n} - {c}")).collect::<Vec<_>>().join(", ")));
    let v = drain((&rv).select(Same::<R>::new().and(k().select(origid).select(&uo).select(Ident::<User>::new().and(&ua).and(&pc).and((&ub).opt())))));
    rows(v.into_iter().map(|(_, ((p, n), (((u, (s, bn)), c), b)))| {
        let mut f = post_fields(db, p, &["id", "title", "views"]);
        f.extend([user_col(db, u, "uid"), V::I(c), nullable(s, bn)]);
        f.extend([V::S(if n <= 5 { "Top Post" } else { "Regular Post" }), V::S("Active"), V::S(b.unwrap_or("No Badges"))]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(SUM(V.BountyAmount), 0) AS TotalBounty, COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(CASE
//        WHEN P.PostTypeId = 1 THEN 1 END) AS TotalQuestions, COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS TotalAnswers FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId
//        LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName, U.Reputation), TopTagUsers AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(T.Tag) AS TagCount FROM
//        Users U JOIN Posts P ON U.Id = P.OwnerUserId CROSS JOIN LATERAL (SELECT unnest(string_to_array(P.Tags, '><')) AS Tag) T GROUP BY U.Id, U.DisplayName HAVING COUNT(T.Tag) >
//        10), PopularPosts AS (SELECT P.Id AS PostId, P.Title, P.Score, P.AnswerCount, RANK() OVER (ORDER BY P.Score DESC, P.AnswerCount DESC) AS PostRank FROM Posts P WHERE
//        P.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days')) SELECT US.UserId, US.DisplayName, US.Reputation, US.TotalBounty, US.TotalPosts,
//        US.TotalQuestions, US.TotalAnswers, T.TagCount, PP.Title, PP.Score, PP.AnswerCount FROM UserStats US LEFT JOIN TopTagUsers T ON US.UserId = T.UserId LEFT JOIN
//        PopularPosts PP ON PP.PostId IN (SELECT R.PostId FROM PopularPosts R WHERE R.PostRank <= 10) WHERE US.Reputation > 1000 ORDER BY US.Reputation DESC, PP.Score DESC FETCH
//        FIRST 10 ROWS ONLY;
//
// `PP.PostId IN (...)` does not mention US, so PopularPosts is crossed with every user. Each user yields at least one row and the ORDER BY leads with
// Reputation, so only the users ranked in the top 10 by Reputation can reach the LIMIT.
fn q660(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, answer_count, tags_str, .. } = &db.post;
    let User { reputation, .. } = &db.user;
    let uw = whole(db.user.with(reputation.gt(1000))).select(Ident::<User>::new().and(reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let cand: MatSet<Id<User>> = (&uw).filt(|(_, k)| k <= 10).map(|((u, _), _)| u).collect();
    let cu = || (&cand).group_by(Ident::<User>::new());
    let us = cu()
        .select(posts_of(db).select(post_type_id).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 3], |a, (t, b)| [a[0] + b.flatten().unwrap_or(0), a[1] + (t == Some(1)) as i64, a[2] + (t == Some(2)) as i64]);
    let ud = cu().select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let ttu = cu().select(posts_of(db).select(tags_str.flat_map(|t: Str| t.split("><")))).fold(0i64, |a, _| a + 1);
    let pw = whole(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))))
        .select(Ident::<Post>::new().and(score).and(answer_count.opt()))
        .window(rank, |((_, s), a)| (Reverse(s), a.is_none(), Reverse(a)), asc);
    let ppi: HashIdx<(), Id<Post>> = (&pw).filt(|(_, k)| k <= 10).map(|(((p, _), _), _)| p).collect();
    let v = drain((&cand).select(Ident::<User>::new().and((&us).and(&ud)).and((&ttu).filt(|n| n > 10).opt()).and(Ident::<User>::new().map(|_| ()).select((&ppi).opt()))));
    let v = top_n(v, |&(u, (_, p))| (Reverse(reputation.get(u).unwrap()), p.is_none(), p.map(|p| Reverse(score.get(p).unwrap()))), 10);
    rows(v.into_iter().map(|(_, (((u, (s, n)), t), p))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(s[0]), V::I(n), V::I(s[1]), V::I(s[2]), oint(t)]);
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "score", "answers"]),
            None => vec![V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id, DisplayName, Reputation, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM Users), RecentPosts AS (SELECT P.Id AS PostId,
//        P.OwnerUserId, P.Title, P.CreationDate, P.Score, U.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS
//        PostRank FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'), PostStats AS (SELECT
//        RP.OwnerUserId, COUNT(RP.PostId) AS PostCount, SUM(RP.Score) AS TotalScore, AVG(RP.Score) AS AverageScore, STRING_AGG(DISTINCT RP.Title, ', ') AS PostTitles FROM
//        RecentPosts RP GROUP BY RP.OwnerUserId), TopUsers AS (SELECT UR.Id AS UserId, UR.DisplayName, UR.Reputation, PS.PostCount, PS.TotalScore FROM UserReputation UR LEFT JOIN
//        PostStats PS ON UR.Id = PS.OwnerUserId WHERE UR.Reputation > 1000) SELECT TU.UserId, TU.DisplayName, TU.Reputation, COALESCE(TU.PostCount, 0) AS PostCount,
//        COALESCE(TU.TotalScore, 0) AS TotalScore, CASE WHEN TU.PostCount > 5 THEN 'Active Contributor' WHEN TU.PostCount BETWEEN 1 AND 5 THEN 'New Contributor' ELSE 'No
//        Contributions' END AS ContributionLevel FROM TopUsers TU LEFT JOIN Badges B ON TU.UserId = B.UserId AND B.Class = 1 WHERE B.Id IS NULL ORDER BY TU.Reputation DESC,
//        TU.TotalScore DESC;
fn q2772(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let User { reputation, .. } = &db.user;
    let ps = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(Ident::<Post>::new().with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))).select(score))
        .fold((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let gold = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).eq(1)));
    let v = drain(db.user.with(reputation.gt(1000)).minus(gold).select(Ident::<User>::new().and((&ps).opt())));
    rows(v.into_iter().map(|(_, (u, a))| {
        let (n, s) = a.unwrap_or((0, 0));
        let lvl = if n > 5 {
            "Active Contributor"
        } else if (1..=5).contains(&n) {
            "New Contributor"
        } else {
            "No Contributions"
        };
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(s), V::S(lvl)]);
        row(f)
    }))
}

// WITH RECURSIVE UserPostCount AS (SELECT u.Id AS UserId, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN
//        p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id), TopUsers AS (SELECT UserId, PostCount,
//        QuestionCount, AnswerCount, RANK() OVER (ORDER BY PostCount DESC) AS Rank FROM UserPostCount WHERE PostCount > 0), TagStats AS (SELECT p.OwnerUserId, COUNT(t.Tag) AS
//        TagCount FROM Posts p LEFT JOIN (SELECT DISTINCT UNNEST(string_to_array(p.Tags, '><')) AS Tag) t ON true WHERE p.OwnerUserId IS NOT NULL GROUP BY p.OwnerUserId),
//        CombinedStats AS (SELECT t.UserId, t.PostCount, t.QuestionCount, t.AnswerCount, COALESCE(ts.TagCount, 0) AS TagCount FROM TopUsers t LEFT JOIN TagStats ts ON t.UserId =
//        ts.OwnerUserId) SELECT u.DisplayName, c.PostCount, c.QuestionCount, c.AnswerCount, c.TagCount, c.QuestionCount * 1.0 / NULLIF(c.PostCount, 0) AS QuestionRatio, CASE WHEN
//        c.TagCount > 10 THEN 'Highly Active Tagger' ELSE 'Moderate Tagger' END AS TaggingBehavior, COALESCE(SUM(b.Class), 0) AS BadgeCount FROM CombinedStats c JOIN Users u ON
//        c.UserId = u.Id LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 100 GROUP BY u.DisplayName, c.PostCount, c.QuestionCount, c.AnswerCount, c.TagCount ORDER BY
//        c.PostCount DESC LIMIT 10;
//
// WITH RECURSIVE, but no CTE refers to itself.
fn q30564(db: &'static So) -> String {
    let Post { post_type_id, tags_str, .. } = &db.post;
    let pdt = db.post.group_by(Ident::<Post>::new()).select(tags_str.flat_map(|t: Str| t.split("><"))).count_distinct();
    let uc = db
        .user
        .with((&db.user.reputation).gt(100))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and((&pdt).opt())))
        .fold([0i64; 4], |a, (t, d)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + d.unwrap_or(0)]);
    let g = db
        .user
        .with((&db.user.reputation).gt(100))
        .group_by((&db.user.display_name).and(&uc))
        .select(badges_of(db).select(&db.badge.class).opt())
        .fold(0i64, |s, c| s + c.unwrap_or(0));
    let v = top_n(drain(&g), |&((n, a), _)| (Reverse(a[0]), n, a), 10);
    rows(v.into_iter().map(|((n, a), b)| {
        row(vec![
            V::S(n),
            V::I(a[0]),
            V::I(a[1]),
            V::I(a[2]),
            V::I(a[3]),
            V::F(a[1] as f64 / a[0] as f64),
            V::S(if a[3] > 10 { "Highly Active Tagger" } else { "Moderate Tagger" }),
            V::I(b),
        ])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostID, p.Title, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankPerType,
//        COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount, COALESCE(MAX(v.BountyAmount) OVER (PARTITION BY p.Id), 0) AS MaxBounty FROM Posts p LEFT JOIN Comments c ON p.Id =
//        c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 9 WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'), FilteredPosts AS
//        (SELECT rp.PostID, rp.Title, rp.CreationDate, rp.Score, rp.CommentCount, rp.MaxBounty FROM RankedPosts rp WHERE rp.RankPerType <= 10 AND rp.Score > (SELECT AVG(Score)
//        FROM Posts) AND (rp.MaxBounty > 0 OR rp.CommentCount >= 5)) SELECT fp.PostID, fp.Title, fp.CreationDate, fp.Score, fp.CommentCount, fp.MaxBounty, CASE WHEN fp.Score IS
//        NULL THEN 'No Votes' WHEN fp.MaxBounty > 0 THEN 'Bounty Available' WHEN fp.CommentCount > 5 THEN 'Highly Discussed' ELSE 'Normal' END AS PostCategory, STRING_AGG(DISTINCT
//        t.TagName, ', ') AS Tags FROM FilteredPosts fp LEFT JOIN Posts p ON fp.PostID = p.Id LEFT JOIN Tags t ON t.ExcerptPostId = p.Id GROUP BY fp.PostID, fp.Title,
//        fp.CreationDate, fp.Score, fp.CommentCount, fp.MaxBounty HAVING SUM(fp.CommentCount) IS NOT NULL ORDER BY fp.Score DESC, fp.CommentCount DESC;
//
// RankPerType numbers the joined rows; a score tie between different posts goes to the smaller post id (the SQL leaves it open).
// The distinct tag names are joined in name order.
fn q23129(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let bv = || votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(9))).select((&db.vote.bounty_amount).opt());
    let rw = recent()
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(comments_of(db).opt().and(bv().opt())))
        .window(row_number, |((p, s), cv)| (Reverse(s), p, cv), asc);
    let w = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(bv().opt())).fold((0i64, None::<i64>), |(n, m), (c, b)| {
        (n + c.is_some() as i64, match b.flatten() {
            Some(x) => Some(m.map_or(x, |m: i64| m.max(x))),
            None => m,
        })
    });
    let (ss, sn) = db.post.select(score).fold_flat((0i64, 0i64), |(s, n), x| (s + x, n + 1));
    let avg_score = ss as f64 / sn as f64;
    let fp: MatSet<Id<Post>> = (&rw)
        .filt(|(_, k)| k <= 10)
        .map(|(((p, _), _), _)| p)
        .with(score.filt(move |s| s as f64 > avg_score))
        .with((&w).filt(|(n, m): (i64, Option<i64>)| m.unwrap_or(0) > 0 || n >= 5))
        .collect();
    let ex: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    let tg = (&fp).group_by(Ident::<Post>::new()).select(ex.select(&db.tag.tag_name).opt()).buf_fold(|v| agg_distinct(v.iter().flatten().copied().collect(), ", "));
    let v = drain((&fp).select(Ident::<Post>::new().and(&w).and(&tg)));
    rows(v.into_iter().map(|(_, ((p, (n, m)), t))| {
        let m = m.unwrap_or(0);
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(n), V::I(m)]);
        f.push(V::S(if m > 0 {
            "Bounty Available"
        } else if n > 5 {
            "Highly Discussed"
        } else {
            "Normal"
        }));
        f.push(ostr(t));
        row(f)
    }))
}

// WITH ParsedTags AS (SELECT p.Id AS PostId, UNNEST(string_to_array(SUBSTRING(p.Tags FROM 2 FOR LENGTH(p.Tags) - 2), '><')) AS Tag FROM Posts p WHERE p.PostTypeId = 1),
//        TagStatistics AS (SELECT Tag, COUNT(*) AS TagCount, AVG(u.Reputation) AS AvgReputation, COUNT(DISTINCT p.OwnerUserId) AS UniqueUsers FROM ParsedTags pt JOIN Posts p ON
//        pt.PostId = p.Id JOIN Users u ON p.OwnerUserId = u.Id GROUP BY Tag), TopTags AS (SELECT Tag, TagCount, AvgReputation, UniqueUsers, RANK() OVER (ORDER BY TagCount DESC) AS
//        TagRank FROM TagStatistics), UserActivities AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS QuestionsAsked, COUNT(DISTINCT c.Id) AS CommentsMade,
//        COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty, COALESCE(SUM(CASE WHEN v.Id IS NOT NULL THEN 1 ELSE 0 END), 0) AS TotalVotes FROM Users u LEFT JOIN Posts p ON u.Id =
//        p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName) SELECT tt.Tag, tt.TagCount, tt.AvgReputation,
//        tt.UniqueUsers, ua.DisplayName AS ActiveUser, ua.QuestionsAsked, ua.CommentsMade, ua.TotalBounty, ua.TotalVotes FROM TopTags tt JOIN UserActivities ua ON
//        ua.QuestionsAsked > 0 WHERE tt.TagRank <= 10 ORDER BY tt.TagCount DESC, ua.TotalVotes DESC;
//
// `ON ua.QuestionsAsked > 0` does not mention tt: the top tags are crossed with every user who has a post.
fn q27548(db: &'static So) -> String {
    let Post { post_type_id, tags_str, owner_user, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1)).with(owner_user);
    let tn = qs().group_by(tags_str.flat_map(tag_list)).select(owner_user.select(&db.user.reputation)).fold((0i64, 0i64), |(n, s), r| (n + 1, s + r));
    let tu = qs().group_by(tags_str.flat_map(tag_list)).select(owner_user).count_distinct();
    let tags: MatSet<Str> = qs().select(tags_str.flat_map(tag_list)).collect();
    let tw = whole(&tags).select(Same::<Str>::new().and(&tn)).window(rank, |(_, (n, _))| Reverse(n), asc);
    let tt: MatSet<Str> = (&tw).filt(|(_, k)| k <= 10).map(|((t, _), _)| t).collect();
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |a, _| a + 1);
    let cc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db))).fold(0i64, |a, _| a + 1);
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(comments_of(db).opt()).and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 2], |a, (_, v)| [a[0] + v.flatten().unwrap_or(0), a[1] + v.is_some() as i64]);
    let us = (&pc).and((&cc).opt()).and(&ua);
    let v = drain((&tt).select(Same::<Str>::new().and((&tn).and(&tu))).cross(&us));
    rows(v.into_iter().map(|((_, u), ((t, ((n, r), d)), ((q, c), a)))| {
        row(vec![V::S(t), V::I(n), avg(r, n), V::I(d), user_col(db, u, "name"), V::I(q), V::I(c.unwrap_or(0)), V::I(a[0]), V::I(a[1])])
    }))
}

// WITH TagStatistics AS (SELECT TRIM(tag) AS TagName, COUNT(DISTINCT p.Id) AS PostCount, COUNT(DISTINCT c.Id) AS CommentCount, SUM(CASE WHEN p.ViewCount IS NOT NULL THEN
//        p.ViewCount ELSE 0 END) AS TotalViews, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS
//        TotalDownvotes, AVG(p.Score) AS AverageScore FROM Posts p JOIN (SELECT UNNEST(string_to_array(substring(Tags, 2, length(Tags) - 2), '><')) AS tag, Id FROM Posts WHERE
//        PostTypeId = 1) AS tags ON p.Id = tags.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56'
//        as timestamp) - INTERVAL '1 year' GROUP BY TRIM(tag)), TopTags AS (SELECT TagName, PostCount, CommentCount, TotalViews, TotalUpvotes, TotalDownvotes, AverageScore, RANK()
//        OVER (ORDER BY PostCount DESC) AS RankByPosts, RANK() OVER (ORDER BY AverageScore DESC) AS RankByScore FROM TagStatistics) SELECT TT.TagName, TT.PostCount,
//        TT.CommentCount, TT.TotalViews, TT.TotalUpvotes, TT.TotalDownvotes, TT.AverageScore, CASE WHEN RankByPosts <= 10 THEN 'Top 10 by Posts' WHEN RankByScore <= 10 THEN 'Top
//        10 by Score' ELSE 'Other' END AS TagCategory FROM TopTags TT ORDER BY TT.PostCount DESC, TT.AverageScore DESC;
fn q29324(db: &'static So) -> String {
    let Post { post_type_id, tags_str, creation_date, view_count, score, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1)).with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let tg = || qs().group_by(tags_str.flat_map(tag_list).map(|t: Str| t.trim()));
    let f = tg()
        .select(view_count.opt().and(score).and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 5], |a, (((w, s), _), t)| [a[0] + w.unwrap_or(0), a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + s, a[4] + 1]);
    let pc = tg().select(Ident::<Post>::new()).count_distinct();
    let cc = tg().select(comments_of(db)).count_distinct();
    let tags: MatSet<Str> = qs().select(tags_str.flat_map(tag_list).map(|t: Str| t.trim())).collect();
    let w1 = whole(&tags).select(Same::<Str>::new().and((&pc).and((&cc).opt()).and(&f))).window(rank, |(_, ((n, _), _))| Reverse(n), asc);
    let w2 = (&w1).window(rank, |((_, (_, a)), _)| Reverse(fkey(a[3] as f64 / a[4] as f64)), asc);
    rows(drain(&w2).into_iter().map(|(_, (((t, ((n, c), a)), rp), rs))| {
        let a = [n, c.unwrap_or(0), a[0], a[1], a[2], a[3], a[4]];
        let cat = if rp <= 10 {
            "Top 10 by Posts"
        } else if rs <= 10 {
            "Top 10 by Score"
        } else {
            "Other"
        };
        row(vec![V::S(t), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), avg(a[5], a[6]), V::S(cat)])
    }))
}

// WITH RECURSIVE VotesCTE AS (SELECT PostId, COUNT(*) AS TotalVotes, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0
//        END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY PostId ORDER BY COUNT(*) DESC) AS rn FROM Votes GROUP BY PostId HAVING COUNT(*) > 0), RecentPosts AS (SELECT p.Id,
//        p.Title, p.CreationDate, p.ViewCount, p.Score, COALESCE(STRING_AGG(DISTINCT t.TagName, ', '), 'No Tags') AS Tags, u.DisplayName AS Author FROM Posts p JOIN Users u ON
//        p.OwnerUserId = u.Id LEFT JOIN Tags t ON t.ExcerptPostId = p.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY p.Id,
//        p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName), ClosedPosts AS (SELECT ph.PostId, ph.CreationDate, ph.UserDisplayName, ph.Comment AS CloseReason FROM
//        PostHistory ph WHERE ph.PostHistoryTypeId = 10) SELECT rp.Id AS PostID, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.Tags, rp.Author, COALESCE(vs.TotalVotes, 0)
//        AS TotalVotes, COALESCE(vs.UpVotes, 0) AS UpVotes, COALESCE(vs.DownVotes, 0) AS DownVotes, CASE WHEN cp.PostId IS NOT NULL THEN 'Closed' ELSE 'Open' END AS PostStatus,
//        cp.CloseReason AS CloseReasonDetails FROM RecentPosts rp LEFT JOIN VotesCTE vs ON rp.Id = vs.PostId LEFT JOIN ClosedPosts cp ON rp.Id = cp.PostId WHERE rp.ViewCount >=
//        100 ORDER BY rp.CreationDate DESC LIMIT 50;
//
// WITH RECURSIVE, but no CTE refers to itself. The distinct tag names are joined in name order.
fn q31400(db: &'static So) -> String {
    let Post { creation_date, owner_user, view_count, .. } = &db.post;
    let rp: MatSet<Id<Post>> = db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(owner_user).with(view_count.ge(100)).collect();
    let vs = (&rp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let ex: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    let tg = (&rp).group_by(Ident::<Post>::new()).select(ex.select(&db.tag.tag_name).opt()).buf_fold(|v| agg_distinct(v.iter().flatten().copied().collect(), ", "));
    let PostHistory { post_history_type_id, comment, .. } = &db.post_history;
    let cp = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.eq(10)).select(comment.opt()));
    let v = drain((&rp).select(Ident::<Post>::new().and(&tg).and((&vs).opt()).and(cp.opt())));
    let v = top_n(v, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 50);
    rows(v.into_iter().map(|(_, (((p, t), a), c))| {
        let a = a.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.push(V::S(t.unwrap_or("No Tags")));
        f.push(post_fields(db, p, &["owner"]).remove(0));
        f.extend(a.map(V::I));
        f.push(V::S(if c.is_some() { "Closed" } else { "Open" }));
        f.push(c.map_or(V::Null, ostr));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.Score, p.CreationDate, u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC)
//        AS PostRank, COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount, COUNT(v.Id) OVER (PARTITION BY p.Id) AS VoteCount FROM Posts p LEFT JOIN Users u ON p.OwnerUserId =
//        u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
//        ClosedPosts AS (SELECT ph.PostId, COUNT(*) AS CloseCount, STRING_AGG(DISTINCT cr.Name, ', ') AS CloseReasons FROM PostHistory ph JOIN CloseReasonTypes cr ON
//        CAST(ph.Comment AS INTEGER) = cr.Id WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId), TopPosts AS (SELECT rp.Title, rp.OwnerDisplayName, rp.Score,
//        COALESCE(cp.CloseCount, 0) AS CloseCount, COALESCE(cp.CloseReasons, 'No close reasons') AS CloseReasons FROM RankedPosts rp LEFT JOIN ClosedPosts cp ON rp.Id = cp.PostId
//        WHERE rp.PostRank <= 10) SELECT tp.Title, tp.OwnerDisplayName, tp.Score, tp.CloseCount, tp.CloseReasons, CASE WHEN tp.CloseCount > 0 THEN 'Closed' ELSE 'Open' END AS
//        PostStatus, CASE WHEN tp.Score IS NULL THEN 'Unscored' ELSE 'Scored' END AS ScoreStatus FROM TopPosts tp ORDER BY tp.Score DESC, tp.Title;
//
// PostRank numbers the joined rows; a score tie between different posts goes to the smaller post id (the SQL leaves it open).
// The distinct close reasons are joined in name order.
fn q22980(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(comments_of(db).opt().and(votes_of(db).opt())))
        .window(row_number, |((p, s), cv)| (Reverse(s), p, cv), asc);
    let cr = close_reasons(db);
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cp = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&cr).select(&db.close_reason_type.name))
        .buf_fold(|v| (v.len() as i64, agg_distinct(v.to_vec(), ", ").unwrap()));
    let v = drain((&w).filt(|(_, k)| k <= 10).map(|(((p, _), _), _)| p).select(Ident::<Post>::new().and((&cp).opt())));
    rows(v.into_iter().map(|(_, (p, c))| {
        let (n, r) = c.unwrap_or((0, "No close reasons"));
        let mut f = post_fields(db, p, &["title", "owner", "score"]);
        f.extend([V::I(n), V::S(r), V::S(if n > 0 { "Closed" } else { "Open" }), V::S("Scored")]);
        row(f)
    }))
}

// WITH RankedUserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(p.ViewCount) AS TotalViews, RANK() OVER (ORDER BY COUNT(DISTINCT p.Id) DESC) AS UserRank FROM
//        Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId WHERE u.Reputation > 0 GROUP BY u.Id, u.DisplayName), TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalAnswers,
//        TotalQuestions, TotalViews FROM RankedUserActivity WHERE UserRank <= 10), PostTags AS (SELECT DISTINCT p.Id AS PostId, unnest(string_to_array(substring(p.Tags, 2,
//        length(p.Tags)-2), '><')) AS Tag FROM Posts p WHERE p.Tags IS NOT NULL AND p.Tags <> ''), TagActivity AS (SELECT t.Tag, COUNT(DISTINCT p.Id) AS PostsWithTag,
//        SUM(COALESCE(c.CommentCount, 0)) AS TotalComments FROM PostTags t JOIN Posts p ON t.PostId = p.Id LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP
//        BY PostId) c ON p.Id = c.PostId GROUP BY t.Tag ORDER BY PostsWithTag DESC LIMIT 5) SELECT tu.DisplayName, tu.TotalPosts, tu.TotalAnswers, tu.TotalQuestions,
//        tu.TotalViews, ta.Tag, ta.PostsWithTag, ta.TotalComments FROM TopUsers tu JOIN TagActivity ta ON true ORDER BY tu.UserId, ta.PostsWithTag DESC;
fn q26692(db: &'static So) -> String {
    let Post { post_type_id, view_count, tags_str, .. } = &db.post;
    let ua = db
        .user
        .with((&db.user.reputation).gt(0))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(view_count.opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some((t, w)) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 1) as i64, a[3] + w.unwrap_or(0), a[4] + w.is_some() as i64],
            None => a,
        });
    let uw = whole(db.user.with((&db.user.reputation).gt(0))).select(Ident::<User>::new().and(&ua)).window(rank, |(_, a)| Reverse(a[0]), asc);
    let tu: MatSet<(Id<User>, [i64; 5])> = (&uw).filt(|(_, k)| k <= 10).map(|(x, _)| x).collect();
    type P = (Id<Post>, Str);
    let pt: MatSet<P> = db.post.with(tags_str.filt(|t| !t.is_empty())).select(Ident::<Post>::new().and(tags_str.flat_map(tag_list))).collect();
    let cc = comments_per_post(db);
    let tg = || (&pt).group_by(Same::<P>::new().map(|x: P| x.1));
    let tn = tg().select(Same::<P>::new().map(|x: P| x.0)).count_distinct();
    let tsum = tg().select(Same::<P>::new().map(|x: P| x.0).select(&cc)).fold(0i64, |a, c| a + c);
    let ta = top_n(drain((&tn).and(&tsum)), |&(t, (n, _))| (Reverse(n), t), 5);
    let v = drain((&tu).cross(rel(ta)));
    rows(v.into_iter().map(|(_, ((u, a), (t, (n, c))))| {
        row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[3], a[4]), V::S(t), V::I(n), V::I(c)])
    }))
}

// WITH RankedPost AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT CASE WHEN v.VoteTypeId = 2 THEN v.Id END) AS
//        UpVotes, COUNT(DISTINCT CASE WHEN v.VoteTypeId = 3 THEN v.Id END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS RN FROM Posts p
//        LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= DATE '2020-01-01' AND p.Body IS NOT NULL AND (p.Title IS NOT NULL OR
//        p.Tags IS NOT NULL) GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.OwnerUserId, p.Score), FilteredPost AS (SELECT rp.PostId, rp.Title, rp.CreationDate,
//        rp.ViewCount, rp.CommentCount, rp.UpVotes, rp.DownVotes FROM RankedPost rp WHERE rp.RN = 1 AND (EXTRACT(DOW FROM rp.CreationDate) = 0 OR rp.ViewCount > 100)) SELECT
//        fp.PostId, fp.Title, fp.CreationDate, fp.ViewCount, fp.CommentCount, COALESCE(fp.UpVotes - fp.DownVotes, 0) AS NetVotes, CASE WHEN fp.CommentCount = 0 THEN 'No Comments'
//        WHEN fp.UpVotes >= 10 THEN 'Popular' ELSE 'Regular' END AS PostStatus, STRING_AGG(t.TagName, ', ') AS Tags FROM FilteredPost fp LEFT JOIN Posts p ON fp.PostId = p.Id LEFT
//        JOIN Tags t ON t.WikiPostId = fp.PostId GROUP BY fp.PostId, fp.Title, fp.CreationDate, fp.ViewCount, fp.CommentCount, fp.UpVotes, fp.DownVotes ORDER BY NetVotes DESC,
//        fp.ViewCount DESC LIMIT 50;
//
// RN reads only base columns, so the posts are ranked first; a score tie goes to the smaller post id (the SQL leaves it open).
fn q21296(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, score, title, tags_str, view_count, .. } = &db.post;
    let base = db.post.with(creation_date.ge(date(2020, 1, 1))).with(title.or(tags_str));
    let w = base.group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let fp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k == 1).map(|((p, _), _)| p).with(creation_date.filt(|d| dow(d) == 0).or(view_count.gt(100))).collect();
    let rpc = (&fp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let rpv = (&fp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let rp = (&rpc).and(&rpv).map(|(c, a): (i64, [i64; 2])| [c, a[0], a[1]]);
    let wi: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.wiki_post).inv().collect();
    let tg = (&fp).group_by(Ident::<Post>::new()).select(wi.select(&db.tag.tag_name)).buf_fold(|v| leak(v.join(", ")));
    let v = drain((&fp).select(Ident::<Post>::new().and(&rp).and((&tg).opt())));
    let v = top_n(v, |&(p, ((_, a), _))| {
        let w = view_count.get(p);
        (Reverse(a[1] - a[2]), w.is_none(), Reverse(w))
    }, 50);
    rows(v.into_iter().map(|(_, ((p, a), t))| {
        let st = if a[0] == 0 {
            "No Comments"
        } else if a[1] >= 10 {
            "Popular"
        } else {
            "Regular"
        };
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        f.extend([V::I(a[0]), V::I(a[1] - a[2]), V::S(st), ostr(t)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, RANK() OVER (ORDER BY p.Score DESC) AS ScoreRank FROM Posts p LEFT OUTER JOIN Votes v ON p.Id
//        = v.PostId GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount), ClosedPosts AS (SELECT ph.PostId, COUNT(*) AS CloseCount, ARRAY_AGG(DISTINCT ct.Name) AS
//        CloseReasons FROM PostHistory ph JOIN CloseReasonTypes ct ON ph.Comment = ct.Id::text WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId), UserBadges AS (SELECT
//        b.UserId, COUNT(*) FILTER (WHERE b.Class = 1) AS GoldCount, COUNT(*) FILTER (WHERE b.Class = 2) AS SilverCount, COUNT(*) FILTER (WHERE b.Class = 3) AS BronzeCount FROM
//        Badges b GROUP BY b.UserId) SELECT rp.Id AS PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.UpVotes, rp.DownVotes, rp.ScoreRank, cp.CloseCount,
//        cp.CloseReasons, ub.GoldCount, ub.SilverCount, ub.BronzeCount FROM RankedPosts rp LEFT JOIN ClosedPosts cp ON rp.Id = cp.PostId LEFT JOIN Users u ON rp.Id = u.Id LEFT
//        JOIN UserBadges ub ON u.Id = ub.UserId WHERE (rp.Score > 10 OR cp.CloseCount IS NOT NULL) AND rp.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '90 days' ORDER
//        BY rp.ScoreRank, rp.ViewCount DESC LIMIT 100;
//
// `rp.Id = u.Id` joins a post id to a user id, as written. The ARRAY_AGG(DISTINCT ...) is listed in name order.
fn q1727(db: &'static So) -> String {
    let Post { creation_date, score, origid, view_count, .. } = &db.post;
    let rw = whole(&db.post.id).select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| Reverse(s), asc);
    let crt: HashIdx<Str, Id<CloseReasonType>> = (&db.close_reason_type.origid).map(|i: i64| -> Str { leak(i.to_string()) }).inv().collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cp = db
        .post_history
        .with(post_history_type_id.is_in([10, 11]))
        .group_by(post)
        .select(comment.select(&crt).select(&db.close_reason_type.name))
        .buf_fold(|v| {
            let mut x = v.to_vec();
            x.sort();
            x.dedup();
            (v.len() as i64, &*Box::leak(x.into_boxed_slice()))
        });
    let Badge { class, .. } = &db.badge;
    let ub = db.badge.group_by(&db.badge.user).select(class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let uo = users_by_origid(db);
    type R = ((Id<Post>, i64), i64);
    type C = (i64, &'static [Str]);
    let rv: MatSet<R> = (&rw).map(|x: R| x).collect();
    let k = || Same::<R>::new().map(|x: R| x.0 .0);
    let cut = add_days(ts(2024, 10, 1, 12, 34, 56), -90);
    let uv = db.post.with(creation_date.gt(cut)).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let m = || Same::<(R, Option<C>)>::new().map(|x: (R, Option<C>)| x.0 .0 .0);
    let v = drain(
        (&rv)
            .select(Same::<R>::new().with(k().select(Ident::<Post>::new().with(creation_date.gt(cut)))))
            .select(Same::<R>::new().and(k().select(&cp).opt()))
            .filt(|(((_, s), _), c): (R, Option<C>)| s > 10 || c.is_some())
            .select(Same::<(R, Option<C>)>::new().and(m().select(origid.select(&uo).select(&ub).opt())).and(m().select((&uv).opt()))),
    );
    let v = top_n(v, |&(_, (((((p, _), k), _), _), _))| {
        let w = view_count.get(p);
        (k, w.is_none(), Reverse(w), p)
    }, 100);
    rows(v.into_iter().map(|(_, (((((p, _), k), c), b), u))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        let u = u.unwrap_or([0; 2]);
        f.extend(u.map(V::I));
        f.push(V::I(k));
        f.extend(match c {
            Some((n, l)) => [V::I(n), V::L(l.iter().map(|s| V::S(s)).collect())],
            None => [V::Null, V::Null],
        });
        f.extend(match b {
            Some(b) => b.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn FROM Posts
//        p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'), PostBadges AS (SELECT b.UserId, COUNT(DISTINCT b.Id) AS BadgeCount, STRING_AGG(b.Name, ',
//        ') AS Badges FROM Badges b GROUP BY b.UserId), ClosedPosts AS (SELECT ph.PostId, ph.CreationDate, STRING_AGG(c.Name, ', ') AS CloseReasons FROM PostHistory ph JOIN
//        CloseReasonTypes c ON CAST(ph.Comment AS INTEGER) = c.Id WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId, ph.CreationDate) SELECT rp.PostId, rp.Title, rp.Score,
//        rp.CreationDate AS PostCreationDate, COALESCE(pb.BadgeCount, 0) AS UserBadgeCount, COALESCE(pb.Badges, 'No Badges') AS UserBadges, cp.CloseReasons, CASE WHEN
//        cp.CloseReasons IS NOT NULL THEN CASE WHEN rp.Score >= 10 THEN 'Highly Rated & Closed' ELSE 'Closed Post' END ELSE CASE WHEN rp.Score >= 10 THEN 'Popular Content' ELSE
//        'Standard Post' END END AS PostStatus FROM RankedPosts rp LEFT JOIN PostBadges pb ON pb.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = rp.PostId) LEFT JOIN
//        ClosedPosts cp ON cp.PostId = rp.PostId WHERE rp.rn = 1 ORDER BY rp.Score DESC, rp.CreationDate DESC OFFSET 10 ROWS FETCH NEXT 20 ROWS ONLY;
//
// The STRING_AGG orders are left open by the SQL; the port joins in badge and history id order.
fn q24608(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, score, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(post_type_id).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k == 1).map(|((p, _), _)| p).collect();
    let pb = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.name)).buf_fold(|v| (v.len() as i64, leak(v.join(", "))));
    let cr = close_reasons(db);
    let PostHistory { post, post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let cp = db
        .post_history
        .with(post_history_type_id.is_in([10, 11]))
        .group_by(post.and(hd))
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&cr).select(&db.close_reason_type.name))
        .buf_fold(|v| leak(v.join(", ")));
    type C = ((Id<Post>, i64), Str);
    let cpv = rel(drain(&cp));
    let cpi: HashIdx<Id<Post>, C> = (&cpv).map(|x: C| x.0 .0).inv().select(&cpv).collect();
    let v = drain((&tp).select(Ident::<Post>::new().and(owner_user.select(&pb).opt()).and((&cpi).opt())));
    let v = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 30);
    rows(v.into_iter().skip(10).map(|(_, ((p, b), c))| {
        let (n, bs) = b.unwrap_or((0, "No Badges"));
        let s = score.get(p).unwrap();
        let st = match (c.is_some(), s >= 10) {
            (true, true) => "Highly Rated & Closed",
            (true, false) => "Closed Post",
            (false, true) => "Popular Content",
            (false, false) => "Standard Post",
        };
        let mut f = post_fields(db, p, &["id", "title", "score", "created"]);
        f.extend([V::I(n), V::S(bs), ostr(c.map(|c| c.1)), V::S(st)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY
//        p.CreationDate DESC) AS Rank, COALESCE(NULLIF(p.Body, ''), 'No content') AS PostBody FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) -
//        INTERVAL '1 year'), UserScore AS (SELECT u.Id AS UserId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 WHEN v.VoteTypeId = 3 THEN -1 ELSE 0 END) AS TotalScore FROM Users u LEFT
//        JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id), PostHistoryDetails AS (SELECT ph.PostId, MAX(ph.CreationDate) AS LatestEdit, STRING_AGG(DISTINCT pht.Name, ', ') AS
//        EditTypes FROM PostHistory ph JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id GROUP BY ph.PostId), ClosedPosts AS (SELECT p.Id AS ClosedPostId, ph.Comment AS
//        CloseReason FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId WHERE ph.PostHistoryTypeId = 10) SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount,
//        us.TotalScore, phd.LatestEdit, phd.EditTypes, cp.CloseReason FROM RankedPosts rp LEFT JOIN UserScore us ON rp.OwnerUserId = us.UserId LEFT JOIN PostHistoryDetails phd ON
//        rp.PostId = phd.PostId LEFT JOIN ClosedPosts cp ON rp.PostId = cp.ClosedPostId WHERE rp.Rank = 1 AND (rp.Score > 0 OR cp.CloseReason IS NOT NULL) AND EXISTS (SELECT 1
//        FROM Tags t WHERE t.ExcerptPostId = rp.PostId AND t.Count > 100) ORDER BY COALESCE(rp.Score, 0) DESC, rp.CreationDate DESC;
//
// Rank reads only base columns, so the posts are ranked first (a tie goes to the smaller post id). The distinct type names are joined in name order.
fn q20565(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, owner_user, score, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k == 1).map(|((p, _), _)| p).collect();
    let big: HashIdx<Id<Post>, Id<Tag>> = db.tag.with((&db.tag.count).gt(100)).select(&db.tag.excerpt_post).inv().collect();
    let us = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold(0i64, |a, t| {
        a + match t {
            Some(2) => 1,
            Some(3) => -1,
            _ => 0,
        }
    });
    let PostHistory { post, post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let phd = db.post_history.group_by(post).select(hd.and(htype_name(db))).buf_fold(|v| (v.iter().map(|x| x.0).max().unwrap(), agg_distinct(v.iter().map(|x| x.1).collect(), ", ").unwrap()));
    let cp = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.eq(10)).select(comment.opt()));
    type X = ((((Id<Post>, i64), Option<i64>), Option<(i64, Str)>), Option<Option<Str>>);
    let v = drain(
        (&tp)
            .with(&big)
            .select(Ident::<Post>::new().and(score).and(owner_user.select(&us).opt()).and((&phd).opt()).and(cp.opt()))
            .filt(|((((_, s), _), _), c): X| s > 0 || c.flatten().is_some()),
    );
    rows(v.into_iter().map(|(_, ((((p, _), u), h), c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.push(oint(u));
        f.extend(match h {
            Some((d, n)) => [V::T(d), V::S(n)],
            None => [V::Null, V::Null],
        });
        f.push(ostr(c.flatten()));
        row(f)
    }))
}

// WITH UserVoteCounts AS (SELECT UserId, COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS Upvotes, COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS Downvotes, COUNT(DISTINCT
//        PostId) AS TotalVotes FROM Votes GROUP BY UserId), PostStatistics AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, COALESCE(COUNT(c.Id), 0) AS CommentCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpvoteCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownvoteCount,
//        p.CreationDate, ROW_NUMBER() OVER (ORDER BY p.CreationDate DESC) AS RowNum FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND
//        v.VoteTypeId IN (2, 3) GROUP BY p.Id, p.Title, p.ViewCount, p.CreationDate), TagPostCounts AS (SELECT t.Id AS TagId, COUNT(p.Id) AS PostCount FROM Tags t LEFT JOIN Posts
//        p ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY t.Id) SELECT ps.PostId, ps.Title, ps.ViewCount, ps.CommentCount, ps.UpvoteCount, ps.DownvoteCount, COALESCE(upc.Upvotes,
//        0) AS UserUpvotes, COALESCE(upc.Downvotes, 0) AS UserDownvotes, COALESCE(tpc.PostCount, 0) AS TagsUsed, CASE WHEN ps.RowNum <= 10 THEN 'Hot Post' ELSE 'Regular Post' END
//        AS PostCategory FROM PostStatistics ps LEFT JOIN UserVoteCounts upc ON upc.UserId = ps.PostId LEFT JOIN TagPostCounts tpc ON tpc.TagId = (SELECT MIN(Id) FROM Tags WHERE
//        TagName IN (SELECT unnest(string_to_array(ps.Title, ' ')))) WHERE ps.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days' ORDER BY ps.ViewCount
//        DESC, ps.UpvoteCount DESC;
//
// `upc.UserId = ps.PostId` joins a user id to a post id, as written.
fn q3547(db: &'static So) -> String {
    let Post { creation_date, title, origid, .. } = &db.post;
    let hot: MatSet<Id<Post>> = rel(top_n(drain(creation_date), |&(p, d)| (Reverse(d), p), 10)).map(|x: (Id<Post>, i64)| x.0).collect();
    let recent = || db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)));
    let Vote { user_id, vote_type_id, .. } = &db.vote;
    let ps = recent()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.is_in([2, 3]))).select(vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let upc = db.vote.group_by(user_id).select(vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let tn: HashIdx<Str, Id<Tag>> = (&db.tag.tag_name).inv().collect();
    let mt = recent().group_by(Ident::<Post>::new()).select(title.flat_map(|t: Str| t.split(' ')).select(&tn)).buf_fold(|v| *v.iter().min().unwrap());
    let tpc = tag_stats(db);
    let v = drain(recent().select(Ident::<Post>::new().and(&ps).and(origid.select(&upc).opt()).and((&mt).select(&tpc).opt()).and(Ident::<Post>::new().with(&hot).opt())));
    rows(v.into_iter().map(|(_, ((((p, a), u), t), h))| {
        let u = u.unwrap_or([0; 2]);
        let mut f = post_fields(db, p, &["id", "title", "views"]);
        f.extend(a.map(V::I));
        f.extend([V::I(u[0]), V::I(u[1]), V::I(t.map_or(0, |t| t[0])), V::S(if h.is_some() { "Hot Post" } else { "Regular Post" })]);
        row(f)
    }))
}

// WITH RecentUserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews, SUM(CASE
//        WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesReceived, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesReceived FROM Users U LEFT JOIN Posts P ON U.Id
//        = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId WHERE U.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY U.Id, U.DisplayName,
//        U.Reputation), TopPosts AS (SELECT P.Id AS PostId, P.Title, P.Score, P.ViewCount, P.CreationDate, ROW_NUMBER() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC,
//        P.CreationDate DESC) AS Rank FROM Posts P WHERE P.Score IS NOT NULL AND P.ViewCount IS NOT NULL), UserBadgeCount AS (SELECT B.UserId, COUNT(B.Id) AS BadgeCount,
//        STRING_AGG(B.Name, ', ') AS BadgeNames FROM Badges B GROUP BY B.UserId) SELECT UA.UserId, UA.DisplayName, UA.Reputation, UA.PostCount, UA.TotalViews, UA.UpVotesReceived,
//        UA.DownVotesReceived, COALESCE(UB.BadgeCount, 0) AS TotalBadges, COALESCE(UB.BadgeNames, 'No badges') AS Badges, TP.Title AS TopPostTitle, TP.ViewCount AS TopPostViews,
//        TP.Score AS TopPostScore FROM RecentUserActivity UA LEFT JOIN UserBadgeCount UB ON UA.UserId = UB.UserId LEFT JOIN TopPosts TP ON UA.UserId = (SELECT P.OwnerUserId FROM
//        Posts P WHERE P.Id = TP.PostId LIMIT 1) WHERE UA.Reputation > 1000 ORDER BY UA.Reputation DESC, UA.TotalViews DESC LIMIT 10;
//
// The STRING_AGG order is left open by the SQL; the port joins in badge id order.
fn q23299(db: &'static So) -> String {
    let Post { view_count, .. } = &db.post;
    let User { creation_date, reputation, .. } = &db.user;
    let ua = db
        .user
        .with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(reputation.gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(view_count.opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 3], |a, p| match p {
            Some((w, t)) => [a[0] + w.unwrap_or(0), a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64],
            None => a,
        });
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.name)).buf_fold(|v| (v.len() as i64, leak(v.join(", "))));
    let v = drain((&ua).and(&pc).and((&ub).opt()).and(posts_of(db).select(Ident::<Post>::new().with(view_count)).opt()));
    let v = top_n(v, |&(u, (((a, _), _), p))| (Reverse(reputation.get(u).unwrap()), Reverse(a[0]), u, p), 10);
    rows(v.into_iter().map(|(u, (((a, n), b), p))| {
        let (bn, bs) = b.unwrap_or((0, "No badges"));
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(bn), V::S(bs)]);
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "views", "score"]),
            None => vec![V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.AnswerCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY
//        p.CreationDate DESC, p.Id) AS RowNum, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) OVER (PARTITION BY p.Id) AS UpvoteCount, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3) OVER
//        (PARTITION BY p.Id) AS DownvoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) OVER (PARTITION BY p.ParentId) AS ParentUpvotes FROM Posts p LEFT JOIN Votes v ON
//        p.Id = v.PostId WHERE p.CreationDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year') AND p.Score IS NOT NULL), UserStats AS (SELECT u.Id AS UserId,
//        u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(COALESCE(u.UpVotes, 0) - COALESCE(u.DownVotes, 0)) AS ReputationDelta, MAX(b.Class) AS HighestBadgeClass FROM Users u LEFT
//        JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName), ClosedPosts AS (SELECT ph.PostId, COUNT(*) AS CloseCount, STRING_AGG(DISTINCT ph.Comment, ', ') AS
//        CloseReasons FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId) SELECT rp.Title, us.DisplayName, rp.CreationDate, rp.Score, rp.AnswerCount,
//        rp.UpvoteCount, rp.DownvoteCount, rp.ParentUpvotes, us.BadgeCount, COALESCE(cp.CloseCount, 0) AS CloseCount, cp.CloseReasons FROM RankedPosts rp JOIN UserStats us ON
//        rp.OwnerUserId = us.UserId LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId WHERE rp.RowNum = 1 AND (rp.Score BETWEEN 10 AND 100 OR rp.AnswerCount > 5) AND
//        COALESCE(cp.CloseCount, 0) < 2 ORDER BY rp.CreationDate DESC LIMIT 50;
//
// (uses rewrites/20230.sql: p.Id breaks the CreationDate tie in RowNum)
//
// RowNum is taken over the joined rows, but every row of a post carries the same values, so the newest post per owner is picked first
// (a tie goes to the smaller post id). The distinct close comments are joined in sorted order.
fn q20230(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, answer_count, parent_id, .. } = &db.post;
    let recent = || db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let w = recent().group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k == 1).map(|((p, _), _)| p).collect();
    let vt = || votes_of(db).select(&db.vote.vote_type_id);
    let vc = recent().group_by(Ident::<Post>::new()).select(vt()).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let pu = recent().group_by(parent_id.opt()).select(vt().opt()).fold(0i64, |a, t| a + (t == Some(2)) as i64);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |a, b| a + b.is_some() as i64);
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.is_in([10, 11])).group_by(post).select(comment.opt()).buf_fold(|v| (v.len() as i64, agg_distinct(v.iter().flatten().copied().collect(), ", ")));
    type X = ((((Id<Post>, Id<User>), i64), Option<[i64; 2]>), i64);
    let v = drain(
        (&tp)
            .with(score.filt(|s| (10..=100).contains(&s)).or(answer_count.gt(5)))
            .select(Ident::<Post>::new().and(owner_user).and(owner_user.select(&bc)).and((&vc).opt()).and(parent_id.opt().select(&pu)).and((&cp).opt()))
            .filt(|(_, c): (X, Option<(i64, Option<Str>)>)| c.map_or(0, |c| c.0) < 2),
    );
    let v = top_n(v, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|(_, (((((p, u), b), a), pu), c))| {
        let a = a.unwrap_or([0; 2]);
        let mut f = post_fields(db, p, &["title"]);
        f.push(user_col(db, u, "name"));
        f.extend(post_fields(db, p, &["created", "score", "answers"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(pu), V::I(b), V::I(c.map_or(0, |c| c.0)), ostr(c.and_then(|c| c.1))]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS
//        AnswerCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, AVG(COALESCE(P.Score, 0)) AS AvgScore, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews FROM
//        Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation), TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount,
//        AnswerCount, QuestionCount, AvgScore, TotalViews, ROW_NUMBER() OVER (ORDER BY Reputation DESC, PostCount DESC, AvgScore DESC) AS Rnk FROM UserStats), PostTags AS (SELECT
//        P.Id AS PostId, UNNEST(STRING_TO_ARRAY(P.Tags, '<>')) AS Tag FROM Posts P WHERE P.Tags IS NOT NULL), TagStats AS (SELECT T.Tag AS TagName, COUNT(DISTINCT T.PostId) AS
//        PostCount, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews FROM PostTags T JOIN Posts P ON T.PostId = P.Id GROUP BY T.Tag), TopTags AS (SELECT TagName, PostCount, TotalViews,
//        ROW_NUMBER() OVER (ORDER BY PostCount DESC, TotalViews DESC) AS Rnk FROM TagStats) SELECT U.DisplayName AS UserName, U.Reputation, U.PostCount, U.AnswerCount,
//        U.QuestionCount, U.AvgScore, U.TotalViews, T.TagName, T.PostCount AS TagPostCount, T.TotalViews AS TagTotalViews FROM TopUsers U JOIN TopTags T ON U.Rnk <= 10 AND T.Rnk
//        <= 10 ORDER BY U.Reputation DESC, U.PostCount DESC, T.PostCount DESC;
//
// `string_to_array(P.Tags, '<>')` never splits (the separator is '><'), so each tag is the whole Tags string.
fn q5841(db: &'static So) -> String {
    let Post { tags_str, view_count, .. } = &db.post;
    let up = user_posts(db);
    let tu = top_n(drain(&up), |&(u, a)| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[1]), Reverse(fkey(a[4] as f64 / a[0] as f64)), u), 10);
    let tk = || db.post.group_by(tags_str.flat_map(|t: Str| t.split("<>")));
    let tn = tk().select(Ident::<Post>::new()).count_distinct();
    let tw = tk().select(view_count.opt()).fold(0i64, |s, w| s + w.unwrap_or(0));
    let tt = top_n(drain((&tn).and(&tw)), |&(t, (n, w))| (Reverse(n), Reverse(w), t), 10);
    let v = drain(rel(tu).cross(rel(tt)));
    rows(v.into_iter().map(|(_, ((u, a), (t, (n, w))))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[1]), V::I(a[3]), V::I(a[2]), avg(a[4], a[0]), V::I(a[6]), V::S(t), V::I(n), V::I(w)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, p.PostTypeId, u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION
//        BY p.PostTypeId ORDER BY p.Score DESC) AS Rank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year'), RecentVotes AS
//        (SELECT PostId, COUNT(*) AS VoteCount FROM Votes WHERE CreationDate >= CURRENT_DATE - INTERVAL '6 months' GROUP BY PostId), ClosedPosts AS (SELECT ph.PostId,
//        MIN(ph.CreationDate) AS ClosedDate, STRING_AGG(DISTINCT ctr.Name, ', ') AS CloseReasons FROM PostHistory ph JOIN CloseReasonTypes ctr ON CAST(ph.Comment AS INT) = ctr.Id
//        WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId), TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CreationDate, rp.OwnerDisplayName,
//        COALESCE(rv.VoteCount, 0) AS RecentVoteCount, cp.ClosedDate, cp.CloseReasons, CASE WHEN cp.ClosedDate IS NOT NULL THEN 'Closed' ELSE 'Active' END AS PostStatus FROM
//        RankedPosts rp LEFT JOIN RecentVotes rv ON rp.PostId = rv.PostId LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId WHERE rp.Rank <= 10) SELECT tp.PostId, tp.Title,
//        tp.Score, tp.ViewCount, tp.CreationDate, tp.OwnerDisplayName, tp.RecentVoteCount, tp.ClosedDate, tp.CloseReasons, tp.PostStatus FROM TopPosts tp ORDER BY tp.Score DESC,
//        tp.ViewCount DESC;
//
// A score tie inside the ROW_NUMBER goes to the smaller post id (the SQL leaves it open). The distinct close reasons are joined in name order.
fn q30632(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, score, .. } = &db.post;
    let cd = current_date();
    let w = db.post.with(creation_date.ge(add_years(cd, -1))).with(owner_user).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 10).map(|((p, _), _)| p).collect();
    let rv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(Ident::<Vote>::new().with((&db.vote.creation_date).ge(add_months(cd, -6))))).fold(0i64, |a, _| a + 1);
    let cr = close_reasons(db);
    let PostHistory { post, post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let cp = db
        .post_history
        .with(post_history_type_id.is_in([10, 11]))
        .group_by(post)
        .select(hd.and(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&cr).select(&db.close_reason_type.name)))
        .buf_fold(|v| (v.iter().map(|x| x.0).min().unwrap(), agg_distinct(v.iter().map(|x| x.1).collect(), ", ").unwrap()));
    let v = drain((&tp).select(Ident::<Post>::new().and((&rv).opt()).and((&cp).opt())));
    rows(v.into_iter().map(|(_, ((p, n), c))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "created", "owner"]);
        f.push(V::I(n.unwrap_or(0)));
        f.extend(match c {
            Some((d, r)) => [V::T(d), V::S(r), V::S("Closed")],
            None => [V::Null, V::Null, V::S("Active")],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.PostTypeId, P.OwnerUserId, P.Score, P.ViewCount, ROW_NUMBER() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score
//        DESC, P.CreationDate ASC) AS Rank FROM Posts P WHERE P.CreationDate >= CAST('2024-10-01 12:34:56' AS timestamp) - INTERVAL '1 year'), UserStats AS (SELECT U.Id AS UserId,
//        U.DisplayName, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1
//        ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName), CloseReasons AS (SELECT PH.PostId, STRING_AGG(CRT.Name, ',
//        ') AS CloseReasons FROM PostHistory PH JOIN CloseReasonTypes CRT ON CAST(PH.Comment AS int) = CRT.Id WHERE PH.PostHistoryTypeId = 10 GROUP BY PH.PostId) SELECT RP.PostId,
//        RP.Title, RP.Score, RP.ViewCount, U.DisplayName AS OwnerDisplayName, COALESCE(UR.GoldBadges, 0) AS GoldBadges, COALESCE(UR.SilverBadges, 0) AS SilverBadges,
//        COALESCE(UR.BronzeBadges, 0) AS BronzeBadges, CR.CloseReasons, CASE WHEN RP.Rank > 10 THEN 'Not Featured' ELSE 'Featured' END AS PostRankStatus FROM RankedPosts RP LEFT
//        JOIN Users U ON RP.OwnerUserId = U.Id LEFT JOIN UserStats UR ON U.Id = UR.UserId LEFT JOIN CloseReasons CR ON RP.PostId = CR.PostId WHERE RP.PostId NOT IN (SELECT
//        DISTINCT PL.RelatedPostId FROM PostLinks PL WHERE PL.LinkTypeId = 3) AND (U.Reputation > 500 OR U.Location IS NOT NULL) ORDER BY RP.Score DESC, RP.ViewCount DESC,
//        RP.Title ASC;
//
// A tie inside the ROW_NUMBER goes to the smaller post id (the SQL leaves it open). The STRING_AGG order is left open; the port joins in history id order.
fn q24546(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, score, origid, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), d, p), asc);
    let linked: MatSet<i64> = db.post_link.with((&db.post_link.link_type_id).eq(3)).select(&db.post_link.related_post_id).collect();
    let User { reputation, location, .. } = &db.user;
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(badges_of(db).select(&db.badge.class).opt())
        .fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let cr = close_reasons(db);
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let crs = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&cr).select(&db.close_reason_type.name))
        .buf_fold(|v| leak(v.join(", ")));
    type R = (Id<Post>, i64);
    let rv: MatSet<R> = (&w).map(|(((p, _), _), k)| (p, k)).collect();
    let k = || Same::<R>::new().map(|x: R| x.0);
    let v = drain(
        (&rv)
            .select(Same::<R>::new().with(k().select(Ident::<Post>::new().minus(origid.select(&linked)))))
            .select(
                Same::<R>::new()
                    .and(k().select(owner_user.select(Ident::<User>::new().with(reputation.gt(500).or(location)).and(&us))))
                    .and(k().select((&crs).opt())),
            ),
    );
    rows(v.into_iter().map(|(_, (((p, n), (_, b)), c))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "owner"]);
        f.extend(b.map(V::I));
        f.extend([ostr(c), V::S(if n > 10 { "Not Featured" } else { "Featured" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerName, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER
//        BY p.CreationDate DESC) AS RN FROM Posts p INNER JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'), TopPosts AS
//        (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerName FROM RankedPosts rp WHERE rp.RN <= 10), PostVoteCounts AS (SELECT PostId, COUNT(CASE
//        WHEN VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes GROUP BY PostId), PostHistorySummary AS (SELECT p.Id AS
//        PostId, ph.PostHistoryTypeId, STRING_AGG(DISTINCT ph.Comment, ', ') AS HistoryComments FROM PostHistory ph INNER JOIN Posts p ON ph.PostId = p.Id WHERE ph.CreationDate >=
//        cast('2024-10-01' as date) - INTERVAL '6 months' GROUP BY p.Id, ph.PostHistoryTypeId) SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.OwnerName,
//        COALESCE(pvc.UpVotes, 0) AS UpVotes, COALESCE(pvc.DownVotes, 0) AS DownVotes, SUM(CASE WHEN phs.PostId IS NOT NULL THEN 1 ELSE 0 END) AS InHistory, phs.HistoryComments
//        FROM TopPosts tp LEFT JOIN PostVoteCounts pvc ON tp.PostId = pvc.PostId LEFT JOIN PostHistorySummary phs ON tp.PostId = phs.PostId GROUP BY tp.PostId, tp.Title,
//        tp.CreationDate, tp.Score, tp.ViewCount, tp.OwnerName, pvc.UpVotes, pvc.DownVotes, phs.HistoryComments ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// A CreationDate tie inside the ROW_NUMBER goes to the smaller post id (the SQL leaves it open). The distinct comments are joined in sorted order.
fn q30832(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, .. } = &db.post;
    let w = db.post.with(creation_date.ge(date(2023, 10, 1))).with(owner_user).group_by(post_type_id).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 10).map(|((p, _), _)| p).collect();
    let pvc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let PostHistory { post, post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let phs = db
        .post_history
        .with(hd.ge(add_months(date(2024, 10, 1), -6)))
        .group_by(post.and(post_history_type_id))
        .select(comment.opt())
        .buf_fold(|v| agg_distinct(v.iter().flatten().copied().collect(), ", "));
    type H = ((Id<Post>, i64), Option<Str>);
    let hv = rel(drain(&phs));
    let hi: HashIdx<Id<Post>, H> = (&hv).map(|x: H| x.0 .0).inv().select(&hv).collect();
    type X = (Id<Post>, (Option<[i64; 2]>, Option<H>));
    let g = (&tp)
        .select(Ident::<Post>::new().and((&pvc).opt().and((&hi).opt())))
        .group_by(Same::<X>::new().map(|(p, (a, h)): X| (p, a, h.and_then(|h| h.1))))
        .select(Same::<X>::new().map(|x: X| x.1 .1))
        .fold(0i64, |n, h| n + h.is_some() as i64);
    rows(drain(&g).into_iter().map(|((p, a, c), n)| {
        let a = a.unwrap_or([0; 2]);
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(n), ostr(c)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, COALESCE(a.AnswerCount, 0) AS AnswerCount, COALESCE(v.UpVotes -
//        v.DownVotes, 0) AS NetVotes, RANK() OVER (PARTITION BY pt.Name ORDER BY p.CreationDate DESC) AS RankInType FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT
//        JOIN (SELECT ParentId, COUNT(*) AS AnswerCount FROM Posts WHERE PostTypeId = 2 GROUP BY ParentId) a ON a.ParentId = p.Id LEFT JOIN (SELECT PostId, SUM(CASE WHEN
//        VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY PostId) v ON v.PostId = p.Id JOIN PostTypes
//        pt ON pt.Id = p.PostTypeId WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'), ClosedPosts AS (SELECT ph.PostId, MIN(ph.CreationDate)
//        AS FirstClosedDate, STRING_AGG(DISTINCT cr.Name, ', ') AS CloseReasons FROM PostHistory ph JOIN CloseReasonTypes cr ON CAST(cr.Id AS TEXT) = ph.Comment WHERE
//        ph.PostHistoryTypeId = 10 GROUP BY ph.PostId) SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.CreationDate, rp.AnswerCount, rp.NetVotes, cp.FirstClosedDate,
//        cp.CloseReasons FROM RankedPosts rp LEFT JOIN ClosedPosts cp ON cp.PostId = rp.PostId WHERE (rp.RankInType = 1 OR (cp.FirstClosedDate IS NOT NULL AND cp.FirstClosedDate <
//        CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '6 months')) ORDER BY rp.NetVotes DESC, rp.CreationDate ASC LIMIT 100;
//
// The distinct close reasons are joined in name order.
fn q24143(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let w = db.post.with(creation_date.ge(add_years(t0, -1))).group_by(ptype_name(db)).select(Ident::<Post>::new().and(creation_date)).window(rank, |(_, d)| Reverse(d), asc);
    let ac = db.post.with((&db.post.post_type_id).eq(2)).group_by(&db.post.parent).fold(0i64, |a, _| a + 1);
    let nv = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold(0i64, |a, t| a + (t == 2) as i64 - (t == 3) as i64);
    let crt: HashIdx<Str, Id<CloseReasonType>> = (&db.close_reason_type.origid).map(|i: i64| -> Str { leak(i.to_string()) }).inv().collect();
    let PostHistory { post, post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let cp = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(post)
        .select(hd.and(comment.select(&crt).select(&db.close_reason_type.name)))
        .buf_fold(|v| (v.iter().map(|x| x.0).min().unwrap(), agg_distinct(v.iter().map(|x| x.1).collect(), ", ").unwrap()));
    type R = (Id<Post>, i64);
    let rv: MatSet<R> = (&w).map(|((p, _), k)| (p, k)).collect();
    let cut = add_months(t0, -6);
    let v = drain(
        (&rv)
            .select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0).select((&cp).opt())))
            .filt(|((_, n), c): (R, Option<(i64, Str)>)| n == 1 || c.map_or(false, |c| c.0 < cut))
            .select(Same::<(R, Option<(i64, Str)>)>::new().and(Same::<(R, Option<(i64, Str)>)>::new().map(|x: (R, Option<(i64, Str)>)| x.0 .0).select((&ac).opt().and((&nv).opt())))),
    );
    let v = top_n(v, |&(_, (((p, _), _), (_, n)))| (Reverse(n.unwrap_or(0)), creation_date.get(p).unwrap(), p), 100);
    rows(v.into_iter().map(|(_, (((p, _), c), (a, n)))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created"]);
        f.extend([V::I(a.unwrap_or(0)), V::I(n.unwrap_or(0))]);
        f.extend(match c {
            Some((d, s)) => [V::T(d), V::S(s)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score
//        DESC, p.CreationDate DESC) AS UserRank FROM Posts p WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days'), UserStats AS (SELECT u.Id AS UserId,
//        u.DisplayName, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id =
//        b.UserId GROUP BY u.Id, u.DisplayName), PostHistoryDetails AS (SELECT ph.PostId, ph.CreationDate AS HistoryDate, ph.Comment, p.Title AS PostTitle FROM PostHistory ph JOIN
//        Posts p ON ph.PostId = p.Id WHERE ph.PostHistoryTypeId IN (10, 11, 12)), TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, us.UserId, us.DisplayName,
//        us.TotalBounty, us.BadgeCount, PHD.HistoryDate, PHD.Comment FROM RankedPosts rp JOIN UserStats us ON rp.PostId = us.UserId LEFT JOIN PostHistoryDetails PHD ON rp.PostId =
//        PHD.PostId WHERE rp.UserRank <= 5) SELECT tgt.*, CASE WHEN tgt.HistoryDate IS NOT NULL THEN 'Edited' ELSE 'New' END AS PostStatus, ARRAY_AGG(DISTINCT t.TagName) AS Tags
//        FROM TopPosts tgt LEFT JOIN Tags t ON tgt.PostId = t.ExcerptPostId GROUP BY tgt.PostId, tgt.Title, tgt.Score, tgt.ViewCount, tgt.UserId, tgt.DisplayName, tgt.TotalBounty,
//        tgt.BadgeCount, tgt.HistoryDate, tgt.Comment ORDER BY tgt.Score DESC, tgt.ViewCount DESC;
//
// `rp.PostId = us.UserId` joins a post id to a user id, as written. A tie inside the ROW_NUMBER goes to the smaller post id.
// The ARRAY_AGG(DISTINCT ...) is listed in name order.
fn q4550(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, score, origid, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_days(date(2024, 10, 1), -30)))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|(((p, _), _), _)| p).collect();
    let uo = users_by_origid(db);
    let mu: MatSet<Id<User>> = (&tp).select(origid).select(&uo).collect();
    let us = (&mu)
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select((&db.vote.bounty_amount).opt()).opt().and(badges_of(db).opt()))
        .fold((0i64, 0i64), |(s, n), (b, x)| (s + b.flatten().unwrap_or(0), n + x.is_some() as i64));
    let PostHistory { post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let phd = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([10, 11, 12])).select(hd.and(comment.opt())));
    type X = (Id<Post>, ((Id<User>, (i64, i64)), Option<(i64, Option<Str>)>));
    let ex: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    let g = (&tp)
        .select(Ident::<Post>::new().and(origid.select(&uo).select(Ident::<User>::new().and(&us)).and(phd.opt())))
        .group_by(Same::<X>::new())
        .select(Same::<X>::new().map(|x: X| x.0).select(ex.select(&db.tag.tag_name).opt()))
        .buf_fold(|v| {
            let mut t: Vec<Option<Str>> = v.to_vec();
            t.sort();
            t.dedup();
            &*Box::leak(t.into_boxed_slice())
        });
    let v = top_n(drain(&g), |&((p, _), _)| {
        let w = db.post.view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    }, 0);
    rows(v.into_iter().map(|((p, ((u, (b, n)), h)), t)| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend(ucols(db, u, &["uid", "name"]));
        f.extend([V::I(b), V::I(n)]);
        f.extend(match h {
            Some((d, c)) => [V::T(d), ostr(c), V::S("Edited")],
            None => [V::Null, V::Null, V::S("New")],
        });
        f.push(V::L(t.iter().map(|&s| ostr(s)).collect()));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, p.CreationDate, p.OwnerUserId, ROW_NUMBER() OVER(PARTITION BY p.OwnerUserId ORDER BY p.Score
//        DESC) AS RankByScore FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.Score > 0), TopUsers AS (SELECT u.Id AS UserId,
//        u.DisplayName, SUM(COALESCE(b.Class, 0)) AS TotalBadgeClass FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
//        PostClosureDetails AS (SELECT ph.PostId, ph.UserDisplayName AS ClosedBy, ph.CreationDate AS ClosureDate, ct.Name AS CloseReason FROM PostHistory ph JOIN CloseReasonTypes
//        ct ON CAST(ph.Comment AS int) = ct.Id WHERE ph.PostHistoryTypeId = 10), UserPostLinks AS (SELECT pl.PostId, COUNT(pl.RelatedPostId) AS LinkCount FROM PostLinks pl GROUP
//        BY pl.PostId) SELECT up.UserId, up.DisplayName, COUNT(DISTINCT rp.PostId) AS TotalPosts, SUM(CASE WHEN rp.RankByScore = 1 THEN 1 ELSE 0 END) AS AcceptedAnswers,
//        COALESCE(SUM(CASE WHEN pc.CloseReason IS NOT NULL THEN 1 ELSE 0 END), 0) AS TotalClosures, COALESCE(SUM(DISTINCT upl.LinkCount), 0) AS TotalLinks, SUM(TotalBadgeClass) AS
//        UserBadges, STRING_AGG(DISTINCT pc.CloseReason, ', ') AS CloseReasons FROM TopUsers up JOIN RankedPosts rp ON up.UserId = rp.OwnerUserId LEFT JOIN PostClosureDetails pc
//        ON pc.PostId = rp.PostId LEFT JOIN UserPostLinks upl ON upl.PostId = rp.PostId GROUP BY up.UserId, up.DisplayName HAVING COUNT(DISTINCT rp.PostId) > 5 ORDER BY UserBadges
//        DESC, TotalPosts DESC LIMIT 20;
//
// A score tie inside the ROW_NUMBER goes to the smaller post id (the SQL leaves it open). The distinct close reasons are joined in name order.
fn q20756(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, score, .. } = &db.post;
    let rp = || Ident::<Post>::new().with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(score.gt(0));
    let w = db.post.select(rp()).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let r1: MatSet<Id<Post>> = (&w).filt(|(_, k)| k == 1).map(|((p, _), _)| p).collect();
    let tbc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold(0i64, |a, c| a + c.unwrap_or(0));
    let cr = close_reasons(db);
    let PostHistory { post_history_type_id, comment, .. } = &db.post_history;
    let pc = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.eq(10)).select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&cr).select(&db.close_reason_type.name)));
    let upl = db.post_link.group_by(&db.post_link.post).fold(0i64, |a, _| a + 1);
    let tu = || db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new());
    let np = tu().select(posts_of(db).select(rp())).count_distinct();
    let g = tu()
        .select((&tbc).and(posts_of(db).select(rp()).select(Ident::<Post>::new().and(Ident::<Post>::new().with(&r1).opt()).and(pc.opt()).and((&upl).opt()))))
        .buf_fold(|v| {
            let a = v.iter().filter(|x| x.1 .0 .0 .1.is_some()).count() as i64;
            let c = v.iter().filter(|x| x.1 .0 .1.is_some()).count() as i64;
            let mut l: Vec<i64> = v.iter().filter_map(|x| x.1 .1).collect();
            l.sort();
            l.dedup();
            let b: i64 = v.iter().map(|x| x.0).sum();
            let r = agg_distinct(v.iter().filter_map(|x| x.1 .0 .1).collect(), ", ");
            (a, c, l.iter().sum::<i64>(), b, r)
        });
    let v = top_n(drain((&np).filt(|n| n > 5).and(&g)), |&(_, (n, (_, _, _, b, _)))| (Reverse(b), Reverse(n)), 20);
    rows(v.into_iter().map(|(u, (n, (a, c, l, b, r)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(a), V::I(c), V::I(l), V::I(b), ostr(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS
//        RankScore, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS RankDate FROM Posts p), FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.Score,
//        rp.CreationDate, rp.ViewCount, CASE WHEN rp.RankScore <= 5 THEN 'Top 5' WHEN rp.RankDate <= 10 THEN 'Recent 10' END AS PostCategory FROM RankedPosts rp WHERE rp.RankScore
//        <= 5 OR rp.RankDate <= 10), PostVoteSummary AS (SELECT p.Id AS PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1
//        ELSE 0 END) AS DownVotes FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id) SELECT fp.PostId, fp.Title, fp.Score, fp.CreationDate, fp.ViewCount,
//        COALESCE(pvs.UpVotes, 0) AS TotalUpVotes, COALESCE(pvs.DownVotes, 0) AS TotalDownVotes, CASE WHEN fp.Score > 10 THEN 'High Score Group' ELSE 'Low Score Group' END AS
//        ScoreGroup, COALESCE(t.TagName, 'No Tags') AS TagName, CASE WHEN fp.PostCategory IS NOT NULL THEN 'Featured Post' ELSE 'Regular Post' END AS PostType FROM FilteredPosts
//        fp LEFT JOIN PostVoteSummary pvs ON fp.PostId = pvs.PostId LEFT JOIN Posts AS p ON fp.PostId = p.Id LEFT JOIN LATERAL (SELECT unnest(string_to_array(p.Tags, '><')) AS
//        TagName) AS t ON TRUE WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days' AND (p.OwnerUserId IS NOT NULL OR fp.PostCategory IS NOT NULL) ORDER BY
//        fp.Score DESC, fp.ViewCount DESC;
//
// A score tie inside the ROW_NUMBER goes to the smaller post id (the SQL leaves it open).
fn q22715(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, tags_str, owner_user_id, .. } = &db.post;
    let sw = db.post.group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let dw = db.post.group_by(post_type_id).select(Ident::<Post>::new().and(creation_date)).window(rank, |(_, d)| Reverse(d), asc);
    let fp: MatSet<Id<Post>> = (&sw).filt(|(_, k)| k <= 5).map(|((p, _), _)| p).union((&dw).filt(|(_, k)| k <= 10).map(|((p, _), _)| p)).collect();
    let pvs = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain(
        (&fp)
            .with(creation_date.ge(add_days(date(2024, 10, 1), -30)))
            .with(owner_user_id.or(Ident::<Post>::new()))
            .select(Ident::<Post>::new().and(&pvs).and(tags_str.flat_map(|t: Str| t.split("><")).opt())),
    );
    rows(v.into_iter().map(|(_, ((p, a), t))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "created", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::S(if score.get(p).unwrap() > 10 { "High Score Group" } else { "Low Score Group" }), V::S(t.unwrap_or("No Tags")), V::S("Featured Post")]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC,
//        p.CreationDate DESC) AS Rank FROM Posts p WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' AND p.Score IS NOT NULL), UserInteractions
//        AS (SELECT u.Id AS UserId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(DISTINCT
//        c.Id) AS CommentCount, MAX(b.Date) AS LastBadgeDate FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Comments c ON u.Id = c.UserId LEFT JOIN Badges b ON u.Id =
//        b.UserId GROUP BY u.Id), PostStatistics AS (SELECT p.Id AS PostId, COALESCE(rp.Rank, 0) AS PostRank, COALESCE(ui.UpVotes, 0) AS TotalUpVotes, COALESCE(ui.DownVotes, 0) AS
//        TotalDownVotes, COALESCE(ui.CommentCount, 0) AS TotalComments FROM Posts p LEFT JOIN RankedPosts rp ON p.Id = rp.PostId LEFT JOIN UserInteractions ui ON p.OwnerUserId =
//        ui.UserId), CloseReasons AS (SELECT ph.PostId, STRING_AGG(cr.Name, ', ') AS CloseReasonNames FROM PostHistory ph INNER JOIN CloseReasonTypes cr ON ph.Comment::INTEGER =
//        cr.Id WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId) SELECT ps.PostId, ps.PostRank, ps.TotalUpVotes, ps.TotalDownVotes, ps.TotalComments, cr.CloseReasonNames FROM
//        PostStatistics ps LEFT JOIN CloseReasons cr ON ps.PostId = cr.PostId WHERE ps.PostRank <= 5 AND ps.TotalUpVotes - ps.TotalDownVotes > 0 ORDER BY ps.TotalUpVotes DESC,
//        ps.PostRank ASC LIMIT 10;
//
// A tie inside the ROW_NUMBER goes to the smaller post id (the SQL leaves it open). The STRING_AGG order is left open; the port joins in history id order.
fn q22426(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, owner_user, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let ri = by_first(&(&w).map(|(((p, _), _), k)| (p, k)).collect());
    let ui = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(comments_by(db).opt()).and(badges_of(db).opt()))
        .fold([0i64; 2], |a, ((t, _), _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let uc = db.user.group_by(Ident::<User>::new()).select(comments_by(db)).fold(0i64, |a, _| a + 1);
    let cr = close_reasons(db);
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let crn = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&cr).select(&db.close_reason_type.name))
        .buf_fold(|v| leak(v.join(", ")));
    type X = (((Id<Post>, Option<i64>), Option<([i64; 2], Option<i64>)>), Option<Str>);
    let v = drain(
        db.post
            .select(Ident::<Post>::new().and((&ri).opt()).and(owner_user.select((&ui).and((&uc).opt())).opt()).and((&crn).opt()))
            .filt(|(((_, r), u), _): X| {
                let a = u.map_or([0; 2], |u| u.0);
                r.unwrap_or(0) <= 5 && a[0] - a[1] > 0
            }),
    );
    let v = top_n(v, |&(_, (((p, r), u), _))| (Reverse(u.map_or(0, |u| u.0[0])), r.unwrap_or(0), p), 10);
    rows(v.into_iter().map(|(_, (((p, r), u), c))| {
        let (a, n) = u.map_or(([0; 2], None), |u| u);
        let mut f = post_fields(db, p, &["id"]);
        f.extend([V::I(r.unwrap_or(0)), V::I(a[0]), V::I(a[1]), V::I(n.unwrap_or(0)), ostr(c)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.PostTypeId, P.ViewCount, P.CreationDate, P.Score, RANK() OVER (PARTITION BY P.PostTypeId ORDER BY P.CreationDate
//        DESC) AS RecentRank, DENSE_RANK() OVER (ORDER BY P.Score DESC) AS ScoreRank, COALESCE(P.AcceptedAnswerId, -1) AS AcceptedAnswerId FROM Posts P LEFT JOIN Users U ON
//        P.OwnerUserId = U.Id WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'), RecentActivity AS (SELECT C.PostId, COUNT(C.Id) AS CommentCount,
//        SUM(CASE WHEN C.UserId IS NOT NULL THEN 1 ELSE 0 END) AS UserComments FROM Comments C JOIN Posts P ON C.PostId = P.Id WHERE C.CreationDate >= TIMESTAMP '2024-10-01
//        12:34:56' - INTERVAL '30 days' GROUP BY C.PostId), PostCloseReasons AS (SELECT PH.PostId, STRING_AGG(DISTINCT CRT.Name, ', ') AS CloseReasons FROM PostHistory PH JOIN
//        CloseReasonTypes CRT ON CAST(PH.Comment AS int) = CRT.Id WHERE PH.PostHistoryTypeId IN (10, 11) GROUP BY PH.PostId), UserBadges AS (SELECT U.Id AS UserId, B.Class,
//        COUNT(B.Id) AS BadgeCount, STRING_AGG(B.Name, ', ') AS BadgeNames FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, B.Class) SELECT RP.PostId, RP.Title,
//        RP.PostTypeId, RP.ViewCount, RP.CreationDate, RP.Score, RA.CommentCount, RA.UserComments, PCR.CloseReasons, UB.BadgeCount, UB.BadgeNames FROM RankedPosts RP LEFT JOIN
//        RecentActivity RA ON RP.PostId = RA.PostId LEFT JOIN PostCloseReasons PCR ON RP.PostId = PCR.PostId LEFT JOIN UserBadges UB ON RP.PostId = UB.UserId WHERE RP.RecentRank
//        <= 10 AND RP.ScoreRank > 5 ORDER BY RP.ViewCount DESC, RP.Score DESC;
//
// `RP.PostId = UB.UserId` joins a post id to a user id, as written. The STRING_AGG orders are left open; the port joins in badge id order.
fn q24556(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, origid, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let base = || db.post.with(creation_date.ge(add_years(t0, -1)));
    let rw = base().group_by(post_type_id).select(Ident::<Post>::new().and(creation_date)).window(rank, |(_, d)| Reverse(d), asc);
    let sw = whole(base()).select(Ident::<Post>::new().and(score)).window(dense_rank, |(_, s)| Reverse(s), asc);
    let srk = by_first(&(&sw).map(|((p, _), k)| (p, k)).collect());
    let rp: MatSet<Id<Post>> = (&rw).filt(|(_, k)| k <= 10).map(|((p, _), _)| p).with((&srk).filt(|k| k > 5)).collect();
    let Comment { creation_date: cd, user_id, .. } = &db.comment;
    let ra = db.comment.with(cd.ge(add_days(t0, -30))).group_by(&db.comment.post).select(user_id.opt()).fold([0i64; 2], |a, u| [a[0] + 1, a[1] + u.is_some() as i64]);
    let cr = close_reasons(db);
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let pcr = db
        .post_history
        .with(post_history_type_id.is_in([10, 11]))
        .group_by(post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&cr).select(&db.close_reason_type.name))
        .buf_fold(|v| agg_distinct(v.to_vec(), ", ").unwrap());
    let ub = db.badge.group_by((&db.badge.user).and(&db.badge.class)).select(&db.badge.name).buf_fold(|v| (v.len() as i64, leak(v.join(", "))));
    type B = ((Id<User>, i64), (i64, Str));
    let bv = rel(drain(&ub));
    let ubi: HashIdx<Id<User>, B> = (&bv).map(|x: B| x.0 .0).inv().select(&bv).collect();
    let uo = users_by_origid(db);
    let v = drain((&rp).select(Ident::<Post>::new().and((&ra).opt()).and((&pcr).opt()).and(origid.select(&uo).select(Ident::<User>::new().and((&ubi).opt())).opt())));
    let v = top_n(v, |&(_, (((p, _), _), _))| {
        let w = db.post.view_count.get(p);
        (w.is_none(), Reverse(w), Reverse(score.get(p).unwrap()))
    }, 0);
    rows(v.into_iter().map(|(_, (((p, a), c), u))| {
        let mut f = post_fields(db, p, &["id", "title", "type_id", "views", "created", "score"]);
        f.extend(match a {
            Some(a) => [V::I(a[0]), V::I(a[1])],
            None => [V::Null, V::Null],
        });
        f.push(ostr(c));
        f.extend(match u {
            Some((_, Some((_, (n, s))))) => [V::I(n), V::S(s)],
            Some((_, None)) => [V::I(0), V::Null],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RecursivePostHistory AS (SELECT ph.Id, ph.PostId, ph.PostHistoryTypeId, ph.CreationDate, ph.UserDisplayName, ph.Comment, ph.Text, ROW_NUMBER() OVER (PARTITION BY
//        ph.PostId ORDER BY ph.CreationDate DESC) AS rn FROM PostHistory ph), UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COALESCE(SUM(b.Class), 0) AS TotalBadgeClass
//        FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation), ClosedPosts AS (SELECT p.Id AS PostId, p.Title, COUNT(*) AS CloseReasonCount,
//        STRING_AGG(DISTINCT ct.Name, ', ') AS CloseReasons FROM Posts p INNER JOIN PostHistory ph ON p.Id = ph.PostId INNER JOIN PostHistoryTypes ht ON ph.PostHistoryTypeId =
//        ht.Id INNER JOIN CloseReasonTypes ct ON ph.Comment = CAST(ct.Id AS TEXT) WHERE ht.Name = 'Post Closed' GROUP BY p.Id, p.Title) SELECT p.Id AS PostId, p.Title,
//        p.ViewCount, p.Score, COALESCE(up.Reputation, 0) AS UserReputation, COALESCE(up.TotalBadgeClass, 0) AS TotalBadgeClass, MAX(CASE WHEN ph.rn = 1 THEN ph.CreationDate END)
//        AS MostRecentActivity, COALESCE(cp.CloseReasonCount, 0) AS CloseReasonCount, COALESCE(cp.CloseReasons, 'No Close Reasons') AS CloseReasons FROM Posts p LEFT JOIN Users u
//        ON p.OwnerUserId = u.Id LEFT JOIN UserReputation up ON u.Id = up.UserId LEFT JOIN RecursivePostHistory ph ON p.Id = ph.PostId LEFT JOIN ClosedPosts cp ON p.Id = cp.PostId
//        WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.ViewCount IS NOT NULL AND (p.Score IS NULL OR p.Score > 5) GROUP BY p.Id,
//        p.Title, p.ViewCount, p.Score, up.Reputation, up.TotalBadgeClass, cp.CloseReasonCount, cp.CloseReasons ORDER BY p.Score DESC, UserReputation DESC, MostRecentActivity DESC
//        LIMIT 50;
//
// `MAX(CASE WHEN ph.rn = 1 ...)` is the date of the post's newest history row, which is its latest history date. The distinct close reasons are joined in name order.
fn q23020(db: &'static So) -> String {
    let Post { creation_date, view_count, score, owner_user, .. } = &db.post;
    let up = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold(0i64, |a, c| a + c.unwrap_or(0));
    let crt: HashIdx<Str, Id<CloseReasonType>> = (&db.close_reason_type.origid).map(|i: i64| -> Str { leak(i.to_string()) }).inv().collect();
    let PostHistory { post, comment, creation_date: hd, .. } = &db.post_history;
    let cp = db
        .post_history
        .with(htype_name(db).eq("Post Closed"))
        .group_by(post)
        .select(comment.select(&crt).select(&db.close_reason_type.name))
        .buf_fold(|v| (v.len() as i64, agg_distinct(v.to_vec(), ", ").unwrap()));
    let mr = db.post_history.group_by(post).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let v = drain(
        db.post
            .with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
            .with(view_count)
            .with(score.gt(5))
            .select(Ident::<Post>::new().and(owner_user.select(Ident::<User>::new().and(&up)).opt()).and((&mr).opt()).and((&cp).opt())),
    );
    let v = top_n(v, |&(_, (((p, u), m), _))| (Reverse(score.get(p).unwrap()), Reverse(u.map_or(0, |u| db.user.reputation.get(u.0).unwrap())), m.is_none(), Reverse(m)), 50);
    rows(v.into_iter().map(|(_, (((p, u), m), c))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "score"]);
        f.extend(match u {
            Some((u, b)) => [user_col(db, u, "rep"), V::I(b)],
            None => [V::I(0), V::I(0)],
        });
        f.push(ots(m));
        let (n, s) = c.unwrap_or((0, "No Close Reasons"));
        f.extend([V::I(n), V::S(s)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.ViewCount, P.CreationDate, RANK() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC NULLS LAST) AS PostRank,
//        ROW_NUMBER() OVER (PARTITION BY P.PostTypeId ORDER BY P.CreationDate DESC) AS NewestPost FROM Posts P WHERE P.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1
//        year' AND P.Score IS NOT NULL), ClosedPosts AS (SELECT PH.PostId, COUNT(*) AS CloseCount, STRING_AGG(CAST(PH.Comment AS VARCHAR), '; ') AS CloseComments FROM PostHistory
//        PH WHERE PH.PostHistoryTypeId IN (10, 11) GROUP BY PH.PostId), UserStats AS (SELECT U.Id AS UserId, U.DisplayName, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS
//        TotalUpvotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes, COUNT(DISTINCT B.Id) AS BadgeCount FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId
//        LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName) SELECT RP.PostId, RP.Title, RP.ViewCount, RP.CreationDate, RP.PostRank, CP.CloseCount,
//        COALESCE(CP.CloseComments, 'No closure comments found') AS ClosureComments, US.DisplayName, US.TotalUpvotes, US.TotalDownvotes, US.BadgeCount, CASE WHEN RP.NewestPost = 1
//        THEN 'Newest Post of Type' ELSE 'Older Post' END AS PostStatus, CASE WHEN RP.PostRank <= 5 THEN 'Top Ranked' WHEN RP.PostRank IS NULL THEN 'No Rank' ELSE 'Other Ranked
//        Posts' END AS PopularityStatus FROM RankedPosts RP LEFT JOIN ClosedPosts CP ON RP.PostId = CP.PostId LEFT JOIN UserStats US ON RP.PostId = US.UserId WHERE RP.PostRank IS
//        NOT NULL AND (RP.ViewCount > 100 OR US.TotalUpvotes > 10) ORDER BY RP.PostRank, RP.Title DESC NULLS LAST OFFSET 10 ROWS FETCH NEXT 20 ROWS ONLY;
//
// `RP.PostId = US.UserId` joins a post id to a user id, as written. A tie inside the ROW_NUMBER, and among rows equal on the ORDER BY, goes to the smaller post id;
// the STRING_AGG order is left open by the SQL, and the port joins in history id order.
fn q20747(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, origid, view_count, title, .. } = &db.post;
    let base = || db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1)));
    let rw = base().group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| Reverse(s), asc);
    let nw = base().group_by(post_type_id).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    type R = (Id<Post>, i64);
    let rv: MatSet<R> = (&rw).map(|((p, _), k)| (p, k)).collect();
    let nk = by_first(&(&nw).map(|((p, _), k)| (p, k)).collect());
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.is_in([10, 11])).group_by(post).select(comment.opt()).buf_fold(|v| {
        let c: Vec<Str> = v.iter().flatten().copied().collect();
        (v.len() as i64, if c.is_empty() { None } else { Some(leak(c.join("; "))) })
    });
    let uo = users_by_origid(db);
    let k = || Same::<R>::new().map(|x: R| x.0);
    let mu: MatSet<Id<User>> = (&rv).select(k().select(origid).select(&uo)).collect();
    let us = (&mu)
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (t, _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let bc = (&mu).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |a, b| a + b.is_some() as i64);
    type X = ((((R, i64), Option<(i64, Option<Str>)>), Option<(Id<User>, ([i64; 2], i64))>), Option<i64>);
    let v = drain(
        (&rv)
            .select(
                Same::<R>::new()
                    .and(k().select(&nk))
                    .and(k().select((&cp).opt()))
                    .and(k().select(origid.select(&uo).select(Ident::<User>::new().and((&us).and(&bc)))).opt())
                    .and(k().select(view_count.opt())),
            )
            .filt(|((_, u), w): X| w.map_or(false, |w| w > 100) || u.map_or(false, |u| u.1 .0[0] > 10)),
    );
    let v = top_n(v, |&(_, (((((p, r), _), _), _), _))| (r, title.get(p).is_none(), Reverse(title.get(p)), p), 30);
    rows(v.into_iter().skip(10).map(|(_, (((((p, r), n), c), u), _))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "created"]);
        f.push(V::I(r));
        f.extend(match c {
            Some((n, s)) => [V::I(n), V::S(s.unwrap_or("No closure comments found"))],
            None => [V::Null, V::S("No closure comments found")],
        });
        f.extend(match u {
            Some((u, (a, b))) => [user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(b)],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        f.push(V::S(if n == 1 { "Newest Post of Type" } else { "Older Post" }));
        f.push(V::S(if r <= 5 { "Top Ranked" } else { "Other Ranked Posts" }));
        row(f)
    }))
}

// WITH UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN
//        v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COALESCE(SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END), 0) AS CommentCount, COALESCE(SUM(CASE WHEN p.Id IS NOT
//        NULL THEN 1 ELSE 0 END), 0) AS PostCount FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1 LEFT JOIN
//        Comments c ON u.Id = c.UserId GROUP BY u.Id, u.DisplayName), TopUsers AS (SELECT UserId, DisplayName, UpVotes, DownVotes, CommentCount, PostCount, RANK() OVER (ORDER BY
//        UpVotes - DownVotes DESC) AS VoteRank FROM UserEngagement), ActivePosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.LastActivityDate, COALESCE((SELECT COUNT(*)
//        FROM Comments c WHERE c.PostId = p.Id), 0) AS TotalComments FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL
//        '30 days'), PostDetails AS (SELECT ap.PostId, ap.Title, ap.CreationDate, ap.LastActivityDate, ap.TotalComments, COALESCE((SELECT STRING_AGG(DISTINCT t.TagName, ', ') FROM
//        Tags t WHERE t.WikiPostId = ap.PostId), 'No Tags') AS Tags FROM ActivePosts ap) SELECT tu.DisplayName, p.Title, p.TotalComments, tu.UpVotes, tu.DownVotes, tu.VoteRank,
//        CASE WHEN p.TotalComments > 10 THEN 'High Engagement' WHEN p.TotalComments BETWEEN 5 AND 10 THEN 'Moderate Engagement' ELSE 'Low Engagement' END AS EngagementLevel FROM
//        TopUsers tu JOIN PostDetails p ON tu.UserId = p.PostId WHERE tu.VoteRank <= 10 ORDER BY tu.VoteRank, p.TotalComments DESC;
//
// `tu.UserId = p.PostId` joins a user id to a post id, as written.
fn q3091(db: &'static So) -> String {
    let Post { creation_date, post_type_id, origid, .. } = &db.post;
    let ue = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1))).opt()).and(comments_by(db).opt()))
        .fold([0i64; 4], |a, ((t, p), c)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64, a[3] + p.is_some() as i64]);
    let uw = whole(&db.user.id).select(Ident::<User>::new().and(&ue)).window(rank, |(_, a)| Reverse(a[0] - a[1]), asc);
    type T = ((Id<User>, [i64; 4]), i64);
    let tv: MatSet<T> = (&uw).filt(|(_, k)| k <= 10).map(|x: T| x).collect();
    let pbo: HashIdx<i64, Id<Post>> = db.post.with(post_type_id.eq(1)).with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(origid).inv().collect();
    let cc = comments_per_post(db);
    let v = drain((&tv).select(Same::<T>::new().and(Same::<T>::new().map(|x: T| x.0 .0).select(&db.user.origid).select(&pbo).select(Ident::<Post>::new().and(&cc)))));
    rows(v.into_iter().map(|(_, (((u, a), r), (p, n)))| {
        let lvl = if n > 10 {
            "High Engagement"
        } else if (5..=10).contains(&n) {
            "Moderate Engagement"
        } else {
            "Low Engagement"
        };
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title"]));
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(r), V::S(lvl)]);
        row(f)
    }))
}

// WITH PostAnalytics AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, u.DisplayName AS Author, COUNT(c.Id) AS CommentCount, COALESCE(SUM(CASE WHEN vt.VoteTypeId
//        = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN vt.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, ARRAY_AGG(DISTINCT pt.Name) AS PostTypeNames,
//        ARRAY_AGG(DISTINCT t.TagName) AS Tags FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON c.PostId = p.Id LEFT JOIN Votes vt ON vt.PostId = p.Id
//        LEFT JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN UNNEST(string_to_array(SUBSTRING(p.Tags, 2, LENGTH(p.Tags)-2), '><')) AS t(TagName) ON t.TagName IS NOT NULL
//        GROUP BY p.Id, p.Title, p.Body, p.CreationDate, u.DisplayName), RecentEdits AS (SELECT ph.PostId, MAX(ph.CreationDate) AS LastEditDate FROM PostHistory ph WHERE
//        ph.PostHistoryTypeId IN (4, 5, 6) GROUP BY ph.PostId), PostPerformance AS (SELECT pa.PostId, pa.Title, pa.Author, pa.CommentCount, pa.UpVotes, pa.DownVotes,
//        pa.CreationDate, re.LastEditDate, EXTRACT(EPOCH FROM (TIMESTAMP '2024-10-01 12:34:56' - pa.CreationDate)) / 86400 AS DaysSinceCreation, EXTRACT(EPOCH FROM (TIMESTAMP
//        '2024-10-01 12:34:56' - re.LastEditDate)) / 86400 AS DaysSinceLastEdit FROM PostAnalytics pa LEFT JOIN RecentEdits re ON pa.PostId = re.PostId) SELECT PostId, Title,
//        Author, CommentCount, UpVotes, DownVotes, DaysSinceCreation, DaysSinceLastEdit, CASE WHEN DaysSinceLastEdit < 7 THEN 'Recently Edited' WHEN DaysSinceCreation < 30 THEN
//        'New' ELSE 'Established' END AS PostStatus FROM PostPerformance WHERE UpVotes > DownVotes ORDER BY UpVotes DESC, DaysSinceCreation ASC;
fn q25198(db: &'static So) -> String {
    let Post { owner_user, creation_date, tags_str, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let pa = db
        .post
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(tags_str.flat_map(tag_list).opt()))
        .fold([0i64; 3], |a, ((c, t), _)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let re = db.post_history.with(post_history_type_id.is_in([4, 5, 6])).group_by(post).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let v = drain((&pa).filt(|a: [i64; 3]| a[1] > a[2]).and((&re).opt()));
    rows(v.into_iter().map(|(p, (a, e))| {
        let dc = secs(t0 - creation_date.get(p).unwrap()) / 86400.0;
        let de = e.map(|e| secs(t0 - e) / 86400.0);
        let st = if de.map_or(false, |d| d < 7.0) {
            "Recently Edited"
        } else if dc < 30.0 {
            "New"
        } else {
            "Established"
        };
        let mut f = post_fields(db, p, &["id", "title", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::F(dc), ofloat(de), V::S(st)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes, COALESCE(SUM(CASE
//        WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes, COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT C.Id) AS TotalComments FROM Users U LEFT JOIN Votes V
//        ON U.Id = V.UserId LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY U.Id, U.DisplayName, U.Reputation), PostHistorySummary AS
//        (SELECT PH.PostId, MAX(CASE WHEN PH.PostHistoryTypeId IN (10, 11) THEN PH.CreationDate ELSE NULL END) AS LastClosedDate, COUNT(CASE WHEN PH.PostHistoryTypeId = 9 THEN 1
//        ELSE NULL END) AS TotalRollbackTags FROM PostHistory PH GROUP BY PH.PostId), ActivePosts AS (SELECT P.Id, P.Title, P.OwnerUserId, P.ViewCount, PH.LastClosedDate,
//        PH.TotalRollbackTags, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS PostRank FROM Posts P LEFT JOIN PostHistorySummary PH ON P.Id =
//        PH.PostId WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR' AND (PH.LastClosedDate IS NULL OR PH.LastClosedDate > P.LastActivityDate))
//        SELECT U.UserId, U.DisplayName AS UserDisplayName, U.Reputation AS UserReputation, COUNT(DISTINCT A.Id) AS ActivePostCount, SUM(A.ViewCount) AS TotalViewCount,
//        AVG(U.TotalUpVotes - U.TotalDownVotes) AS AverageVoteDifference, STRING_AGG(DISTINCT A.Title, ', ' ORDER BY A.Title) AS ActivePostTitles FROM UserStats U JOIN ActivePosts
//        A ON U.UserId = A.OwnerUserId WHERE U.TotalPosts > 0 AND U.Reputation > 100 GROUP BY U.UserId, U.DisplayName, U.Reputation HAVING COUNT(DISTINCT A.Id) > 5 ORDER BY
//        TotalViewCount DESC, UserReputation DESC LIMIT 10;
//
// Rewritten (rewrites/24007.sql): the STRING_AGG gets ORDER BY its own argument. Each group row is one active post, so COUNT(DISTINCT A.Id) is the row count.
fn q24007(db: &'static So) -> String {
    let Post { creation_date, last_activity_date, owner_user, view_count, title, .. } = &db.post;
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let phs = db.post_history.group_by(post).select(post_history_type_id.and(hd)).fold(i64::MIN, |m, (t, d)| if t == 10 || t == 11 { m.max(d) } else { m });
    type P = ((Id<Post>, i64), Option<i64>);
    let ap: MatSet<Id<Post>> = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .select(Ident::<Post>::new().and(last_activity_date).and((&phs).opt()))
        .filt(|((_, l), m): P| m.map_or(true, |m| m == i64::MIN || m > l))
        .map(|x: P| x.0 .0)
        .collect();
    let owners: MatSet<Id<User>> = (&ap).select(owner_user).collect();
    let us = (&owners)
        .with((&db.user.reputation).gt(100))
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(posts_of(db).select(comments_of(db).opt()).opt()))
        .fold([0i64; 2], |a, (t, _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let aby: HashIdx<Id<User>, Id<Post>> = (&ap).select(owner_user).inv().collect();
    let g = (&owners)
        .with(&us)
        .with(posts_of(db))
        .group_by(Ident::<User>::new())
        .select((&us).and(aby.select(Ident::<Post>::new().and(view_count.opt()).and(title.opt()))))
        .buf_fold(|v| {
            let n = v.len() as i64;
            let w: Vec<i64> = v.iter().filter_map(|x| x.1 .0 .1).collect();
            let d: i64 = v.iter().map(|x| x.0[0] - x.0[1]).sum();
            let mut t: Vec<Str> = v.iter().filter_map(|x| x.1 .1).collect();
            t.sort();
            t.dedup();
            (n, if w.is_empty() { None } else { Some(w.iter().sum::<i64>()) }, d, v.len() as i64, if t.is_empty() { None } else { Some(leak(t.join(", "))) })
        });
    let v = top_n(drain((&g).filt(|x: (i64, Option<i64>, i64, i64, Option<Str>)| x.0 > 5)), |&(u, (_, w, _, _, _))| (w.is_none(), Reverse(w), Reverse(db.user.reputation.get(u).unwrap())), 10);
    rows(v.into_iter().map(|(u, (n, w, d, m, t))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), oint(w), avg(d, m), ostr(t)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.PostTypeId, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate
//        DESC) AS Rank FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'), UserReputation AS (SELECT u.Id AS UserId, u.Reputation,
//        u.Location, CASE WHEN u.Reputation IS NULL THEN 'Unknown' WHEN u.Reputation < 100 THEN 'Newbie' WHEN u.Reputation >= 100 AND u.Reputation < 1000 THEN 'Intermediate' ELSE
//        'Expert' END AS ReputationLevel FROM Users u), ClosedPosts AS (SELECT ph.PostId, COUNT(*) AS CloseCount, STRING_AGG(DISTINCT cr.Name, ', ') AS CloseReasons FROM
//        PostHistory ph JOIN CloseReasonTypes cr ON ph.Comment::int = cr.Id WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId), UserBadges AS (SELECT b.UserId, COUNT(*) AS
//        BadgeCount, MAX(b.Class) AS MaxBadgeClass FROM Badges b WHERE b.Class IN (1, 2) GROUP BY b.UserId) SELECT rp.PostId, rp.Title, ur.Location, ur.ReputationLevel,
//        COALESCE(cb.CloseCount, 0) AS TotalCloseCount, COALESCE(cb.CloseReasons, 'No close reasons') AS CloseReasons, ub.BadgeCount AS TotalBadges, CASE WHEN ur.ReputationLevel =
//        'Expert' AND COALESCE(ub.BadgeCount, 0) > 5 THEN 'Super Expert' ELSE ur.ReputationLevel END AS FinalReputationLevel, (SELECT COUNT(*) FROM Votes v WHERE v.PostId =
//        rp.PostId AND v.VoteTypeId = 2) AS Upvotes FROM RankedPosts rp LEFT JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId LEFT JOIN ClosedPosts cb ON rp.PostId = cb.PostId
//        LEFT JOIN UserBadges ub ON ur.UserId = ub.UserId WHERE rp.Rank = 1 AND rp.PostTypeId = 1 ORDER BY rp.CreationDate DESC LIMIT 100;
//
// A CreationDate tie inside the ROW_NUMBER goes to the smaller post id (the SQL leaves it open). The distinct close reasons are joined in name order.
fn q23376(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(post_type_id).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k == 1).map(|((p, _), _)| p).with(post_type_id.eq(1)).collect();
    let cr = close_reasons(db);
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cb = db
        .post_history
        .with(post_history_type_id.is_in([10, 11]))
        .group_by(post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&cr).select(&db.close_reason_type.name))
        .buf_fold(|v| (v.len() as i64, agg_distinct(v.to_vec(), ", ").unwrap()));
    let ub = db.badge.with((&db.badge.class).is_in([1, 2])).group_by(&db.badge.user).fold(0i64, |a, _| a + 1);
    let up = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2))).opt()).fold(0i64, |a, v| a + v.is_some() as i64);
    let v = drain((&tp).select(Ident::<Post>::new().and(owner_user.select(Ident::<User>::new().and((&ub).opt())).opt()).and((&cb).opt()).and(&up)));
    let v = top_n(v, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 100);
    rows(v.into_iter().map(|(_, (((p, u), c), n))| {
        let lvl = u.map(|(u, _)| {
            let r = db.user.reputation.get(u).unwrap();
            if r < 100 {
                "Newbie"
            } else if r < 1000 {
                "Intermediate"
            } else {
                "Expert"
            }
        });
        let b = u.and_then(|u| u.1);
        let (cc, cs) = c.unwrap_or((0, "No close reasons"));
        let fl = match lvl {
            Some("Expert") if b.unwrap_or(0) > 5 => Some("Super Expert"),
            l => l,
        };
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([ostr(u.and_then(|u| db.user.location.get(u.0))), ostr(lvl), V::I(cc), V::S(cs), oint(b), ostr(fl), V::I(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS
//        RowNum, AVG(v.VoteTypeId) OVER (PARTITION BY p.Id) AS AvgVoteType FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= '2024-10-01
//        12:34:56'::timestamp - INTERVAL '1 year' AND p.Score IS NOT NULL), ClosedPostStats AS (SELECT ph.PostId, COUNT(*) AS CloseCount, MAX(ph.CreationDate) AS LastCloseDate,
//        STRING_AGG(ct.Name, ', ') AS CloseReasons FROM PostHistory ph INNER JOIN CloseReasonTypes ct ON CAST(ph.Comment AS INT) = ct.Id WHERE ph.PostHistoryTypeId IN (10, 11)
//        GROUP BY ph.PostId), ActiveUsers AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostsCount, COALESCE(SUM(b.Class), 0) AS BadgeCount,
//        MAX(u.LastAccessDate) AS LastActiveDate FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 100 GROUP BY u.Id,
//        u.DisplayName) SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, COALESCE(cps.CloseCount, 0) AS CloseCount, cps.LastCloseDate,
//        COALESCE(cps.CloseReasons, 'No Reasons') AS CloseReasons, au.UserId, au.DisplayName AS AuthorName, au.PostsCount, au.BadgeCount, au.LastActiveDate, (CASE WHEN
//        rp.AvgVoteType IS NULL THEN 'No Votes' WHEN rp.AvgVoteType > 3 THEN 'Highly Voted' ELSE 'Low Voted' END) AS VoteCategory FROM RankedPosts rp LEFT JOIN ClosedPostStats cps
//        ON rp.PostId = cps.PostId JOIN ActiveUsers au ON rp.PostId IN (SELECT p.Id FROM Posts p WHERE p.OwnerUserId = au.UserId) WHERE rp.RowNum <= 5 ORDER BY rp.CreationDate
//        DESC, rp.Score DESC LIMIT 100;
//
// RowNum numbers the joined rows; a CreationDate tie between different posts goes to the smaller post id (the SQL leaves it open).
// `rp.PostId IN (posts of au.UserId)` is the post's owner. The STRING_AGG order is left open; the port joins in history id order.
fn q23780(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, score, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let w = recent()
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(creation_date).and(votes_of(db).opt()))
        .window(row_number, |((p, d), v)| (Reverse(d), p, v), asc);
    let avt = recent().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold((0i64, 0i64), |(s, n), t| (s + t, n + 1));
    let cr = close_reasons(db);
    let PostHistory { post, post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let cps = db
        .post_history
        .with(post_history_type_id.is_in([10, 11]))
        .group_by(post)
        .select(hd.and(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&cr).select(&db.close_reason_type.name)))
        .buf_fold(|v| (v.len() as i64, v.iter().map(|x| x.0).max().unwrap(), leak(v.iter().map(|x| x.1).collect::<Vec<_>>().join(", "))));
    let au = db
        .user
        .with((&db.user.reputation).gt(100))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).and(badges_of(db).select(&db.badge.class).opt()))
        .fold(0i64, |a, (_, c)| a + c.unwrap_or(0));
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |a, _| a + 1);
    let v = drain(
        (&w)
            .filt(|(_, k)| k <= 5)
            .map(|(((p, _), _), _)| p)
            .select(Ident::<Post>::new().and((&avt).opt()).and((&cps).opt()).and(owner_user.select(Ident::<User>::new().and(&au).and(&pc)))),
    );
    let v = top_n(v, |&(_, (((p, _), _), _))| (Reverse(creation_date.get(p).unwrap()), Reverse(score.get(p).unwrap())), 100);
    rows(v.into_iter().map(|(_, (((p, a), c), ((u, b), n)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend(match c {
            Some((n, d, s)) => [V::I(n), V::T(d), V::S(s)],
            None => [V::I(0), V::Null, V::S("No Reasons")],
        });
        f.extend(ucols(db, u, &["uid", "name"]));
        f.extend([V::I(n), V::I(b), user_col(db, u, "last_access")]);
        f.push(V::S(match a {
            None => "No Votes",
            Some((s, n)) if s > 3 * n => "Highly Voted",
            _ => "Low Voted",
        }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS Author, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY
//        p.Score DESC, p.CreationDate DESC) AS Rank, COUNT(*) OVER (PARTITION BY p.PostTypeId) AS TotalPosts FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate
//        >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'), PostTags AS (SELECT p.Id AS PostId, unnest(string_to_array(p.Tags, '><')) AS TagName FROM Posts p WHERE
//        p.PostTypeId = 1), PostVotes AS (SELECT v.PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes FROM
//        Votes v GROUP BY v.PostId), RecentEdits AS (SELECT ph.PostId, COUNT(*) AS EditCount, MAX(ph.CreationDate) AS LastEdit FROM PostHistory ph WHERE ph.PostHistoryTypeId IN
//        (4, 5, 6) GROUP BY ph.PostId), PostsSummary AS (SELECT rp.PostId, rp.Title, rp.Author, rp.CreationDate, rp.Score, rp.ViewCount, pt.TagName, pv.UpVotes, pv.DownVotes,
//        re.EditCount, re.LastEdit, rp.Rank, rp.TotalPosts FROM RankedPosts rp LEFT JOIN PostTags pt ON rp.PostId = pt.PostId LEFT JOIN PostVotes pv ON rp.PostId = pv.PostId LEFT
//        JOIN RecentEdits re ON rp.PostId = re.PostId WHERE rp.Rank <= 5) SELECT DISTINCT ps.PostId, ps.Title, ps.Author, ps.CreationDate, ps.Score, ps.ViewCount, ps.TagName,
//        COALESCE(ps.UpVotes, 0) AS UpVotes, COALESCE(ps.DownVotes, 0) AS DownVotes, COALESCE(ps.EditCount, 0) AS EditCount, ps.LastEdit, ps.Rank, ps.TotalPosts FROM PostsSummary
//        ps ORDER BY ps.CreationDate DESC, ps.Score DESC;
//
// A tie inside the ROW_NUMBER goes to the smaller post id (the SQL leaves it open).
fn q33051(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, score, tags_str, .. } = &db.post;
    let base = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).group_by(post_type_id);
    let tot = base().fold(0i64, |a, _| a + 1);
    let w = base()
        .select(Ident::<Post>::new().and(post_type_id).and(score).and(creation_date))
        .window(row_number, |(((p, _), s), d)| (Reverse(s), Reverse(d), p), asc);
    type R = ((Id<Post>, i64), i64);
    let rv: MatSet<R> = (&w).filt(|(_, k)| k <= 5).map(|(((x, _), _), k)| (x, k)).collect();
    let pv = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let re = db.post_history.with(post_history_type_id.is_in([4, 5, 6])).group_by(post).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let q = || Ident::<Post>::new().with(post_type_id.eq(1)).select(tags_str.flat_map(|t: Str| t.split("><")));
    let k = || Same::<R>::new().map(|x: R| x.0 .0);
    let ds: MatSet<(R, Option<Str>)> = (&rv).select(Same::<R>::new().and(k().select(q()).opt())).collect();
    type X = (R, Option<Str>);
    let m = || Same::<X>::new().map(|x: X| x.0 .0 .0);
    let v = drain((&ds).select(Same::<X>::new().and(m().select((&pv).opt())).and(m().select((&re).opt())).and(Same::<X>::new().map(|x: X| x.0 .0 .1).select(&tot))));
    rows(v.into_iter().map(|(_, ((((((p, _), r), t), a), e), n))| {
        let a = a.unwrap_or([0; 2]);
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score", "views"]);
        f.extend([ostr(t), V::I(a[0]), V::I(a[1]), V::I(e.map_or(0, |e| e.0)), e.map_or(V::Null, |e| V::T(e.1)), V::I(r), V::I(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS
//        RowNum FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'), UserActivity AS (SELECT u.Id AS UserId,
//        COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT b.Id) AS BadgeCount, MAX(u.Reputation) AS Reputation FROM Users u LEFT JOIN
//        Votes v ON u.Id = v.UserId LEFT JOIN Comments c ON u.Id = c.UserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id), ClosedPosts AS (SELECT ph.PostId, p.Title,
//        COUNT(*) AS CloseVoteCount, MAX(ph.CreationDate) AS LastCloseDate FROM PostHistory ph JOIN Posts p ON ph.PostId = p.Id WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId,
//        p.Title), CombinedStats AS (SELECT p.PostId, p.Title, p.Score, u.UserId, u.Reputation, u.TotalBounty, u.CommentCount, COALESCE(cp.CloseVoteCount, 0) AS CloseVoteCount
//        FROM RankedPosts p JOIN UserActivity u ON p.PostId = u.UserId LEFT JOIN ClosedPosts cp ON p.PostId = cp.PostId WHERE p.RowNum <= 10) SELECT cs.PostId, cs.Title, cs.Score,
//        cs.Reputation, cs.TotalBounty, cs.CommentCount, cs.CloseVoteCount, CASE WHEN cs.CloseVoteCount > 0 THEN 'Closed' ELSE 'Active' END AS PostStatus, CASE WHEN cs.Score IS
//        NULL THEN 'No Score Available' ELSE (SELECT STRING_AGG(CAST(t.TagName AS VARCHAR), ', ') FROM Posts p JOIN Tags t ON t.ExcerptPostId = p.Id WHERE p.Id = cs.PostId) END AS
//        AssociatedTags FROM CombinedStats cs ORDER BY cs.Score DESC, cs.Reputation DESC;
//
// `p.PostId = u.UserId` joins a post id to a user id, as written. A tie inside the ROW_NUMBER goes to the smaller post id.
// The STRING_AGG order is left open; the port joins in tag id order.
fn q20527(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, origid, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((p, s), w)| (Reverse(s), w.is_none(), Reverse(w), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 10).map(|(((p, _), _), _)| p).collect();
    let uo = users_by_origid(db);
    let mu: MatSet<Id<User>> = (&tp).select(origid).select(&uo).collect();
    let ua = (&mu)
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select((&db.vote.bounty_amount).opt()).opt().and(comments_by(db).opt()).and(badges_of(db).opt()))
        .fold((0i64, 0i64), |(b, c), ((v, cm), _)| (b + v.flatten().unwrap_or(0), c + cm.is_some() as i64));
    let PostHistory { post, post_history_type_id, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.eq(10)).group_by(post).fold(0i64, |a, _| a + 1);
    let ex: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    let at = (&tp).group_by(Ident::<Post>::new()).select(ex.select(&db.tag.tag_name)).buf_fold(|v| leak(v.join(", ")));
    let v = drain((&tp).select(Ident::<Post>::new().and(origid.select(&uo).select(Ident::<User>::new().and(&ua))).and((&cp).opt()).and((&at).opt())));
    rows(v.into_iter().map(|(_, (((p, (u, (b, c))), n), t))| {
        let n = n.unwrap_or(0);
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.extend([user_col(db, u, "rep"), V::I(b), V::I(c), V::I(n), V::S(if n > 0 { "Closed" } else { "Active" }), ostr(t)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS
//        PostRank FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'), UserReputation AS (SELECT u.Id AS UserId, u.Reputation,
//        COALESCE(AVG(CASE WHEN p.ViewCount IS NOT NULL THEN p.ViewCount ELSE 0 END), 0) AS AvgViewCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id,
//        u.Reputation), PostHistoryDetails AS (SELECT ph.PostId, ph.PostHistoryTypeId, ph.CreationDate AS HistoryDate, ph.UserId AS EditorUserId, ROW_NUMBER() OVER (PARTITION BY
//        ph.PostId ORDER BY ph.CreationDate DESC) AS HistoryRank FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11, 12)), FilteredPostHistory AS (SELECT ph.PostId,
//        STRING_AGG(DISTINCT pt.Name, ', ') AS PostHistoryTypes, MAX(ph.HistoryDate) AS LastActionDate FROM PostHistoryDetails ph INNER JOIN PostHistoryTypes pt ON pt.Id =
//        ph.PostHistoryTypeId GROUP BY ph.PostId) SELECT u.DisplayName, u.Reputation, up.AvgViewCount, rp.PostId, rp.Title, rp.CreationDate, COALESCE(fph.PostHistoryTypes, 'No
//        Actions') AS PostHistorySummary, fph.LastActionDate, CASE WHEN rp.PostRank > 1 THEN 'Multiple Posts' ELSE 'Single Post' END AS PostMultiplicity FROM RankedPosts rp JOIN
//        Users u ON rp.OwnerUserId = u.Id LEFT JOIN UserReputation up ON u.Id = up.UserId LEFT JOIN FilteredPostHistory fph ON rp.PostId = fph.PostId WHERE u.Reputation > (SELECT
//        AVG(Reputation) FROM Users) AND ((rp.Title ILIKE '%SQL%' OR rp.Title ILIKE '%database%') OR fph.PostHistoryTypes IS NOT NULL) ORDER BY up.AvgViewCount DESC,
//        rp.CreationDate DESC LIMIT 100 OFFSET 0;
//
// A CreationDate tie inside the ROW_NUMBER goes to the smaller post id (the SQL leaves it open). The distinct type names are joined in name order.
fn q21679(db: &'static So) -> String {
    let Post { creation_date, owner_user, title, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(owner_user).and(creation_date))
        .window(row_number, |((p, _), d)| (Reverse(d), p), asc);
    let (rs, rn) = db.user.select(&db.user.reputation).fold_flat((0i64, 0i64), |(s, n), r| (s + r, n + 1));
    let avg_rep = rs as f64 / rn as f64;
    let up = user_posts(db);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let fph = db
        .post_history
        .with(post_history_type_id.is_in([10, 11, 12]))
        .group_by(post)
        .select(hd.and(htype_name(db)))
        .buf_fold(|v| (agg_distinct(v.iter().map(|x| x.1).collect(), ", ").unwrap(), v.iter().map(|x| x.0).max().unwrap()));
    type R = ((Id<Post>, Id<User>), i64);
    type X = (((R, [i64; 10]), Option<Option<Str>>), Option<(Str, i64)>);
    let rv: MatSet<R> = (&w).map(|((x, _), k)| (x, k)).collect();
    let k = || Same::<R>::new().map(|x: R| x.0 .0);
    let ilike = |t: Str| {
        let t = t.to_lowercase();
        t.contains("sql") || t.contains("database")
    };
    let v = drain(
        (&rv)
            .select(Same::<R>::new().with(Same::<R>::new().map(|x: R| x.0 .1).select(Ident::<User>::new().with((&db.user.reputation).filt(|r| r as f64 > avg_rep)))))
            .select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0 .1).select(&up)).and(k().select(title.opt()).opt()).and(k().select((&fph).opt())))
            .filt(|(((_, _), t), f): X| t.flatten().map_or(false, ilike) || f.is_some()),
    );
    let v = top_n(v, |&(_, (((x, a), _), _))| (Reverse(fkey(a[6] as f64 / a[0] as f64)), Reverse(creation_date.get(x.0 .0).unwrap()), x.0 .0), 100);
    rows(v.into_iter().map(|(_, (((((p, u), n), a), _), f))| {
        let mut f2 = ucols(db, u, &["name", "rep"]);
        f2.push(avg(a[6], a[0]));
        f2.extend(post_fields(db, p, &["id", "title", "created"]));
        f2.extend(match f {
            Some((s, d)) => [V::S(s), V::T(d)],
            None => [V::S("No Actions"), V::Null],
        });
        f2.push(V::S(if n > 1 { "Multiple Posts" } else { "Single Post" }));
        row(f2)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.Views, U.UpVotes, U.DownVotes, ROW_NUMBER() OVER (PARTITION BY U.Id ORDER BY
//        U.LastAccessDate DESC) AS AccessRank FROM Users U WHERE U.Reputation > 100), PostStats AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.OwnerUserId, COUNT(CASE WHEN
//        C.Id IS NOT NULL THEN 1 END) AS CommentCount, COALESCE(SUM(V.BountyAmount), 0) AS TotalBounty, P.Score, DENSE_RANK() OVER (ORDER BY P.CreationDate DESC) AS CreationRank
//        FROM Posts P LEFT JOIN Comments C ON C.PostId = P.Id LEFT JOIN Votes V ON V.PostId = P.Id AND V.VoteTypeId IN (8, 9) WHERE P.CreationDate >= TIMESTAMP '2024-10-01
//        12:34:56' - INTERVAL '6 months' GROUP BY P.Id, P.Title, P.CreationDate, P.OwnerUserId, P.Score), PostHistoryAggregates AS (SELECT PH.PostId, ARRAY_AGG(DISTINCT PHT.Name)
//        AS HistoryTypes, COUNT(PH.Id) AS HistoryCount, MAX(PH.CreationDate) AS LastEditDate FROM PostHistory PH JOIN PostHistoryTypes PHT ON PH.PostHistoryTypeId = PHT.Id GROUP
//        BY PH.PostId) SELECT UReputation.DisplayName, UReputation.Reputation, UReputation.Views, UReputation.UpVotes, UReputation.DownVotes, PS.Title, PS.CreationDate,
//        PH.HistoryCount, COALESCE(PS.CommentCount, 0) AS TotalComments, PS.TotalBounty, PH.LastEditDate, CASE WHEN PS.Score > 0 THEN 'Active' ELSE 'Inactive' END AS PostActivity,
//        CASE WHEN UReputation.Reputation IS NULL THEN 'No Reputation' ELSE 'Reputed' END AS UserReputationStatus FROM UserReputation UReputation FULL OUTER JOIN PostStats PS ON
//        UReputation.UserId = PS.OwnerUserId JOIN PostHistoryAggregates PH ON PS.PostId = PH.PostId WHERE (UReputation.Reputation IS NOT NULL OR PS.Score > 0) AND (PH.HistoryCount
//        > 5 OR PH.HistoryCount IS NULL) ORDER BY UReputation.Reputation DESC NULLS LAST, PS.CreationRank ASC, PS.Title;
//
// The FULL OUTER JOIN is followed by an inner join on PS.PostId, so only the PostStats side survives: posts LEFT JOIN users.
fn q23255(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let ps = db
        .post
        .with(creation_date.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -6)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 2], |a, (c, b)| [a[0] + c.is_some() as i64, a[1] + b.flatten().unwrap_or(0)]);
    let ph = db.post_history.group_by(&db.post_history.post).select(&db.post_history.creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    type X = ((([i64; 2], Option<Id<User>>), (i64, i64)), i64);
    let v = drain(
        (&ps)
            .and(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(100))).opt())
            .and((&ph).filt(|x: (i64, i64)| x.0 > 5))
            .and(score)
            .filt(|(((_, u), _), s): X| u.is_some() || s > 0),
    );
    rows(v.into_iter().map(|(p, (((a, u), (n, d)), _))| {
        let mut f = match u {
            Some(u) => ucols(db, u, &["name", "rep", "uviews", "uup", "udown"]),
            None => vec![V::Null, V::Null, V::Null, V::Null, V::Null],
        };
        f.extend(post_fields(db, p, &["title", "created"]));
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::T(d)]);
        f.push(V::S(if score.get(p).unwrap() > 0 { "Active" } else { "Inactive" }));
        f.push(V::S(if u.is_some() { "Reputed" } else { "No Reputation" }));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id AS UserId, Reputation, RANK() OVER (ORDER BY Reputation DESC) AS UserRank FROM Users), RecentVotes AS (SELECT PostId, VoteTypeId,
//        COUNT(*) AS VoteCount, SUM(CASE WHEN VoteTypeId IN (2, 4) THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes WHERE
//        CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month' GROUP BY PostId, VoteTypeId), PostSummary AS (SELECT p.Id AS PostId, p.Title,
//        COALESCE(ARRAY_AGG(DISTINCT t.TagName) FILTER (WHERE t.TagName IS NOT NULL), ARRAY[]::VARCHAR[]) AS Tags, SUM(rv.UpVotes) AS TotalUpVotes, COUNT(c.Id) AS CommentCount,
//        COALESCE(MAX(b.Class), 0) AS HighestBadgeClass FROM Posts p LEFT JOIN PostLinks pl ON p.Id = pl.PostId LEFT JOIN Tags t ON pl.RelatedPostId = t.Id LEFT JOIN RecentVotes
//        rv ON p.Id = rv.PostId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges b ON p.OwnerUserId = b.UserId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' -
//        INTERVAL '1 year' GROUP BY p.Id, p.Title), ClosedPosts AS (SELECT ph.PostId, ph.CreationDate, ph.Comment, MAX(ph.UserDisplayName) FILTER (WHERE ph.PostHistoryTypeId IN
//        (10, 11)) AS UserDisplayName FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId, ph.CreationDate, ph.Comment) SELECT ps.PostId, ps.Title,
//        ps.Tags, ps.TotalUpVotes, ps.CommentCount, COALESCE(cr.UserDisplayName, 'No action') AS ClosedBy, RANK() OVER (PARTITION BY ps.HighestBadgeClass ORDER BY ps.TotalUpVotes
//        DESC) AS RankWithinBadgeClass, COALESCE(u.Reputation, 0) AS UserReputation FROM PostSummary ps LEFT JOIN ClosedPosts cr ON ps.PostId = cr.PostId LEFT JOIN UserReputation
//        u ON ps.HighestBadgeClass = u.UserRank WHERE ps.TotalUpVotes > 0 AND ps.CommentCount > 0 AND ps.HighestBadgeClass BETWEEN 1 AND 3 ORDER BY ps.TotalUpVotes DESC,
//        ps.CommentCount DESC, ps.PostId;
//
// `pl.RelatedPostId = t.Id` joins a post id to a tag id, as written. The ARRAY_AGG(DISTINCT ...) is listed in name order, and a closure's
// display name is the MAX over its group.
fn q24021(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let tag_o: HashIdx<i64, Id<Tag>> = (&db.tag.origid).inv().collect();
    let Vote { vote_type_id, creation_date: vd, post: vp, .. } = &db.vote;
    let rvf = db.vote.with(vd.ge(add_months(t0, -1))).group_by(vp.and(vote_type_id)).select(vote_type_id).fold(0i64, |a, t| a + (t == 2 || t == 4) as i64);
    type RV = ((Id<Post>, i64), i64);
    let rvv = rel(drain(&rvf));
    let rvi: HashIdx<Id<Post>, RV> = (&rvv).map(|x: RV| x.0 .0).inv().select(&rvv).collect();
    let recent = || db.post.with(creation_date.ge(add_years(t0, -1)));
    let rel_tag = || (&db.post_link.related_post_id).select(&tag_o);
    let ps = recent()
        .group_by(Ident::<Post>::new())
        .select(links_of(db).select(rel_tag().opt()).opt().and((&rvi).opt()).and(comments_of(db).opt()).and(owner_user.select(badges_of(db).select(&db.badge.class)).opt()))
        .fold((0i64, 0i64, 0i64), |(s, c, m), (((_, r), cm), b)| (s + r.map_or(0, |r| r.1), c + cm.is_some() as i64, m.max(b.unwrap_or(0))));
    let tg = recent().group_by(Ident::<Post>::new()).select(links_of(db).select(rel_tag().select(&db.tag.tag_name))).buf_fold(|v| {
        let mut t = v.to_vec();
        t.sort();
        t.dedup();
        &*Box::leak(t.into_boxed_slice())
    });
    let PostHistory { post, post_history_type_id, comment, creation_date: hd, user_display_name, .. } = &db.post_history;
    let cr = db
        .post_history
        .with(post_history_type_id.is_in([10, 11]))
        .group_by(post.and(hd).and(comment.opt()))
        .select(user_display_name.opt())
        .fold(None::<Str>, |m, n| match (m, n) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (a, b) => a.or(b),
        });
    type C = (((Id<Post>, i64), Option<Str>), Option<Str>);
    let crv = rel(drain(&cr));
    let cri: HashIdx<Id<Post>, C> = (&crv).map(|x: C| x.0 .0 .0).inv().select(&crv).collect();
    let uw = whole(&db.user.id).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    type U = ((Id<User>, i64), i64);
    let uri = by_first(&(&uw).map(|x: U| (x.1, x)).collect());
    type P = (i64, i64, i64);
    type Y = ((((Id<Post>, P), Option<&'static [Str]>), Option<C>), Option<U>);
    let w = db
        .post
        .with((&ps).filt(|(s, c, m): P| s > 0 && c > 0 && (1..=3).contains(&m)))
        .select(Ident::<Post>::new().and(&ps).and((&tg).opt()).and((&cri).opt()).and((&ps).map(|x: P| x.2).select(&uri).opt()))
        .group_by(Same::<Y>::new().map(|y: Y| y.0 .0 .0 .1 .2))
        .select(Same::<Y>::new())
        .window(rank, |y: Y| Reverse(y.0 .0 .0 .1 .0), asc);
    let v = drain(&w);
    rows(v.into_iter().map(|(_, (((((p, (s, c, _)), t), cl), u), r))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.push(V::L(t.unwrap_or(&[]).iter().map(|s| V::S(s)).collect()));
        f.extend([V::I(s), V::I(c), V::S(cl.and_then(|c| c.1).unwrap_or("No action")), V::I(r), V::I(u.map_or(0, |u| u.0 .1))]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN
//        V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(DISTINCT P.Id) AS PostCount, COUNT(DISTINCT C.Id) AS CommentCount, DENSE_RANK() OVER (ORDER BY
//        COALESCE(U.Reputation, 0) DESC) AS Rank FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id =
//        V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation), PostHistoryDetails AS (SELECT PH.PostId, MIN(PH.CreationDate) AS FirstEntityChange, MAX(PH.CreationDate) AS
//        LastEntityChange, COUNT(CASE WHEN PH.PostHistoryTypeId IN (10, 11) THEN 1 END) AS CloseReopenCount, STRING_AGG(DISTINCT PH.UserDisplayName, ', ') AS UsersInvolved,
//        COUNT(*) AS HistoryCount FROM PostHistory PH WHERE PH.CreationDate BETWEEN DATE '2024-10-01' - INTERVAL '1 year' AND DATE '2024-10-01' GROUP BY PH.PostId) SELECT
//        U.UserId, U.DisplayName, U.Reputation, U.UpVotes, U.DownVotes, PHD.PostId, PHD.FirstEntityChange, PHD.LastEntityChange, PHD.CloseReopenCount, PHD.UsersInvolved,
//        PHD.HistoryCount, CASE WHEN PHD.HistoryCount > 5 THEN 'Active Contributor' WHEN PHD.HistoryCount IS NULL THEN 'No Activity' ELSE 'Regular' END AS ActivityLevel, CASE WHEN
//        U.Reputation IS NULL THEN 'No Reputation Data' ELSE CASE WHEN U.Reputation > 1000 THEN 'High Reputation' WHEN U.Reputation BETWEEN 100 AND 1000 THEN 'Medium Reputation'
//        ELSE 'Low Reputation' END END AS ReputationTier FROM UserStats U LEFT JOIN PostHistoryDetails PHD ON U.UserId = PHD.PostId WHERE U.Reputation IS NOT NULL ORDER BY
//        U.Reputation DESC, U.UserId ASC FETCH FIRST 100 ROWS ONLY;
//
// Only the top 100 users by (Reputation, Id) can be returned and those are base columns, so they are picked first.
// `U.UserId = PHD.PostId` joins a user id to a post id, as written. The distinct display names are joined in sorted order.
fn q22611(db: &'static So) -> String {
    let top = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 100);
    let tu: MatSet<Id<User>> = rel(top).map(|x: (Id<User>, i64)| x.0).collect();
    let us = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 2], |a, x| {
            let t = x.and_then(|x| x.1);
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
        });
    let PostHistory { post, post_history_type_id, creation_date: hd, user_display_name, .. } = &db.post_history;
    let phd = db
        .post_history
        .with(hd.filt(|d| d >= add_years(date(2024, 10, 1), -1) && d <= date(2024, 10, 1)))
        .group_by(post)
        .select(hd.and(post_history_type_id).and(user_display_name.opt()))
        .buf_fold(|v| {
            let lo = v.iter().map(|x| x.0 .0).min().unwrap();
            let hi = v.iter().map(|x| x.0 .0).max().unwrap();
            let cr = v.iter().filter(|x| x.0 .1 == 10 || x.0 .1 == 11).count() as i64;
            (lo, hi, cr, agg_distinct(v.iter().filter_map(|x| x.1).collect(), ", "), v.len() as i64)
        });
    let po: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let v = drain((&tu).select(Ident::<User>::new().and(&us).and((&db.user.origid).select(&po).select(Ident::<Post>::new().and(&phd)).opt())));
    let v = top_n(v, |&(u, _)| (Reverse(db.user.reputation.get(u).unwrap()), u), 100);
    rows(v.into_iter().map(|(_, ((u, a), h))| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        match h {
            Some((p, (lo, hi, cr, un, n))) => {
                f.extend(post_fields(db, p, &["id"]));
                f.extend([V::T(lo), V::T(hi), V::I(cr), ostr(un), V::I(n)]);
                f.push(V::S(if n > 5 { "Active Contributor" } else { "Regular" }));
            }
            None => {
                f.extend([V::Null, V::Null, V::Null, V::Null, V::Null, V::Null, V::S("No Activity")]);
            }
        }
        f.push(V::S(if r > 1000 {
            "High Reputation"
        } else if (100..=1000).contains(&r) {
            "Medium Reputation"
        } else {
            "Low Reputation"
        }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, COALESCE(v.upVotes, 0) AS UpVotes, COALESCE(v.downVotes, 0) AS DownVotes, p.CreationDate, ROW_NUMBER()
//        OVER (PARTITION BY pt.Name ORDER BY p.CreationDate DESC) AS RowNum, pt.Name AS PostTypeName FROM Posts p LEFT JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN (SELECT
//        PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS upVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS downVotes FROM Votes GROUP BY PostId) v ON p.Id =
//        v.PostId WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'), RecentClosedPosts AS (SELECT p.Id AS PostId, MAX(ph.CreationDate) AS LastClosedDate,
//        STRING_AGG(DISTINCT ctr.Name, ', ') AS CloseReasons FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId JOIN CloseReasonTypes ctr ON ph.Comment::INTEGER = ctr.Id WHERE
//        ph.PostHistoryTypeId = 10 GROUP BY p.Id), PostMeta AS (SELECT r.PostId, r.Title, r.ViewCount, r.UpVotes, r.DownVotes, rc.LastClosedDate, rc.CloseReasons, CASE WHEN
//        (r.UpVotes - r.DownVotes) > 0 THEN 'Positive' WHEN (r.UpVotes - r.DownVotes) < 0 THEN 'Negative' ELSE 'Neutral' END AS Sentiment FROM RankedPosts r LEFT JOIN
//        RecentClosedPosts rc ON r.PostId = rc.PostId WHERE r.RowNum = 1) SELECT pm.PostId, pm.Title, pm.ViewCount, pm.UpVotes, pm.DownVotes, pm.LastClosedDate, pm.CloseReasons,
//        pm.Sentiment, CASE WHEN pm.UpVotes > 5 THEN 'Popular' ELSE 'Regular' END AS Popularity, CASE WHEN pm.LastClosedDate IS NOT NULL AND pm.Sentiment = 'Negative' THEN
//        'Revise' WHEN pm.LastClosedDate IS NULL AND pm.Sentiment = 'Positive' THEN 'Promote' ELSE 'Evaluate' END AS ActionRecommended FROM PostMeta pm WHERE pm.ViewCount >
//        (SELECT AVG(ViewCount) FROM Posts) ORDER BY pm.ViewCount DESC;
//
// A CreationDate tie inside the ROW_NUMBER goes to the smaller post id (the SQL leaves it open). The distinct close reasons are joined in name order.
fn q24538(db: &'static So) -> String {
    let Post { creation_date, view_count, .. } = &db.post;
    let w = db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(ptype_name(db)).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k == 1).map(|((p, _), _)| p).collect();
    let (ws, wn) = db.post.select(view_count).fold_flat((0i64, 0i64), |(s, n), w| (s + w, n + 1));
    let aw = ws as f64 / wn as f64;
    let vs = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let cr = close_reasons(db);
    let PostHistory { post, post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let rc = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(post)
        .select(hd.and(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&cr).select(&db.close_reason_type.name)))
        .buf_fold(|v| (v.iter().map(|x| x.0).max().unwrap(), agg_distinct(v.iter().map(|x| x.1).collect(), ", ").unwrap()));
    let v = drain((&tp).with(view_count.filt(|w| w as f64 > aw)).select(Ident::<Post>::new().and((&vs).opt()).and((&rc).opt())));
    rows(v.into_iter().map(|(_, ((p, a), c))| {
        let a = a.unwrap_or([0; 2]);
        let s = if a[0] - a[1] > 0 {
            "Positive"
        } else if a[0] - a[1] < 0 {
            "Negative"
        } else {
            "Neutral"
        };
        let act = if c.is_some() && s == "Negative" {
            "Revise"
        } else if c.is_none() && s == "Positive" {
            "Promote"
        } else {
            "Evaluate"
        };
        let mut f = post_fields(db, p, &["id", "title", "views"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(match c {
            Some((d, r)) => [V::T(d), V::S(r)],
            None => [V::Null, V::Null],
        });
        f.extend([V::S(s), V::S(if a[0] > 5 { "Popular" } else { "Regular" }), V::S(act)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS rn FROM
//        Posts p WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year'), PostWithComments AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, COALESCE(rc.CommentCount,
//        0) AS CommentCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBountyAmount FROM RankedPosts rp LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY
//        PostId) rc ON rp.PostId = rc.PostId LEFT JOIN Votes v ON v.PostId = rp.PostId AND v.VoteTypeId IN (9, 8) GROUP BY rp.PostId, rp.Title, rp.CreationDate, rp.Score,
//        rc.CommentCount), TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(p.Score) AS TotalScore FROM Users u INNER JOIN Posts p ON p.OwnerUserId = u.Id WHERE
//        p.CreationDate >= CURRENT_DATE - INTERVAL '1 year' GROUP BY u.Id, u.DisplayName), PostHistorySummary AS (SELECT ph.PostId, ph.PostHistoryTypeId, ph.UserDisplayName,
//        ph.CreationDate, COUNT(*) AS EditCount, STRING_AGG(DISTINCT CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 'Closed' ELSE 'Edited' END, ', ') AS EditTypes FROM
//        PostHistory ph WHERE ph.CreationDate >= CURRENT_DATE - INTERVAL '1 year' GROUP BY ph.PostId, ph.PostHistoryTypeId, ph.UserDisplayName, ph.CreationDate) SELECT p.Title,
//        p.CreationDate, p.Score, p.CommentCount, p.TotalBountyAmount, u.DisplayName AS TopUser, u.TotalScore, phs.EditCount, phs.EditTypes FROM PostWithComments p JOIN TopUsers u
//        ON p.Score = u.TotalScore LEFT JOIN PostHistorySummary phs ON p.PostId = phs.PostId WHERE p.Score > 100 AND p.CommentCount > 5 AND u.TotalScore IS NOT NULL ORDER BY
//        p.Score DESC, p.CommentCount DESC;
//
// `p.Score = u.TotalScore` joins a post's score to a user's total, as written.
fn q32544(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let cut = add_years(current_date(), -1);
    let recent = || db.post.with(creation_date.ge(cut));
    let tu = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(Ident::<Post>::new().with(creation_date.ge(cut))).select(score)).fold(0i64, |a, s| a + s);
    let tus: HashIdx<i64, Id<User>> = (&tu).inv().collect();
    let pb = recent()
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt()).opt())
        .fold(0i64, |a, b| a + b.flatten().unwrap_or(0));
    let cc = comments_per_post(db);
    let PostHistory { post, post_history_type_id, user_display_name, creation_date: hd, .. } = &db.post_history;
    let phs = db.post_history.with(hd.ge(cut)).group_by(post.and(post_history_type_id).and(user_display_name.opt()).and(hd)).fold(0i64, |a, _| a + 1);
    type H = ((((Id<Post>, i64), Option<Str>), i64), i64);
    let hv = rel(drain(&phs));
    let hi: HashIdx<Id<Post>, H> = (&hv).map(|x: H| x.0 .0 .0 .0).inv().select(&hv).collect();
    let v = drain(recent().with(score.gt(100)).with((&cc).filt(|n| n > 5)).select(Ident::<Post>::new().and(&cc).and(&pb).and(score.select(&tus)).and((&hi).opt())));
    rows(v.into_iter().map(|(_, ((((p, c), b), u), h))| {
        let mut f = post_fields(db, p, &["title", "created", "score"]);
        f.extend([V::I(c), V::I(b), user_col(db, u, "name"), V::I(score.get(p).unwrap())]);
        f.extend(match h {
            Some(((((_, t), _), _), n)) => [V::I(n), V::S(if t == 10 || t == 11 { "Closed" } else { "Edited" })],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COALESCE(v.VoteCount, 0) AS Score, COALESCE(c.CommentCount, 0) AS Comments, COALESCE(b.BadgeCount, 0)
//        AS BadgeCount, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY COALESCE(v.VoteCount, 0) DESC, COALESCE(c.CommentCount, 0) DESC) AS Rank FROM Posts p LEFT JOIN (SELECT
//        PostId, COUNT(*) AS VoteCount FROM Votes GROUP BY PostId) v ON p.Id = v.PostId LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id
//        = c.PostId LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) b ON p.OwnerUserId = b.UserId), PostHistoryDetail AS (SELECT ph.PostId,
//        ph.PostHistoryTypeId, pht.Name AS HistoryType, ph.CreationDate FROM PostHistory ph JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id WHERE ph.CreationDate >=
//        TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND ph.Comment IS NOT NULL), FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.Comments,
//        rp.BadgeCount, rp.Rank, COUNT(pd.PostId) AS RecentHistoryCount, STRING_AGG(DISTINCT pd.HistoryType, ', ' ORDER BY pd.HistoryType) AS RecentHistoryTypes FROM RankedPosts
//        rp LEFT JOIN PostHistoryDetail pd ON rp.PostId = pd.PostId GROUP BY rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.Comments, rp.BadgeCount, rp.Rank) SELECT fp.PostId,
//        fp.Title, fp.CreationDate, fp.Score, fp.Comments, fp.BadgeCount, fp.Rank, CASE WHEN fp.RecentHistoryCount = 0 THEN 'No Recent Edits' ELSE fp.RecentHistoryTypes END AS
//        Edits_Info FROM FilteredPosts fp WHERE fp.Rank <= 5 AND (fp.Score > 0 OR fp.Comments > 0) ORDER BY fp.Score DESC, fp.Comments DESC;
//
// Rewritten (rewrites/22898.sql): the STRING_AGG gets ORDER BY its own argument.
fn q22898(db: &'static So) -> String {
    let Post { post_type_id, owner_user, .. } = &db.post;
    let vc = votes_per_post(db);
    let cc = comments_per_post(db);
    let bc = badges_per_user(db);
    let w = db
        .post
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(post_type_id.and(&vc).and(&cc)))
        .window(rank, |(_, ((_, v), c))| (Reverse(v), Reverse(c)), asc);
    type R = ((Id<Post>, ((i64, i64), i64)), i64);
    let rv: MatSet<R> = (&w).filt(|(_, k)| k <= 5).map(|x: R| x).collect();
    let PostHistory { post, comment, creation_date: hd, .. } = &db.post_history;
    let pd = db
        .post_history
        .with(hd.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(comment)
        .group_by(post)
        .select(htype_name(db))
        .buf_fold(|v| agg_distinct(v.to_vec(), ", ").unwrap());
    let k = || Same::<R>::new().map(|x: R| x.0 .0);
    let v = drain(
        (&rv)
            .filt(|((_, ((_, s), c)), _): R| s > 0 || c > 0)
            .select(Same::<R>::new().and(k().select(owner_user.select(&bc)).opt()).and(k().select((&pd).opt()))),
    );
    rows(v.into_iter().map(|(_, ((((p, ((_, s), c)), r), b), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(s), V::I(c), V::I(b.unwrap_or(0)), V::I(r), V::S(h.unwrap_or("No Recent Edits"))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC)
//        AS Rank, p.OwnerUserId, COALESCE(SUBSTRING(p.Tags, 2, LENGTH(p.Tags) - 2), 'No Tags') AS CleanTags FROM Posts p WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS
//        TIMESTAMP) - INTERVAL '1 year'), UserStats AS (SELECT u.Id AS UserId, u.Reputation, u.Views, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN COALESCE(b.Class, 0) = 1 THEN 1 ELSE
//        0 END) AS GoldBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation, u.Views), PostHistoryDetails AS (SELECT ph.PostId, MAX(CASE WHEN
//        ph.PostHistoryTypeId IN (10, 11) THEN ph.CreationDate END) AS ClosedOrReopenedAt, ARRAY_AGG(DISTINCT ch.Name) AS CloseReasons FROM PostHistory ph LEFT JOIN
//        CloseReasonTypes ch ON ph.Comment = CAST(ch.Id AS varchar) GROUP BY ph.PostId), PopularPosts AS (SELECT rp.PostId, rp.Title, rp.CleanTags, us.Reputation,
//        ps.ClosedOrReopenedAt, array_length(ps.CloseReasons, 1) AS CloseReasonCount, CASE WHEN rp.ViewCount > 1000 THEN 'High Traffic' ELSE 'Normal Traffic' END AS TrafficLabel,
//        RANK() OVER (ORDER BY us.Views DESC, rp.ViewCount DESC) AS ViewRank FROM RankedPosts rp JOIN UserStats us ON rp.OwnerUserId = us.UserId LEFT JOIN PostHistoryDetails ps ON
//        rp.PostId = ps.PostId WHERE rp.Rank <= 5) SELECT pp.PostId, pp.Title, pp.CleanTags, pp.Reputation, pp.CloseReasonCount, pp.ClosedOrReopenedAt, pp.TrafficLabel, CASE WHEN
//        pp.CloseReasonCount IS NOT NULL AND pp.ClosedOrReopenedAt IS NOT NULL THEN 'Closed or Reopened' ELSE 'Active' END AS PostStatus FROM PopularPosts pp WHERE pp.ViewRank <=
//        10 ORDER BY pp.Reputation DESC, pp.CloseReasonCount ASC LIMIT 25;
//
// A tie inside the ROW_NUMBER goes to the smaller post id (the SQL leaves it open). `ARRAY_AGG(DISTINCT ch.Name)` over the LEFT JOIN keeps one NULL,
// which `array_length` counts.
fn q23889(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, owner_user, view_count, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|(((p, _), _), _)| p).collect();
    let crt: HashIdx<Str, Id<CloseReasonType>> = (&db.close_reason_type.origid).map(|i: i64| -> Str { leak(i.to_string()) }).inv().collect();
    let PostHistory { post, post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let phd = db
        .post_history
        .group_by(post)
        .select(post_history_type_id.and(hd).and(comment.select(&crt).select(&db.close_reason_type.name).opt()))
        .buf_fold(|v| {
            let m = v.iter().filter(|x| x.0 .0 == 10 || x.0 .0 == 11).map(|x| x.0 .1).max();
            let mut n: Vec<Option<Str>> = v.iter().map(|x| x.1).collect();
            n.sort();
            n.dedup();
            (m, n.len() as i64)
        });
    let vw = whole(&tp)
        .select(Ident::<Post>::new().and(owner_user).and((&phd).opt()).and(owner_user.select(&db.user.views)).and(view_count.opt()))
        .window(rank, |((_, uv), w)| (Reverse(uv), w.is_none(), Reverse(w)), asc);
    let v = drain((&vw).filt(|(_, k)| k <= 10).map(|((((x, h), _), _), _)| (x, h)));
    let v = top_n(v, |&(_, ((p, u), h))| (Reverse(db.user.reputation.get(u).unwrap()), h.is_none(), h.map(|h| h.1), p), 25);
    rows(v.into_iter().map(|(_, ((p, u), h))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.push(V::S(db.post.tags_str.get(p).map_or("No Tags", |t| &t[1..t.len() - 1])));
        f.push(user_col(db, u, "rep"));
        let (m, n) = match h {
            Some((m, n)) => (m, Some(n)),
            None => (None, None),
        };
        f.extend([oint(n), ots(m)]);
        f.push(V::S(if view_count.get(p).map_or(false, |w| w > 1000) { "High Traffic" } else { "Normal Traffic" }));
        f.push(V::S(if n.is_some() && m.is_some() { "Closed or Reopened" } else { "Active" }));
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U WHERE U.Reputation >
//        500), RecentPosts AS (SELECT P.Id AS PostId, P.OwnerUserId, P.CreationDate, P.Score, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC, P.Id) AS
//        PostRank FROM Posts P WHERE P.CreationDate > (cast('2024-10-01' as date) - INTERVAL '30 days')), ClosedPosts AS (SELECT PH.PostId, P.OwnerUserId, PH.CreationDate AS
//        CloseCreationDate, COUNT(*) AS CloseCount, STRING_AGG(DISTINCT C.UserDisplayName, ', ') AS ClosingUserNames FROM PostHistory PH JOIN Posts P ON P.Id = PH.PostId LEFT JOIN
//        Comments C ON C.PostId = PH.PostId WHERE PH.PostHistoryTypeId IN (10, 11) GROUP BY PH.PostId, P.OwnerUserId, PH.CreationDate), VotingStats AS (SELECT V.PostId, COUNT(CASE
//        WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes, COUNT(CASE WHEN V.VoteTypeId = 10 THEN 1 END) AS DeleteVotes FROM
//        Votes V GROUP BY V.PostId), FinalReport AS (SELECT RU.UserId, RU.DisplayName, RU.Reputation, COALESCE(RP.PostId, 0) AS RecentPostId, COALESCE(RP.Score, 0) AS
//        RecentPostScore, CP.CloseCount, CP.ClosingUserNames, VS.UpVotes, VS.DownVotes, (CASE WHEN CP.CloseCount > 0 THEN 'Closed Posts' ELSE 'Active Posts' END) AS PostStatus
//        FROM RankedUsers RU LEFT JOIN RecentPosts RP ON RU.UserId = RP.OwnerUserId AND RP.PostRank = 1 LEFT JOIN ClosedPosts CP ON RP.PostId = CP.PostId LEFT JOIN VotingStats VS
//        ON COALESCE(RP.PostId, 0) = VS.PostId WHERE RU.ReputationRank <= 10) SELECT UserId, DisplayName, Reputation, RecentPostId, RecentPostScore, CloseCount, ClosingUserNames,
//        UpVotes, DownVotes, PostStatus FROM FinalReport WHERE UserId IS NOT NULL ORDER BY Reputation DESC, UpVotes DESC NULLS LAST LIMIT 100;
//
// Rewritten (rewrites/24942.sql): the ROW_NUMBER gets `, P.Id`. The distinct names are joined in sorted order.
fn q24942(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let uw = whole(db.user.with((&db.user.reputation).gt(500))).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let ru: MatSet<Id<User>> = (&uw).filt(|(_, k)| k <= 10).map(|((u, _), _)| u).collect();
    let pw = db.post.with(creation_date.gt(add_days(date(2024, 10, 1), -30))).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let rpi: HashIdx<Id<User>, Id<Post>> = (&pw).filt(|(_, k)| k == 1).map(|((p, _), _)| p).collect();
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cp = db
        .post_history
        .with(post_history_type_id.is_in([10, 11]))
        .group_by(post.and(hd))
        .select(post.select(comments_of(db).select((&db.comment.user_display_name).opt())).opt())
        .buf_fold(|v| (v.len() as i64, agg_distinct(v.iter().flatten().flatten().copied().collect(), ", ")));
    type C = ((Id<Post>, i64), (i64, Option<Str>));
    let cv = rel(drain(&cp));
    let ci: HashIdx<Id<Post>, C> = (&cv).map(|x: C| x.0 .0).inv().select(&cv).collect();
    let vs = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let v = drain((&ru).select(Ident::<User>::new().and((&rpi).select(Ident::<Post>::new().and((&ci).opt()).and((&vs).opt())).opt())));
    let v = top_n(v, |&(u, (_, p))| (Reverse(db.user.reputation.get(u).unwrap()), p.and_then(|p| p.1).is_none(), Reverse(p.and_then(|p| p.1).map(|a| a[0]))), 100);
    rows(v.into_iter().map(|(_, (u, p))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        match p {
            Some(((p, c), a)) => {
                f.extend(post_fields(db, p, &["id", "score"]));
                f.extend(match c {
                    Some((_, (n, s))) => [V::I(n), ostr(s)],
                    None => [V::Null, V::Null],
                });
                f.extend(match a {
                    Some(a) => [V::I(a[0]), V::I(a[1])],
                    None => [V::Null, V::Null],
                });
                f.push(V::S(if c.map_or(false, |c| c.1 .0 > 0) { "Closed Posts" } else { "Active Posts" }));
            }
            None => f.extend([V::I(0), V::I(0), V::Null, V::Null, V::Null, V::Null, V::S("Active Posts")]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS
//        RankScore, COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= cast('2024-10-01' as date) -
//        INTERVAL '30 days' AND p.Score > 0), TopRankedPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.RankScore, COALESCE(b.UserId, -1) AS
//        BadgeHolderId, COALESCE(b.Name, 'No Badge') AS BadgeName FROM RankedPosts rp LEFT JOIN Badges b ON b.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = rp.PostId) WHERE
//        rp.RankScore <= 5), PostAnalytics AS (SELECT trp.PostId, trp.Title, trp.CreationDate, trp.ViewCount, trp.Score, trp.RankScore, COALESCE((SELECT MAX(p2.ViewCount) FROM
//        Posts p2 WHERE p2.Id = trp.PostId AND p2.ViewCount IS NOT NULL), 0) AS MaxViewCount, CASE WHEN trp.BadgeHolderId IS NOT NULL THEN 'Has Badge' ELSE 'No Badge' END AS
//        BadgeStatus FROM TopRankedPosts trp) SELECT pa.PostId, pa.Title, pa.CreationDate, pa.ViewCount, pa.Score, pa.RankScore, pa.MaxViewCount, pa.BadgeStatus, CASE WHEN
//        pa.Score > 10 THEN 'Highly Engaged' WHEN pa.Score BETWEEN 5 AND 10 THEN 'Moderately Engaged' ELSE 'Low Engagement' END AS EngagementLevel, STRING_AGG(DISTINCT t.TagName,
//        ', ') AS Tags, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounties FROM PostAnalytics pa LEFT JOIN Posts p ON pa.PostId = p.Id LEFT JOIN Tags t ON t.ExcerptPostId = p.Id
//        LEFT JOIN Votes v ON v.PostId = p.Id AND v.VoteTypeId IN (8, 9) GROUP BY pa.PostId, pa.Title, pa.CreationDate, pa.ViewCount, pa.Score, pa.RankScore, pa.MaxViewCount,
//        pa.BadgeStatus ORDER BY pa.Score DESC, pa.CreationDate ASC OFFSET 5 ROWS FETCH NEXT 10 ROWS ONLY;
//
// RankScore numbers the joined rows; a score tie between different posts goes to the smaller post id, and rows equal on the final ORDER BY
// go in RankScore order (the SQL leaves both open). The distinct tag names are joined in name order.
fn q23592(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, owner_user, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_days(date(2024, 10, 1), -30)))
        .with(score.gt(0))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(comments_of(db).opt()))
        .window(row_number, |((p, s), c)| (Reverse(s), p, c), asc);
    type R = (Id<Post>, i64);
    let rv: MatSet<R> = (&w).filt(|(_, k)| k <= 5).map(|(((p, _), _), k)| (p, k)).collect();
    let ex: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    let bv = || votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let g = (&rv)
        .group_by(Same::<R>::new())
        .select(Same::<R>::new().map(|x: R| x.0).select(owner_user.select(badges_of(db)).opt().and(ex.select(&db.tag.tag_name).opt()).and(bv().opt())))
        .buf_fold(|v| (agg_distinct(v.iter().filter_map(|x| x.0 .1).collect(), ", "), v.iter().map(|x| x.1.flatten().unwrap_or(0)).sum::<i64>()));
    let v = top_n(drain(&g), |&((p, r), _)| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap(), r), 15);
    rows(v.into_iter().skip(5).map(|((p, r), (t, b))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(r), V::I(db.post.view_count.get(p).unwrap_or(0)), V::S("Has Badge")]);
        f.push(V::S(if s > 10 {
            "Highly Engaged"
        } else if (5..=10).contains(&s) {
            "Moderately Engaged"
        } else {
            "Low Engagement"
        }));
        f.extend([ostr(t), V::I(b)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId
//        = 3 THEN 1 ELSE 0 END) AS DownVotes, DENSE_RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY COUNT(c.Id) DESC) AS Rank FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId
//        LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.OwnerUserId),
//        PostHistoryWithTags AS (SELECT ph.PostId, ph.CreationDate, p.Title, STRING_AGG(DISTINCT t.TagName, ', ') AS Tags, COUNT(bp.Id) FILTER (WHERE bp.Id IS NOT NULL) AS
//        BadgesCount FROM PostHistory ph JOIN Posts p ON ph.PostId = p.Id LEFT JOIN Tags t ON t.ExcerptPostId = p.Id LEFT JOIN Badges bp ON bp.UserId = p.OwnerUserId WHERE
//        ph.CreationDate >= p.CreationDate GROUP BY ph.PostId, ph.CreationDate, p.Title), PostsWithPerformance AS (SELECT r.PostId, r.Title, r.CommentCount, r.UpVotes,
//        r.DownVotes, pht.Tags, CASE WHEN r.Rank = 1 THEN 'Top Post' ELSE 'Regular Post' END AS PerformanceCategory FROM RankedPosts r LEFT JOIN PostHistoryWithTags pht ON
//        r.PostId = pht.PostId) SELECT pwp.PostId, pwp.Title, COALESCE(pwp.CommentCount, 0) AS CommentCount, COALESCE(pwp.UpVotes, 0) AS UpVotes, COALESCE(pwp.DownVotes, 0) AS
//        DownVotes, COALESCE(pwp.Tags, 'No Tags') AS Tags, pwp.PerformanceCategory, CASE WHEN pwp.UpVotes - pwp.DownVotes > 0 THEN 'Positive Feedback' WHEN pwp.UpVotes -
//        pwp.DownVotes < 0 THEN 'Negative Feedback' ELSE 'Neutral Feedback' END AS FeedbackType, (SELECT COUNT(*) FROM Votes v WHERE v.PostId = pwp.PostId AND v.VoteTypeId IN (1,
//        2, 5)) AS EngagementScore FROM PostsWithPerformance pwp WHERE pwp.CommentCount > 0 OR pwp.UpVotes > 0 ORDER BY FeedbackType DESC, EngagementScore DESC;
//
// One row per history instant of the post at or after its creation. The distinct tag names are joined in name order.
fn q23381(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, .. } = &db.post;
    let rp = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let w = db.post.with(&rp).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and((&rp).and(owner_user_id.opt()))).window(dense_rank, |(_, (a, _))| Reverse(a[0]), asc);
    type R = ((Id<Post>, ([i64; 3], Option<i64>)), i64);
    let rv: MatSet<R> = (&w).map(|x: R| x).collect();
    let PostHistory { post, creation_date: hd, .. } = &db.post_history;
    let pht = db.post_history.with(hd.and(post.select(creation_date)).filt(|(h, c): (i64, i64)| h >= c)).group_by(post.and(hd)).fold(0i64, |a, _| a + 1);
    type H = ((Id<Post>, i64), i64);
    let hv = rel(drain(&pht));
    let hi: HashIdx<Id<Post>, H> = (&hv).map(|x: H| x.0 .0).inv().select(&hv).collect();
    let ex: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    let tg = db.post.group_by(Ident::<Post>::new()).select(ex.select(&db.tag.tag_name)).buf_fold(|v| agg_distinct(v.to_vec(), ", ").unwrap());
    let es = db.vote.with((&db.vote.vote_type_id).is_in([1, 2, 5])).group_by(&db.vote.post).fold(0i64, |a, _| a + 1);
    let k = || Same::<R>::new().map(|x: R| x.0 .0);
    let v = drain(
        (&rv)
            .filt(|((_, (a, _)), _): R| a[0] > 0 || a[1] > 0)
            .select(Same::<R>::new().and(k().select(Ident::<Post>::new().and((&hi).opt()))).and(k().select((&tg).opt())).and(k().select((&es).opt()))),
    );
    rows(v.into_iter().map(|(_, (((((p, (a, _)), r), (_, h)), t), e))| {
        let d = a[1] - a[2];
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.push(V::S(if h.is_some() { t.unwrap_or("No Tags") } else { "No Tags" }));
        f.push(V::S(if r == 1 { "Top Post" } else { "Regular Post" }));
        f.push(V::S(if d > 0 {
            "Positive Feedback"
        } else if d < 0 {
            "Negative Feedback"
        } else {
            "Neutral Feedback"
        }));
        f.push(V::I(e.unwrap_or(0)));
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT Id, DisplayName, Reputation, CreationDate, ROW_NUMBER() OVER (PARTITION BY CASE WHEN Reputation >= 10000 THEN 'High' WHEN Reputation >= 1000
//        THEN 'Medium' ELSE 'Low' END ORDER BY Reputation DESC) AS Rank FROM Users), PostStats AS (SELECT P.Id AS PostId, P.Title, P.CreationDate AS PostCreationDate,
//        COALESCE(P.AcceptedAnswerId, -1) AS AcceptedAnswerId, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount, SUM(V.BountyAmount) AS TotalBounties, COUNT(DISTINCT
//        P2.Id) AS RelatedPostCount FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId IN (8, 9) LEFT JOIN PostLinks PL ON
//        P.Id = PL.PostId LEFT JOIN Posts P2 ON PL.RelatedPostId = P2.Id WHERE P.CreationDate >= '2020-01-01' GROUP BY P.Id, P.Title, P.CreationDate, P.AcceptedAnswerId),
//        PostHistoryStats AS (SELECT PH.PostId, STRING_AGG(PHT.Name, ', ' ORDER BY PH.CreationDate) AS HistoryTypes, COUNT(*) AS HistoryCount, MAX(PH.CreationDate) AS
//        LastHistoryDate FROM PostHistory PH JOIN PostHistoryTypes PHT ON PH.PostHistoryTypeId = PHT.Id GROUP BY PH.PostId), FinalStats AS (SELECT U.DisplayName, U.Reputation,
//        P.Title, P.CommentCount, PH.HistoryCount, PH.HistoryTypes, U.Rank, CASE WHEN P.TotalBounties > 0 THEN 'Has Bounties' ELSE 'No Bounties' END AS BountyStatus, EXTRACT(EPOCH
//        FROM (TIMESTAMP '2024-10-01 12:34:56' - P.PostCreationDate)) / 3600 AS AgeInHours FROM RankedUsers U JOIN PostStats P ON U.Id = P.PostId JOIN PostHistoryStats PH ON
//        P.PostId = PH.PostId) SELECT DisplayName, Reputation, Title, CommentCount, HistoryCount, HistoryTypes, Rank, BountyStatus, AgeInHours, CASE WHEN AgeInHours < 24 THEN 'New
//        Post' WHEN AgeInHours >= 24 AND AgeInHours < 168 THEN 'Recent Post' ELSE 'Old Post' END AS PostAgeCategory FROM FinalStats WHERE Rank <= 10 ORDER BY Reputation DESC,
//        Title ASC;
//
// `U.Id = P.PostId` joins a user id to a post id, as written. A reputation tie inside the ROW_NUMBER goes to the smaller user id (the SQL leaves it open).
fn q20820(db: &'static So) -> String {
    let lvl = |r: i64| if r >= 10000 { 0 } else if r >= 1000 { 1 } else { 2 };
    let reputation = &db.user.reputation;
    let w = db.user.group_by(reputation.map(lvl)).select(Ident::<User>::new().and(reputation)).window(row_number, |(u, r)| (Reverse(r), u), asc);
    type U = ((Id<User>, i64), i64);
    let ruv: MatSet<U> = (&w).map(|x: U| x).collect();
    let po: HashIdx<i64, Id<Post>> = db.post.with((&db.post.creation_date).ge(date(2020, 1, 1))).select(&db.post.origid).inv().collect();
    let k = || Same::<U>::new().map(|x: U| x.0 .0).select(&db.user.origid).select(&po);
    let mp: MatSet<Id<Post>> = (&ruv).filt(|x: U| x.1 <= 10).select(k()).collect();
    let bv = || votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let ps = (&mp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(bv().opt()).and(links_of(db).select(&db.post_link.related_post).opt()))
        .buf_fold(|v| {
            let c = v.iter().filter(|x| x.0 .0.is_some()).count() as i64;
            let b: Vec<i64> = v.iter().filter_map(|x| x.0 .1.flatten()).collect();
            (c, if b.is_empty() { None } else { Some(b.iter().sum::<i64>()) })
        });
    let PostHistory { post, creation_date: hd, .. } = &db.post_history;
    let phs = (&mp).group_by(Ident::<Post>::new()).select(history_of(db).select(hd.and(htype_name(db)).and(Ident::<PostHistory>::new()))).buf_fold(|v| {
        let mut x = v.to_vec();
        x.sort();
        (leak(x.iter().map(|y| y.0 .1).collect::<Vec<_>>().join(", ")), x.len() as i64)
    });
    let _ = post;
    let v = drain((&ruv).filt(|x: U| x.1 <= 10).select(Same::<U>::new().and(k().select(Ident::<Post>::new().and(&ps).and(&phs)))));
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    rows(v.into_iter().map(|(_, (((u, _), r), ((p, (c, b)), (ht, hn))))| {
        let age = secs(t0 - db.post.creation_date.get(p).unwrap()) / 3600.0;
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title"]));
        f.extend([V::I(c), V::I(hn), V::S(ht), V::I(r), V::S(if b.map_or(false, |b| b > 0) { "Has Bounties" } else { "No Bounties" }), V::F(age)]);
        f.push(V::S(if age < 24.0 {
            "New Post"
        } else if age < 168.0 {
            "Recent Post"
        } else {
            "Old Post"
        }));
        row(f)
    }))
}

// WITH PostStats AS (SELECT p.Id AS PostID, p.Title, COALESCE(COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2), 0) AS UpvoteCount, COALESCE(COUNT(v.Id) FILTER (WHERE
//        v.VoteTypeId = 3), 0) AS DownvoteCount, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn, p.CreationDate, u.Reputation AS OwnerReputation FROM
//        Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= (CAST('2024-10-01' AS DATE) - INTERVAL '1 year') GROUP BY
//        p.Id, p.Title, p.CreationDate, u.Reputation), AcceptedAnswers AS (SELECT p.Id AS QuestionID, COUNT(a.Id) AS AcceptedAnswersCount FROM Posts p LEFT JOIN Posts a ON
//        p.AcceptedAnswerId = a.Id WHERE p.PostTypeId = 1 GROUP BY p.Id), UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount, STRING_AGG(DISTINCT b.Name, ', ') AS
//        BadgeNames FROM Badges b GROUP BY b.UserId), CombinedStats AS (SELECT ps.PostID, ps.Title, ps.UpvoteCount - ps.DownvoteCount AS NetVotes, aa.AcceptedAnswersCount,
//        ub.BadgeCount, ub.BadgeNames, ps.OwnerReputation FROM PostStats ps LEFT JOIN AcceptedAnswers aa ON ps.PostID = aa.QuestionID LEFT JOIN UserBadges ub ON ps.PostID =
//        ub.UserId LEFT JOIN (SELECT DISTINCT UserId, Reputation FROM Users WHERE Location IS NOT NULL AND Location != '') ua ON 1=1) SELECT PostID, Title, NetVotes,
//        AcceptedAnswersCount, COALESCE(BadgeCount, 0) AS UserBadgeCount, CONCAT('User has badges: ', COALESCE(BadgeNames, 'None')) AS BadgeDetails, CASE WHEN OwnerReputation IS
//        NULL THEN 'Reputation data unavailable' WHEN OwnerReputation > 1000 THEN 'Highly trusted user' ELSE 'User with limited reputation' END AS ReputationStatus FROM
//        CombinedStats WHERE (NetVotes > 0 OR AcceptedAnswersCount > 0) ORDER BY NetVotes DESC, Title FETCH FIRST 50 ROWS ONLY;
//
// The `ua` subquery names `UserId`, which Users does not have, so DuckDB binds it to `ub.UserId` and the subquery is the distinct reputations of
// users with a location, crossed with every row (`ON 1=1`). `ps.PostID = ub.UserId` joins a post id to a user id, as written.
// Every row is repeated once per distinct reputation and the ORDER BY reads only the row's own columns, so only the rows ranked in the top 50
// can reach the LIMIT. The distinct badge names are joined in name order.
fn q21064(db: &'static So) -> String {
    let Post { creation_date, owner_user, origid, title, post_type_id, .. } = &db.post;
    let ps = db
        .post
        .with(creation_date.ge(add_years(date(2024, 10, 1), -1)))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt())
        .fold(0i64, |a, t| a + (t == Some(2)) as i64 - (t == Some(3)) as i64);
    let aa = db.post.with(post_type_id.eq(1)).group_by(Ident::<Post>::new()).select((&db.post.accepted_answer).opt()).fold(0i64, |a, x| a + x.is_some() as i64);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.name)).buf_fold(|v| (v.len() as i64, agg_distinct(v.to_vec(), ", ").unwrap()));
    let uo = users_by_origid(db);
    let reps: MatSet<i64> = db.user.with((&db.user.location).filt(|l| !l.is_empty())).select(&db.user.reputation).collect();
    type Y = (((Id<Post>, Option<Str>), (i64, Option<i64>)), Option<(i64, Str)>);
    let cw = whole(db.post.with((&ps).and((&aa).opt()).filt(|(n, q): (i64, Option<i64>)| n > 0 || q.map_or(false, |q| q > 0))))
        .select(Ident::<Post>::new().and(title.opt()).and((&ps).and((&aa).opt())).and(origid.select(&uo).select(&ub).opt()))
        .window(rank, |(((_, t), (n, _)), _): Y| (Reverse(n), t.is_none(), t), asc);
    let cand: MatSet<Y> = (&cw).filt(|(_, k)| k <= 50).map(|(y, _)| y).collect();
    let v = top_n(drain((&cand).cross(&reps)), |&(_, ((((p, t), (n, _)), _), _))| (Reverse(n), t.is_none(), t, p), 50);
    rows(v.into_iter().map(|(_, ((((p, _), (n, q)), u), _))| {
        let r = owner_user.get(p).map(|u| db.user.reputation.get(u).unwrap());
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(n), oint(q), V::I(u.map_or(0, |u| u.0))]);
        f.push(V::Owned(format!("User has badges: {}", u.map_or("None", |u| u.1))));
        f.push(V::S(match r {
            None => "Reputation data unavailable",
            Some(r) if r > 1000 => "Highly trusted user",
            _ => "User with limited reputation",
        }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.Reputation AS OwnerReputation, p.Score, p.ViewCount, ROW_NUMBER() OVER(PARTITION BY p.OwnerUserId
//        ORDER BY p.Score DESC) AS RankByScore, COUNT(c.Id) OVER(PARTITION BY p.Id) AS CommentCount FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON
//        p.Id = c.PostId WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'), PostVoteStats AS (SELECT PostId, SUM(CASE WHEN v.VoteTypeId = 2
//        THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes v GROUP BY PostId), ClosedPosts AS (SELECT ph.PostId,
//        ph.UserDisplayName AS ClosedBy, STRING_AGG(DISTINCT pht.Name, ', ') AS CloseReasons FROM PostHistory ph JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id WHERE
//        ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId, ph.UserDisplayName), FinalPostStats AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.OwnerReputation,
//        COALESCE(pvs.UpVotes, 0) AS UpVotes, COALESCE(pvs.DownVotes, 0) AS DownVotes, COALESCE(closed.CloseReasons, 'Not Closed') AS CloseReasons, rp.CommentCount, CASE WHEN
//        rp.RankByScore <= 5 THEN 'Top Performer' ELSE 'Regular Performer' END AS PerformanceCategory FROM RankedPosts rp LEFT JOIN PostVoteStats pvs ON rp.PostId = pvs.PostId
//        LEFT JOIN ClosedPosts closed ON rp.PostId = closed.PostId) SELECT f.Title, f.OwnerReputation, f.UpVotes, f.DownVotes, f.CommentCount, f.CloseReasons, (f.UpVotes -
//        f.DownVotes) AS NetVotes, CASE WHEN (f.UpVotes - f.DownVotes) > 10 THEN 'Highly Engaged' WHEN (f.UpVotes - f.DownVotes) BETWEEN 1 AND 10 THEN 'Moderately Engaged' ELSE
//        'Less Engaged' END AS EngagementLevel FROM FinalPostStats f WHERE f.CommentCount > 0 AND f.OwnerReputation IS NOT NULL ORDER BY f.CreationDate DESC OFFSET 0 ROWS FETCH
//        NEXT 30 ROWS ONLY;
//
// One row per comment of the post (the RankByScore label is not projected). A CreationDate tie between posts goes to the smaller post id.
// The distinct type names are joined in name order.
fn q22566(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let pvs = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let cc = comments_per_post(db);
    let PostHistory { post, post_history_type_id, user_display_name, .. } = &db.post_history;
    let cl = db
        .post_history
        .with(post_history_type_id.is_in([10, 11]))
        .group_by(post.and(user_display_name.opt()))
        .select(htype_name(db))
        .buf_fold(|v| agg_distinct(v.to_vec(), ", ").unwrap());
    type C = ((Id<Post>, Option<Str>), Str);
    let clv = rel(drain(&cl));
    let cli: HashIdx<Id<Post>, C> = (&clv).map(|x: C| x.0 .0).inv().select(&clv).collect();
    let v = drain(
        db.post
            .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
            .with(owner_user)
            .select(Ident::<Post>::new().and(comments_of(db)).and(&cc).and((&pvs).opt()).and((&cli).opt())),
    );
    let v = top_n(v, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 30);
    rows(v.into_iter().map(|(_, ((((p, _), c), a), cl))| {
        let a = a.unwrap_or([0; 2]);
        let d = a[0] - a[1];
        let mut f = post_fields(db, p, &["title", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c), V::S(cl.map_or("Not Closed", |c| c.1)), V::I(d)]);
        f.push(V::S(if d > 10 {
            "Highly Engaged"
        } else if (1..=10).contains(&d) {
            "Moderately Engaged"
        } else {
            "Less Engaged"
        }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.Score, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank FROM
//        Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate,
//        p.OwnerUserId, p.Score), CloseReasons AS (SELECT ph.PostId, STRING_AGG(DISTINCT crt.Name, ', ') AS Reasons FROM PostHistory ph JOIN CloseReasonTypes crt ON
//        ph.Comment::int = crt.Id WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId), UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS
//        PostCount, SUM(p.Score) AS TotalScore, AVG(COALESCE(EXTRACT(EPOCH FROM (cast('2024-10-01 12:34:56' as timestamp) - p.CreationDate)), 0)) AS AvgPostAgeInSeconds FROM Users
//        u LEFT JOIN Posts p ON u.Id = p.OwnerUserId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName), FinalReport AS (SELECT us.UserId, us.DisplayName, us.PostCount,
//        us.TotalScore, us.AvgPostAgeInSeconds, rq.PostId, rq.Title, rq.UpVotes, rq.DownVotes, cr.Reasons FROM UserStatistics us JOIN RankedPosts rq ON us.UserId = rq.OwnerUserId
//        LEFT JOIN CloseReasons cr ON rq.PostId = cr.PostId WHERE rq.PostRank = 1) SELECT fr.DisplayName, fr.PostCount, fr.TotalScore, (CASE WHEN fr.AvgPostAgeInSeconds IS NULL
//        THEN 'New User' WHEN fr.AvgPostAgeInSeconds < 604800 THEN 'Newly Active' ELSE 'Long-Term User' END) AS UserType, STRING_AGG(DISTINCT fr.Reasons, '; ') AS CloseReasons
//        FROM FinalReport fr GROUP BY fr.DisplayName, fr.PostCount, fr.TotalScore, fr.AvgPostAgeInSeconds ORDER BY fr.TotalScore DESC, fr.DisplayName LIMIT 10;
//
// A CreationDate tie inside the ROW_NUMBER goes to the smaller post id (the SQL leaves it open). The distinct reasons are joined in sorted order.
fn q20578(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let w = db.post.with(creation_date.ge(add_years(t0, -1))).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let rq: HashIdx<Id<User>, Id<Post>> = (&w).filt(|(_, k)| k == 1).map(|((p, _), _)| p).collect();
    let cr = close_reasons(db);
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let crs = db
        .post_history
        .with(post_history_type_id.is_in([10, 11]))
        .group_by(post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&cr).select(&db.close_reason_type.name))
        .buf_fold(|v| agg_distinct(v.to_vec(), ", ").unwrap());
    let us = db
        .user
        .with((&db.user.reputation).gt(1000))
        .with(&rq)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(creation_date)).opt())
        .fold((0i64, 0i64, 0i64, 0.0f64), |(n, s, sn, a), p| match p {
            Some((sc, d)) => (n + 1, s + sc, sn + 1, a + secs(t0 - d)),
            None => (n, s, sn, a),
        });
    type K = (i64, Option<i64>, u64);
    let key = (&us).map(|(n, s, sn, a): (i64, i64, i64, f64)| -> K { (n, if sn == 0 { None } else { Some(s) }, (a / n.max(1) as f64).to_bits()) });
    let g = db.user.with(&us).group_by((&db.user.display_name).and(key)).select((&rq).select((&crs).opt())).buf_fold(|v| agg_distinct(v.iter().flatten().copied().collect(), "; "));
    let v = top_n(drain(&g), |&((d, (_, s, _)), _)| (s.is_none(), Reverse(s), d), 10);
    rows(v.into_iter().map(|((d, (n, s, a)), c)| {
        let ut = if f64::from_bits(a) < 604800.0 { "Newly Active" } else { "Long-Term User" };
        row(vec![V::S(d), V::I(n), oint(s), V::S(ut), ostr(c)])
    }))
}

fn json_get(s: &str, key: &str) -> Option<String> {
    let b = s.as_bytes();
    let mut i = 0;
    let skip_ws = |i: &mut usize| {
        while *i < b.len() && (b[*i] as char).is_whitespace() {
            *i += 1;
        }
    };
    let read_str = |i: &mut usize| -> String {
        let mut out = String::new();
        *i += 1;
        while *i < b.len() && b[*i] != b'"' {
            if b[*i] == b'\\' {
                *i += 1;
                match b[*i] {
                    b'n' => out.push('\n'),
                    b't' => out.push('\t'),
                    b'r' => out.push('\r'),
                    b'u' => {
                        let h = u32::from_str_radix(&s[*i + 1..*i + 5], 16).unwrap_or(0xfffd);
                        out.push(char::from_u32(h).unwrap_or('\u{fffd}'));
                        *i += 4;
                    }
                    c => out.push(c as char),
                }
                *i += 1;
            } else {
                let c = s[*i..].chars().next().unwrap();
                out.push(c);
                *i += c.len_utf8();
            }
        }
        *i += 1;
        out
    };
    let skip_val = |i: &mut usize| {
        let mut depth = 0i32;
        loop {
            if *i >= b.len() {
                return;
            }
            match b[*i] {
                b'"' => {
                    *i += 1;
                    while *i < b.len() && b[*i] != b'"' {
                        if b[*i] == b'\\' {
                            *i += 1;
                        }
                        *i += 1;
                    }
                    *i += 1;
                }
                b'{' | b'[' => {
                    depth += 1;
                    *i += 1;
                }
                b'}' | b']' => {
                    if depth == 0 {
                        return;
                    }
                    depth -= 1;
                    *i += 1;
                }
                b',' if depth == 0 => return,
                _ => *i += 1,
            }
            if depth == 0 && *i < b.len() && (b[*i] == b',' || b[*i] == b'}') {
                return;
            }
        }
    };
    skip_ws(&mut i);
    if i >= b.len() || b[i] != b'{' {
        return None;
    }
    i += 1;
    loop {
        skip_ws(&mut i);
        if i >= b.len() || b[i] == b'}' {
            return None;
        }
        let k = read_str(&mut i);
        skip_ws(&mut i);
        i += 1;
        skip_ws(&mut i);
        let start = i;
        if k == key {
            if b[i] == b'"' {
                return Some(read_str(&mut i));
            }
            skip_val(&mut i);
            let v = s[start..i].trim();
            return if v == "null" { None } else { Some(v.to_string()) };
        }
        skip_val(&mut i);
        skip_ws(&mut i);
        if i < b.len() && b[i] == b',' {
            i += 1;
        }
    }
}

// WITH UserPostMetrics AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(vt.VoteCount), 0) AS TotalVotes, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT c.Id) AS
//        TotalComments, DENSE_RANK() OVER (ORDER BY COALESCE(SUM(vt.VoteCount), 0) DESC) AS VoteRank FROM Users AS u LEFT JOIN Posts AS p ON u.Id = p.OwnerUserId LEFT JOIN (SELECT
//        PostId, COUNT(*) AS VoteCount FROM Votes GROUP BY PostId) AS vt ON p.Id = vt.PostId LEFT JOIN Comments AS c ON p.Id = c.PostId GROUP BY u.Id, u.DisplayName),
//        PostEngagement AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, EXTRACT(EPOCH FROM (cast('2024-10-01 12:34:56' as timestamp) - p.CreationDate)) / 3600 AS AgeInHours,
//        COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.UserId) AS UniqueVoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3
//        THEN 1 ELSE 0 END) AS DownVotes FROM Posts AS p LEFT JOIN Comments AS c ON p.Id = c.PostId LEFT JOIN Votes AS v ON p.Id = v.PostId GROUP BY p.Id, p.Title,
//        p.CreationDate), ClosePostAnalysis AS (SELECT ph.PostId, CASE WHEN COUNT(*) > 0 THEN 'Closed' ELSE 'Active' END AS PostStatus, STRING_AGG(DISTINCT ctr.Name, ', ') AS
//        CloseReasonTypes FROM PostHistory AS ph JOIN CloseReasonTypes AS ctr ON ph.Comment IS NOT NULL AND ph.PostHistoryTypeId = 10 GROUP BY ph.PostId), FinalMetrics AS (SELECT
//        upm.UserId, upm.DisplayName, pm.PostId, pm.Title, pm.AgeInHours, pm.CommentCount, pm.UniqueVoteCount, pm.UpVotes, pm.DownVotes, cpa.PostStatus,
//        COALESCE(cpa.CloseReasonTypes, 'N/A') AS CloseReasonTypes FROM UserPostMetrics AS upm JOIN PostEngagement AS pm ON upm.TotalPosts > 0 LEFT JOIN ClosePostAnalysis AS cpa
//        ON pm.PostId = cpa.PostId WHERE upm.VoteRank <= 10) SELECT UserId, DisplayName, PostId, Title, AgeInHours, CommentCount, UniqueVoteCount, UpVotes, DownVotes, PostStatus,
//        CloseReasonTypes FROM FinalMetrics ORDER BY UpVotes DESC, CommentCount DESC, PostId, UserId LIMIT 100;
//
// Rewritten (rewrites/21819.sql): the final ORDER BY gets `, PostId, UserId`. `ON upm.TotalPosts > 0` and the CloseReasonTypes join do not
// mention the other side, so both are cross joins. Each post meets every top user and the ORDER BY leads with the post's own key, so only the
// first 100 posts by that key can reach the LIMIT.
fn q21819(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let vt = db.vote.group_by(&db.vote.post).fold(0i64, |a, _| a + 1);
    let upm = db
        .user
        .with(posts_of(db))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&vt).opt().and(comments_of(db).opt())))
        .fold(0i64, |a, (v, _)| a + v.unwrap_or(0));
    let uw = whole(db.user.with(posts_of(db))).select(Ident::<User>::new().and(&upm)).window(dense_rank, |(_, n)| Reverse(n), asc);
    let tu: MatSet<(Id<User>, i64)> = (&uw).filt(|(_, k)| k <= 10).map(|(x, _)| x).collect();
    let pe = db
        .post
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let uv = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.user_id)).count_distinct();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    type Y = (Id<Post>, Str);
    let cpa = db
        .post_history
        .with(post_history_type_id.eq(10))
        .with(comment)
        .select(post)
        .cross(&db.close_reason_type.name)
        .group_by(Same::<Y>::new().map(|x: Y| x.0))
        .select(Same::<Y>::new().map(|x: Y| x.1))
        .buf_fold(|v| agg_distinct(v.to_vec(), ", ").unwrap());
    type P = (Id<Post>, (([i64; 3], Option<i64>), Option<Str>));
    let pw = whole(&db.post.id).select(Ident::<Post>::new().and((&pe).and((&uv).opt()).and((&cpa).opt()))).window(row_number, |(p, ((a, _), _)): P| (Reverse(a[1]), Reverse(a[0]), p), asc);
    let pc: MatSet<P> = (&pw).filt(|(_, k)| k <= 100).map(|(x, _)| x).collect();
    let v = top_n(drain((&pc).cross(&tu)), |&(_, ((p, ((a, _), _)), (u, _)))| (Reverse(a[1]), Reverse(a[0]), p, u), 100);
    rows(v.into_iter().map(|(_, ((p, ((a, n), c)), (u, _)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(post_fields(db, p, &["id", "title"]));
        f.push(V::F(secs(t0 - db.post.creation_date.get(p).unwrap()) / 3600.0));
        f.extend([V::I(a[0]), V::I(n.unwrap_or(0)), V::I(a[1]), V::I(a[2])]);
        f.extend(match c {
            Some(c) => [V::S("Closed"), V::S(c)],
            None => [V::Null, V::S("N/A")],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate
//        DESC) AS Rank, COALESCE(NULLIF(p.AcceptedAnswerId, -1), 0) AS AcceptedAnswerId FROM Posts p WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '2 years'), UserStats AS
//        (SELECT u.Id AS UserId, u.Reputation, COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges, COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0)
//        AS SilverBadges, COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE
//        0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Users u LEFT JOIN Badges b ON b.UserId = u.Id LEFT JOIN Posts p ON p.OwnerUserId =
//        u.Id LEFT JOIN Votes v ON v.UserId = u.Id GROUP BY u.Id, u.Reputation), PostHistoryDetails AS (SELECT ph.PostId, ph.Comment AS CloseComment, ph.CreationDate AS CloseDate,
//        CONCAT(EXTRACT(YEAR FROM ph.CreationDate), '-', LPAD(EXTRACT(MONTH FROM ph.CreationDate)::TEXT, 2, '0')) AS CloseMonthYear FROM PostHistory ph WHERE ph.PostHistoryTypeId
//        = 10), FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, us.UserId, us.Reputation, pd.CloseComment, pd.CloseDate, pd.CloseMonthYear FROM RankedPosts rp LEFT
//        JOIN UserStats us ON us.UserId = rp.AcceptedAnswerId LEFT JOIN PostHistoryDetails pd ON pd.PostId = rp.PostId WHERE rp.Rank <= 5) SELECT fp.PostId, fp.Title,
//        fp.CreationDate, fp.Reputation, CASE WHEN fp.CloseComment IS NOT NULL THEN 'Closed' ELSE 'Open' END AS PostStatus, COALESCE(fp.CloseMonthYear, 'Not Closed') AS
//        ClosedMonthYear, (SELECT STRING_AGG(DISTINCT t.TagName, ', ') FROM Tags t JOIN Posts p ON t.ExcerptPostId = p.Id WHERE p.Id = fp.PostId) AS AssociatedTags FROM
//        FilteredPosts fp ORDER BY fp.CreationDate DESC;
//
// `us.UserId = rp.AcceptedAnswerId` joins a user id to a post id, as written; only us.Reputation is read from UserStats. A CreationDate tie
// inside the ROW_NUMBER goes to the smaller post id. The distinct tag names are joined in name order.
fn q21670(db: &'static So) -> String {
    let Post { creation_date, post_type_id, accepted_answer_id, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(current_date(), -2))).group_by(post_type_id).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|((p, _), _)| p).collect();
    let uo = users_by_origid(db);
    let PostHistory { post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let pd = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.eq(10)).select(comment.opt().and(hd)));
    let ex: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    let tg = (&tp).group_by(Ident::<Post>::new()).select(ex.select(&db.tag.tag_name)).buf_fold(|v| agg_distinct(v.to_vec(), ", ").unwrap());
    let v = drain(
        (&tp).select(Ident::<Post>::new().and(accepted_answer_id.opt().map(|a: Option<i64>| match a {
            None | Some(-1) => 0,
            Some(x) => x,
        }).select(&uo).opt()).and(pd.opt()).and((&tg).opt())),
    );
    rows(v.into_iter().map(|(_, (((p, u), h), t))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.push(u.map_or(V::Null, |u| user_col(db, u, "rep")));
        f.extend(match h {
            Some((c, d)) => [V::S(if c.is_some() { "Closed" } else { "Open" }), V::Owned(format!("{}-{:02}", year(d), month(d)))],
            None => [V::S("Open"), V::S("Not Closed")],
        });
        f.push(ostr(t));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.PostTypeId, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score
//        DESC) AS RankByScore, DENSE_RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS RankByDate FROM Posts p WHERE p.PostTypeId IN (1, 2)), UserStats AS
//        (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty, COUNT(DISTINCT b.Id) AS BadgeCount, SUM(CASE WHEN bh.Class = 1 THEN 1 ELSE 0 END)
//        AS GoldBadges, SUM(CASE WHEN bh.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN bh.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Votes v
//        ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Badges bh ON u.Id = bh.UserId GROUP BY u.Id, u.DisplayName), PostHistoryDetails AS (SELECT ph.PostId,
//        ph.CreationDate, ph.Comment AS CloseReason, STRING_AGG(ph.Comment, '; ') AS EditComments, MAX(ph.CreationDate) FILTER (WHERE ph.PostHistoryTypeId IN (10, 11, 12)) AS
//        LastActionDate FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11, 12, 24) GROUP BY ph.PostId, ph.CreationDate, ph.Comment) SELECT rp.PostId, rp.Title,
//        rp.CreationDate, rp.Score, rp.ViewCount, us.DisplayName AS UserOwner, us.BadgeCount, us.TotalBounty, phd.EditComments, phd.LastActionDate, CASE WHEN phd.CloseReason IS
//        NOT NULL THEN 'Post is Closed' ELSE 'Post is Active' END AS PostStatus, CASE WHEN us.TotalBounty > 500 THEN 'High Bounty Holder' WHEN us.TotalBounty > 200 THEN 'Medium
//        Bounty Holder' ELSE 'Low Bounty Holder' END AS BountyCategory FROM RankedPosts rp LEFT JOIN UserStats us ON rp.PostId = us.UserId LEFT JOIN PostHistoryDetails phd ON
//        rp.PostId = phd.PostId WHERE (rp.RankByScore <= 5 OR rp.RankByDate <= 10) AND (us.BadgeCount > 0 OR us.TotalBounty > 0) AND (phd.LastActionDate IS NULL OR
//        phd.LastActionDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year') ORDER BY rp.CreationDate DESC LIMIT 100;
//
// `rp.PostId = us.UserId` joins a post id to a user id, as written. A score tie inside the ROW_NUMBER goes to the smaller post id (the SQL
// leaves it open). Every row of a PostHistoryDetails group has the same Comment, so the STRING_AGG repeats it once per row.
fn q20254(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, origid, .. } = &db.post;
    let base = || db.post.with(post_type_id.is_in([1, 2])).group_by(post_type_id);
    let sw = base().select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let dw = base().select(Ident::<Post>::new().and(creation_date)).window(dense_rank, |(_, d)| Reverse(d), asc);
    let rp: MatSet<Id<Post>> = (&sw).filt(|(_, k)| k <= 5).map(|((p, _), _)| p).union((&dw).filt(|(_, k)| k <= 10).map(|((p, _), _)| p)).collect();
    let uo = users_by_origid(db);
    let mu: MatSet<Id<User>> = (&rp).select(origid).select(&uo).collect();
    let us = (&mu)
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select((&db.vote.bounty_amount).opt()).opt().and(badges_of(db).opt()).and(badges_of(db).opt()))
        .fold(0i64, |a, ((b, _), _)| a + b.flatten().unwrap_or(0));
    let bc = (&mu).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let PostHistory { post, post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let phd = db
        .post_history
        .with(post_history_type_id.is_in([10, 11, 12, 24]))
        .group_by(post.and(hd).and(comment.opt()))
        .select(post_history_type_id.and(hd).and(comment.opt()))
        .buf_fold(|v| {
            let c: Vec<Str> = v.iter().filter_map(|x| x.1).collect();
            let m = v.iter().filter(|x| x.0 .0 <= 12).map(|x| x.0 .1).max();
            (if c.is_empty() { None } else { Some(leak(c.join("; "))) }, m)
        });
    type H = (((Id<Post>, i64), Option<Str>), (Option<Str>, Option<i64>));
    let hv = rel(drain(&phd));
    let hi: HashIdx<Id<Post>, H> = (&hv).map(|x: H| x.0 .0 .0).inv().select(&hv).collect();
    let cut = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    type X = ((Id<Post>, Option<(Id<User>, (i64, i64))>), Option<H>);
    let v = drain(
        (&rp)
            .select(Ident::<Post>::new().and(origid.select(&uo).select(Ident::<User>::new().and((&us).and(&bc))).opt()).and((&hi).opt()))
            .filt(|((_, u), h): X| u.map_or(false, |u| u.1 .1 > 0 || u.1 .0 > 0) && h.map_or(true, |h| h.1 .1.map_or(true, |d| d >= cut))),
    );
    let v = top_n(v, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 100);
    rows(v.into_iter().map(|(_, ((p, u), h))| {
        let (u, (b, n)) = u.unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([user_col(db, u, "name"), V::I(n), V::I(b)]);
        f.extend(match h {
            Some((((_, _), c), (e, m))) => [ostr(e), ots(m), V::S(if c.is_some() { "Post is Closed" } else { "Post is Active" })],
            None => [V::Null, V::Null, V::S("Post is Active")],
        });
        f.push(V::S(if b > 500 {
            "High Bounty Holder"
        } else if b > 200 {
            "Medium Bounty Holder"
        } else {
            "Low Bounty Holder"
        }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.Score, P.ViewCount, U.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY P.PostTypeId ORDER BY
//        P.Score DESC, P.ViewCount DESC) AS Rank FROM Posts P INNER JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1
//        year' AND P.ViewCount IS NOT NULL), PostCloseHistory AS (SELECT PH.PostId, PH.CreationDate, PH.Comment, COALESCE(PH.Text::json ->> 'closeReasonId', 'Not Applicable') AS
//        CloseReason FROM PostHistory PH WHERE PH.PostHistoryTypeId = 10), ModerationActions AS (SELECT PH.PostId, COUNT(PH.Id) AS ClosureCount, STRING_AGG(DISTINCT PHT.Name, ',
//        ') AS ClosureReasons FROM PostHistory PH JOIN PostHistoryTypes PHT ON PH.PostHistoryTypeId = PHT.Id WHERE PH.PostHistoryTypeId IN (10, 11) GROUP BY PH.PostId),
//        UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVoteCount, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS
//        DownVoteCount, COUNT(DISTINCT C.Id) AS CommentCount FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Comments C ON U.Id = C.UserId GROUP BY U.Id,
//        U.DisplayName), FinalOutput AS (SELECT RP.PostId, RP.Title, RP.Score, RP.ViewCount, RP.OwnerDisplayName, COALESCE(PCH.CloseReason, 'Open') AS PostCloseReason,
//        MA.ClosureCount AS CloseCount, UA.UpVoteCount, UA.DownVoteCount, UA.CommentCount FROM RankedPosts RP LEFT JOIN PostCloseHistory PCH ON RP.PostId = PCH.PostId LEFT JOIN
//        ModerationActions MA ON RP.PostId = MA.PostId LEFT JOIN UserActivity UA ON RP.PostId = UA.UserId WHERE RP.Rank <= 10) SELECT *, CASE WHEN Score IS NULL THEN 'Score is
//        NULL' WHEN ViewCount IS NOT NULL AND UpVoteCount > DownVoteCount THEN 'Popular Post' ELSE 'Less Engaging' END AS EngagementStatus FROM FinalOutput ORDER BY Score DESC,
//        ViewCount DESC;
//
// `RP.PostId = UA.UserId` joins a post id to a user id, as written. `->> 'closeReasonId'` is a top-level JSON key lookup on the Text.
// A tie inside the ROW_NUMBER goes to the smaller post id (the SQL leaves it open).
fn q20490(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, owner_user, origid, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(view_count)
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(view_count))
        .window(row_number, |((p, s), w)| (Reverse(s), Reverse(w), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 10).map(|(((p, _), _), _)| p).collect();
    let PostHistory { post, post_history_type_id, text, .. } = &db.post_history;
    let pch = history_of(db).select(
        Ident::<PostHistory>::new()
            .with(post_history_type_id.eq(10))
            .select(text.opt().map(|t: Option<Str>| -> Str { leak(t.and_then(|t| json_get(t, "closeReasonId")).unwrap_or_else(|| "Not Applicable".to_string())) })),
    );
    let ma = db.post_history.with(post_history_type_id.is_in([10, 11])).group_by(post).select(htype_name(db)).buf_fold(|v| v.len() as i64);
    let uo = users_by_origid(db);
    let mu: MatSet<Id<User>> = (&tp).select(origid).select(&uo).collect();
    let ua2 = (&mu)
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(comments_by(db).opt()))
        .fold([0i64; 2], |a, (t, _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let uc = (&mu).group_by(Ident::<User>::new()).select(comments_by(db)).count_distinct();
    let ua = (&ua2).and((&uc).opt()).map(|(a, c): ([i64; 2], Option<i64>)| [a[0], a[1], c.unwrap_or(0)]);
    let v = drain((&tp).select(Ident::<Post>::new().and(pch.opt()).and((&ma).opt()).and(origid.select(&uo).select(&ua).opt())));
    rows(v.into_iter().map(|(_, (((p, c), n), u))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "owner"]);
        f.extend([V::S(c.unwrap_or("Open")), oint(n)]);
        f.extend(match u {
            Some(a) => a.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        f.push(V::S(if u.map_or(false, |a| a[0] > a[1]) { "Popular Post" } else { "Less Engaging" }));
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.PostTypeId, p.AcceptedAnswerId, u.DisplayName AS OwnerDisplayName, u.Reputation, ROW_NUMBER() OVER
//        (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS RN FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01' as date) -
//        INTERVAL '30 days'), PostEngagement AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpVotes, COUNT(v.Id) FILTER
//        (WHERE v.VoteTypeId = 3) AS DownVotes FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId IN (1, 2) GROUP BY
//        p.Id), TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.OwnerDisplayName, pe.CommentCount, pe.UpVotes, pe.DownVotes, COALESCE(rp.AcceptedAnswerId, -1) AS
//        AcceptedAnswerId FROM RecentPosts rp JOIN PostEngagement pe ON rp.PostId = pe.PostId WHERE rp.RN <= 10), ClosedPostHistory AS (SELECT ph.PostId, ph.UserDisplayName,
//        ph.CreationDate, STRING_AGG(DISTINCT pht.Name, ', ') AS ClosureReasons FROM PostHistory ph JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id WHERE
//        ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId, ph.UserDisplayName, ph.CreationDate), FinalResults AS (SELECT tp.PostId, tp.Title, tp.CreationDate,
//        tp.OwnerDisplayName, tp.CommentCount, tp.UpVotes, tp.DownVotes, COALESCE(cph.ClosureReasons, 'No Closure') AS ClosureReasons FROM TopPosts tp LEFT JOIN ClosedPostHistory
//        cph ON tp.PostId = cph.PostId) SELECT fr.PostId, fr.Title, fr.CreationDate, fr.OwnerDisplayName, fr.CommentCount, fr.UpVotes, fr.DownVotes, fr.ClosureReasons, CASE WHEN
//        fr.UpVotes > fr.DownVotes THEN 'Positive Engagement' WHEN fr.UpVotes < fr.DownVotes THEN 'Negative Engagement' ELSE 'Neutral Engagement' END AS EngagementSentiment, CASE
//        WHEN fr.ClosureReasons IS NOT NULL THEN 'Closed' ELSE 'Open' END AS PostStatus FROM FinalResults fr ORDER BY fr.CreationDate DESC, fr.UpVotes DESC, fr.CommentCount DESC;
//
// A CreationDate tie inside the ROW_NUMBER goes to the smaller post id (the SQL leaves it open). The distinct type names are joined in name order.
fn q23824(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_days(date(2024, 10, 1), -30))).with(owner_user).group_by(post_type_id).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 10).map(|((p, _), _)| p).collect();
    let pe = (&tp)
        .with(post_type_id.is_in([1, 2]))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let PostHistory { post, post_history_type_id, user_display_name, creation_date: hd, .. } = &db.post_history;
    let cph = db
        .post_history
        .with(post_history_type_id.is_in([10, 11]))
        .group_by(post.and(user_display_name.opt()).and(hd))
        .select(htype_name(db))
        .buf_fold(|v| agg_distinct(v.to_vec(), ", ").unwrap());
    type C = (((Id<Post>, Option<Str>), i64), Str);
    let cv = rel(drain(&cph));
    let ci: HashIdx<Id<Post>, C> = (&cv).map(|x: C| x.0 .0 .0).inv().select(&cv).collect();
    let v = drain((&pe).and((&ci).opt()));
    rows(v.into_iter().map(|(p, (a, c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend(a.map(V::I));
        f.push(V::S(c.map_or("No Closure", |c| c.1)));
        f.push(V::S(if a[1] > a[2] {
            "Positive Engagement"
        } else if a[1] < a[2] {
            "Negative Engagement"
        } else {
            "Neutral Engagement"
        }));
        f.push(V::S("Closed"));
        row(f)
    }))
}

// WITH UserVotes AS (SELECT U.Id AS UserId, U.Reputation, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN V.VoteTypeId = 3
//        THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN P.PostTypeId = 1 THEN P.Id END) AS Questions, COUNT(DISTINCT CASE WHEN
//        P.PostTypeId = 2 THEN P.Id END) AS Answers FROM Users U LEFT JOIN Posts P ON P.OwnerUserId = U.Id LEFT JOIN Votes V ON V.UserId = U.Id AND V.PostId = P.Id GROUP BY U.Id,
//        U.Reputation), UserBadges AS (SELECT B.UserId, COUNT(B.Id) AS BadgeCount, STRING_AGG(B.Name, ', ') AS BadgeNames FROM Badges B GROUP BY B.UserId), PostStatistics AS
//        (SELECT P.Id AS PostId, P.Title, P.CreationDate, EXTRACT(EPOCH FROM (cast('2024-10-01 12:34:56' as timestamp) - P.CreationDate)) / 3600 AS AgeInHours, P.Score,
//        COALESCE(H.TypeCount, 0) AS HistoryCount, COALESCE(VoteCounts.UpVotes, 0) AS TotalUpVotes, COALESCE(VoteCounts.DownVotes, 0) AS TotalDownVotes FROM Posts P LEFT JOIN
//        (SELECT PostId, COUNT(*) AS TypeCount FROM PostHistory GROUP BY PostId) H ON P.Id = H.PostId LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS
//        UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY PostId) VoteCounts ON P.Id = VoteCounts.PostId), RankedPosts AS (SELECT
//        PS.PostId, PS.Title, PS.AgeInHours, PS.Score, PS.TotalUpVotes, PS.TotalDownVotes, RANK() OVER (ORDER BY PS.Score DESC, PS.AgeInHours ASC) AS ScoreRank FROM PostStatistics
//        PS) SELECT U.DisplayName, U.Reputation, UV.UpVotes, UV.DownVotes, COALESCE(UB.BadgeCount, 0) AS UserBadgeCount, COALESCE(UB.BadgeNames, 'No Badges') AS UserBadges,
//        RP.Title, RP.AgeInHours, RP.Score, RP.ScoreRank FROM UserVotes UV JOIN Users U ON UV.UserId = U.Id LEFT JOIN UserBadges UB ON U.Id = UB.UserId JOIN RankedPosts RP ON
//        RP.ScoreRank <= 10 WHERE (U.Reputation > 100 OR (U.Reputation IS NULL AND U.Location IS NOT NULL)) AND (RP.AgeInHours IS NOT NULL AND RP.AgeInHours < 72) ORDER BY
//        RP.ScoreRank, U.Reputation DESC;
//
// `JOIN RankedPosts RP ON RP.ScoreRank <= 10` does not mention the users, so it is a cross join.
fn q24444(db: &'static So) -> String {
    let Post { creation_date, post_type_id, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let age = |p: Id<Post>| secs(t0 - creation_date.get(p).unwrap()) / 3600.0;
    let rw = whole(&db.post.id).select(Ident::<Post>::new().and(&db.post.score).and(creation_date)).window(rank, |((_, s), d)| (Reverse(s), Reverse(d)), asc);
    type R = ((Id<Post>, i64), i64);
    let rp: MatSet<R> = (&rw).filt(move |(((_, _), d), k)| k <= 10 && secs(t0 - d) / 3600.0 < 72.0).map(|(((p, s), _), k)| ((p, s), k)).collect();
    let ov = own_votes(db);
    let uv = db
        .user
        .with((&db.user.reputation).gt(100))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(Ident::<Post>::new().and(post_type_id).and((&ov).select(&db.vote.vote_type_id).opt())).opt())
        .fold((0i64, 0i64), |(up, dn), x| match x {
            Some((_, t)) => (up + (t == Some(2)) as i64, dn + (t == Some(3)) as i64),
            None => (up, dn),
        });
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.name)).buf_fold(|v| (v.len() as i64, leak(v.join(", "))));
    let v = drain((&uv).and((&ub).opt()).cross(&rp));
    let v = top_n(v, |&((u, _), (_, (_, k)))| (k, Reverse(db.user.reputation.get(u).unwrap())), 0);
    rows(v.into_iter().map(|((u, _), (((up, dn), b), ((p, s), k)))| {
        let (bn, bs) = b.unwrap_or((0, "No Badges"));
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(up), V::I(dn), V::I(bn), V::S(bs)]);
        f.extend(post_fields(db, p, &["title"]));
        f.extend([V::F(age(p)), V::I(s), V::I(k)]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, COUNT(DISTINCT P.Id) AS PostCount, SUM(COALESCE(CASE WHEN V.VoteTypeId = 2 THEN
//        1 ELSE 0 END, 0)) AS TotalUpvotes, SUM(COALESCE(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END, 0)) AS TotalDownvotes, SUM(CASE WHEN B.Name IS NOT NULL THEN 1 ELSE 0 END)
//        AS BadgeCount, (SELECT COUNT(*) FROM Posts WHERE OwnerUserId = U.Id AND CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 YEAR') AS RecentPostsLastYear FROM
//        Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName, U.Reputation,
//        U.CreationDate), PostDetails AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, COALESCE(pc.CommentCount, 0) AS CommentCount,
//        COALESCE(a.AnswerCount, 0) AS AnswerCount, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.LastActivityDate DESC) AS rn FROM Posts P LEFT JOIN (SELECT PostId,
//        COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) pc ON P.Id = pc.PostId LEFT JOIN (SELECT ParentId, COUNT(*) AS AnswerCount FROM Posts WHERE PostTypeId = 2 GROUP
//        BY ParentId) a ON P.Id = a.ParentId WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 DAY') SELECT UA.UserId, UA.DisplayName, UA.Reputation,
//        PD.PostId, PD.Title, PD.CreationDate, PD.Score, PD.ViewCount, PD.CommentCount, PD.AnswerCount, CASE WHEN UA.Reputation > 1000 THEN 'Expert' WHEN UA.Reputation BETWEEN 500
//        AND 1000 THEN 'Intermediate' ELSE 'Novice' END AS ExpertiseLevel, ARRAY_AGG(DISTINCT B.Name) AS BadgeNames FROM UserActivity UA LEFT JOIN PostDetails PD ON UA.UserId =
//        PD.PostId LEFT JOIN Badges B ON UA.UserId = B.UserId WHERE (UA.Reputation IS NOT NULL OR UA.Reputation > 0) AND (COALESCE(PD.ViewCount, 0) - COALESCE(PD.CommentCount, 0)
//        > 10 OR PD.PostId IS NULL) AND PD.rn <= 3 GROUP BY UA.UserId, UA.DisplayName, UA.Reputation, PD.PostId, PD.Title, PD.CreationDate, PD.Score, PD.ViewCount,
//        PD.CommentCount, PD.AnswerCount ORDER BY UA.Reputation DESC, PD.Score DESC LIMIT 50;
//
// `UA.UserId = PD.PostId` joins a user id to a post id, as written; `PD.rn <= 3` makes it an inner join, and none of UserActivity's
// aggregates is read. A LastActivityDate tie inside the ROW_NUMBER goes to the smaller post id. The ARRAY_AGG(DISTINCT ...) is listed in name order.
fn q23227(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, last_activity_date, view_count, score, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(last_activity_date))
        .window(row_number, |(p, l)| (Reverse(l), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 3).map(|((p, _), _)| p).collect();
    let cc = comments_per_post(db);
    let ac = typed_answers_per_post(db);
    let po: HashIdx<i64, Id<Post>> = (&tp).select(&db.post.origid).inv().collect();
    let bn = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.name).opt()).buf_fold(|v| {
        let mut x: Vec<Option<Str>> = v.to_vec();
        x.sort();
        x.dedup();
        &*Box::leak(x.into_boxed_slice())
    });
    let v = drain(
        db.user
            .select(Ident::<User>::new().and((&db.user.origid).select(&po).select(Ident::<Post>::new().and(&cc).and(&ac).and(view_count.opt()))).and(&bn))
            .filt(|((_, (((_, c), _), w)), _): ((Id<User>, (((Id<Post>, i64), i64), Option<i64>)), &'static [Option<Str>])| w.unwrap_or(0) - c > 10),
    );
    let v = top_n(v, |&(_, ((u, (((p, _), _), _)), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(score.get(p).unwrap()), u, p), 50);
    rows(v.into_iter().map(|(_, ((u, (((p, c), a), _)), b))| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"]));
        f.extend([V::I(c), V::I(a)]);
        f.push(V::S(if r > 1000 {
            "Expert"
        } else if (500..=1000).contains(&r) {
            "Intermediate"
        } else {
            "Novice"
        }));
        f.push(V::L(b.iter().map(|x| ostr(*x)).collect()));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.PostTypeId, p.OwnerUserId, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate
//        DESC) AS PostRank, COUNT(*) OVER (PARTITION BY p.OwnerUserId) AS TotalPosts, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) OVER (PARTITION BY p.OwnerUserId) AS
//        TotalUpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) OVER (PARTITION BY p.OwnerUserId) AS TotalDownVotes FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId
//        WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.ViewCount IS NOT NULL), UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName,
//        u.Reputation, u.Location, COUNT(DISTINCT p.Id) AS PostCount, SUM(p.ViewCount) AS TotalViews, AVG(p.Score) AS AverageScore FROM Users u JOIN Posts p ON u.Id =
//        p.OwnerUserId WHERE u.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '3 years' GROUP BY u.Id, u.DisplayName, u.Reputation, u.Location), TopUsers AS (SELECT
//        us.UserId, us.DisplayName, us.Reputation, RANK() OVER (ORDER BY us.TotalViews DESC, us.AverageScore DESC) AS UserRank FROM UserStatistics us WHERE us.PostCount > 5),
//        PostChanges AS (SELECT ph.PostId, ph.PostHistoryTypeId, COUNT(*) AS ChangeCount FROM PostHistory ph GROUP BY ph.PostId, ph.PostHistoryTypeId), PostLinksAggregated AS
//        (SELECT pl.PostId, ARRAY_AGG(pl.RelatedPostId) AS RelatedPosts FROM PostLinks pl GROUP BY pl.PostId) SELECT p.PostId, p.Title, p.CreationDate, p.TotalPosts,
//        p.TotalUpVotes, p.TotalDownVotes, us.UserId, us.DisplayName AS UserName, us.Reputation, us.Location, tu.UserRank, COALESCE(pc.ChangeCount, 0) AS TotalChanges,
//        COALESCE(pla.RelatedPosts, ARRAY[]::BIGINT[]) AS RelatedPostIds FROM RankedPosts p JOIN UserStatistics us ON p.OwnerUserId = us.UserId LEFT JOIN TopUsers tu ON us.UserId
//        = tu.UserId LEFT JOIN PostChanges pc ON p.PostId = pc.PostId LEFT JOIN PostLinksAggregated pla ON p.PostId = pla.PostId WHERE (p.TotalDownVotes < p.TotalUpVotes OR
//        p.TotalUpVotes IS NULL) AND (p.PostRank <= 3 OR p.PostRank IS NULL) ORDER BY us.TotalViews DESC, p.CreationDate DESC LIMIT 100;
//
// PostRank numbers the joined rows; a CreationDate tie between different posts goes to the smaller post id, and rows equal on the final
// ORDER BY go in post id and change-type order (the SQL leaves both open). The ARRAY_AGG is listed in link id order.
fn q23670(db: &'static So) -> String {
    let Post { creation_date, view_count, owner_user, owner_user_id, score, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(view_count);
    let win = recent().group_by(owner_user_id.opt()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    type T = (Id<Post>, (Option<i64>, Option<(Id<Vote>, i64)>));
    let rw = recent()
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(creation_date).and(owner_user_id.opt().and(votes_of(db).select(Ident::<Vote>::new().and(&db.vote.vote_type_id)).opt())))
        .window(row_number, |((p, d), (_, v))| (Reverse(d), p, v.map(|v| v.0)), asc);
    let tv: MatSet<T> = (&rw).filt(|(_, k)| k <= 3).map(|(((p, _), x), _)| (p, x)).collect();
    let us = db
        .user
        .with((&db.user.creation_date).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -3)))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(view_count.opt().and(score)))
        .fold((0i64, 0i64, 0i64, 0i64), |(n, w, wn, s), (v, sc)| (n + 1, w + v.unwrap_or(0), wn + v.is_some() as i64, s + sc));
    type TU = ((Id<User>, (i64, i64, i64, i64)), i64);
    let tw = whole(db.user.with((&us).filt(|x: (i64, i64, i64, i64)| x.0 > 5)))
        .select(Ident::<User>::new().and(&us))
        .window(rank, |(_, (n, w, wn, s))| (wn == 0, Reverse(w), Reverse(fkey(s as f64 / n as f64))), asc);
    let tui = by_first(&(&tw).map(|x: TU| (x.0 .0, x)).collect());
    let pc = db.post_history.group_by((&db.post_history.post).and(&db.post_history.post_history_type_id)).fold(0i64, |a, _| a + 1);
    type PC = ((Id<Post>, i64), i64);
    let pcv = rel(drain(&pc));
    let pci: HashIdx<Id<Post>, PC> = (&pcv).map(|x: PC| x.0 .0).inv().select(&pcv).collect();
    let pla = db.post_link.group_by(&db.post_link.post).select(Ident::<PostLink>::new().and(&db.post_link.related_post_id)).buf_fold(|v| {
        let mut x = v.to_vec();
        x.sort();
        &*Box::leak(x.into_iter().map(|y| y.1).collect::<Vec<i64>>().into_boxed_slice())
    });
    let k = || Same::<T>::new().map(|x: T| x.0);
    type X = ((((T, [i64; 3]), (Id<User>, (i64, i64, i64, i64))), Option<TU>), Option<PC>);
    let v = drain(
        (&tv)
            .select(
                Same::<T>::new()
                    .and(Same::<T>::new().map(|x: T| x.1 .0).select(&win))
                    .and(k().select(owner_user.select(Ident::<User>::new().and(&us))))
                    .and(k().select(owner_user).select((&tui).opt()))
                    .and(k().select((&pci).opt())),
            )
            .filt(|((((_, a), _), _), _): X| a[2] < a[1])
            .select(Same::<X>::new().and(Same::<X>::new().map(|x: X| x.0 .0 .0 .0 .0).select((&pla).opt()))),
    );
    let v = top_n(v, |&(_, (((((t, _), (_, (_, w, wn, _))), _), c), _))| (wn == 0, Reverse(w), Reverse(creation_date.get(t.0).unwrap()), t.0, c.map(|c| c.0 .1), t.1 .1.map(|v| v.0)), 100);
    rows(v.into_iter().map(|(_, (((((t, a), (u, _)), r), c), l))| {
        let mut f = post_fields(db, t.0, &["id", "title", "created"]);
        f.extend(a.map(V::I));
        f.extend(ucols(db, u, &["uid", "name", "rep"]));
        f.push(ostr(db.user.location.get(u)));
        f.push(r.map_or(V::Null, |r| V::I(r.1)));
        f.push(V::I(c.map_or(0, |c| c.1)));
        f.push(V::L(l.unwrap_or(&[]).iter().map(|x| V::I(*x)).collect()));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RN,
//        COUNT(*) OVER (PARTITION BY p.PostTypeId) AS TotalPosts FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month'), UserActivity
//        AS (SELECT u.Id AS UserId, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounties, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes, COALESCE(SUM(CASE
//        WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes, COUNT(DISTINCT ph.PostId) AS PostsEdited FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN
//        PostHistory ph ON u.Id = ph.UserId GROUP BY u.Id), PopularTags AS (SELECT t.TagName, COUNT(DISTINCT p.Id) AS PostCount FROM Tags t JOIN Posts p ON p.Tags LIKE '%' ||
//        t.TagName || '%' WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month' GROUP BY t.TagName HAVING COUNT(DISTINCT p.Id) >= 5), PostHistories
//        AS (SELECT ph.PostId, ph.PostHistoryTypeId, ph.CreationDate, COUNT(*) OVER (PARTITION BY ph.PostId) AS EditCount, STRING_AGG(DISTINCT ph.Comment, '; ') AS UserComments
//        FROM PostHistory ph WHERE ph.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '6 months' GROUP BY ph.PostId, ph.PostHistoryTypeId, ph.CreationDate)
//        SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, up.UserId, up.TotalBounties, up.Upvotes, up.Downvotes, up.PostsEdited, pt.TagName, ph.EditCount,
//        ph.UserComments, CASE WHEN rp.Score > 100 THEN 'Hot' WHEN rp.Score BETWEEN 50 AND 100 THEN 'Trending' ELSE 'New' END AS PostStatus, CASE WHEN ph.UserComments IS NULL THEN
//        'No Comments' ELSE 'Comments Present' END AS CommentStatus, CASE WHEN up.TotalBounties IS NULL OR up.TotalBounties = 0 THEN 'No Bounties' ELSE CONCAT(up.TotalBounties, '
//        Bounty Points') END AS BountyStatus FROM RankedPosts rp LEFT JOIN UserActivity up ON up.UserId = rp.PostId LEFT JOIN PopularTags pt ON pt.PostCount > 10 LEFT JOIN
//        PostHistories ph ON ph.PostId = rp.PostId WHERE rp.RN <= 5 ORDER BY rp.Score DESC, rp.ViewCount DESC;
//
// `up.UserId = rp.PostId` joins a user id to a post id, as written. `ON pt.PostCount > 10` does not mention rp, so the popular tags are crossed
// with every row. A score tie inside the ROW_NUMBER goes to the smaller post id (the SQL leaves it open). The distinct comments are joined in sorted order.
fn q20439(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, origid, tags_str, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let recent = || db.post.with(creation_date.ge(add_months(t0, -1)));
    let w = recent().group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|((p, _), _)| p).collect();
    let uo = users_by_origid(db);
    let mu: MatSet<Id<User>> = (&tp).select(origid).select(&uo).collect();
    let hb: HashIdx<Id<User>, Id<PostHistory>> = (&db.post_history.user).inv().collect();
    let ua3 = (&mu)
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select((&db.vote.bounty_amount).opt().and(&db.vote.vote_type_id)).opt().and((&hb).opt()))
        .fold([0i64; 3], |a, (x, _)| [a[0] + x.and_then(|y| y.0).unwrap_or(0), a[1] + (x.map(|y| y.1) == Some(2)) as i64, a[2] + (x.map(|y| y.1) == Some(3)) as i64]);
    let ued = (&mu).group_by(Ident::<User>::new()).select((&hb).select(&db.post_history.post_id)).count_distinct();
    let ua = (&ua3).and((&ued).opt()).map(|(a, e): ([i64; 3], Option<i64>)| [a[0], a[1], a[2], e.unwrap_or(0)]);
    let by_tags: HashIdx<Str, Id<Post>> = recent().select(tags_str).inv().collect();
    let ptc = db.tag.group_by(&db.tag.tag_name).select((&db.tag.tag_name).select_where(&by_tags, |n: Str, s: Str| like(s, &format!("%{n}%")))).count_distinct();
    let pti: HashIdx<(), Str> = db.tag.select(&db.tag.tag_name).with((&ptc).filt(|n| n >= 5 && n > 10)).collect::<MatSet<Str>>().map(|_| ()).inv().collect();
    let PostHistory { post, post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let recent_h = || db.post_history.with(hd.ge(add_months(t0, -6)));
    let phg = recent_h().group_by(post.and(post_history_type_id).and(hd)).select(comment.opt()).buf_fold(|v| agg_distinct(v.iter().flatten().copied().collect(), "; "));
    type H = (((Id<Post>, i64), i64), Option<Str>);
    let hv = rel(drain(&phg));
    let hi: HashIdx<Id<Post>, H> = (&hv).map(|x: H| x.0 .0 .0).inv().select(&hv).collect();
    let ec = recent_h().group_by(post).select(post_history_type_id.and(hd)).count_distinct();
    let v = drain(
        (&tp).select(
            Ident::<Post>::new()
                .and(origid.select(&uo).select(Ident::<User>::new().and(&ua)).opt())
                .and((&hi).opt())
                .and((&ec).opt())
                .and(Ident::<Post>::new().map(|_| ()).select((&pti).opt())),
        ),
    );
    rows(v.into_iter().map(|(_, ((((p, u), h), e), t))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        match u {
            Some((u, a)) => {
                f.push(user_col(db, u, "uid"));
                f.extend(a.map(V::I));
            }
            None => f.extend([V::Null, V::Null, V::Null, V::Null, V::Null]),
        }
        f.push(ostr(t));
        let c = h.and_then(|h| h.1);
        f.extend([if h.is_some() { oint(e) } else { V::Null }, ostr(c)]);
        f.push(V::S(if s > 100 {
            "Hot"
        } else if (50..=100).contains(&s) {
            "Trending"
        } else {
            "New"
        }));
        f.push(V::S(if c.is_none() { "No Comments" } else { "Comments Present" }));
        f.push(match u {
            Some((_, a)) if a[0] != 0 => V::Owned(format!("{} Bounty Points", a[0])),
            _ => V::S("No Bounties"),
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank
//        FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'), UserReputationHistory AS (SELECT u.Id AS UserId,
//        u.Reputation, u.CreationDate, DENSE_RANK() OVER (PARTITION BY u.Id ORDER BY u.CreationDate) AS ReputationRank FROM Users u WHERE u.Reputation > 0), AverageScores AS
//        (SELECT p.OwnerUserId, AVG(p.Score) AS AvgScore FROM Posts p GROUP BY p.OwnerUserId), ClosedPosts AS (SELECT ph.PostId, ph.CreationDate, ph.Comment, ph.UserId FROM
//        PostHistory ph WHERE ph.PostHistoryTypeId = 10), PostMetrics AS (SELECT p.Id AS PostId, COALESCE(v.UpVotes, 0) AS UpVotes, COALESCE(v.DownVotes, 0) AS DownVotes,
//        COALESCE(v.FavoriteCount, 0) AS FavoriteCount, COALESCE(c.CommentCount, 0) AS CommentCount FROM Posts p LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE
//        0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN VoteTypeId = 5 THEN 1 ELSE 0 END) AS FavoriteCount FROM Votes GROUP BY
//        PostId) v ON p.Id = v.PostId LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId) SELECT up.DisplayName AS
//        UserDisplayName, COUNT(DISTINCT rp.PostId) AS QuestionCount, MAX(uh.Reputation) AS MaxReputation, AVG(pm.AvgScore) AS AvgPostScore, COUNT(DISTINCT cp.PostId) AS
//        ClosedPostCount, STRING_AGG(DISTINCT pt.Name, ', ') AS PostTypes, COUNT(DISTINCT pt.Id) FILTER (WHERE pt.Id IS NOT NULL) AS DistinctPostTypeCount, CASE WHEN
//        MAX(uh.Reputation) IS NULL THEN 'No Reputation' ELSE 'Has Reputation' END AS ReputationStatus FROM RankedPosts rp LEFT JOIN Users up ON rp.OwnerUserId = up.Id JOIN
//        UserReputationHistory uh ON uh.UserId = up.Id LEFT JOIN AverageScores pm ON pm.OwnerUserId = up.Id LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId LEFT JOIN PostTypes
//        pt ON rp.PostId = pt.Id GROUP BY up.DisplayName HAVING COUNT(DISTINCT rp.PostId) > 5 ORDER BY MaxReputation DESC, QuestionCount DESC;
//
// `pt.Id = rp.PostId` joins a post-type id to a post id, as written. The groups are keyed by DisplayName, so users sharing a name share a row.
// The distinct type names are joined in name order.
fn q21580(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, score, origid, .. } = &db.post;
    let asc = db.post.group_by(owner_user).select(score).fold((0i64, 0i64), |(s, n), x| (s + x, n + 1));
    let pto: HashIdx<i64, Id<PostType>> = (&db.post_type.origid).inv().collect();
    let cp = || history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    let User { reputation, display_name, .. } = &db.user;
    let gk = || {
        db.post
            .with(post_type_id.eq(1))
            .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
            .with(owner_user.select(Ident::<User>::new().with(reputation.gt(0))))
            .group_by(owner_user.select(display_name))
    };
    let q = gk().select(Ident::<Post>::new()).count_distinct();
    let c = gk().select(Ident::<Post>::new().with(cp())).count_distinct();
    let nt = gk().select(origid.select(&pto)).count_distinct();
    let g = gk().select(owner_user.select(reputation.and((&asc).opt())).and(cp().opt()).and(origid.select(&pto).select(&db.post_type.name).opt())).buf_fold(|v| {
        let m = v.iter().map(|x| x.0 .0 .0).max().unwrap();
        let a: Vec<f64> = v.iter().filter_map(|x| x.0 .0 .1.map(|(s, n)| s as f64 / n as f64)).collect();
        let av = if a.is_empty() { None } else { Some(a.iter().fold(0.0, |s, x| s + x) / a.len() as f64) };
        (m, av, agg_distinct(v.iter().filter_map(|x| x.1).collect(), ", "))
    });
    rows(drain((&q).filt(|n| n > 5).and(&g).and((&c).opt()).and((&nt).opt())).into_iter().map(|(n, (((q, (m, a, t)), c), nt))| {
        row(vec![V::S(n), V::I(q), V::I(m), ofloat(a), V::I(c.unwrap_or(0)), ostr(t), V::I(nt.unwrap_or(0)), V::S("Has Reputation")])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY CASE WHEN p.ParentId IS NOT NULL THEN 1
//        ELSE 0 END ORDER BY p.CreationDate DESC) AS rn, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id), 0) AS UpVotesCount, COALESCE(SUM(CASE
//        WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id), 0) AS DownVotesCount FROM Posts p LEFT JOIN Votes v ON v.PostId = p.Id WHERE p.CreationDate >=
//        CURRENT_TIMESTAMP - INTERVAL '1 year'), UserAnalysis AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT b.Id) AS BadgeCount, AVG(u.Reputation) AS AvgReputation,
//        COUNT(DISTINCT p.Id) AS PostsCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount
//        FROM Users u LEFT JOIN Badges b ON b.UserId = u.Id LEFT JOIN Posts p ON p.OwnerUserId = u.Id WHERE u.LastAccessDate > CURRENT_TIMESTAMP - INTERVAL '6 months' GROUP BY
//        u.Id, u.DisplayName), PostHistoryDetails AS (SELECT ph.PostId, STRING_AGG(DISTINCT CONCAT(ph.Comment, ' (', CAST(ph.CreationDate AS date), ')'), '; ') AS HistoryComments,
//        COUNT(*) AS HistoryCount FROM PostHistory ph WHERE ph.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '3 months' GROUP BY ph.PostId), FinalReport AS (SELECT up.UserId,
//        up.DisplayName, up.AvgReputation, up.PostsCount, up.QuestionsCount, up.AnswersCount, rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.AnswerCount, rp.UpVotesCount,
//        rp.DownVotesCount, COALESCE(ph.HistoryComments, 'No history') AS PostHistory, ph.HistoryCount AS PostHistoryCount FROM UserAnalysis up INNER JOIN RankedPosts rp ON
//        up.UserId = rp.PostId LEFT JOIN PostHistoryDetails ph ON rp.PostId = ph.PostId WHERE rp.rn = 1 ORDER BY up.AvgReputation DESC, rp.ViewCount DESC) SELECT * FROM
//        FinalReport WHERE COALESCE(PostHistoryCount, 0) > 1 AND UpVotesCount > DownVotesCount AND (QuestionsCount > 0 OR AnswersCount > 0) AND NOT EXISTS (SELECT 1 FROM Posts p
//        WHERE p.OwnerUserId = FinalReport.UserId AND p.CreationDate < CURRENT_TIMESTAMP - INTERVAL '1 year') ORDER BY CreationDate DESC LIMIT 50 OFFSET 0;
//
// `up.UserId = rp.PostId` joins a user id to a post id, as written. CURRENT_TIMESTAMP is taken in the session zone, America/New_York.
// A tie inside the ROW_NUMBER goes to the smaller post id. The distinct history strings are joined in sorted order.
fn q24317(db: &'static So) -> String {
    let Post { creation_date, parent_id, origid, post_type_id, .. } = &db.post;
    let now = utc_to_ny(now_utc());
    let after = |d: i64, c: i64| ny_to_utc(d) >= ny_to_utc(c);
    let y1 = add_years(now, -1);
    let w = db
        .post
        .with(creation_date.filt(move |d| after(d, y1)))
        .group_by(parent_id.opt().map(|x: Option<i64>| x.is_some()))
        .select(Ident::<Post>::new().and(creation_date).and(votes_of(db).opt()))
        .window(row_number, |((p, d), v)| (Reverse(d), p, v), asc);
    let rp = || (&w).filt(|(_, k)| k == 1).map(|(((p, _), _), _)| p);
    let tp: MatSet<Id<Post>> = rp().collect();
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let uo = users_by_origid(db);
    let m6 = add_months(now, -6);
    let mu: MatSet<Id<User>> = (&tp).select(origid).select(&uo).select(Ident::<User>::new().with((&db.user.last_access_date).filt(move |d| ny_to_utc(d) > ny_to_utc(m6)))).collect();
    let mg = || (&mu).group_by(Ident::<User>::new());
    let qa = mg().select(badges_of(db).opt().and(posts_of(db).select(post_type_id).opt())).fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(1)) as i64, a[1] + (t == Some(2)) as i64]);
    let nb = mg().select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let np = mg().select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let ua = (&nb).and(&np).and(&qa).map(|((b, p), a): ((i64, i64), [i64; 2])| [b, p, a[0], a[1]]);
    let old = add_years(now, -1);
    let has_old = posts_of(db).select(Ident::<Post>::new().with(creation_date.filt(move |d| !after(d, old))));
    let m3 = add_months(now, -3);
    let PostHistory { post, comment, creation_date: hd, .. } = &db.post_history;
    let phd = db
        .post_history
        .with(hd.filt(move |d| after(d, m3)))
        .group_by(post)
        .select(comment.opt().and(hd))
        .buf_fold(|v| (agg_distinct(v.iter().map(|(c, d)| leak(format!("{} ({})", c.unwrap_or(""), fmt_date(*d)))).collect(), "; ").unwrap(), v.len() as i64));
    type X = (((Id<Post>, [i64; 2]), (Id<User>, [i64; 4])), Option<(Str, i64)>);
    let v = drain(
        rp()
            .select(Ident::<Post>::new().and(&vc).and(origid.select(&uo).select(Ident::<User>::new().with(&mu).minus(has_old).and(&ua))).and((&phd).opt()))
            .filt(|(((_, a), (_, u)), h): X| h.map_or(0, |h| h.1) > 1 && a[0] > a[1] && (u[2] > 0 || u[3] > 0)),
    );
    let v = top_n(v, |&(_, (((p, _), _), _))| (Reverse(creation_date.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|(_, (((p, a), (u, x)), h))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::F(db.user.reputation.get(u).unwrap() as f64));
        f.extend([V::I(x[1]), V::I(x[2]), V::I(x[3])]);
        f.extend(post_fields(db, p, &["id", "title", "created", "views", "answers"]));
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(match h {
            Some((s, n)) => [V::S(s), V::I(n)],
            None => [V::S("No history"), V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.ViewCount DESC, p.Score
//        DESC) AS Rank, COALESCE(p.AcceptedAnswerId, -1) AS AnswerStatus FROM Posts p WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' AND p.ViewCount IS NOT
//        NULL), PostStatistics AS (SELECT u.Id AS UserId, SUM(CASE WHEN p.AnswerCount > 0 THEN 1 ELSE 0 END) AS TotalQuestionsAnswered, COUNT(DISTINCT p.Id) AS TotalPosts,
//        AVG(p.Score) AS AverageScore, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN
//        b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
//        ClosedPostReasons AS (SELECT ph.PostId, COUNT(*) AS CloseVoteCount, STRING_AGG(CASE WHEN ph.PostHistoryTypeId = 10 THEN cr.Name END, ', ') AS CloseReasons FROM
//        PostHistory ph LEFT JOIN CloseReasonTypes cr ON cr.Id::text = ph.Comment GROUP BY ph.PostId), UserPostLinkages AS (SELECT pl.PostId, pl.RelatedPostId, COUNT(pl.Id) AS
//        LinkCount FROM PostLinks pl JOIN Posts p ON pl.PostId = p.Id WHERE p.CreationDate < cast('2024-10-01' as date) - INTERVAL '6 months' GROUP BY pl.PostId,
//        pl.RelatedPostId), FinalStats AS (SELECT ps.UserId, ps.TotalQuestionsAnswered, ps.TotalPosts, ps.AverageScore, ps.GoldBadges, ps.SilverBadges, ps.BronzeBadges,
//        COALESCE(rp.PostId, 0) AS TopPostId, COALESCE(rp.Title, 'No Trending Post') AS TopPostTitle, COALESCE(rp.ViewCount, 0) AS TopPostViewCount, COALESCE(rp.Score, 0) AS
//        TopPostScore, COALESCE(cpr.CloseVoteCount, 0) AS TotalCloseVotes, COALESCE(cpr.CloseReasons, 'No Close Reasons') AS CloseReasons, COALESCE(pl.LinkCount, 0) AS
//        TotalRelatedLinks FROM PostStatistics ps LEFT JOIN RankedPosts rp ON ps.UserId = rp.PostId LEFT JOIN ClosedPostReasons cpr ON rp.PostId = cpr.PostId LEFT JOIN
//        UserPostLinkages pl ON ps.UserId = pl.PostId) SELECT *, CASE WHEN TotalPosts = 0 THEN 'No activity' ELSE 'Active User' END AS UserActivityStatus FROM FinalStats WHERE
//        TotalQuestionsAnswered > 5 AND GoldBadges > 0 ORDER BY TotalPosts DESC, AverageScore DESC;
//
// `ps.UserId = rp.PostId` and `ps.UserId = pl.PostId` join a user id to a post id, as written. The STRING_AGG order is left open; the port
// joins in history id order.
fn q20832(db: &'static So) -> String {
    let Post { creation_date, view_count, answer_count, score, origid, .. } = &db.post;
    let ps = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(answer_count.opt().and(score)).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 6], |a, (p, c)| {
            [
                a[0] + p.map_or(false, |p| p.0.map_or(false, |x| x > 0)) as i64,
                a[1] + p.map_or(0, |p| p.1),
                a[2] + p.is_some() as i64,
                a[3] + (c == Some(1)) as i64,
                a[4] + (c == Some(2)) as i64,
                a[5] + (c == Some(3)) as i64,
            ]
        });
    let ud = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let rpo: HashIdx<i64, Id<Post>> = db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).with(view_count).select(origid).inv().collect();
    let crt: HashIdx<Str, Id<CloseReasonType>> = (&db.close_reason_type.origid).map(|i: i64| -> Str { leak(i.to_string()) }).inv().collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cpr = db
        .post_history
        .group_by(post)
        .select(post_history_type_id.and(comment.select(&crt).select(&db.close_reason_type.name).opt()))
        .buf_fold(|v| {
            let n: Vec<Str> = v.iter().filter(|x| x.0 == 10).filter_map(|x| x.1).collect();
            (v.len() as i64, if n.is_empty() { None } else { Some(leak(n.join(", "))) })
        });
    let PostLink { post_id, related_post_id, .. } = &db.post_link;
    let plg = db
        .post_link
        .with((&db.post_link.post).select(Ident::<Post>::new().with(creation_date.lt(add_months(date(2024, 10, 1), -6)))))
        .group_by(post_id.and(related_post_id))
        .fold(0i64, |a, _| a + 1);
    type L = ((i64, i64), i64);
    let plv = rel(drain(&plg));
    let pli: HashIdx<i64, L> = (&plv).map(|x: L| x.0 .0).inv().select(&plv).collect();
    let uor = &db.user.origid;
    let v = drain(
        (&ps)
            .filt(|a: [i64; 6]| a[0] > 5 && a[3] > 0)
            .and(&ud)
            .and(uor.select(&rpo).select(Ident::<Post>::new().and((&cpr).opt())).opt())
            .and(uor.select(&pli).opt()),
    );
    rows(v.into_iter().map(|(u, (((a, n), r), l))| {
        let mut f = vec![user_col(db, u, "uid"), V::I(a[0]), V::I(n), avg(a[1], a[2]), V::I(a[3]), V::I(a[4]), V::I(a[5])];
        match r {
            Some((p, c)) => {
                f.extend(post_fields(db, p, &["id", "title"]));
                f.extend([V::I(view_count.get(p).unwrap_or(0)), V::I(score.get(p).unwrap())]);
                f.extend(match c {
                    Some((k, s)) => [V::I(k), V::S(s.unwrap_or("No Close Reasons"))],
                    None => [V::I(0), V::S("No Close Reasons")],
                });
            }
            None => f.extend([V::I(0), V::S("No Trending Post"), V::I(0), V::I(0), V::I(0), V::S("No Close Reasons")]),
        }
        f.push(V::I(l.map_or(0, |l| l.1)));
        f.push(V::S(if n == 0 { "No activity" } else { "Active User" }));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpvotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownvotes, COALESCE(COUNT(DISTINCT p.Id), 0) AS TotalPosts, COALESCE(SUM(CASE WHEN b.Class = 1 THEN
//        1 ELSE 0 END), 0) AS GoldBadges, COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges, COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS
//        BronzeBadges FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id,
//        u.DisplayName, u.Reputation, u.CreationDate), PostActivity AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, COUNT(CASE WHEN c.Id IS NOT NULL THEN
//        1 END) AS TotalComments, SUM(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS TotalCloseVotes FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN
//        PostHistory ph ON p.Id = ph.PostId WHERE p.CreationDate >= (CURRENT_DATE - INTERVAL '1 year') GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score), RankedPosts
//        AS (SELECT pa.*, RANK() OVER (ORDER BY pa.Score DESC, pa.ViewCount DESC) AS PostRank FROM PostActivity pa), BizarreLogic AS (SELECT ups.UserId, SUM(CASE WHEN
//        ups.TotalUpvotes > ups.TotalDownvotes THEN 1 ELSE 0 END) AS PositiveContributions, SUM(CASE WHEN ups.TotalDownvotes > ups.TotalUpvotes THEN 1 ELSE 0 END) AS
//        NegativeContributions, COUNT(DISTINCT rp.PostId) AS ContributingPosts, STRING_AGG(rp.Title, '; ') AS PostTitles FROM UserStats ups LEFT JOIN RankedPosts rp ON ups.UserId
//        = rp.PostId WHERE ups.Reputation > 100 AND (ups.GoldBadges + ups.SilverBadges + ups.BronzeBadges) >= 1 GROUP BY ups.UserId) SELECT bl.UserId, u.DisplayName,
//        bl.PositiveContributions, bl.NegativeContributions, bl.ContributingPosts, bl.PostTitles, CASE WHEN bl.PositiveContributions > bl.NegativeContributions THEN 'Overall
//        Positive Contributor' WHEN bl.NegativeContributions > bl.PositiveContributions THEN 'Overall Negative Contributor' ELSE 'Neutral Contributor' END AS ContributionType FROM
//        BizarreLogic bl JOIN Users u ON bl.UserId = u.Id WHERE bl.ContributingPosts > 0 ORDER BY bl.PositiveContributions DESC, bl.NegativeContributions ASC;
//
// `ups.UserId = rp.PostId` joins a user id to a post id, as written; ContributingPosts > 0 keeps only the users that match a post, so
// UserStats is computed for those alone. Each group row is a distinct post, so COUNT(DISTINCT rp.PostId) is the row count. The STRING_AGG order
// is left open (one title per user here).
fn q23267(db: &'static So) -> String {
    let Post { creation_date, origid, .. } = &db.post;
    let rp: HashIdx<i64, Id<Post>> = db.post.with(creation_date.ge(add_years(current_date(), -1))).select(origid).inv().collect();
    let mu: MatSet<Id<User>> = db.user.with((&db.user.reputation).gt(100)).with(badges_of(db)).with((&db.user.origid).select(&rp)).collect();
    let us = (&mu)
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(posts_of(db).opt()).and(badges_of(db).opt()))
        .fold([0i64; 2], |a, ((t, _), _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let bl = (&mu)
        .group_by(Ident::<User>::new())
        .select((&us).and((&db.user.origid).select(&rp).select(Ident::<Post>::new().and((&db.post.title).opt()))))
        .buf_fold(|v| {
            let pc = v.iter().filter(|x| x.0[0] > x.0[1]).count() as i64;
            let nc = v.iter().filter(|x| x.0[1] > x.0[0]).count() as i64;
            let n = v.len() as i64;
            let t: Vec<Str> = v.iter().filter_map(|x| x.1 .1).collect();
            (pc, nc, n, if t.is_empty() { None } else { Some(leak(t.join("; "))) })
        });
    let v = top_n(drain((&bl).filt(|x: (i64, i64, i64, Option<Str>)| x.2 > 0)), |&(u, (p, n, _, _))| (Reverse(p), n, u), 0);
    rows(v.into_iter().map(|(u, (p, n, c, t))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(p), V::I(n), V::I(c), ostr(t)]);
        f.push(V::S(if p > n {
            "Overall Positive Contributor"
        } else if n > p {
            "Overall Negative Contributor"
        } else {
            "Neutral Contributor"
        }));
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN
//        v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 4 THEN 1 ELSE 0 END), 0) AS OffensiveVotes, COUNT(DISTINCT CASE WHEN
//        p.PostTypeId = 1 THEN p.Id END) AS QuestionCount, COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS AnswerCount FROM Users u LEFT JOIN Votes v ON u.Id =
//        v.UserId LEFT JOIN Posts p ON v.PostId = p.Id GROUP BY u.Id, u.DisplayName), PopularQuestions AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, ROW_NUMBER()
//        OVER (ORDER BY p.Score DESC) AS PopularityRank FROM Posts p WHERE p.PostTypeId = 1 AND p.Score IS NOT NULL AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp)
//        - INTERVAL '30 days'), RecentActivities AS (SELECT p.Id AS PostId, p.Title, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT ph.Id) AS EditCount, MAX(c.CreationDate) AS
//        LastCommentDate FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId AND ph.PostHistoryTypeId IN (5, 4) WHERE p.PostTypeId =
//        1 GROUP BY p.Id, p.Title), CombinedStats AS (SELECT u.UserId, u.DisplayName, COALESCE(up.UpVotes, 0) AS TotalUpVotes, COALESCE(down.DownVotes, 0) AS TotalDownVotes,
//        COALESCE(pq.PostId, 0) AS PopularPostId, COALESCE(pq.Title, 'N/A') AS PopularPostTitle, COALESCE(pq.Score, 0) AS PopularPostScore, COALESCE(ra.CommentCount, 0) AS
//        RecentCommentCount, COALESCE(ra.LastCommentDate, '1900-01-01') AS LastCommentDate FROM UserVoteStats u LEFT JOIN UserVoteStats up ON u.UserId = up.UserId AND up.UpVotes >
//        0 LEFT JOIN UserVoteStats down ON u.UserId = down.UserId AND down.DownVotes > 0 LEFT JOIN PopularQuestions pq ON pq.PopularityRank = 1 LEFT JOIN RecentActivities ra ON
//        ra.PostId IS NOT NULL AND ra.CommentCount > 0) SELECT cs.DisplayName, cs.TotalUpVotes, cs.TotalDownVotes, cs.PopularPostTitle, cs.PopularPostScore, cs.RecentCommentCount,
//        (CASE WHEN cs.TotalUpVotes > cs.TotalDownVotes THEN 'Active User' WHEN cs.TotalUpVotes = cs.TotalDownVotes THEN 'Neutral' ELSE 'Inactive User' END) AS UserActivityStatus,
//        (SELECT STRING_AGG(DISTINCT COALESCE(t.TagName, 'No Tags'), ', ') FROM Posts p LEFT JOIN Tags t ON p.Id = t.ExcerptPostId WHERE p.OwnerUserId = cs.UserId) AS UserTags
//        FROM CombinedStats cs WHERE cs.TotalUpVotes > 0 OR cs.TotalDownVotes > 0 ORDER BY cs.TotalUpVotes DESC, cs.TotalDownVotes ASC LIMIT 50;
//
// PopularQuestions and RecentActivities are joined on conditions that do not mention the users, so both are cross joins. A score tie inside the
// ROW_NUMBER goes to the smaller post id (the SQL leaves it open). The distinct tag names are joined in name order.
fn q21216(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, .. } = &db.post;
    let uvs = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select((&db.vote.vote_type_id).and((&db.vote.post).select(Ident::<Post>::new().and(post_type_id)).opt())).opt())
        .fold([0i64; 2], |a, x| [a[0] + (x.map(|y| y.0) == Some(2)) as i64, a[1] + (x.map(|y| y.0) == Some(3)) as i64]);
    let pw = whole(db.post.with(post_type_id.eq(1)).with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))))
        .select(Ident::<Post>::new().and(score))
        .window(row_number, |(p, s)| (Reverse(s), p), asc);
    let pqi: HashIdx<(), Id<Post>> = (&pw).filt(|(_, k)| k == 1).map(|((p, _), _)| p).collect();
    let ra = db
        .post
        .with(post_type_id.eq(1))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([4, 5]))).opt()))
        .fold(0i64, |a, (c, _)| a + c.is_some() as i64);
    let rai: HashIdx<(), (Id<Post>, i64)> = db.post.select(Ident::<Post>::new().and((&ra).filt(|n| n > 0))).map(|x: (Id<Post>, i64)| x).collect::<MatSet<(Id<Post>, i64)>>().map(|_| ()).inv().collect();
    let ex: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    let ut = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(ex.select(&db.tag.tag_name).opt()))
        .buf_fold(|v| agg_distinct(v.iter().map(|x| x.unwrap_or("No Tags")).collect(), ", ").unwrap());
    let uw = whole(db.user.with((&uvs).filt(|a: [i64; 2]| a[0] > 0 || a[1] > 0)))
        .select(Ident::<User>::new().and(&uvs))
        .window(rank, |(_, a)| (Reverse(a[0]), a[1]), asc);
    let cand: MatSet<Id<User>> = (&uw).filt(|(_, k)| k <= 50).map(|((u, _), _)| u).collect();
    let unit = || Ident::<User>::new().map(|_| ());
    let v = drain((&cand).select(Ident::<User>::new().and(&uvs).and((&ut).opt()).and(unit().select((&pqi).opt())).and(unit().select((&rai).opt()))));
    let v = top_n(v, |&(_, ((((u, a), _), _), r))| (Reverse(a[0]), a[1], u, r.map(|r| r.0)), 50);
    rows(v.into_iter().map(|(_, ((((u, a), t), pq), r))| {
        let n = r.map_or(0, |r| r.1);
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1])];
        match pq {
            Some(p) => f.extend(post_fields(db, p, &["title", "score"])),
            None => f.extend([V::S("N/A"), V::I(0)]),
        }
        f.push(V::I(n));
        f.push(V::S(if a[0] > a[1] {
            "Active User"
        } else if a[0] == a[1] {
            "Neutral"
        } else {
            "Inactive User"
        }));
        f.push(ostr(t));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.Score, p.ViewCount, p.AnswerCount, u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY p.Score
//        DESC, p.Id) AS rn, COUNT(c.Id) AS CommentCount, STRING_AGG(t.TagName, ', ' ORDER BY t.TagName) AS TagsList FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN
//        Comments c ON p.Id = c.PostId LEFT JOIN UNNEST(string_to_array(p.Tags, '><')) AS t(TagName) ON TRUE WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1
//        year' GROUP BY p.Id, p.Title, p.Score, p.ViewCount, p.AnswerCount, u.DisplayName, p.Tags), FilteredPosts AS (SELECT rp.* FROM RankedPosts rp WHERE rp.rn = 1) SELECT
//        fp.Id, fp.Title, fp.Score, fp.ViewCount, fp.AnswerCount, fp.CommentCount, fp.OwnerDisplayName, fp.TagsList FROM FilteredPosts fp WHERE fp.Score > 10 ORDER BY fp.Score
//        DESC, fp.ViewCount DESC FETCH FIRST 100 ROWS ONLY;
//
// Rewritten (rewrites/9728.sql): the ROW_NUMBER gets `, p.Id` and the STRING_AGG gets ORDER BY its own argument. The tag list is taken over
// the comments x tags product, so each tag appears once per comment row.
fn q9728(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, tags_str, view_count, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).group_by(tags_str.opt()).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let fp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k == 1).map(|((p, _), _)| p).collect();
    let g = (&fp)
        .with(score.gt(10))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(tags_str.flat_map(|t: Str| t.split("><")).opt()))
        .buf_fold(|v| {
            let c = v.iter().filter(|x| x.0.is_some()).count() as i64;
            let mut t: Vec<Str> = v.iter().filter_map(|x| x.1).collect();
            t.sort();
            (c, if t.is_empty() { None } else { Some(leak(t.join(", "))) })
        });
    let v = top_n(drain(&g), |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    }, 100);
    rows(v.into_iter().map(|(p, (c, t))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "answers"]);
        f.push(V::I(c));
        f.push(post_fields(db, p, &["owner"]).remove(0));
        f.push(ostr(t));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, COUNT(c.Id) AS CommentCount, ARRAY_AGG(DISTINCT t.TagName) AS Tags,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank, p.OwnerUserId FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN
//        UNNEST(string_to_array(p.Tags, '<>')) AS t(TagName) ON TRUE WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId), TopPosts
//        AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.CommentCount, rp.Tags, u.DisplayName AS OwnerDisplayName, u.Reputation, ROW_NUMBER() OVER
//        (ORDER BY rp.Score DESC) AS OverallRank FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id WHERE rp.Rank <= 5) SELECT tp.PostId, tp.Title, tp.CreationDate,
//        tp.ViewCount, tp.Score, tp.CommentCount, tp.Tags, tp.OwnerDisplayName, tp.Reputation FROM TopPosts tp WHERE tp.OverallRank <= 10 ORDER BY tp.Score DESC, tp.CreationDate
//        DESC;
//
// `string_to_array(p.Tags, '<>')` never splits, so Tags is the whole string (or NULL). Both ranks read only base columns, so the posts are
// ranked first; a score tie goes to the smaller post id (the SQL leaves it open).
fn q9956(db: &'static So) -> String {
    let Post { post_type_id, owner_user_id, owner_user, score, creation_date, tags_str, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let top = top_n(drain((&w).filt(|(_, k)| k <= 5).map(|((p, _), _)| p).with(owner_user)), |&(_, p)| (Reverse(score.get(p).unwrap()), p), 10);
    let tp: MatSet<Id<Post>> = rel(top).map(|x: (Option<i64>, Id<Post>)| x.1).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(tags_str.flat_map(|t: Str| t.split("<>")).opt())).buf_fold(|v| {
        let mut t: Vec<Option<Str>> = v.iter().map(|x| x.1).collect();
        t.sort();
        t.dedup();
        (v.iter().filter(|x| x.0.is_some()).count() as i64, &*Box::leak(t.into_boxed_slice()))
    });
    let v = drain((&tp).select(Ident::<Post>::new().and(&cc)));
    let v = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())), 0);
    rows(v.into_iter().map(|(_, (p, (c, t)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.push(V::I(c));
        f.push(V::L(t.iter().map(|x| ostr(*x)).collect()));
        f.extend(post_fields(db, p, &["owner", "rep"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, COUNT(DISTINCT c.Id) AS CommentCount,
//        COUNT(DISTINCT v.Id) AS VoteCount, RANK() OVER (ORDER BY p.Score DESC) AS RankByScore FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id =
//        c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 2 WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName),
//        TopPosts AS (SELECT rp.* FROM RankedPosts rp WHERE rp.RankByScore <= 10) SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.OwnerDisplayName,
//        tp.CommentCount, tp.VoteCount, STRING_AGG(DISTINCT t.TagName, ', ') AS TagsList FROM TopPosts tp LEFT JOIN LATERAL (SELECT unnest(string_to_array(Tags, '<>')) AS TagName
//        FROM Posts WHERE Id = tp.PostId) t ON true GROUP BY tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.OwnerDisplayName, tp.CommentCount, tp.VoteCount ORDER
//        BY tp.Score DESC;
//
// `string_to_array(Tags, '<>')` never splits, so the tag list is the whole Tags string.
fn q8571(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, tags_str, .. } = &db.post;
    let w = whole(db.post.with(post_type_id.eq(1)).with(owner_user)).select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| Reverse(s), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 10).map(|((p, _), _)| p).collect();
    let uv = || votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)));
    let nc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let nv = (&tp).group_by(Ident::<Post>::new()).select(uv().opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let a = (&nc).and(&nv);
    let tl = (&tp).group_by(Ident::<Post>::new()).select(tags_str.flat_map(|t: Str| t.split("<>"))).buf_fold(|v| agg_distinct(v.to_vec(), ", ").unwrap());
    let v = drain((&tp).select(Ident::<Post>::new().and(&a).and((&tl).opt())));
    rows(v.into_iter().map(|(_, ((p, (c, n)), t))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(c), V::I(n), ostr(t)]);
        row(f)
    }))
}

// WITH TagCounts AS (SELECT Posts.Id AS PostId, UNNEST(string_to_array(substring(Posts.Tags, 2, length(Posts.Tags)-2), '><')) AS Tag FROM Posts WHERE Posts.PostTypeId = 1),
//        TagStatistics AS (SELECT TagCounts.Tag, COUNT(DISTINCT TagCounts.PostId) AS PostCount, COUNT(DISTINCT Votes.UserId) AS UniqueVoteCounts, ARRAY_AGG(DISTINCT
//        Users.DisplayName) AS VotedUsers, AVG(Posts.Score) AS AverageScore, SUM(Posts.ViewCount) AS TotalViews, SUM(Posts.AnswerCount) AS TotalAnswers FROM TagCounts LEFT JOIN
//        Posts ON TagCounts.PostId = Posts.Id LEFT JOIN Votes ON Posts.Id = Votes.PostId AND Votes.VoteTypeId = 2 LEFT JOIN Users ON Votes.UserId = Users.Id GROUP BY
//        TagCounts.Tag), RankedTags AS (SELECT Tag, PostCount, UniqueVoteCounts, VotedUsers, AverageScore, TotalViews, TotalAnswers, ROW_NUMBER() OVER (ORDER BY PostCount DESC,
//        UniqueVoteCounts DESC, AverageScore DESC) AS Rank FROM TagStatistics) SELECT Tag, PostCount, UniqueVoteCounts, VotedUsers, AverageScore, TotalViews, TotalAnswers, Rank
//        FROM RankedTags WHERE Rank <= 10 ORDER BY Rank;
//
// The ARRAY_AGG(DISTINCT ...) is listed with NULL first, then names in order.
fn q28872(db: &'static So) -> String {
    let Post { post_type_id, tags_str, score, view_count, answer_count, .. } = &db.post;
    let Vote { vote_type_id, user, .. } = &db.vote;
    let tg = || db.post.with(post_type_id.eq(1)).group_by(tags_str.flat_map(tag_list));
    let up = || votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.eq(2)));
    let pn = tg().select(Ident::<Post>::new()).count_distinct();
    let vu = tg().select(up().select(&db.vote.user_id)).count_distinct();
    type A = (&'static [Option<Str>], (i64, i64), Option<i64>, Option<i64>);
    let ag = tg().select(score.and(view_count.opt()).and(answer_count.opt()).and(up().select(user.select(&db.user.display_name).opt()).opt())).buf_fold(|v| -> A {
        let mut names: Vec<Option<Str>> = v.iter().map(|x| x.1.flatten()).collect();
        names.sort();
        names.dedup();
        let s: i64 = v.iter().map(|x| x.0 .0 .0).sum();
        let w: Vec<i64> = v.iter().filter_map(|x| x.0 .0 .1).collect();
        let a: Vec<i64> = v.iter().filter_map(|x| x.0 .1).collect();
        (&*Box::leak(names.into_boxed_slice()), (s, v.len() as i64), if w.is_empty() { None } else { Some(w.iter().sum::<i64>()) }, if a.is_empty() { None } else { Some(a.iter().sum::<i64>()) })
    });
    let tags: MatSet<Str> = db.post.with(post_type_id.eq(1)).select(tags_str.flat_map(tag_list)).collect();
    let w = whole(&tags)
        .select(Same::<Str>::new().and((&pn).and((&vu).opt()).and(&ag)))
        .window(row_number, |(t, ((n, u), (_, (s, c), _, _)))| (Reverse(n), Reverse(u.unwrap_or(0)), Reverse(fkey(s as f64 / c as f64)), t), asc);
    let v = drain((&w).filt(|(_, k)| k <= 10));
    rows(v.into_iter().map(|(_, ((t, ((n, u), (l, (s, c), w, a))), k))| {
        row(vec![V::S(t), V::I(n), V::I(u.unwrap_or(0)), V::L(l.iter().map(|x| ostr(*x)).collect()), avg(s, c), oint(w), oint(a), V::I(k)])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Tags, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS Author, ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER
//        BY p.CreationDate DESC) AS TagRank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP)
//        - INTERVAL '1 year'), CloseStats AS (SELECT ph.PostId, COUNT(*) AS CloseReasonsCount, STRING_AGG(crt.Name, ', ') AS CloseReasons FROM PostHistory ph JOIN CloseReasonTypes
//        crt ON CAST(ph.Comment AS INTEGER) = crt.Id WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId), TopTags AS (SELECT UNNEST(STRING_TO_ARRAY(Tags, ',')) AS TagName
//        FROM RankedPosts WHERE TagRank = 1) SELECT rp.Title AS QuestionTitle, rp.Author, rp.CreationDate, rp.ViewCount, rp.Score, ct.CloseReasonsCount, ct.CloseReasons,
//        tt.TagName FROM RankedPosts rp LEFT JOIN CloseStats ct ON rp.PostId = ct.PostId JOIN TopTags tt ON tt.TagName = ANY(STRING_TO_ARRAY(rp.Tags, ',')) WHERE rp.TagRank = 1
//        ORDER BY rp.ViewCount DESC, rp.Score DESC LIMIT 10;
//
// `string_to_array(Tags, ',')` never splits, so TopTags holds each partition's whole Tags string once and the ANY join matches a post to its own
// partition's entry. A CreationDate tie inside the ROW_NUMBER goes to the smaller post id. The STRING_AGG order is left open; the port joins in
// history id order.
fn q28362(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, tags_str, view_count, score, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1))
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user)
        .group_by(tags_str.opt())
        .select(Ident::<Post>::new().and(creation_date).and(tags_str.opt()))
        .window(row_number, |((p, d), _)| (Reverse(d), p), asc);
    type F = (Id<Post>, Option<Str>);
    let fr: MatSet<F> = (&w).filt(|(_, k)| k == 1).map(|(((p, _), t), _)| (p, t)).collect();
    let tt: HashIdx<Str, F> = (&fr).flat_map(|x: F| x.1).inv().select(&fr).collect();
    let cr = close_reasons(db);
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cs = db
        .post_history
        .with(post_history_type_id.is_in([10, 11]))
        .group_by(post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&cr).select(&db.close_reason_type.name))
        .buf_fold(|v| (v.len() as i64, leak(v.join(", "))));
    let v = drain((&fr).select(Same::<F>::new().and(Same::<F>::new().map(|x: F| x.0).select((&cs).opt())).and(Same::<F>::new().flat_map(|x: F| x.1.into_iter().flat_map(|t: Str| t.split(','))).select(&tt))));
    let v = top_n(v, |&(_, (((p, _), _), _))| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), Reverse(score.get(p).unwrap()), p)
    }, 10);
    rows(v.into_iter().map(|(_, (((p, _), c), (_, t)))| {
        let mut f = post_fields(db, p, &["title", "owner", "created", "views", "score"]);
        f.extend(match c {
            Some((n, s)) => [V::I(n), V::S(s)],
            None => [V::Null, V::Null],
        });
        f.push(ostr(t));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.Tags, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE
//        0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn FROM Posts p
//        LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.Tags),
//        TopPosts AS (SELECT PostId, Title, ViewCount, Score, Tags, CommentCount, UpVotes, DownVotes FROM RankedPosts WHERE rn = 1 ORDER BY Score DESC, ViewCount DESC LIMIT 10),
//        PostTags AS (SELECT p.Id AS PostId, STRING_AGG(t.TagName, ', ') AS Tags FROM Posts p JOIN LATERAL (SELECT unnest(string_to_array(p.Tags, '<>')) AS TagName) AS tag ON TRUE
//        JOIN Tags t ON t.TagName = tag.TagName GROUP BY p.Id) SELECT tp.PostId, tp.Title, tp.ViewCount, tp.Score, tp.CommentCount, tp.UpVotes, tp.DownVotes, pt.Tags AS
//        ProcessedTags FROM TopPosts tp JOIN PostTags pt ON tp.PostId = pt.PostId ORDER BY tp.Score DESC;
//
// `string_to_array(p.Tags, '<>')` never splits, so PostTags joins the whole Tags string to Tags.TagName. The STRING_AGG order is left open.
fn q25942(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, tags_str, .. } = &db.post;
    let top = top_n(drain(db.post.with(post_type_id.eq(1))), |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 10);
    let tp: MatSet<Id<Post>> = rel(top).map(|x: (Id<Post>, Id<Post>)| x.0).collect();
    let rp = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let tn: HashIdx<Str, Id<Tag>> = (&db.tag.tag_name).inv().collect();
    let pt = (&tp).group_by(Ident::<Post>::new()).select(tags_str.flat_map(|t: Str| t.split("<>")).select(&tn).select(&db.tag.tag_name)).buf_fold(|v| leak(v.join(", ")));
    let v = drain((&tp).select(Ident::<Post>::new().and(&rp).and(&pt)));
    rows(v.into_iter().map(|(_, ((p, a), t))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "score"]);
        f.extend(a.map(V::I));
        f.push(V::S(t));
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U WHERE U.Reputation
//        > 0), RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.Body, P.CreationDate, P.OwnerUserId, P.PostTypeId, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY
//        P.CreationDate DESC) AS PostRank FROM Posts P WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'), PostTagStats AS (SELECT P.Id AS
//        PostId, COUNT(DISTINCT T.TagName) AS TagCount, STRING_AGG(T.TagName, ', ') AS Tags FROM Posts P JOIN UNNEST(string_to_array(P.Tags, '>')) AS T(TagName) ON T.TagName IS
//        NOT NULL GROUP BY P.Id), PostHistoryStats AS (SELECT PH.PostId, COUNT(PH.Id) AS EditCount, MAX(PH.CreationDate) AS LastEditDate FROM PostHistory PH WHERE
//        PH.PostHistoryTypeId IN (4, 5, 6, 24) GROUP BY PH.PostId) SELECT RU.DisplayName, RU.Reputation, RP.PostId, RP.Title, RP.Body, RP.CreationDate, PTS.TagCount, PTS.Tags,
//        PHS.EditCount, PHS.LastEditDate FROM RankedUsers RU JOIN RecentPosts RP ON RU.UserId = RP.OwnerUserId AND RP.PostRank = 1 JOIN PostTagStats PTS ON RP.PostId = PTS.PostId
//        JOIN PostHistoryStats PHS ON RP.PostId = PHS.PostId WHERE RU.ReputationRank <= 10 ORDER BY RU.Reputation DESC, RP.CreationDate DESC;
//
// A tie inside either ROW_NUMBER goes to the smaller id (the SQL leaves it open). The STRING_AGG order is left open; the port joins in
// Tags-string order.
fn q25514(db: &'static So) -> String {
    let Post { creation_date, owner_user, tags_str, .. } = &db.post;
    let ru = top_n(drain(db.user.with((&db.user.reputation).gt(0)).select(&db.user.reputation)), |&(u, r)| (Reverse(r), u), 10);
    let ru: MatSet<Id<User>> = rel(ru).map(|x: (Id<User>, i64)| x.0).collect();
    let w = db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let rp: HashIdx<Id<User>, Id<Post>> = (&w).filt(|(_, k)| k == 1).map(|((p, _), _)| p).collect();
    let tc = db.post.group_by(Ident::<Post>::new()).select(tags_str.flat_map(|t: Str| t.split('>'))).count_distinct();
    let tj = db.post.group_by(Ident::<Post>::new()).select(tags_str.flat_map(|t: Str| t.split('>'))).buf_fold(|v| leak(v.join(", ")));
    let pts = (&tc).and(&tj);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let phs = db.post_history.with(post_history_type_id.is_in([4, 5, 6, 24])).group_by(post).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let v = drain((&ru).select(Ident::<User>::new().and((&rp).select(Ident::<Post>::new().and(&pts).and(&phs)))));
    let v = top_n(v, |&(u, (_, ((p, _), _)))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(creation_date.get(p).unwrap())), 0);
    rows(v.into_iter().map(|(_, (u, ((p, (n, t)), (e, d))))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["id", "title", "body", "created"]));
        f.extend([V::I(n), V::S(t), V::I(e), V::T(d)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.ViewCount, p.Score, p.Tags, U.DisplayName AS OwnerDisplayName, COUNT(S.Id) AS
//        TotalComments, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS RN FROM Posts p LEFT JOIN Users U ON p.OwnerUserId = U.Id LEFT JOIN Comments S ON
//        p.Id = S.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, p.CreationDate, p.ViewCount, p.Score, p.Tags, U.DisplayName), FilteredTags AS (SELECT p.PostId,
//        TRIM(UNNEST(STRING_TO_ARRAY(p.Tags, ','))) AS Tag FROM RankedPosts p WHERE p.RN = 1), TagStats AS (SELECT Tag, COUNT(DISTINCT PostId) AS PostCount FROM FilteredTags GROUP
//        BY Tag HAVING COUNT(DISTINCT PostId) > 1), TaggedQuestions AS (SELECT rt.PostId, rt.Title, rt.ViewCount, rt.Score, tt.PostCount FROM RankedPosts rt JOIN FilteredTags ft
//        ON rt.PostId = ft.PostId JOIN TagStats tt ON ft.Tag = tt.Tag) SELECT CONCAT('Title: ', TQ.Title, ', View Count: ', TQ.ViewCount, ', Score: ', TQ.Score, ', Tag Count: ',
//        TQ.PostCount, ' Tags: ', STRING_AGG(DISTINCT ft.Tag, ', ')) AS BenchmarkInfo FROM TaggedQuestions TQ JOIN FilteredTags ft ON TQ.PostId = ft.PostId GROUP BY TQ.PostId,
//        TQ.Title, TQ.ViewCount, TQ.Score, TQ.PostCount ORDER BY TQ.PostCount DESC, TQ.Score DESC LIMIT 10;
//
// `STRING_TO_ARRAY(p.Tags, ',')` never splits, so each question's tag is its whole Tags string. Rows equal on the ORDER BY go to the smaller
// post id (the SQL leaves it open).
fn q25748(db: &'static So) -> String {
    let Post { post_type_id, tags_str, score, .. } = &db.post;
    let ft = || tags_str.flat_map(|t: Str| t.split(',').map(|x| x.trim()));
    let ts = db.post.with(post_type_id.eq(1)).group_by(ft()).select(Ident::<Post>::new()).count_distinct();
    let g = db
        .post
        .with(post_type_id.eq(1))
        .group_by(Ident::<Post>::new().and(ft().select((&ts).filt(|n| n > 1))))
        .select(ft())
        .buf_fold(|v| agg_distinct(v.to_vec(), ", ").unwrap());
    let v = top_n(drain(&g), |&((p, n), _)| (Reverse(n), Reverse(score.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|((p, n), t)| {
        let w = db.post.view_count.get(p).map_or(String::new(), |w| w.to_string());
        row(vec![V::Owned(format!("Title: {}, View Count: {}, Score: {}, Tag Count: {} Tags: {}", db.post.title.get(p).unwrap_or(""), w, score.get(p).unwrap(), n, t))])
    }))
}

// WITH ProcessedTags AS (SELECT p.Id AS PostId, unnest(string_to_array(substring(p.Tags, 2, length(p.Tags)-2), '><')) AS TagName FROM Posts p WHERE p.Tags IS NOT NULL),
//        TagPostCounts AS (SELECT TagName, COUNT(DISTINCT PostId) AS PostCount FROM ProcessedTags GROUP BY TagName), TopTags AS (SELECT TagName, PostCount, RANK() OVER (ORDER BY
//        PostCount DESC) AS TagRank FROM TagPostCounts WHERE PostCount > 1), ActiveUsers AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN
//        p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' THEN 1 ELSE 0 END) AS ActivePostsLast30Days FROM Users u JOIN Posts p ON u.Id =
//        p.OwnerUserId GROUP BY u.Id, u.DisplayName), UserBadges AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldCount, COUNT(CASE WHEN b.Class = 2 THEN 1 END)
//        AS SilverCount, COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeCount FROM Badges b GROUP BY b.UserId) SELECT u.UserId, u.DisplayName, COALESCE(b.GoldCount, 0) AS
//        GoldBadges, COALESCE(b.SilverCount, 0) AS SilverBadges, COALESCE(b.BronzeCount, 0) AS BronzeBadges, u.PostCount, u.ActivePostsLast30Days, tt.TagName, tt.PostCount AS
//        TagPostCount FROM ActiveUsers u LEFT JOIN UserBadges b ON u.UserId = b.UserId LEFT JOIN TopTags tt ON tt.TagRank <= 5 ORDER BY u.PostCount DESC, ActivePostsLast30Days
//        DESC;
//
// `LEFT JOIN TopTags tt ON tt.TagRank <= 5` does not mention u, so the top tags are crossed with every active user.
fn q28590(db: &'static So) -> String {
    let Post { tags_str, creation_date, .. } = &db.post;
    let tpc = db.post.group_by(tags_str.flat_map(tag_list)).select(Ident::<Post>::new()).count_distinct();
    let tags: MatSet<Str> = db.post.select(tags_str.flat_map(tag_list)).collect();
    let tw = whole((&tags).with((&tpc).filt(|n| n > 1))).select(Same::<Str>::new().and(&tpc)).window(rank, |(_, n)| Reverse(n), asc);
    let tti: HashIdx<(), (Str, i64)> = (&tw).filt(|(_, k)| k <= 5).map(|(x, _)| x).collect();
    let cut = add_days(ts(2024, 10, 1, 12, 34, 56), -30);
    let au = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(creation_date)).fold([0i64; 2], |a, d| [a[0] + 1, a[1] + (d > cut) as i64]);
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let v = drain(db.user.select(Ident::<User>::new().and(&au).and((&ub).opt()).and(Ident::<User>::new().map(|_| ()).select((&tti).opt()))));
    rows(v.into_iter().map(|(_, (((u, a), b), t))| {
        let b = b.unwrap_or([0; 3]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(b.map(V::I));
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(match t {
            Some((t, n)) => [V::S(t), V::I(n)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, u.Reputation AS OwnerReputation, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score
//        DESC, p.CreationDate DESC) AS rn FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
//        PostVoteSummary AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(*) AS TotalVotes FROM Votes v GROUP BY v.PostId), FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, pvs.UpVotes, pvs.DownVotes, pvs.TotalVotes
//        FROM RankedPosts rp LEFT JOIN PostVoteSummary pvs ON rp.PostId = pvs.PostId WHERE rp.rn <= 5) SELECT fp.PostId, fp.Title, COALESCE(fp.Score, 0) AS EffectiveScore,
//        COALESCE(fp.UpVotes, 0) AS EffectiveUpVotes, COALESCE(fp.DownVotes, 0) AS EffectiveDownVotes, COALESCE(fp.TotalVotes, 0) AS EffectiveTotalVotes, CASE WHEN fp.UpVotes IS
//        NULL AND fp.Score > 0 THEN 'Misleading' WHEN fp.DownVotes IS NULL AND fp.Score < 0 THEN 'Deceptive' ELSE 'Transparent' END AS TransparencyStatus FROM FilteredPosts fp
//        LEFT JOIN (SELECT DISTINCT Tags FROM (SELECT unnest(string_to_array(Tags, '<>')) AS Tags FROM Posts) AS TagData) AS DistinctTags ON true ORDER BY EffectiveScore DESC,
//        EffectiveTotalVotes DESC;
//
// `string_to_array(Tags, '<>')` never splits, so DistinctTags is the distinct Tags strings, crossed with every row (`ON true`). A tie inside the
// ROW_NUMBER goes to the smaller post id (the SQL leaves it open).
fn q21053(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, score, tags_str, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|(((p, _), _), _)| p).collect();
    let pvs = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + 1]);
    let dt: HashIdx<(), Str> = db.post.select(tags_str.flat_map(|t: Str| t.split("<>"))).collect::<MatSet<Str>>().map(|_| ()).inv().collect();
    let v = drain((&tp).select(Ident::<Post>::new().and((&pvs).opt()).and(Ident::<Post>::new().map(|_| ()).select((&dt).opt()))));
    rows(v.into_iter().map(|(_, ((p, a), _))| {
        let s = score.get(p).unwrap();
        let st = if a.is_none() && s > 0 {
            "Misleading"
        } else if a.is_none() && s < 0 {
            "Deceptive"
        } else {
            "Transparent"
        };
        let a = a.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::S(st)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank,
//        COALESCE(TH.TagCount, 0) AS TagCount FROM Posts p LEFT JOIN (SELECT Id, (SELECT COUNT(*) FROM UNNEST(STRING_TO_ARRAY(Tags, '><')) AS tag_table) AS TagCount FROM Posts
//        WHERE Tags IS NOT NULL) TH ON p.Id = TH.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'), ActiveUsers AS (SELECT u.Id AS UserId,
//        u.DisplayName, SUM(COALESCE(b.Class, 0)) AS BadgePoints, COUNT(DISTINCT p.Id) AS PostCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes FROM
//        Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 2 WHERE u.LastAccessDate
//        >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY u.Id, u.DisplayName), ClosedPosts AS (SELECT p.Id AS PostId, COUNT(*) AS CloseCount FROM Posts p JOIN
//        PostHistory ph ON p.Id = ph.PostId WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY p.Id) SELECT rp.PostId, rp.Title, rp.Score, rp.Rank, au.UserId, au.DisplayName,
//        au.BadgePoints, au.PostCount, au.UpVotes, COALESCE(cp.CloseCount, 0) AS CloseCount FROM RankedPosts rp JOIN ActiveUsers au ON rp.PostId = au.PostCount LEFT JOIN
//        ClosedPosts cp ON rp.PostId = cp.PostId WHERE (rp.Score > 0 AND au.BadgePoints > 1) OR (rp.TagCount > 5 AND COALESCE(cp.CloseCount, 0) = 0) ORDER BY rp.Score DESC,
//        au.BadgePoints DESC LIMIT 100 OFFSET 0;
//
// `rp.PostId = au.PostCount` joins a post id to a user's post count, as written; the users are matched on that first and ActiveUsers' product
// is driven for the matched ones alone. A score tie inside the ROW_NUMBER goes to the smaller post id (the SQL leaves it open).
fn q20425(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, origid, tags_str, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let w = db.post.with(creation_date.ge(add_years(t0, -1))).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    type R = ((Id<Post>, i64), i64);
    let rv: MatSet<R> = (&w).map(|x: R| x).collect();
    let active = || db.user.with((&db.user.last_access_date).ge(add_days(t0, -30)));
    let pc = active().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let by_pc: HashIdx<i64, Id<User>> = (&pc).inv().collect();
    let mu: MatSet<Id<User>> = (&rv).select(Same::<R>::new().map(|x: R| x.0 .0).select(origid).select(&by_pc)).collect();
    let au = (&mu)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).select(&db.badge.class).opt().and(posts_of(db).select(votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2))).opt()).opt()))
        .fold([0i64; 2], |a, (c, v)| [a[0] + c.unwrap_or(0), a[1] + v.flatten().is_some() as i64]);
    let tc = db.post.group_by(Ident::<Post>::new()).select(tags_str.flat_map(|t: Str| t.split("><"))).fold(0i64, |a, _| a + 1);
    let cp = db.post_history.with((&db.post_history.post_history_type_id).is_in([10, 11])).group_by(&db.post_history.post).fold(0i64, |a, _| a + 1);
    let k = || Same::<R>::new().map(|x: R| x.0 .0);
    type X = (((R, (Id<User>, ([i64; 2], i64))), Option<i64>), Option<i64>);
    let v = drain(
        (&rv)
            .select(Same::<R>::new().and(k().select(origid).select(&by_pc).select(Ident::<User>::new().and((&au).and(&pc)))).and(k().select((&tc).opt())).and(k().select((&cp).opt())))
            .filt(|((((p, _), (_, (a, _))), t), c): X| (p.1 > 0 && a[0] > 1) || (t.unwrap_or(0) > 5 && c.unwrap_or(0) == 0)),
    );
    let v = top_n(v, |&(_, ((((p, _), (u, (a, _))), _), _))| (Reverse(p.1), Reverse(a[0]), p.0, u), 100);
    rows(v.into_iter().map(|(_, ((((p, r), (u, (a, n))), _), c))| {
        let mut f = post_fields(db, p.0, &["id", "title", "score"]);
        f.push(V::I(r));
        f.extend(ucols(db, u, &["uid", "name"]));
        f.extend([V::I(a[0]), V::I(n), V::I(a[1]), V::I(c.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, p.CreationDate, p.OwnerUserId, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        COUNT(DISTINCT v.UserId) AS VoteCount, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT
//        JOIN Votes v ON p.Id = v.PostId LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, p.Tags, p.CreationDate, p.OwnerUserId,
//        u.DisplayName), FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.Body, rp.Tags, rp.CreationDate, rp.OwnerDisplayName, rp.CommentCount, rp.VoteCount FROM RankedPosts rp
//        WHERE rp.rn = 1 AND rp.VoteCount > 10 AND rp.CommentCount > 5), FinalOutput AS (SELECT fp.*, STRING_AGG(DISTINCT t.TagName, ', ') AS RelatedTags, COUNT(DISTINCT
//        pl.RelatedPostId) AS RelatedPostCount FROM FilteredPosts fp LEFT JOIN LATERAL (SELECT UNNEST(STRING_TO_ARRAY(SUBSTRING(fp.Tags, 2, LENGTH(fp.Tags) - 2), '><')) AS tag) AS
//        tag ON TRUE LEFT JOIN Tags t ON tag.tag = t.TagName LEFT JOIN PostLinks pl ON fp.PostId = pl.PostId GROUP BY fp.PostId, fp.Title, fp.Body, fp.Tags, fp.CreationDate,
//        fp.OwnerDisplayName, fp.CommentCount, fp.VoteCount) SELECT f.PostId, f.Title, f.OwnerDisplayName, f.CommentCount, f.VoteCount, f.RelatedTags, f.RelatedPostCount,
//        f.CreationDate FROM FinalOutput f ORDER BY f.VoteCount DESC, f.CommentCount DESC;
//
// The distinct tag names are joined in name order.
fn q26221(db: &'static So) -> String {
    let Post { post_type_id, tags_str, .. } = &db.post;
    let qg = || db.post.with(post_type_id.eq(1)).group_by(Ident::<Post>::new());
    let cc = qg().select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let vu = qg().select(votes_of(db).select(&db.vote.user_id)).count_distinct();
    let rp = (&cc).and((&vu).opt()).map(|(c, n): (i64, Option<i64>)| (c, n.unwrap_or(0)));
    let fp: MatSet<Id<Post>> = db.post.with((&rp).filt(|(c, n): (i64, i64)| n > 10 && c > 5)).collect();
    let tn: HashIdx<Str, Id<Tag>> = (&db.tag.tag_name).inv().collect();
    let ft = (&fp).group_by(Ident::<Post>::new()).select(tags_str.flat_map(tag_list).select(&tn).select(&db.tag.tag_name).opt()).buf_fold(|v| agg_distinct(v.iter().filter_map(|x| *x).collect(), ", "));
    let fl = (&fp).group_by(Ident::<Post>::new()).select(links_of(db).select(&db.post_link.related_post_id)).count_distinct();
    let fo = (&ft).and((&fl).opt()).map(|(t, l): (Option<Str>, Option<i64>)| (t, l.unwrap_or(0)));
    let v = drain((&fp).select(Ident::<Post>::new().and(&rp).and(&fo)));
    rows(v.into_iter().map(|(_, ((p, (c, n)), (t, l)))| {
        let mut f = post_fields(db, p, &["id", "title", "owner"]);
        f.extend([V::I(c), V::I(n), ostr(t), V::I(l)]);
        f.extend(post_fields(db, p, &["created"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId
//        ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId IN (1, 2)), UserBadgeCounts AS (SELECT b.UserId,
//        COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class
//        = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId), ClosedPosts AS (SELECT p.Id AS ClosedPostId, p.Title, ph.CreationDate AS ClosedDate, ph.Comment
//        AS CloseReason FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId WHERE ph.PostHistoryTypeId = 10), PostTags AS (SELECT p.Id AS PostId, STRING_AGG(t.TagName, ', ' ORDER
//        BY t.TagName) AS Tags FROM Posts p LEFT JOIN UNNEST(string_to_array(substring(p.Tags, 2, length(p.Tags)-2), '><')) AS t(TagName) ON true GROUP BY p.Id) SELECT rp.PostId,
//        rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerDisplayName, ub.BadgeCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, ct.ClosedPostId, ct.ClosedDate,
//        ct.CloseReason, pt.Tags FROM RankedPosts rp LEFT JOIN UserBadgeCounts ub ON rp.OwnerDisplayName = (SELECT DisplayName FROM Users WHERE Id = ub.UserId) LEFT JOIN
//        ClosedPosts ct ON rp.PostId = ct.ClosedPostId LEFT JOIN PostTags pt ON rp.PostId = pt.PostId WHERE rp.Rank <= 5 ORDER BY rp.PostId, rp.Score DESC;
//
// Rewritten (rewrites/31613.sql): the STRING_AGG gets ORDER BY its own argument. The UserBadgeCounts join matches every badge holder whose
// display name equals the owner's. A tie inside the ROW_NUMBER goes to the smaller post id.
fn q31613(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, creation_date, tags_str, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.is_in([1, 2]))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|(((p, _), _), _)| p).collect();
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let by_name: HashIdx<Str, Id<User>> = db.user.with(&ub).select(&db.user.display_name).inv().collect();
    let PostHistory { post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let ct = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.eq(10)).select(hd.and(comment.opt())));
    let pt = (&tp).group_by(Ident::<Post>::new()).select(tags_str.flat_map(tag_list)).buf_fold(|v| {
        let mut t = v.to_vec();
        t.sort();
        leak(t.join(", "))
    });
    let v = drain((&tp).select(Ident::<Post>::new().and(owner_user.select(&db.user.display_name).select(&by_name).select(&ub).opt()).and(ct.opt()).and((&pt).opt())));
    rows(v.into_iter().map(|(_, (((p, b), c), t))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend(match b {
            Some(b) => b.map(V::I),
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        f.extend(match c {
            Some((d, r)) => [post_fields(db, p, &["id"]).remove(0), V::T(d), ostr(r)],
            None => [V::Null, V::Null, V::Null],
        });
        f.push(ostr(t));
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS Upvotes, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS
//        Downvotes, COALESCE(SUM(CASE WHEN V.BountyAmount IS NOT NULL THEN V.BountyAmount ELSE 0 END), 0) AS TotalBounty FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY
//        U.Id, U.DisplayName), PostTagCounts AS (SELECT P.Id AS PostId, COUNT(DISTINCT T.TagName) AS UniqueTagCount FROM Posts P LEFT JOIN UNNEST(string_to_array(P.Tags, '><')) AS
//        T(TagName) ON true GROUP BY P.Id), RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.AnswerCount, P.ViewCount, COALESCE(PH.UsersWhoEdited, 0) AS
//        EditorCount, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS RowNum, P.OwnerUserId FROM Posts P LEFT JOIN (SELECT PostId, COUNT(DISTINCT
//        UserId) AS UsersWhoEdited FROM PostHistory WHERE PostHistoryTypeId IN (4, 5) GROUP BY PostId) AS PH ON P.Id = PH.PostId WHERE P.CreationDate >= (CAST('2024-10-01
//        12:34:56' AS TIMESTAMP) - INTERVAL '30 days')) SELECT R.PostId, R.Title, R.CreationDate, R.AnswerCount, R.ViewCount, CURRENT_TIMESTAMP AS BenchmarkTimestamp, U.UserId,
//        U.DisplayName, U.Upvotes, U.Downvotes, U.TotalBounty, PC.UniqueTagCount, CASE WHEN R.RowNum = 1 THEN TRUE ELSE FALSE END AS IsMostRecent FROM RecentPosts R JOIN
//        UserVoteStats U ON R.OwnerUserId = U.UserId LEFT JOIN PostTagCounts PC ON R.PostId = PC.PostId WHERE U.TotalBounty > 100 AND R.EditorCount > 3 ORDER BY R.CreationDate
//        DESC;
//
// No row reaches the output on this data, so the CURRENT_TIMESTAMP column is never printed; it would be the run's instant.
// A CreationDate tie inside the ROW_NUMBER goes to the smaller post id.
fn q1908(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, owner_user, tags_str, .. } = &db.post;
    let uvs = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select((&db.vote.vote_type_id).and((&db.vote.bounty_amount).opt())).opt())
        .fold([0i64; 3], |a, v| match v {
            Some((t, b)) => [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + b.unwrap_or(0)],
            None => a,
        });
    let PostHistory { post, post_history_type_id, user_id, .. } = &db.post_history;
    let ph = db.post_history.with(post_history_type_id.is_in([4, 5])).group_by(post).select(user_id).count_distinct();
    let w = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(owner_user_id.opt()).and(creation_date))
        .window(row_number, |((p, _), d)| (Reverse(d), p), asc);
    type R = ((Id<Post>, Option<i64>), i64);
    let rv: MatSet<R> = (&w).map(|((x, _), k)| (x, k)).collect();
    let ptc = db.post.group_by(Ident::<Post>::new()).select(tags_str.flat_map(|t: Str| t.split("><"))).count_distinct();
    let k = || Same::<R>::new().map(|x: R| x.0 .0);
    let v = drain(
        (&rv)
            .select(Same::<R>::new().with(k().select((&ph).filt(|n| n > 3))))
            .select(Same::<R>::new().and(k().select(owner_user.select(Ident::<User>::new().and((&uvs).filt(|a: [i64; 3]| a[2] > 100))))).and(k().select((&ptc).opt()))),
    );
    let v = top_n(v, |&(_, (((p, _), _), _))| Reverse(creation_date.get(p.0).unwrap()), 0);
    let now = now_utc();
    rows(v.into_iter().map(|(_, ((((p, _), r), (u, a)), n))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "answers", "views"]);
        f.push(V::T(now));
        f.extend(ucols(db, u, &["uid", "name"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(n.unwrap_or(0)), V::B(r == 1)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS
//        Rank, p.Tags FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title,
//        p.CreationDate, p.Score, p.ViewCount, p.Tags, p.PostTypeId), PopularTags AS (SELECT UNNEST(STRING_TO_ARRAY(TRIM(BOTH '{}' FROM Tags), ',')) AS Tag FROM RankedPosts WHERE
//        Rank <= 3), TagPopularity AS (SELECT Tag, COUNT(*) AS UsageCount FROM PopularTags GROUP BY Tag), UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT
//        p.Id) AS PostsCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty, MAX(CASE WHEN p.CreationDate <= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month' THEN
//        p.CreationDate END) AS LastActiveBeforeLastMonth FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id,
//        u.DisplayName), UserBadges AS (SELECT b.UserId, COUNT(DISTINCT CASE WHEN b.Class = 1 THEN b.Id END) AS GoldBadges, COUNT(DISTINCT CASE WHEN b.Class = 2 THEN b.Id END) AS
//        SilverBadges, COUNT(DISTINCT CASE WHEN b.Class = 3 THEN b.Id END) AS BronzeBadges FROM Badges b GROUP BY b.UserId) SELECT ua.UserId, ua.DisplayName,
//        COALESCE(ub.GoldBadges, 0) AS GoldBadges, COALESCE(ub.SilverBadges, 0) AS SilverBadges, COALESCE(ub.BronzeBadges, 0) AS BronzeBadges, ua.PostsCount, ua.TotalBounty,
//        COUNT(tp.Tag) FILTER (WHERE tp.UsageCount > 10) AS FrequentTags, COUNT(tp.Tag) FILTER (WHERE tp.UsageCount <= 10) AS RareTags, CASE WHEN ua.LastActiveBeforeLastMonth IS
//        NULL THEN 'Inactive' ELSE 'Active' END AS ActivityStatus FROM UserActivity ua LEFT JOIN UserBadges ub ON ua.UserId = ub.UserId LEFT JOIN TagPopularity tp ON TRUE GROUP BY
//        ua.UserId, ua.DisplayName, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, ua.PostsCount, ua.TotalBounty, ua.LastActiveBeforeLastMonth ORDER BY TotalBounty DESC,
//        PostsCount DESC, ua.DisplayName;
//
// `TRIM(BOTH '{}' FROM Tags)` then `STRING_TO_ARRAY(..., ',')` never splits, so each popular tag is a whole Tags string; TagPopularity is
// crossed with every user (`ON TRUE`). A tie inside the ROW_NUMBER goes to the smaller post id (the SQL leaves it open).
fn q21038(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, tags_str, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let w = db
        .post
        .with(creation_date.ge(add_years(t0, -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let trim = |t: Str| t.trim_matches(|c| c == '{' || c == '}');
    let tpop = (&w)
        .filt(|(_, k)| k <= 3)
        .map(|(((p, _), _), _)| p)
        .select(tags_str.flat_map(move |t: Str| trim(t).split(',')))
        .group_by(Same::<Str>::new())
        .fold(0i64, |a, _| a + 1);
    let tc = (&tpop).group_by(Same::<i64>::new().map(|_| ())).fold([0i64; 2], |a, n| [a[0] + (n > 10) as i64, a[1] + (n <= 10) as i64]);
    let cut = add_months(t0, -1);
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(creation_date.and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt())).opt())
        .fold((0i64, i64::MIN), |(b, m), x| match x {
            Some((d, v)) => (b + v.flatten().unwrap_or(0), if d <= cut { m.max(d) } else { m }),
            None => (b, m),
        });
    let ud = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let g = (&ua).and(&ud).and((&ub).opt()).and(Ident::<User>::new().map(|_| ()).select((&tc).opt()).map(|a: Option<[i64; 2]>| a.unwrap_or([0; 2])));
    rows(drain(g).into_iter().map(|(u, ((((b, m), n), g), a))| {
        let g = g.unwrap_or([0; 3]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(g.map(V::I));
        f.extend([V::I(n), V::I(b), V::I(a[0]), V::I(a[1]), V::S(if m == i64::MIN { "Inactive" } else { "Active" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS
//        RN, COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id) AS UpVotes, LAG(p.Score, 1, 0) OVER
//        (ORDER BY p.CreationDate, p.Id, c.Id, v.Id) AS PrevScore, c.Id AS CId, v.Id AS VId FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id =
//        v.PostId AND v.VoteTypeId IN (2, 3) WHERE p.CreationDate > CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'), PostHistoryData AS (SELECT ph.PostId,
//        ph.PostHistoryTypeId, ph.CreationDate, MAX(ph.CreationDate) OVER (PARTITION BY ph.PostId) AS MaxHistoryDate, STRING_AGG(DISTINCT CASE WHEN ph.PostHistoryTypeId = 10 THEN
//        ph.Comment END, ', ' ORDER BY CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.Comment END) AS CloseReasons FROM PostHistory ph GROUP BY ph.PostId, ph.PostHistoryTypeId,
//        ph.CreationDate), AggregatedData AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.CommentCount, rp.UpVotes, pd.CloseReasons, (rp.Score -
//        rp.PrevScore) AS ScoreChange, CASE WHEN rp.Score < 0 THEN 'Negative' WHEN rp.Score > 0 THEN 'Positive' ELSE 'Neutral' END AS ScoreTrend , rp.CId, rp.VId,
//        pd.PostHistoryTypeId AS PHType, pd.CreationDate AS PHDate FROM RankedPosts rp LEFT JOIN PostHistoryData pd ON rp.PostId = pd.PostId) SELECT ag.PostId, ag.Title,
//        ag.CreationDate, ag.ViewCount, ag.Score, ag.CommentCount, ag.UpVotes, COALESCE(ag.CloseReasons, 'No close reasons') AS CloseReasons, ag.ScoreChange, ag.ScoreTrend, CASE
//        WHEN ag.ScoreTrend = 'Negative' AND ag.CommentCount > 5 THEN 'Watch' ELSE 'Normal' END AS MonitoringStatus FROM AggregatedData ag WHERE ag.ViewCount > 100 AND
//        ag.ScoreTrend = 'Positive' AND (ag.CloseReasons IS NULL OR ag.CloseReasons NOT LIKE '%Duplicate%') ORDER BY ag.CreationDate DESC, ag.PostId, ag.CId, ag.VId, ag.PHType,
//        ag.PHDate LIMIT 50;
//
// Rewritten (rewrites/20586.sql): the LAG window and the final ORDER BY are refined with the joined rows' ids, and the STRING_AGG gets
// ORDER BY its own argument. PrevScore is LAG over every joined row of RankedPosts, in that order.
fn q20586(db: &'static So) -> String {
    let Post { creation_date, score, view_count, .. } = &db.post;
    let recent = || db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let vt = || votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([2, 3])).and(&db.vote.vote_type_id));
    type R = ((((Id<Post>, i64), i64), Option<Id<Comment>>), Option<(Id<Vote>, i64)>);
    type K = (i64, Id<Post>, bool, Option<Id<Comment>>, bool, Option<Id<Vote>>, i64);
    let key = |((((p, d), s), c), v): R| -> K { (d, p, c.is_none(), c, v.is_none(), v.map(|x| x.0), s) };
    let rn = whole(recent()).select(Ident::<Post>::new().and(creation_date).and(score).and(comments_of(db).opt()).and(vt().opt())).window(lag, key, asc);
    let pw = recent()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(vt().opt()))
        .fold([0i64; 2], |a, (c, v)| [a[0] + c.is_some() as i64, a[1] + (v.map(|x| x.1) == Some(2)) as i64]);
    let PostHistory { post, post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let pd = db
        .post_history
        .group_by(post.and(post_history_type_id).and(hd))
        .select(post_history_type_id.and(comment.opt()))
        .buf_fold(|v| agg_distinct(v.iter().filter(|x| x.0 == 10).filter_map(|x| x.1).collect(), ", "));
    type H = (((Id<Post>, i64), i64), Option<Str>);
    let hv = rel(drain(&pd));
    let hi: HashIdx<Id<Post>, H> = (&hv).map(|x: H| x.0 .0 .0).inv().select(&hv).collect();
    type W = (R, Option<K>);
    let k = || Same::<W>::new().map(|x: W| x.0 .0 .0 .0 .0);
    type X = ((W, [i64; 2]), Option<H>);
    let v = drain(
        (&rn)
            .select(Same::<W>::new().with(k().select(Ident::<Post>::new().with(view_count.gt(100)).with(score.gt(0)))))
            .select(Same::<W>::new().and(k().select(&pw)).and(k().select((&hi).opt())))
            .filt(|(_, h): X| h.and_then(|h| h.1).map_or(true, |c| !like(c, "%Duplicate%"))),
    );
    let v = top_n(v, |&(_, ((w, _), h))| {
        let x = key(w.0);
        (Reverse(x.0), x.1, x.2, x.3, x.4, x.5, h.is_none(), h.map(|h| h.0 .0 .1), h.map(|h| h.0 .1))
    }, 50);
    rows(v.into_iter().map(|(_, (((r, prev), a), h))| {
        let p = r.0 .0 .0 .0;
        let s = r.0 .0 .1;
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::S(h.and_then(|h| h.1).unwrap_or("No close reasons")), V::I(s - prev.map_or(0, |k| k.6)), V::S("Positive"), V::S("Normal")]);
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("6634", q6634),
    ("25028", q25028),
    ("29520", q29520),
    ("21130", q21130),
    ("29639", q29639),
    ("28634", q28634),
    ("22259", q22259),
    ("3681", q3681),
    ("660", q660),
    ("2772", q2772),
    ("30564", q30564),
    ("23129", q23129),
    ("27548", q27548),
    ("29324", q29324),
    ("31400", q31400),
    ("22980", q22980),
    ("26692", q26692),
    ("21296", q21296),
    ("1727", q1727),
    ("24608", q24608),
    ("20565", q20565),
    ("3547", q3547),
    ("23299", q23299),
    ("20230", q20230),
    ("5841", q5841),
    ("30632", q30632),
    ("24546", q24546),
    ("30832", q30832),
    ("24143", q24143),
    ("4550", q4550),
    ("20756", q20756),
    ("22715", q22715),
    ("22426", q22426),
    ("24556", q24556),
    ("23020", q23020),
    ("20747", q20747),
    ("3091", q3091),
    ("25198", q25198),
    ("24007", q24007),
    ("23376", q23376),
    ("23780", q23780),
    ("33051", q33051),
    ("20527", q20527),
    ("21679", q21679),
    ("23255", q23255),
    ("24021", q24021),
    ("22611", q22611),
    ("24538", q24538),
    ("32544", q32544),
    ("22898", q22898),
    ("23889", q23889),
    ("24942", q24942),
    ("23592", q23592),
    ("23381", q23381),
    ("20820", q20820),
    ("21064", q21064),
    ("22566", q22566),
    ("20578", q20578),
    ("21819", q21819),
    ("21670", q21670),
    ("20254", q20254),
    ("20490", q20490),
    ("23824", q23824),
    ("24444", q24444),
    ("23227", q23227),
    ("23670", q23670),
    ("20439", q20439),
    ("21580", q21580),
    ("24317", q24317),
    ("20832", q20832),
    ("23267", q23267),
    ("21216", q21216),
    ("9728", q9728),
    ("9956", q9956),
    ("8571", q8571),
    ("28872", q28872),
    ("28362", q28362),
    ("25942", q25942),
    ("25514", q25514),
    ("25748", q25748),
    ("28590", q28590),
    ("21053", q21053),
    ("20425", q20425),
    ("26221", q26221),
    ("31613", q31613),
    ("1908", q1908),
    ("21038", q21038),
    ("20586", q20586),
];
