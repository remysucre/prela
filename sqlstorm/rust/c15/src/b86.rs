use harness::prelude::*;

// Fifty-nine spellings of the same query with the vote-type sums added:
//
//   SELECT <columns>, COUNT(c.Id), SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END)
//                                [, SUM(CASE WHEN v.VoteTypeId = 3 ...)]
//   FROM Posts p [LEFT] JOIN Users u ON p.OwnerUserId = u.Id
//        [LEFT JOIN Comments c ...] [LEFT JOIN Votes v ...] [LEFT JOIN Posts a ...]
//   [WHERE p.PostTypeId = 1] GROUP BY <them> ORDER BY <one or two> DESC [LIMIT n]
//
// Comments and Votes both joined means the fan-outs cross: COUNT(c.Id) counts
// a comment once per vote (`#cv`) and each vote-type sum counts a vote once
// per comment (`#up`, `#down`). COUNT(DISTINCT c.Id) and COUNT(DISTINCT v.Id)
// undo their own fan-out and are the plain counts (`#c`, `#v`).
// Five at the end do not fit any of that and are written out on their own.

// --- the group is one post -------------------------------------------------

fn q15034(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q15044(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q15109(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q15158(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q15160(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q15258(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q15314(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q15570(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q17188(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q17252(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q17373(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q19043(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q15178(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up"]) }
fn q15189(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up"]) }
fn q16464(db: &'static So) -> String { post_rows(db, true, "cv", "created", 0, &["id", "title", "created", "owner", "#cx", "#up"]) }
fn q19135(db: &'static So) -> String { post_rows(db, false, "cv", "created", 100, &["id", "title", "created", "owner", "#cx", "#up"]) }
fn q18192(db: &'static So) -> String { post_rows(db, false, "cv", "created", 10, &["id", "title", "body", "created", "owner", "#cx", "#up"]) }
fn q18797(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#v"]) }
fn q19801(db: &'static So) -> String { post_rows(db, true, "cv", "created", 100, &["id", "title", "created", "owner", "#cx", "#v"]) }
fn q17925(db: &'static So) -> String { post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "#c", "#v"]) }

// --- the group is (DisplayName, Title, CreationDate) -----------------------

fn q15005(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#up", "#down"]) }
fn q15023(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#up", "#down"]) }
fn q15029(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#up", "#down"]) }
fn q15100(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#up", "#down"]) }
fn q15840(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#up", "#down"]) }
fn q16386(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#up", "#down"]) }
fn q16644(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#up", "#down"]) }
fn q18146(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#up", "#down"]) }
fn q18351(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#up", "#down"]) }
fn q15976(db: &'static So) -> String { tuple_rows(by_name_title_date(db, false, "cv", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#up", "#down"]) }
fn q15098(db: &'static So) -> String { tuple_rows(by_name_title_date(db, false, "cv", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#up", "#down"]) }
fn q15829(db: &'static So) -> String { tuple_rows(by_name_title_date(db, false, "cv", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#up", "#down"]) }
fn q16127(db: &'static So) -> String { tuple_rows(by_name_title_date(db, false, "cv", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#up", "#down"]) }
fn q16800(db: &'static So) -> String { tuple_rows(by_name_title_date(db, false, "cv", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#up", "#down"]) }
fn q16861(db: &'static So) -> String { tuple_rows(by_name_title_date(db, false, "cv", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#up", "#down"]) }
fn q17616(db: &'static So) -> String { tuple_rows(by_name_title_date(db, false, "cv", PostWhere::All), "created", 0, &["title", "created", "owner", "#cx", "#up", "#down"]) }
fn q15224(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#up"]) }
fn q15622(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#up"]) }
fn q15899(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#up"]) }
fn q16039(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#up"]) }
fn q17823(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#up"]) }
fn q19758(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#up"]) }
fn q19483(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["title", "owner", "created", "#cx", "#up"]) }
fn q15185(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#v"]) }
fn q16425(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 0, &["owner", "title", "created", "#cx", "#up"]) }
fn q18789(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 0, &["owner", "title", "created", "#cx", "#up"]) }
fn q16604(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["owner", "title", "created", "#cx", "#up"]) }
fn q17398(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["owner", "title", "created", "#cx", "#up"]) }

// Votes and the answers self-join, so COUNT(a.Id) counts an answer once per
// vote and the upvote sum counts a vote once per answer
fn q16908(db: &'static So) -> String { tuple_rows(by_name_title_date_outer(db, true, "va", PostWhere::All), "created", 10, &["title", "created", "owner", "#ax", "#up"]) }
fn q19923(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "va", PostWhere::All), "created", 10, &["title", "created", "owner", "#ax", "#up"]) }

// --- the group is (DisplayName, Title) -------------------------------------

fn q16363(db: &'static So) -> String { tuple_rows(by_name_title(db, false, "cv", PostWhere::All), "#cx,#up", 0, &["owner", "title", "#cx", "#up", "#down"]) }
fn q19542(db: &'static So) -> String { tuple_rows(by_name_title(db, false, "cv", PostWhere::All), "#cx", 0, &["owner", "title", "#cx", "#up", "#down"]) }
fn q18809(db: &'static So) -> String { tuple_rows(by_name_title(db, true, "cv", PostWhere::All), "#up", 10, &["owner", "title", "#cx", "#up", "#down"]) }
fn q18932(db: &'static So) -> String { tuple_rows(by_name_title(db, true, "cv", PostWhere::All), "#up,#cx", 10, &["title", "owner", "#cx", "#up"]) }

// --- five that fit nothing else --------------------------------------------

// SELECT U.DisplayName, COUNT(P.Id), SUM(P.Score)
// FROM Users U JOIN Posts P ON U.Id = P.OwnerUserId
// GROUP BY U.DisplayName ORDER BY <PostCount|TotalScore> DESC LIMIT 10
//
// Grouped by the name, not the user, so the people who share one merge.
fn by_display_name(db: &'static So) -> Vec<(Str, i64, i64)> {
    let Post { score, owner_user, .. } = &db.post;
    let mut v = Vec::new();
    db.post
        .group_by(owner_user.select(&db.user.display_name))
        .select(score)
        .fold((0i64, 0i64), |(n, s), x| (n + 1, s + x))
        .drive(|dn, (n, s)| v.push((dn, n, s)));
    v
}

fn q17465(db: &'static So) -> String {
    let mut v = by_display_name(db);
    v.sort_by(|a, b| b.1.cmp(&a.1));
    rows(v.iter().take(10).map(|&(dn, n, s)| row(vec![V::S(dn), V::I(n), V::I(s)])))
}

fn q18632(db: &'static So) -> String {
    let mut v = by_display_name(db);
    v.sort_by(|a, b| b.2.cmp(&a.2));
    rows(v.iter().take(10).map(|&(dn, n, s)| row(vec![V::S(dn), V::I(n), V::I(s)])))
}

// SELECT U.Id, U.DisplayName, U.Reputation, COUNT(P.Id), SUM(CASE v=2), SUM(CASE v=3), SUM(P.Score)
// FROM Users U JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId
// GROUP BY U.Id, U.DisplayName, U.Reputation ORDER BY U.Reputation DESC LIMIT 10
//
// The Votes fan-out multiplies the post count and the score sum.
fn q12122(db: &'static So) -> String {
    let Post { score, owner_user, .. } = &db.post;
    let User { origid, display_name, reputation, .. } = &db.user;
    let agg = owner_user.inv().select(score.and(votes_of(db).select(&db.vote.vote_type_id).opt())).dense_fold_outer(
        db.user.id.n,
        (0i64, 0i64, 0i64, 0i64),
        |(n, s, u, d), (sc, vt)| (n + 1, s + sc, u + (vt == Some(2)) as i64, d + (vt == Some(3)) as i64),
    );
    let mut v = Vec::new();
    db.user
        .with(&agg)
        .select(origid.and(display_name).and(reputation).and(&agg))
        .filt(|(_, (n, _, _, _))| n > 0)
        .drive(|_, (((id, name), rep), (n, s, u, d))| v.push((rep, id, name, n, u, d, s)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(10).map(|&(rep, id, name, n, u, d, s)| {
        row(vec![V::I(id), V::S(name), V::I(rep), V::I(n), V::I(u), V::I(d), nullable(s, n)])
    }))
}

// SELECT p.Title, u.Reputation, AVG(p.Score), AVG(p.ViewCount), COUNT(a.Id)
// FROM Posts p LEFT JOIN Users u ... LEFT JOIN Posts a ON p.Id = a.ParentId
// WHERE p.PostTypeId = 1 GROUP BY p.Id, u.Reputation, p.Title
// ORDER BY AverageScore DESC LIMIT 100
//
// p.Id is in the key, so each group is one post and the two averages are just
// that post's Score and ViewCount.
fn q11611(db: &'static So) -> String {
    let Post { score, view_count, owner_user, .. } = &db.post;
    let ac = answers_per_post(db);
    let mut v = Vec::new();
    questions_only(db)
        .select(score.and(view_count.opt()).and(owner_user.select(&db.user.reputation).opt()).and(&ac))
        .drive(|p, (((s, w), rep), n)| v.push((s, p, w, rep, n)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(100).map(|&(s, p, w, rep, n)| {
        row(vec![
            title(db, p),
            oint(rep),
            V::F(s as f64),
            ofloat(w.map(|x| x as f64)),
            V::I(n),
        ])
    }))
}

// SELECT p.Id, p.Title, p.CreationDate, u.Id, u.DisplayName, u.Reputation,
//        COUNT(v.Id), AVG(p.Score), MAX(p.Score), MIN(p.Score), p.ViewCount
// FROM Posts p LEFT JOIN Users u ... LEFT JOIN Votes v ON p.Id = v.PostId
// GROUP BY p.Id, ... ORDER BY p.CreationDate DESC LIMIT 100
fn q12425(db: &'static So) -> String {
    let Post { origid, score, view_count, creation_date, owner_user, .. } = &db.post;
    let vv = votes_per_post(db);
    let mut v = Vec::new();
    db.post
        .select(
            origid
                .and(creation_date)
                .and(score)
                .and(view_count.opt())
                .and(owner_user.select((&db.user.origid).and(&db.user.display_name).and(&db.user.reputation)).opt())
                .and(&vv),
        )
        .drive(|p, (((((id, cd), s), w), owner), nv)| v.push((cd, id, p, s, w, owner, nv)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(100).map(|&(cd, id, p, s, w, owner, nv)| {
        row(vec![
            V::I(id),
            title(db, p),
            V::T(cd),
            oint(owner.map(|((uid, _), _)| uid)),
            ostr(owner.map(|((_, dn), _)| dn)),
            oint(owner.map(|(_, rep)| rep)),
            V::I(nv),
            V::F(s as f64),
            V::I(s),
            V::I(s),
            oint(w),
        ])
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("11611", q11611), ("12122", q12122), ("12425", q12425), ("15005", q15005),
    ("15023", q15023), ("15029", q15029), ("15034", q15034), ("15044", q15044),
    ("15098", q15098), ("15100", q15100), ("15109", q15109), ("15158", q15158),
    ("15160", q15160), ("15178", q15178), ("15185", q15185), ("15189", q15189),
    ("15224", q15224), ("15258", q15258), ("15314", q15314), ("15570", q15570),
    ("15622", q15622), ("15829", q15829), ("15840", q15840), ("15899", q15899),
    ("15976", q15976), ("16039", q16039), ("16127", q16127), ("16363", q16363),
    ("16386", q16386), ("16425", q16425), ("16464", q16464), ("16604", q16604),
    ("16644", q16644), ("16800", q16800), ("16861", q16861), ("16908", q16908),
    ("17188", q17188), ("17252", q17252), ("17373", q17373), ("17398", q17398),
    ("17465", q17465), ("17616", q17616), ("17823", q17823), ("17925", q17925),
    ("18146", q18146), ("18192", q18192), ("18351", q18351), ("18632", q18632),
    ("18789", q18789), ("18797", q18797), ("18809", q18809), ("18932", q18932),
    ("19043", q19043), ("19135", q19135), ("19483", q19483), ("19542", q19542),
    ("19758", q19758), ("19801", q19801), ("19923", q19923),
];
