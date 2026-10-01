use harness::prelude::*;

// A hundred more of the grouped-post query, now over every combination of the
// five children this corpus joins:
//
//   SELECT <columns>, <counts over c | v | a | b | ph>
//   FROM Posts p [LEFT] JOIN Users u ON p.OwnerUserId = u.Id
//        [LEFT JOIN Comments c ...] [LEFT JOIN Votes v ...]
//        [LEFT JOIN Posts a ON p.Id = a.ParentId]
//        [LEFT JOIN Badges b ON u.Id = b.UserId]
//        [LEFT JOIN PostHistory ph ON p.Id = ph.PostId]
//   [WHERE ...] GROUP BY <them> ORDER BY <one or two> DESC [LIMIT n]
//
// Every child crosses with every other, and the view drives that product, so
// `#cx` (COUNT(c.Id)) is a sum over the joined rows while `#c`
// (COUNT(DISTINCT c.Id)) comes from a second fold over one row per post.
// Which children are in the product is what each call's join set names.
//
// The WHERE is a `PostWhere` passed to the view, which builds it into the relation
// the group and the join are taken over.

fn posts(db: &'static So, only_q: bool, outer: bool, joins: &str) -> Vec<(Id<Post>, Agg)> {
    posts_with_counts(db, only_q, outer, joins)
}

// --- no WHERE beyond the post type -----------------------------------------

fn q19111(db: &'static So) -> String { render_posts(db, posts(db, true, false, "cv"), "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q19112(db: &'static So) -> String { render_posts(db, posts(db, true, false, "cv"), "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q17819(db: &'static So) -> String { render_posts(db, posts(db, true, false, "cv"), "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"]) }
fn q15471(db: &'static So) -> String { render_posts(db, posts(db, false, false, "cv"), "created", 10, &["id", "title", "created", "owner", "#cx", "#up"]) }
fn q19000(db: &'static So) -> String { render_posts(db, posts(db, false, false, "cv"), "created", 10, &["id", "title", "created", "score", "owner", "#cx", "#up", "#down"]) }
fn q17491(db: &'static So) -> String { render_posts(db, posts(db, true, false, "cv"), "created", 10, &["id", "title", "owner", "created", "views", "score", "#cx", "#up", "#down"]) }
fn q17271(db: &'static So) -> String { render_posts(db, posts(db, true, false, "cv"), "created", 10, &["id", "title", "owner", "created", "score", "views", "tags", "#cx", "#vx"]) }
fn q15179(db: &'static So) -> String { render_posts(db, posts(db, true, false, "cv"), "score", 10, &["id", "title", "created", "score", "owner", "#cx", "#vx"]) }
fn q11171(db: &'static So) -> String { render_posts(db, posts(db, true, false, "cv"), "score,#vx", 10, &["id", "title", "created", "score", "views", "owner", "rep", "#vx", "#cx"]) }
fn q16931(db: &'static So) -> String { render_posts(db, posts(db, true, true, "ca"), "created", 10, &["id", "title", "owner", "created", "score", "views", "#c", "#a"]) }
fn q10685(db: &'static So) -> String { render_posts(db, posts(db, false, false, "cv"), "created", 100, &["id", "title", "created", "owner", "type", "#cx", "#vx"]) }
fn q11037(db: &'static So) -> String { render_posts(db, posts(db, false, false, "cv"), "created", 100, &["id", "title", "created", "score", "views", "owner", "type", "#cx", "#vx"]) }
fn q13770(db: &'static So) -> String { render_posts(db, posts(db, false, false, "cv"), "created", 100, &["id", "title", "created", "views", "score", "owner", "type", "#cx", "#vx"]) }
fn q11682(db: &'static So) -> String { render_posts(db, posts(db, false, false, "cv"), "created", 100, &["id", "title", "created", "score", "views", "owner", "#cx", "#v", "#up", "#down"]) }
fn q12109(db: &'static So) -> String { render_posts(db, posts(db, false, false, "cv"), "created", 100, &["id", "title", "created", "score", "views", "answers", "comments", "owner_id", "owner", "rep", "#cx", "#up", "#down"]) }
fn q11410(db: &'static So) -> String { render_posts(db, posts(db, false, false, "cv"), "created", 100, &["id", "title", "type_id", "created", "score", "views", "answers", "comments", "owner_id", "owner", "rep", "#vx", "#cx"]) }
fn q13052(db: &'static So) -> String { render_posts(db, posts(db, false, false, "cvb"), "created", 100, &["id", "title", "created", "views", "score", "owner_id", "owner", "rep", "#vx", "#cx", "#bx"]) }
fn q13936(db: &'static So) -> String { render_posts(db, posts(db, false, true, "cv"), "created", 100, &["id", "title", "created", "score", "views", "owner", "#cx", "#up", "#down"]) }
fn q12113(db: &'static So) -> String { render_posts(db, posts_where(db, false, true, "cv", PostWhere::CreatedGe(date(2022, 1, 1))), "created", 100, &["id", "title", "created", "owner", "#c", "#v", "views", "score"]) }
fn q10811(db: &'static So) -> String { render_posts(db, posts(db, false, true, "cv"), "score,views", 100, &["id", "title", "created", "score", "views", "rep", "#cx", "#vx"]) }
fn q13511(db: &'static So) -> String { render_posts(db, posts(db, false, false, "cvb"), "created", 100, &["id", "title", "views", "created", "owner", "#c", "#up", "#down", "#b"]) }
fn q14688(db: &'static So) -> String { render_posts(db, posts(db, false, false, "vb"), "created", 100, &["id", "title", "created", "views", "score", "owner_id", "owner", "rep", "#vx", "#bx"]) }
fn q10693(db: &'static So) -> String { render_posts(db, posts(db, false, false, "cvh"), "created", 100, &["id", "title", "score", "views", "created", "owner", "#cx", "#v", "#hmax"]) }
fn q12201(db: &'static So) -> String { render_posts(db, posts(db, false, false, "cvh"), "created", 100, &["id", "title", "created", "owner", "score", "views", "#cx", "#vx", "#hmax"]) }
fn q14526(db: &'static So) -> String { render_posts(db, posts(db, false, true, "cv"), "created", 1000, &["id", "title", "created", "owner", "#cx", "#vx", "score", "views", "tags"]) }
fn q10410(db: &'static So) -> String { render_posts(db, posts(db, false, true, "cvh"), "created", 1000, &["id", "title", "created", "score", "views", "answers", "owner", "rep", "#vx", "#cx", "#hmax"]) }
fn q14715(db: &'static So) -> String { render_posts(db, posts_where(db, false, true, "cv", PostWhere::CreatedGe(date(2022, 1, 1))), "created", 100, &["id", "title", "created", "views", "score", "answers", "comments", "owner_id", "owner", "#cx", "#vx"]) }

// --- questions only --------------------------------------------------------

fn q10215(db: &'static So) -> String { render_posts(db, posts(db, true, false, "cvh"), "created", 100, &["id", "title", "created", "owner", "score", "views", "#cx", "#up", "#down", "#hmax"]) }
fn q10281(db: &'static So) -> String { render_posts(db, posts(db, true, false, "cv"), "created", 100, &["id", "title", "created", "score", "views", "owner", "rep", "#vx", "#cx"]) }
fn q11595(db: &'static So) -> String { render_posts(db, posts(db, true, false, "cv"), "created", 100, &["id", "title", "created", "score", "views", "#cx", "#vx", "owner", "rep"]) }
fn q11507(db: &'static So) -> String { render_posts(db, posts(db, true, false, "cvb"), "created", 100, &["id", "title", "created", "owner", "#cx", "#up", "#down", "#b"]) }
fn q11870(db: &'static So) -> String { render_posts(db, posts(db, true, false, "cvh"), "created", 100, &["id", "title", "created", "score", "views", "answers", "comments", "owner", "rep", "#cx", "#up", "#down", "#hmax"]) }
fn q11943(db: &'static So) -> String { render_posts(db, posts(db, true, true, "cvb"), "created", 100, &["id", "title", "created", "score", "views", "owner", "#cx", "#up", "#down", "#b"]) }
fn q12681(db: &'static So) -> String { render_posts(db, posts(db, true, false, "cvb"), "created", 100, &["id", "title", "created", "score", "views", "owner", "#cx", "#v", "#b"]) }
fn q12813(db: &'static So) -> String { render_posts(db, posts(db, true, true, "cvh"), "created", 100, &["id", "title", "created", "score", "views", "owner", "#cx", "#vx", "#hmax"]) }
fn q13042(db: &'static So) -> String { render_posts(db, posts(db, true, false, "cvh"), "created", 100, &["id", "title", "created", "views", "score", "owner", "#cx", "#vx", "#hmax"]) }
fn q13524(db: &'static So) -> String { render_posts(db, posts(db, true, true, "cv"), "created", 100, &["id", "title", "created", "views", "score", "owner", "#cx", "#vx"]) }
fn q13529(db: &'static So) -> String { render_posts(db, posts(db, true, true, "cvh"), "created", 100, &["id", "title", "owner", "created", "score", "views", "answers", "#cx", "#up", "#down", "#hmax"]) }
fn q13564(db: &'static So) -> String { render_posts(db, posts(db, true, false, "cvb"), "created", 100, &["id", "title", "created", "score", "views", "owner", "#cx", "#bx", "#up", "#down"]) }
fn q13590(db: &'static So) -> String { render_posts(db, posts(db, true, false, "cvh"), "created", 100, &["id", "title", "created", "score", "views", "owner", "#cx", "#vx", "#hmax"]) }
fn q13678(db: &'static So) -> String { render_posts(db, posts(db, true, false, "cv"), "created", 100, &["id", "title", "created", "views", "score", "owner", "#cx", "#vx"]) }
fn q14145(db: &'static So) -> String { render_posts(db, posts(db, true, false, "cvb"), "created", 100, &["id", "title", "created", "score", "views", "owner", "#cx", "#up", "#down", "#b"]) }
fn q14384(db: &'static So) -> String { render_posts(db, posts(db, true, false, "cvb"), "created", 100, &["id", "title", "created", "views", "score", "owner", "#cx", "#v", "#up", "#down", "#bx"]) }
fn q14590(db: &'static So) -> String { render_posts(db, posts(db, true, false, "cvb"), "created", 100, &["id", "title", "created", "views", "score", "rep", "#cx", "#bx", "#up", "#down"]) }
fn q14625(db: &'static So) -> String { render_posts(db, posts(db, true, true, "cvb"), "created", 100, &["id", "title", "created", "owner", "#cx", "#up", "#down", "#b"]) }
fn q14871(db: &'static So) -> String { render_posts(db, posts(db, true, false, "cvh"), "created", 100, &["id", "title", "created", "score", "views", "owner", "#cx", "#up", "#down", "#h"]) }
fn q12623(db: &'static So) -> String { render_posts(db, posts(db, true, false, "cvh"), "created", 1000, &["id", "title", "created", "owner", "#cx", "#vx", "#hmax"]) }
fn q10925(db: &'static So) -> String { render_posts(db, posts(db, true, false, "cv"), "score,created", 100, &["id", "title", "created", "views", "score", "#cx", "#vx", "owner"]) }
fn q11889(db: &'static So) -> String { render_posts(db, posts(db, true, true, "cvb"), "views", 100, &["id", "title", "created", "views", "score", "owner", "#cx", "#up", "#down", "#b"]) }
fn q12459(db: &'static So) -> String { render_posts(db, posts(db, true, true, "cvb"), "created", 100, &["title", "created", "owner", "#cx", "#vx", "#b"]) }
fn q11769(db: &'static So) -> String { render_posts(db, posts(db, true, false, "cvh"), "#cx,created", 100, &["id", "title", "created", "owner", "#cx", "#up", "#down", "#hmax"]) }
fn q13555(db: &'static So) -> String { render_posts(db, posts(db, true, true, "cva"), "created", 0, &["id", "title", "created", "views", "score", "owner", "#cx", "#a", "#v"]) }
// GROUP BY p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName, u.Reputation: no p.Id.
fn q12352(db: &'static So) -> String {
    let Post { title, creation_date, score, view_count, owner_user, .. } = &db.post;
    let key = owner_user
        .select(&db.user.display_name)
        .and(owner_user.select(&db.user.reputation))
        .and(title.opt())
        .and(creation_date)
        .and(score)
        .and(view_count.opt());
    let g = group_posts(db, &post_base(db, true, true, PostWhere::All), &key, "cv");
    let v = g
        .into_iter()
        .map(|((((((o, r), t), cd), s), w), agg)| TupleGroup {
            owner: Some(o),
            title: t,
            created: cd,
            score: s,
            views: w,
            ptype: None,
            body: None,
            rep: Some(r),
            owner_id: None,
            agg,
        })
        .collect();
    tuple_rows(v, "created", 0, &["title", "created", "score", "views", "#cx", "#vx", "owner", "rep"])
}
fn q14044(db: &'static So) -> String { tuple_rows(by_name_title_date_score_views(db, true, "cvh", PostWhere::All), "score,created", 100, &["title", "created", "owner", "views", "score", "#cx", "#vx", "#hmax"]) }

// --- a CreationDate filter -------------------------------------------------

fn q15918(db: &'static So) -> String {
    let v = by_name_title_date_ptype(db, false, "cv", PostWhere::CreatedGt(date(2023, 1, 1)));
    tuple_rows(v, "created", 0, &["owner", "title", "created", "ptype", "#cx", "#up", "#down"])
}

fn q15465(db: &'static So) -> String {
    let v = by_name_title_date_ptype(db, false, "cv", PostWhere::CreatedGe(date(2022, 1, 1)));
    tuple_rows(v, "created", 0, &["title", "created", "owner", "ptype", "#cx", "#up", "#down"])
}

fn q13419(db: &'static So) -> String {
    let v = by_name_title_date_score_views_ptype(db, false, "cv", PostWhere::CreatedGe(date(2022, 1, 1)));
    tuple_rows(v, "created", 100, &["title", "created", "owner", "#cx", "#up", "#down", "views", "score", "ptype"])
}

fn q11698(db: &'static So) -> String { render_posts(db, posts_where(db, false, false, "cv", PostWhere::CreatedGe(date(2023, 1, 1))), "created", 0, &["id", "title", "created", "owner", "rep", "#cx", "#vx"]) }
fn q11247(db: &'static So) -> String { render_posts(db, posts_where(db, false, false, "cvb", PostWhere::CreatedGe(date(2023, 1, 1))), "created", 0, &["id", "title", "created", "owner", "#cx", "#vx", "#up", "#down", "#b"]) }
fn q11916(db: &'static So) -> String { render_posts(db, posts_where(db, false, false, "cvb", PostWhere::CreatedGe(date(2023, 1, 1))), "created", 0, &["id", "title", "created", "views", "score", "answers", "comments", "owner", "rep", "#c", "#v", "#b"]) }
fn q13512(db: &'static So) -> String { render_posts(db, posts_where(db, false, false, "cvb", PostWhere::CreatedGe(date(2023, 1, 1))), "created", 0, &["id", "title", "created", "score", "views", "answers", "comments", "owner_id", "owner", "rep", "#v", "#c", "#b"]) }
fn q13889(db: &'static So) -> String { render_posts(db, posts_where(db, false, false, "cvb", PostWhere::CreatedGe(date(2023, 1, 1))), "created", 0, &["id", "title", "created", "views", "score", "owner", "#cx", "#up", "#down", "#bx"]) }
fn q11155(db: &'static So) -> String { render_posts(db, posts_where(db, false, false, "cvb", PostWhere::CreatedGe(date(2023, 1, 1))), "created", 0, &["id", "title", "created", "views", "score", "answers", "comments", "owner", "rep", "type", "#cx", "#up", "#down", "#bx"]) }
fn q11795(db: &'static So) -> String { render_posts(db, posts_where(db, false, false, "cv", PostWhere::CreatedGe(date(2023, 1, 1))), "created", 0, &["id", "title", "created", "views", "score", "owner", "#cx", "#vx", "#up", "#down"]) }
fn q13450(db: &'static So) -> String { render_posts(db, posts_where(db, false, false, "cv", PostWhere::CreatedGe(date(2023, 1, 1))), "created", 0, &["id", "title", "created", "score", "views", "answers", "comments", "owner", "rep", "#cx", "#up", "#down"]) }
fn q13550(db: &'static So) -> String { render_posts(db, posts_where(db, false, false, "cv", PostWhere::CreatedGe(date(2023, 1, 1))), "created", 0, &["id", "title", "created", "owner", "#vx", "#cx", "#up", "#down"]) }
fn q12421(db: &'static So) -> String { render_posts(db, posts_where(db, false, false, "cvh", PostWhere::CreatedGe(date(2022, 1, 1))), "created", 0, &["id", "title", "created", "views", "score", "owner", "#cx", "#v", "#hmax"]) }
fn q14379(db: &'static So) -> String { render_posts(db, posts_where(db, false, false, "cv", PostWhere::CreatedGe(date(2023, 1, 1))), "created,score", 100, &["id", "title", "created", "score", "views", "rep", "#cx", "#vx"]) }
fn q10536(db: &'static So) -> String { render_posts(db, posts_where(db, false, false, "v", PostWhere::CreatedGe(date(2023, 1, 1))), "created", 100, &["id", "title", "created", "score", "views", "owner", "rep", "#vx", "#up", "#down"]) }
fn q10538(db: &'static So) -> String { render_posts(db, posts_where(db, false, false, "cv", PostWhere::CreatedGe(date(2023, 1, 1))), "created", 100, &["id", "title", "created", "score", "views", "#cx", "#vx", "#up", "#down", "owner"]) }
fn q11137(db: &'static So) -> String { render_posts(db, posts_where(db, false, true, "cv", PostWhere::CreatedGe(date(2023, 1, 1))), "created", 100, &["id", "title", "owner", "created", "views", "score", "#cx", "#vx"]) }
fn q11628(db: &'static So) -> String { render_posts(db, posts_where(db, false, false, "cv", PostWhere::CreatedGe(date(2023, 1, 1))), "created", 100, &["id", "title", "created", "score", "views", "answers", "comments", "owner", "#cx", "#vx"]) }
fn q11953(db: &'static So) -> String { render_posts(db, posts_where(db, false, false, "cv", PostWhere::CreatedGe(date(2023, 1, 1))), "created", 100, &["id", "title", "created", "score", "views", "#cx", "#vx", "owner", "rep"]) }
fn q12605(db: &'static So) -> String { render_posts(db, posts_where(db, false, false, "cv", PostWhere::CreatedGe(date(2023, 1, 1))), "created", 100, &["id", "title", "created", "owner", "#vx", "#cx", "#up", "#down"]) }
fn q13554(db: &'static So) -> String { render_posts(db, posts_where(db, false, false, "cv", PostWhere::CreatedGe(date(2023, 1, 1))), "created", 100, &["id", "title", "created", "score", "views", "owner", "rep", "#cx", "#vx"]) }
fn q14104(db: &'static So) -> String { render_posts(db, posts_where(db, false, false, "cv", PostWhere::CreatedGe(date(2023, 1, 1))), "created", 100, &["id", "title", "views", "score", "created", "#cx", "#v", "owner", "rep"]) }
fn q12596(db: &'static So) -> String { render_posts(db, posts_where(db, false, false, "cv", PostWhere::CreatedGe(date(2023, 1, 1))), "created", 100, &["id", "title", "created", "views", "owner", "rep", "#cx", "#up", "#down"]) }
fn q11986(db: &'static So) -> String { render_posts(db, posts_where(db, false, false, "cvb", PostWhere::CreatedGe(date(2020, 1, 1))), "created", 100, &["id", "title", "created", "score", "views", "owner", "#vx", "#up", "#down", "#cx", "#bx"]) }
fn q12324(db: &'static So) -> String { render_posts(db, posts_where(db, false, false, "cv", PostWhere::CreatedGe(date(2020, 1, 1))), "created", 100, &["id", "title", "created", "owner", "#cx", "#up", "#down", "views", "score", "answers"]) }
fn q14534(db: &'static So) -> String { render_posts(db, posts_where(db, false, false, "cvb", PostWhere::CreatedGe(date(2020, 1, 1))), "created", 100, &["id", "title", "created", "score", "views", "owner", "#c", "#b", "#up", "#down"]) }
fn q14354(db: &'static So) -> String { render_posts(db, posts_where(db, false, false, "cv", PostWhere::CreatedGe(date(2022, 1, 1))), "created", 100, &["id", "title", "created", "score", "views", "answers", "comments", "owner_id", "owner", "rep", "#cx", "#vx"]) }
fn q14060(db: &'static So) -> String { render_posts(db, posts_where(db, false, true, "cvh", PostWhere::CreatedGe(date(2022, 1, 1))), "created", 100, &["id", "title", "created", "score", "owner", "#cx", "#vx", "#hx"]) }
fn q13756(db: &'static So) -> String { render_posts(db, posts(db, false, true, "cvh"), "created", 100, &["id", "title", "created", "score", "views", "owner", "rep", "#cx", "#v", "#hmax"]) }
fn q13024(db: &'static So) -> String { render_posts(db, posts_where(db, false, false, "b", PostWhere::CreatedGe(date(2022, 1, 1))), "score,created", 0, &["id", "title", "created", "score", "views", "answers", "owner_id", "owner", "rep", "#bx"]) }
fn q11123(db: &'static So) -> String { render_posts(db, posts_where(db, false, false, "cv", PostWhere::CreatedGe(date(2023, 1, 1))), "score,created", 0, &["id", "title", "owner", "created", "score", "views", "answers", "comments", "#cx", "#up", "#down"]) }
fn q14579(db: &'static So) -> String { render_posts(db, posts_where(db, false, false, "cv", PostWhere::CreatedGe(date(2020, 1, 1))), "score,created", 100, &["id", "title", "created", "score", "views", "owner", "#cx", "#vx"]) }
fn q11605(db: &'static So) -> String { render_posts(db, posts_where(db, false, false, "cvb", PostWhere::CreatedGe(date(2021, 1, 1))), "score,created", 100, &["id", "title", "created", "score", "views", "owner", "#cx", "#up", "#down", "#bx"]) }
fn q10094(db: &'static So) -> String { render_posts(db, posts_where(db, false, false, "cvb", PostWhere::CreatedGe(date(2020, 1, 1))), "score,views", 0, &["id", "title", "created", "views", "score", "owner_id", "owner", "rep", "#vx", "#up", "#down", "#cx", "#bx"]) }
fn q13721(db: &'static So) -> String { render_posts(db, posts_where(db, false, false, "cvb", PostWhere::CreatedGe(date(2023, 1, 1))), "score,views", 0, &["id", "title", "created", "score", "views", "#cx", "#vx", "#b", "owner"]) }
fn q14891(db: &'static So) -> String { render_posts(db, posts_where(db, false, false, "cvb", PostWhere::CreatedGe(date(2022, 1, 1))), "views", 0, &["id", "title", "created", "views", "#c", "#v", "#b", "owner", "rep", "activity"]) }
fn q13148(db: &'static So) -> String { render_posts(db, posts_where(db, false, false, "cv", PostWhere::CreatedGe(date(2020, 1, 1))), "#vx,created", 0, &["id", "title", "created", "owner_id", "owner", "rep", "#vx", "#up", "#down", "#cx"]) }
fn q12287(db: &'static So) -> String { render_posts(db, posts(db, false, false, "cv"), "created", 100, &["title", "created", "score", "views", "answers", "owner", "rep", "#vx", "#cx"]) }

// --- other filters ---------------------------------------------------------

fn q13924(db: &'static So) -> String {
    let v = posts_where(db, false, true, "cv", PostWhere::ViewsGt(0));
    render_posts(db, v, "created", 100, &["id", "title", "created", "score", "owner", "#c", "#v"])
}

fn q11796(db: &'static So) -> String {
    let v = posts_where(db, false, true, "cv", PostWhere::ScoreGt(0));
    render_posts(db, v, "score,created", 0, &["id", "title", "views", "score", "created", "owner", "#c", "#v"])
}

// --- grouped by a value tuple ----------------------------------------------

fn q15123(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#up", "#down"]) }
fn q16140(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#up", "#down"]) }
fn q19697(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#up", "#down"]) }
fn q19071(db: &'static So) -> String { tuple_rows(by_name_title_date(db, true, "c", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx"]) }
fn q18732(db: &'static So) -> String { tuple_rows(by_name_title_date_views(db, false, "cv", PostWhere::All), "created", 10, &["owner", "title", "created", "views", "#cx", "#up", "#down"]) }

// SELECT u.DisplayName, COUNT(p.Id), SUM(CASE v=2), SUM(CASE v=3)
// FROM Users u JOIN Posts p ... LEFT JOIN Votes v ON p.Id = v.PostId
// WHERE u.Reputation > 1000 GROUP BY u.DisplayName ORDER BY PostCount DESC
//
// Grouped by the name, so the people who share one merge; the Votes fan-out
// multiplies COUNT(p.Id).
fn q19116(db: &'static So) -> String {
    let Post { owner_user, .. } = &db.post;
    let mut v = Vec::new();
    db.post
        .with(owner_user.select(&db.user.reputation).gt(1000))
        .group_by(owner_user.select(&db.user.display_name))
        .select(votes_of(db).select(&db.vote.vote_type_id).opt())
        .fold((0i64, 0i64, 0i64), |(n, u, d), vt| (n + 1, u + (vt == Some(2)) as i64, d + (vt == Some(3)) as i64))
        .drive(|dn, (n, u, d)| v.push((n, dn, u, d)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().map(|&(n, name, u, d)| row(vec![V::S(name), V::I(n), V::I(u), V::I(d)])))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("10094", q10094), ("10215", q10215), ("10281", q10281), ("10410", q10410),
    ("10536", q10536), ("10538", q10538), ("10685", q10685), ("10693", q10693),
    ("10811", q10811), ("10925", q10925), ("11037", q11037), ("11123", q11123),
    ("11137", q11137), ("11155", q11155), ("11171", q11171), ("11247", q11247),
    ("11410", q11410), ("11507", q11507), ("11595", q11595), ("11605", q11605),
    ("11628", q11628), ("11682", q11682), ("11698", q11698), ("11769", q11769),
    ("11795", q11795), ("11796", q11796), ("11870", q11870), ("11889", q11889),
    ("11916", q11916), ("11943", q11943), ("11953", q11953), ("11986", q11986),
    ("12109", q12109), ("12113", q12113), ("12201", q12201), ("12287", q12287),
    ("12324", q12324), ("12352", q12352), ("12421", q12421), ("12459", q12459),
    ("12596", q12596), ("12605", q12605), ("12623", q12623), ("12681", q12681),
    ("12813", q12813), ("13024", q13024), ("13042", q13042), ("13052", q13052),
    ("13148", q13148), ("13419", q13419), ("13450", q13450), ("13511", q13511),
    ("13512", q13512), ("13524", q13524), ("13529", q13529), ("13550", q13550),
    ("13554", q13554), ("13555", q13555), ("13564", q13564), ("13590", q13590),
    ("13678", q13678), ("13721", q13721), ("13756", q13756), ("13770", q13770),
    ("13889", q13889), ("13924", q13924), ("13936", q13936), ("14044", q14044),
    ("14060", q14060), ("14104", q14104), ("14145", q14145), ("14354", q14354),
    ("14379", q14379), ("14384", q14384), ("14526", q14526), ("14534", q14534),
    ("14579", q14579), ("14590", q14590), ("14625", q14625), ("14688", q14688),
    ("14715", q14715), ("14871", q14871), ("14891", q14891), ("15123", q15123),
    ("15179", q15179), ("15465", q15465), ("15471", q15471), ("15918", q15918),
    ("16140", q16140), ("16931", q16931), ("17271", q17271), ("17491", q17491),
    ("17819", q17819), ("18732", q18732), ("19000", q19000), ("19071", q19071),
    ("19111", q19111), ("19112", q19112), ("19116", q19116), ("19697", q19697),
];
