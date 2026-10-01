use crate::fmt::V;
use crate::schema::*;
use prela::engine::*;
use prela::loader::{Col, Key, Set, Str};

pub struct Question {
    pub pid: Id<Post>,
    pub uid: Id<User>,
    pub id: i64,
    pub score: i64,
    pub views: Option<i64>,
    pub created: i64,
    pub body: &'static str,
    pub display_name: &'static str,
    pub reputation: i64,
}

pub fn questions(db: &'static So) -> Vec<Question> {
    let Post {
        post_type_id, origid, score, view_count, creation_date, body, owner_user, ..
    } = &db.post;
    let User { display_name, reputation, .. } = &db.user;

    let mut v = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .select(
            origid
                .and(score)
                .and(view_count.opt())
                .and(creation_date)
                .and(body)
                .and(owner_user)
                .and(owner_user.select(display_name.and(reputation))),
        )
        .drive(
            |pid,
             ((((((id, score), views), created), body), uid), (display_name, reputation))| {
                v.push(Question {
                    pid,
                    uid,
                    id,
                    score,
                    views,
                    created,
                    body,
                    display_name,
                    reputation,
                })
            },
        );
    v
}

pub fn by_created(db: &'static So) -> Vec<Question> {
    let mut v = questions(db);
    v.sort_by(|a, b| b.created.cmp(&a.created));
    v.truncate(10);
    v
}

pub fn by_score(db: &'static So) -> Vec<Question> {
    let mut v = questions(db);
    v.sort_by(|a, b| b.score.cmp(&a.score));
    v.truncate(10);
    v
}

pub fn by_views(db: &'static So) -> Vec<Question> {
    let mut v = questions(db);
    v.sort_by(|a, b| desc_nulls_last(a.views, b.views));
    v.truncate(10);
    v
}

#[derive(Clone, Copy)]
pub struct TypeAgg {
    pub type_id: i64,
    pub name: &'static str,
    pub n: i64,
    pub score_sum: i64,
    pub views_n: i64,
    pub views_sum: i64,
    pub views_max: i64,
    pub answers_n: i64,
    pub answers_sum: i64,
    pub last_activity_max: i64,
    pub score_max: i64,
    pub score_min: i64,
    pub rep_n: i64,
    pub rep_sum: i64,
    pub comment_sum: i64,
    pub views_min: i64,
    pub created_max: i64,
    pub created_min: i64,
    /// SUM(CASE WHEN p.OwnerUserId / AcceptedAnswerId / ClosedDate IS NOT NULL THEN 1 ELSE 0 END)
    pub owner_n: i64,
    pub accepted_n: i64,
    pub closed_n: i64,
    /// SUM(p.LastActivityDate - p.CreationDate) in microseconds, exactly.
    pub active_us: i128,
}

pub fn type_aggs(db: &'static So) -> Vec<TypeAgg> {
    static CACHE: std::sync::OnceLock<Vec<TypeAgg>> = std::sync::OnceLock::new();
    CACHE.get_or_init(|| compute_type_aggs(db)).clone()
}

fn compute_type_aggs(db: &'static So) -> Vec<TypeAgg> {
    let Post {
        post_type,
        score,
        view_count,
        answer_count,
        last_activity_date,
        comment_count,
        creation_date,
        owner_user,
        owner_user_id,
        accepted_answer_id,
        closed_date,
        ..
    } = &db.post;
    let PostType { name, origid: type_id, .. } = &db.post_type;
    let User { reputation, .. } = &db.user;

    let init = TypeAgg {
        type_id: 0,
        name: "",
        n: 0,
        score_sum: 0,
        views_n: 0,
        views_sum: 0,
        views_max: i64::MIN,
        answers_n: 0,
        answers_sum: 0,
        last_activity_max: i64::MIN,
        score_max: i64::MIN,
        score_min: i64::MAX,
        rep_n: 0,
        rep_sum: 0,
        comment_sum: 0,
        views_min: i64::MAX,
        created_max: i64::MIN,
        created_min: i64::MAX,
        owner_n: 0,
        accepted_n: 0,
        closed_n: 0,
        active_us: 0,
    };

    let mut out = Vec::new();
    db.post
        .group_by(post_type.select(name))
        .select(
            score
                .and(comment_count)
                .and(post_type.select(type_id))
                .and(creation_date)
                .and(last_activity_date)
                .and(view_count.opt())
                .and(answer_count.opt())
                .and(owner_user.select(reputation).opt())
                .and(owner_user_id.opt())
                .and(accepted_answer_id.opt())
                .and(closed_date.opt()),
        )
        .fold(init, |a, ((((((((((sc, cc), ti), cd), la), vc), ac), rep), oi), ai), cl)| TypeAgg {
            type_id: ti,
            owner_n: a.owner_n + oi.is_some() as i64,
            accepted_n: a.accepted_n + ai.is_some() as i64,
            closed_n: a.closed_n + cl.is_some() as i64,
            active_us: a.active_us + (la - cd) as i128,
            n: a.n + 1,
            score_sum: a.score_sum + sc,
            score_max: a.score_max.max(sc),
            score_min: a.score_min.min(sc),
            comment_sum: a.comment_sum + cc,
            created_max: a.created_max.max(cd),
            created_min: a.created_min.min(cd),
            last_activity_max: a.last_activity_max.max(la),
            views_n: a.views_n + vc.is_some() as i64,
            views_sum: a.views_sum + vc.unwrap_or(0),
            views_max: vc.map_or(a.views_max, |x| a.views_max.max(x)),
            views_min: vc.map_or(a.views_min, |x| a.views_min.min(x)),
            answers_n: a.answers_n + ac.is_some() as i64,
            answers_sum: a.answers_sum + ac.unwrap_or(0),
            rep_n: a.rep_n + rep.is_some() as i64,
            rep_sum: a.rep_sum + rep.unwrap_or(0),
            ..a
        })
        .drive(|k, a| out.push(TypeAgg { name: k, ..a }));
    out
}

pub fn by_count(db: &'static So) -> Vec<TypeAgg> {
    let mut a = type_aggs(db);
    a.sort_by(|x, y| y.n.cmp(&x.n));
    a
}

pub fn avg(sum: i64, n: i64) -> V {
    if n == 0 { V::Null } else { V::F(sum as f64 / n as f64) }
}

pub fn nullable(sum: i64, n: i64) -> V {
    if n == 0 { V::Null } else { V::I(sum) }
}

pub fn title(db: &'static So, p: Id<Post>) -> V {
    crate::fmt::ostr(db.post.title.get(p))
}

#[derive(Clone)]
pub struct OwnedTypeAgg {
    pub name: &'static str,
    pub n: i64,
    pub score_sum: i64,
    pub views_n: i64,
    pub views_sum: i64,
    pub rep_sum: i64,
}

pub fn owned_type_aggs(db: &'static So) -> Vec<OwnedTypeAgg> {
    static CACHE: std::sync::OnceLock<Vec<OwnedTypeAgg>> = std::sync::OnceLock::new();
    CACHE.get_or_init(|| compute_owned(db)).clone()
}

fn compute_owned(db: &'static So) -> Vec<OwnedTypeAgg> {
    let Post { post_type, score, view_count, owner_user, .. } = &db.post;
    let PostType { name, .. } = &db.post_type;
    let User { reputation, .. } = &db.user;

    let mut out = Vec::new();
    db.post
        .with(owner_user)
        .group_by(post_type.select(name))
        .select(score.and(owner_user.select(reputation)).and(view_count.opt()))
        .fold(
            (0i64, 0i64, 0i64, 0i64, 0i64),
            |(n, s, r, vn, vs), ((x, rep), v)| {
                (n + 1, s + x, r + rep, vn + v.is_some() as i64, vs + v.unwrap_or(0))
            },
        )
        .drive(|k, (n, s, r, vn, vs)| {
            out.push(OwnedTypeAgg {
                name: k,
                n,
                score_sum: s,
                views_n: vn,
                views_sum: vs,
                rep_sum: r,
            })
        });
    out
}

pub fn owned_by_count(db: &'static So) -> Vec<OwnedTypeAgg> {
    let mut a = owned_type_aggs(db);
    a.sort_by(|x, y| y.n.cmp(&x.n));
    a
}

/// `$name(db)` is the child index parent -> child, memoised. `$maybe(db, on)`
/// is the same index when `on`, and the *empty* relation of the same type when
/// not — which is what a child table the query does not join contributes:
/// probing it yields nothing, so `.opt()` gives the one `None` row a LEFT JOIN
/// of an empty table would, and the product is unchanged.
///
/// It has to be an empty index rather than a filtered one. A `Filter` decides
/// per value, so it still walks the whole child list before discarding it, and
/// a table the query never mentions would then cost its own size on every row
/// of the product it is not part of.
macro_rules! child_index {
    ($name:ident, $maybe:ident, $parent:ty, $child:ty, $edge:ident . $field:ident) => {
        pub fn $name(db: &'static So) -> &'static HashIdx<Id<$parent>, Id<$child>> {
            static CACHE: std::sync::OnceLock<HashIdx<Id<$parent>, Id<$child>>> = std::sync::OnceLock::new();
            CACHE.get_or_init(|| (&db.$edge.$field).inv().collect())
        }
        pub fn $maybe(db: &'static So, on: bool) -> &'static HashIdx<Id<$parent>, Id<$child>> {
            static EMPTY: std::sync::OnceLock<HashIdx<Id<$parent>, Id<$child>>> = std::sync::OnceLock::new();
            if on {
                $name(db)
            } else {
                EMPTY.get_or_init(|| HashIdx { idx: Default::default() })
            }
        }
    };
}

child_index!(comments_of, comments_of_if, Post, Comment, comment.post);
child_index!(votes_of, votes_of_if, Post, Vote, vote.post);
child_index!(history_of, history_of_if, Post, PostHistory, post_history.post);
child_index!(children_of, children_of_if, Post, Post, post.parent);

/// The children of a post with `PostTypeId = 2`: `LEFT JOIN Posts a ON
/// p.Id = a.ParentId AND a.PostTypeId = 2`.
pub fn answers_of(db: &'static So) -> &'static HashIdx<Id<Post>, Id<Post>> {
    static CACHE: std::sync::OnceLock<HashIdx<Id<Post>, Id<Post>>> = std::sync::OnceLock::new();
    CACHE.get_or_init(|| db.post.with((&db.post.post_type_id).eq(2)).select(&db.post.parent).inv().collect())
}

pub fn answers_of_if(db: &'static So, on: bool) -> &'static HashIdx<Id<Post>, Id<Post>> {
    if on { answers_of(db) } else { children_of_if(db, false) }
}
child_index!(posts_of, posts_of_if, User, Post, post.owner_user);
child_index!(badges_of, badges_of_if, User, Badge, badge.user);
child_index!(comments_by, comments_by_if, User, Comment, comment.user);
child_index!(votes_by, votes_by_if, User, Vote, vote.user);
child_index!(links_of, links_of_if, Post, PostLink, post_link.post);

pub fn engagement<Q: Drive<R = Id<Post>>>(db: &'static So, posts: Q) -> Fold<Id<Post>, (i64, i64, i64, i64)> {
    posts
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold((0i64, 0i64, 0i64, 0i64), |(c, v, u, d), (ci, vt)| {
            (
                c + ci.is_some() as i64,
                v + vt.is_some() as i64,
                u + (vt == Some(2)) as i64,
                d + (vt == Some(3)) as i64,
            )
        })
}

pub fn comments_per_post(db: &'static So) -> DenseFold<Id<Post>, i64> {
    (&db.comment.post).inv().dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1)
}

pub fn votes_per_post(db: &'static So) -> DenseFold<Id<Post>, i64> {
    (&db.vote.post).inv().dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1)
}

pub fn answers_per_post(db: &'static So) -> DenseFold<Id<Post>, i64> {
    (&db.post.parent).inv().dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1)
}

pub fn typed_answers_per_post(db: &'static So) -> DenseFold<Id<Post>, i64> {
    db.post
        .with((&db.post.post_type_id).eq(2))
        .select(&db.post.parent)
        .inv()
        .dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1)
}

pub fn history_per_post(db: &'static So) -> DenseFold<Id<Post>, i64> {
    (&db.post_history.post).inv().dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1)
}

/// The post's latest PostHistory instant, `i64::MIN` where it has none.
pub fn history_max_date(db: &'static So) -> DenseFold<Id<Post>, i64> {
    (&db.post_history.post)
        .inv()
        .select(&db.post_history.creation_date)
        .dense_fold_outer(db.post.id.n, i64::MIN, |a, d| a.max(d))
}

/// Comments the user wrote, which is not the same as comments on their posts.
pub fn comments_per_user(db: &'static So) -> DenseFold<Id<User>, i64> {
    (&db.comment.user).inv().dense_fold_outer(db.user.id.n, 0i64, |a, _| a + 1)
}

/// Votes the user cast. Votes.UserId is NULL for all but a handful of rows.
pub fn votes_per_user(db: &'static So) -> DenseFold<Id<User>, i64> {
    (&db.vote.user).inv().dense_fold_outer(db.user.id.n, 0i64, |a, _| a + 1)
}

/// (sum, count) of the non-NULL BountyAmounts on each post's votes.
pub fn bounty_per_post(db: &'static So) -> DenseFold<Id<Post>, (i64, i64)> {
    (&db.vote.post)
        .inv()
        .select((&db.vote.bounty_amount).opt())
        .dense_fold_outer(db.post.id.n, (0i64, 0i64), |(s, n), b| {
            (s + b.unwrap_or(0), n + b.is_some() as i64)
        })
}

pub fn badges_per_user(db: &'static So) -> DenseFold<Id<User>, i64> {
    (&db.badge.user).inv().dense_fold_outer(db.user.id.n, 0i64, |a, _| a + 1)
}

pub fn owned(db: &'static So) -> Restrict<Key<Post>, &'static Set<Post, Id<User>>> {
    db.post.with(&db.post.owner_user)
}

/// The aggregates SQL computes over a group of posts, given which children
/// the query LEFT JOINed.
///
/// The children cross: a post with 3 comments and 5 votes is 15 rows, so
/// `COUNT(c.Id)` is 3*5 while `COUNT(DISTINCT c.Id)` is 3. Which multipliers
/// apply depends on which joins are present, so the join set is passed in
/// rather than guessed. See notes/translation-failures.md 3.
#[derive(Clone, Copy, Default)]
pub struct Agg {
    /// COUNT(DISTINCT p.Id): the posts in the group.
    pub n: i64,
    /// COUNT(p.Id) / COUNT(*): a post once per combination of its children.
    pub rows: i64,
    /// COUNT(DISTINCT c.Id) and friends — each child's own count.
    pub c: i64,
    pub v: i64,
    pub a: i64,
    pub b: i64,
    pub h: i64,
    /// COUNT(c.Id) and friends — the same, multiplied by the other children.
    pub cx: i64,
    pub vx: i64,
    pub ax: i64,
    pub bx: i64,
    pub hx: i64,
    /// SUM(CASE WHEN v.VoteTypeId = 2|3 THEN 1 ELSE 0 END), multiplied by the
    /// children other than Votes.
    pub up: i64,
    pub down: i64,
    /// MAX(ph.CreationDate), `i64::MIN` where the post has no history.
    pub hmax: i64,
}

/// Which children a query LEFT JOINed: any of `c`omments, `v`otes,
/// `a`nswers, `b`adges, post `h`istory. `A` is the answers joined with
/// `AND a.PostTypeId = 2`.
#[derive(Clone, Copy)]
pub struct Joins {
    c: bool,
    v: bool,
    a: bool,
    b: bool,
    h: bool,
    l: bool,
    typed: bool,
}

impl Joins {
    pub fn parse(spec: &str) -> Joins {
        let j = Joins {
            c: spec.contains('c'),
            v: spec.contains('v'),
            a: spec.contains('a') || spec.contains('A'),
            typed: spec.contains('A'),
            l: spec.contains('l'),
            b: spec.contains('b'),
            h: spec.contains('h'),
        };
        assert!(spec.chars().all(|x| "cvabhAl".contains(x)), "joins: {spec}");
        j
    }
}

/// Group posts by `key` and aggregate the children named by `joins`.
///
/// The children are *joined*, not counted: `.and` is SQL's product, probing
/// each child at the post and emitting every combination, and `.opt()` turns
/// "no children" into the one `None` row a LEFT JOIN keeps. A post with 3
/// comments and 2 votes drives 6 rows, so `COUNT(c.Id)` falls out of the fold
/// as `c.is_some()` summed over them. A child the query does not join is
/// filtered to nothing, which `.opt()` then makes one `None` row — exactly
/// what not joining that table contributes. See notes/limitations.md,
/// "A LEFT JOIN of two children is `.and`".
///
/// `COUNT(DISTINCT x.Id)` does not want the product, so it comes from a
/// second fold over one row per post. Each child row belongs to exactly one
/// post, so the group's distinct child count is the sum of its posts' own
/// counts. Two aggregate shapes over one group means two folds.
pub fn group_posts<Q, K>(db: &'static So, posts: Q, key: K, joins: &str) -> Vec<(ROf<K>, Agg)>
where
    Q: IntoQuery + Copy,
    Q::Q: Drive<D = Id<Post>, R = Id<Post>>,
    K: IntoQuery + Copy,
    K::Q: Probe<D = Id<Post>>,
    ROf<K>: Eq + std::hash::Hash + Copy,
{
    let j = Joins::parse(joins);

    // The joined rows. Each child is the relation post -> child row, kept only
    // when the query joins that table, and `.opt()` keeps the post either way.
    let cs = comments_of_if(db, j.c).opt();
    let vs = votes_of_if(db, j.v).select(&db.vote.vote_type_id).opt();
    let as_ = if j.typed { answers_of_if(db, true) } else { children_of_if(db, j.a) }.opt();
    let bs = (&db.post.owner_user).select(badges_of_if(db, j.b)).opt();
    let hs = history_of_if(db, j.h).select(&db.post_history.creation_date).opt();

    let rows = posts
        .group_by(key)
        .select(cs.and(vs).and(as_).and(bs).and(hs))
        .fold(
            (0i64, 0i64, 0i64, 0i64, 0i64, 0i64, 0i64, 0i64, i64::MIN),
            |(n, c, v, a, b, h, up, down, hmax), ((((c1, v1), a1), b1), h1)| {
                (
                    n + 1,
                    c + c1.is_some() as i64,
                    v + v1.is_some() as i64,
                    a + a1.is_some() as i64,
                    b + b1.is_some() as i64,
                    h + h1.is_some() as i64,
                    up + (v1 == Some(2)) as i64,
                    down + (v1 == Some(3)) as i64,
                    match h1 {
                        Some(d) => hmax.max(d),
                        None => hmax,
                    },
                )
            },
        );

    // One row per post: COUNT(DISTINCT p.Id) and each child's own count.
    let cc = comments_per_post(db);
    let vv = votes_per_post(db);
    let aa = if j.typed { typed_answers_per_post(db) } else { answers_per_post(db) };
    let bb = badges_per_user(db);
    let hh = history_per_post(db);
    let distinct = posts
        .group_by(key)
        .select(
            (&cc)
                .and(&vv)
                .and(&aa)
                .and((&db.post.owner_user).select(&bb).opt())
                .and(&hh),
        )
        .fold(
            (0i64, 0i64, 0i64, 0i64, 0i64, 0i64),
            move |(n, c, v, a, b, h), ((((nc, nv), na), _nb), nh)| {
                (
                    n + 1,
                    c + if j.c { nc } else { 0 },
                    v + if j.v { nv } else { 0 },
                    a + if j.a { na } else { 0 },
                    b,
                    h + if j.h { nh } else { 0 },
                )
            },
        );

    let bdist = posts.group_by(key).select((&db.post.owner_user).select(badges_of_if(db, j.b))).count_distinct();
    let mut out = Vec::new();
    rows.and(&distinct).and((&bdist).opt()).drive(
        |k, (((rows, cx, vx, ax, bx, hx, up, down, hmax), (n, c, v, a, _, h)), b)| {
            let b = b.unwrap_or(0);
            out.push((
                k,
                Agg { n, rows, c, v, a, b, h, cx, vx, ax, bx, hx, up, down, hmax },
            ))
        },
    );
    out
}

/// One row per post, for the queries whose GROUP BY names p.Id — there the
/// group is a single post and no grouping is needed at all.
pub fn post_counts<Q>(db: &'static So, posts: Q, joins: &str) -> Vec<(Id<Post>, Agg)>
where
    Q: IntoQuery,
    Q::Q: Drive<D = Id<Post>, R = Id<Post>>,
{
    let base = posts.iq();
    let key = Ident::<Post>::new();
    group_posts(db, &base, &key, joins)
}

/// A post grouped by a value tuple, flattened so one projector and one sort
/// serve every key. `score` and `views` are only meaningful when the key holds
/// them, and SQL only lets a query print a column its own GROUP BY named.
#[derive(Clone, Copy)]
pub struct TupleGroup {
    pub owner: Option<Str>,
    pub title: Option<Str>,
    pub created: i64,
    pub score: i64,
    pub views: Option<i64>,
    pub ptype: Option<Str>,
    pub body: Option<Str>,
    pub rep: Option<i64>,
    pub owner_id: Option<i64>,
    pub agg: Agg,
}

pub fn tuple_fields(g: &TupleGroup, cols: &[&str]) -> Vec<V> {
    cols.iter()
        .map(|c| match *c {
            "owner" => crate::fmt::ostr(g.owner),
            "title" => crate::fmt::ostr(g.title),
            "created" => V::T(g.created),
            "score" => V::I(g.score),
            "views" => crate::fmt::oint(g.views),
            "ptype" => crate::fmt::ostr(g.ptype),
            "body" => crate::fmt::ostr(g.body),
            "#n" => V::I(g.agg.n),
            "#rows" => V::I(g.agg.rows),
            "#c" => V::I(g.agg.c),
            "#v" => V::I(g.agg.v),
            "#a" => V::I(g.agg.a),
            "#b" => V::I(g.agg.b),
            "#h" => V::I(g.agg.h),
            "#cx" => V::I(g.agg.cx),
            "#vx" => V::I(g.agg.vx),
            "#ax" => V::I(g.agg.ax),
            "#bx" => V::I(g.agg.bx),
            "#hx" => V::I(g.agg.hx),
            "#up" => V::I(g.agg.up),
            "#down" => V::I(g.agg.down),
            "#hmax" => if g.agg.h == 0 { V::Null } else { V::T(g.agg.hmax) },
            "rep" => crate::fmt::oint(g.rep),
            "owner_id" => crate::fmt::oint(g.owner_id),
            _ => panic!("tuple_fields: unknown column {c}"),
        })
        .collect()
}

/// `ORDER BY x DESC`, which in DuckDB puts the NULLs last.
pub fn desc_nulls_last(a: Option<i64>, b: Option<i64>) -> std::cmp::Ordering {
    match (a, b) {
        (Some(x), Some(y)) => y.cmp(&x),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}

pub fn sort_tuples(v: &mut [TupleGroup], by: &str) {
    match by {
        "created" => v.sort_by(|a, b| b.created.cmp(&a.created)),
        "score" => v.sort_by(|a, b| b.score.cmp(&a.score)),
        "score,created" => v.sort_by(|a, b| (b.score, b.created).cmp(&(a.score, a.created))),
        "views" => v.sort_by(|a, b| desc_nulls_last(a.views, b.views)),
        "score,views" => v.sort_by(|a, b| {
            b.score.cmp(&a.score).then_with(|| desc_nulls_last(a.views, b.views))
        }),
        "#c" => v.sort_by(|a, b| b.agg.c.cmp(&a.agg.c)),
        "#cx" => v.sort_by(|a, b| b.agg.cx.cmp(&a.agg.cx)),
        "#v" => v.sort_by(|a, b| b.agg.v.cmp(&a.agg.v)),
        "#vx" => v.sort_by(|a, b| b.agg.vx.cmp(&a.agg.vx)),
        "#up" => v.sort_by(|a, b| b.agg.up.cmp(&a.agg.up)),
        "#n" => v.sort_by(|a, b| b.agg.n.cmp(&a.agg.n)),
        "#rows" => v.sort_by(|a, b| b.agg.rows.cmp(&a.agg.rows)),
        "#vx,#cx" => v.sort_by(|a, b| (b.agg.vx, b.agg.cx).cmp(&(a.agg.vx, a.agg.cx))),
        "#up,#cx" => v.sort_by(|a, b| (b.agg.up, b.agg.cx).cmp(&(a.agg.up, a.agg.cx))),
        "#cx,#up" => v.sort_by(|a, b| (b.agg.cx, b.agg.up).cmp(&(a.agg.cx, a.agg.up))),
        "#vx,created" => v.sort_by(|a, b| (b.agg.vx, b.created).cmp(&(a.agg.vx, a.created))),
        "rep,created" => v.sort_by(|a, b| (b.rep, b.created).cmp(&(a.rep, a.created))),
        "rep" => v.sort_by(|a, b| b.rep.cmp(&a.rep)),
        _ => panic!("sort_tuples: unknown key {by}"),
    }
}

/// Sort, cut at `n` (0 meaning no LIMIT) and print `cols`.
pub fn tuple_rows(mut v: Vec<TupleGroup>, by: &str, n: usize, cols: &[&str]) -> String {
    sort_tuples(&mut v, by);
    let n = if n == 0 { v.len() } else { n };
    crate::fmt::rows(v.iter().take(n).map(|g| crate::fmt::row(tuple_fields(g, cols))))
}

fn tup(owner: Str, title: Option<Str>, created: i64, score: i64, views: Option<i64>, agg: Agg) -> TupleGroup {
    TupleGroup {
        owner: Some(owner),
        title,
        created,
        score,
        views,
        ptype: None,
        body: None,
        rep: None,
        owner_id: None,
        agg,
    }
}

pub fn questions_only(
    db: &'static So,
) -> Restrict<Key<Post>, Filter<&'static Col<Post, i64>, impl Fn(i64) -> bool>> {
    db.post.with((&db.post.post_type_id).eq(1))
}

/// The value tuples this corpus groups posts by. They are all
/// (u.DisplayName, p.Title, p.CreationDate) plus some of Score, ViewCount,
/// Body, pt.Name and u.Reputation; GROUP BY is order-insensitive, so the many
/// orderings of those columns are these same few keys.
pub fn by_name_title_date(db: &'static So, only_q: bool, joins: &str, w: PostWhere) -> Vec<TupleGroup> {
    let Post { title, creation_date, owner_user, .. } = &db.post;
    let key = owner_user.select(&db.user.display_name).and(title.opt()).and(creation_date);
    let g = group_posts(db, &post_base(db, only_q, true, w), &key, joins);
    g.into_iter().map(|(((o, t), cd), a)| tup(o, t, cd, 0, None, a)).collect()
}

/// `LEFT JOIN Users`: the owner may be missing, and those posts group under a
/// NULL DisplayName rather than dropping out.
pub fn by_name_title_date_outer(db: &'static So, only_q: bool, joins: &str, w: PostWhere) -> Vec<TupleGroup> {
    let Post { title, creation_date, owner_user, .. } = &db.post;
    let key = owner_user.select(&db.user.display_name).opt().and(title.opt()).and(creation_date);
    let g = group_posts(db, &post_base(db, only_q, true, w), &key, joins);
    g.into_iter()
        .map(|(((o, t), cd), a)| TupleGroup { owner: o, ..tup("", t, cd, 0, None, a) })
        .collect()
}

pub fn by_name_title_date_score(db: &'static So, only_q: bool, joins: &str, w: PostWhere) -> Vec<TupleGroup> {
    let Post { title, creation_date, score, owner_user, .. } = &db.post;
    let key = owner_user.select(&db.user.display_name).and(title.opt()).and(creation_date).and(score);
    let g = group_posts(db, &post_base(db, only_q, true, w), &key, joins);
    g.into_iter().map(|((((o, t), cd), s), a)| tup(o, t, cd, s, None, a)).collect()
}

pub fn by_name_title_date_score_outer(db: &'static So, only_q: bool, joins: &str, w: PostWhere) -> Vec<TupleGroup> {
    let Post { title, creation_date, score, owner_user, .. } = &db.post;
    let key = owner_user
        .select(&db.user.display_name)
        .opt()
        .and(title.opt())
        .and(creation_date)
        .and(score);
    let g = group_posts(db, &post_base(db, only_q, true, w), &key, joins);
    g.into_iter()
        .map(|((((o, t), cd), s), a)| TupleGroup { owner: o, ..tup("", t, cd, s, None, a) })
        .collect()
}

pub fn by_name_title_date_views(db: &'static So, only_q: bool, joins: &str, w: PostWhere) -> Vec<TupleGroup> {
    let Post { title, creation_date, view_count, owner_user, .. } = &db.post;
    let key = owner_user
        .select(&db.user.display_name)
        .and(title.opt())
        .and(creation_date)
        .and(view_count.opt());
    let g = group_posts(db, &post_base(db, only_q, true, w), &key, joins);
    g.into_iter().map(|((((o, t), cd), w), a)| tup(o, t, cd, 0, w, a)).collect()
}

pub fn by_name_title_date_score_views(db: &'static So, only_q: bool, joins: &str, w: PostWhere) -> Vec<TupleGroup> {
    let Post { title, creation_date, score, view_count, owner_user, .. } = &db.post;
    let key = owner_user
        .select(&db.user.display_name)
        .and(title.opt())
        .and(creation_date)
        .and(score)
        .and(view_count.opt());
    let g = group_posts(db, &post_base(db, only_q, true, w), &key, joins);
    g.into_iter().map(|(((((o, t), cd), s), w), a)| tup(o, t, cd, s, w, a)).collect()
}

pub fn by_name_title_date_body(db: &'static So, only_q: bool, joins: &str, w: PostWhere) -> Vec<TupleGroup> {
    let Post { title, creation_date, body, owner_user, .. } = &db.post;
    let key = owner_user.select(&db.user.display_name).and(title.opt()).and(creation_date).and(body);
    let g = group_posts(db, &post_base(db, only_q, true, w), &key, joins);
    g.into_iter()
        .map(|((((o, t), cd), b), a)| TupleGroup { body: Some(b), ..tup(o, t, cd, 0, None, a) })
        .collect()
}

pub fn by_name_title_date_views_body(db: &'static So, only_q: bool, joins: &str, w: PostWhere) -> Vec<TupleGroup> {
    let Post { title, creation_date, view_count, body, owner_user, .. } = &db.post;
    let key = owner_user
        .select(&db.user.display_name)
        .and(title.opt())
        .and(creation_date)
        .and(view_count.opt())
        .and(body);
    let g = group_posts(db, &post_base(db, only_q, true, w), &key, joins);
    g.into_iter()
        .map(|(((((o, t), cd), w), b), a)| TupleGroup { body: Some(b), ..tup(o, t, cd, 0, w, a) })
        .collect()
}

pub fn by_name_title_date_ptype(db: &'static So, only_q: bool, joins: &str, w: PostWhere) -> Vec<TupleGroup> {
    let Post { title, creation_date, post_type, owner_user, .. } = &db.post;
    let key = owner_user
        .select(&db.user.display_name)
        .and(title.opt())
        .and(creation_date)
        .and(post_type.select(&db.post_type.name));
    let g = group_posts(db, &post_base(db, only_q, true, w), &key, joins);
    g.into_iter()
        .map(|((((o, t), cd), pt), a)| TupleGroup { ptype: Some(pt), ..tup(o, t, cd, 0, None, a) })
        .collect()
}

pub fn by_name_title_date_score_views_ptype(db: &'static So, only_q: bool, joins: &str, w: PostWhere) -> Vec<TupleGroup> {
    let Post { title, creation_date, score, view_count, post_type, owner_user, .. } = &db.post;
    let key = owner_user
        .select(&db.user.display_name)
        .and(title.opt())
        .and(creation_date)
        .and(score)
        .and(view_count.opt())
        .and(post_type.select(&db.post_type.name));
    let g = group_posts(db, &post_base(db, only_q, true, w), &key, joins);
    g.into_iter()
        .map(|((((((o, t), cd), s), w), pt), a)| {
            TupleGroup { ptype: Some(pt), ..tup(o, t, cd, s, w, a) }
        })
        .collect()
}

/// Keyed on u.Id as well as the display name, which separates the users who
/// share one.
pub fn by_owner_name_title_date_score_views(db: &'static So, only_q: bool, joins: &str, w: PostWhere) -> Vec<TupleGroup> {
    let Post { title, creation_date, score, view_count, owner_user, .. } = &db.post;
    let key = owner_user
        .select(&db.user.origid)
        .and(owner_user.select(&db.user.display_name))
        .and(title.opt())
        .and(creation_date)
        .and(score)
        .and(view_count.opt());
    let g = group_posts(db, &post_base(db, only_q, true, w), &key, joins);
    g.into_iter()
        .map(|((((((uid, o), t), cd), s), w), a)| {
            TupleGroup { owner_id: Some(uid), ..tup(o, t, cd, s, w, a) }
        })
        .collect()
}

/// (DisplayName, Title, ViewCount) — the one key here with no CreationDate.
pub fn by_name_title_views(db: &'static So, only_q: bool, joins: &str, w: PostWhere) -> Vec<TupleGroup> {
    let Post { title, view_count, owner_user, .. } = &db.post;
    let key = owner_user.select(&db.user.display_name).and(title.opt()).and(view_count.opt());
    let g = group_posts(db, &post_base(db, only_q, true, w), &key, joins);
    g.into_iter().map(|(((o, t), w), a)| tup(o, t, 0, 0, w, a)).collect()
}

/// (DisplayName, Title) alone, with no CreationDate to separate reposts.
pub fn by_name_title(db: &'static So, only_q: bool, joins: &str, w: PostWhere) -> Vec<TupleGroup> {
    let Post { title, owner_user, .. } = &db.post;
    let key = owner_user.select(&db.user.display_name).and(title.opt());
    let g = group_posts(db, &post_base(db, only_q, true, w), &key, joins);
    g.into_iter().map(|((o, t), a)| tup(o, t, 0, 0, None, a)).collect()
}

pub fn by_name_rep_title_date(db: &'static So, only_q: bool, joins: &str, w: PostWhere) -> Vec<TupleGroup> {
    let Post { title, creation_date, owner_user, .. } = &db.post;
    let key = owner_user
        .select(&db.user.display_name)
        .and(owner_user.select(&db.user.reputation))
        .and(title.opt())
        .and(creation_date);
    let g = group_posts(db, &post_base(db, only_q, true, w), &key, joins);
    g.into_iter()
        .map(|((((o, r), t), cd), a)| TupleGroup { rep: Some(r), ..tup(o, t, cd, 0, None, a) })
        .collect()
}

pub fn by_name_rep_title_date_score(db: &'static So, only_q: bool, joins: &str, w: PostWhere) -> Vec<TupleGroup> {
    let Post { title, creation_date, score, owner_user, .. } = &db.post;
    let key = owner_user
        .select(&db.user.display_name)
        .and(owner_user.select(&db.user.reputation))
        .and(title.opt())
        .and(creation_date)
        .and(score);
    let g = group_posts(db, &post_base(db, only_q, true, w), &key, joins);
    g.into_iter()
        .map(|(((((o, r), t), cd), s), a)| TupleGroup { rep: Some(r), ..tup(o, t, cd, s, None, a) })
        .collect()
}

/// A post column or one of the counts, for the queries whose GROUP BY names
/// p.Id — there the group is one post and no grouping is needed at all.
pub fn count_fields(db: &'static So, p: Id<Post>, a: Agg, cols: &[&str]) -> Vec<V> {
    cols.iter()
        .map(|c| match *c {
            // AVG over a group that is one post is that post's own value
            "post_score_avg" => V::F(db.post.score.get(p).unwrap() as f64),
            "post_rep_avg" => crate::fmt::ofloat(
                db.post.owner_user.get(p).map(|u| db.user.reputation.get(u).unwrap() as f64),
            ),
            "#n" => V::I(a.n),
            "#rows" => V::I(a.rows),
            "#c" => V::I(a.c),
            "#v" => V::I(a.v),
            "#a" => V::I(a.a),
            "#b" => V::I(a.b),
            "#h" => V::I(a.h),
            "#cx" => V::I(a.cx),
            "#vx" => V::I(a.vx),
            "#ax" => V::I(a.ax),
            "#bx" => V::I(a.bx),
            "#hx" => V::I(a.hx),
            "#up" => V::I(a.up),
            "#down" => V::I(a.down),
            "#hmax" => if a.h == 0 { V::Null } else { V::T(a.hmax) },
            _ => post_fields(db, p, &[c]).pop().unwrap(),
        })
        .collect()
}

fn sort_key(db: &'static So, p: Id<Post>, by: &str) -> Option<i64> {
    match by {
        "created" => Some(db.post.creation_date.get(p).unwrap()),
        "score" => Some(db.post.score.get(p).unwrap()),
        "views" => db.post.view_count.get(p),
        "activity" => Some(db.post.last_activity_date.get(p).unwrap()),
        "rep" => db.post.owner_user.get(p).map(|u| db.user.reputation.get(u).unwrap()),
        _ => panic!("sort_posts: unknown key {by}"),
    }
}

pub fn sort_posts(db: &'static So, v: &mut [(Id<Post>, Agg)], by: &str) {
    // `ORDER BY a DESC[, b DESC]`, either of which may be a column or a count.
    let g = |w: &str, r: &(Id<Post>, Agg)| match w {
        "#c" => Some(r.1.c),
        "#v" => Some(r.1.v),
        "#cx" => Some(r.1.cx),
        "#vx" => Some(r.1.vx),
        "#up" => Some(r.1.up),
        "#rows" => Some(r.1.rows),
        _ => sort_key(db, r.0, w),
    };
    if let Some((x, y)) = by.split_once(',') {
        v.sort_by(|a, b| {
            desc_nulls_last(g(x, a), g(x, b)).then_with(|| desc_nulls_last(g(y, a), g(y, b)))
        });
    } else {
        v.sort_by(|a, b| desc_nulls_last(g(by, a), g(by, b)));
    }
}

/// A `WHERE` on a post, or on the post's owner. It belongs to the query, not
/// to the result: `post_base` builds it into the relation the group and the
/// join are taken over, so the excluded rows are never joined in the first
/// place.
#[derive(Clone, Copy)]
pub enum PostWhere {
    All,
    CreatedGe(i64),
    CreatedGt(i64),
    ViewsGt(i64),
    ScoreGt(i64),
}

impl PostWhere {
    fn holds(self, cd: i64, vw: Option<i64>, sc: i64) -> bool {
        match self {
            PostWhere::All => true,
            PostWhere::CreatedGe(c) => cd >= c,
            PostWhere::CreatedGt(c) => cd > c,
            PostWhere::ViewsGt(v) => vw.is_some_and(|w| w > v),
            PostWhere::ScoreGt(v) => sc > v,
        }
    }
}

/// The posts a query is over: `only_q` is `WHERE p.PostTypeId = 1`, `outer` is
/// whether Users is LEFT JOINed (which decides whether the ownerless posts
/// keep a row), and `w` is the rest of the WHERE. All three are one
/// restriction on `db.post`.
pub fn post_base(
    db: &'static So,
    only_q: bool,
    outer: bool,
    w: PostWhere,
) -> impl Drive<D = Id<Post>, R = Id<Post>> {
    let Post { post_type_id, owner_user, creation_date, view_count, score, .. } = &db.post;
    db.post.with(
        post_type_id
            .and(owner_user.opt())
            .and(creation_date)
            .and(view_count.opt())
            .and(score)
            .filt(move |((((ti, ou), cd), vw), sc)| {
                (!only_q || ti == 1) && (outer || ou.is_some()) && w.holds(cd, vw, sc)
            }),
    )
}

/// One row per post with its child counts.
pub fn posts_with_counts(
    db: &'static So,
    only_q: bool,
    outer: bool,
    joins: &str,
) -> Vec<(Id<Post>, Agg)> {
    posts_where(db, only_q, outer, joins, PostWhere::All)
}

/// The same under a `WHERE`.
pub fn posts_where(
    db: &'static So,
    only_q: bool,
    outer: bool,
    joins: &str,
    w: PostWhere,
) -> Vec<(Id<Post>, Agg)> {
    post_counts(db, post_base(db, only_q, outer, w), joins)
}

/// Sort, cut at `n` (0 meaning no LIMIT) and print. A WHERE on a grouped
/// column can be applied to the Vec first, because every post in a group
/// agrees on it.
pub fn render_posts(
    db: &'static So,
    mut v: Vec<(Id<Post>, Agg)>,
    by: &str,
    n: usize,
    cols: &[&str],
) -> String {
    sort_posts(db, &mut v, by);
    let n = if n == 0 { v.len() } else { n };
    crate::fmt::rows(v.iter().take(n).map(|&(p, a)| crate::fmt::row(count_fields(db, p, a, cols))))
}

pub fn post_rows(db: &'static So, only_q: bool, joins: &str, by: &str, n: usize, cols: &[&str]) -> String {
    render_posts(db, posts_with_counts(db, only_q, false, joins), by, n, cols)
}

/// The same where SQL writes `LEFT JOIN Users`, so the ownerless posts stay.
pub fn post_rows_outer(db: &'static So, only_q: bool, joins: &str, by: &str, n: usize, cols: &[&str]) -> String {
    render_posts(db, posts_with_counts(db, only_q, true, joins), by, n, cols)
}


pub fn post_fields(db: &'static So, p: Id<Post>, cols: &[&str]) -> Vec<V> {
    let Post {
        origid,
        title,
        creation_date,
        score,
        view_count,
        answer_count,
        comment_count,
        favorite_count,
        owner_user,
        owner_user_id,
        post_type,
        post_type_id,
        body,
        tags_str,
        last_activity_date,
        accepted_answer_id,
        parent_id,
        closed_date,
        last_edit_date,
        ..
    } = &db.post;
    let u = |p: Id<Post>| owner_user.get(p);
    cols.iter()
        .map(|c| match *c {
            "id" => V::I(origid.get(p).unwrap()),
            "title" => crate::fmt::ostr(title.get(p)),
            "body" => V::S(body.get(p).unwrap()),
            "tags" => crate::fmt::ostr(tags_str.get(p)),
            "created" => V::T(creation_date.get(p).unwrap()),
            "activity" => V::T(last_activity_date.get(p).unwrap()),
            "edited" => crate::fmt::ots(last_edit_date.get(p)),
            "closed" => crate::fmt::ots(closed_date.get(p)),
            "score" => V::I(score.get(p).unwrap()),
            "views" => crate::fmt::oint(view_count.get(p)),
            "answers" => crate::fmt::oint(answer_count.get(p)),
            "comments" => V::I(comment_count.get(p).unwrap()),
            "favorites" => crate::fmt::oint(favorite_count.get(p)),
            "accepted" => crate::fmt::oint(accepted_answer_id.get(p)),
            "parent" => crate::fmt::oint(parent_id.get(p)),
            "owner_id" => crate::fmt::oint(owner_user_id.get(p)),
            "owner" => crate::fmt::ostr(u(p).map(|u| db.user.display_name.get(u).unwrap())),
            "rep" => crate::fmt::oint(u(p).map(|u| db.user.reputation.get(u).unwrap())),
            "uid" => crate::fmt::oint(u(p).map(|u| db.user.origid.get(u).unwrap())),
            "ucreated" => crate::fmt::ots(u(p).map(|u| db.user.creation_date.get(u).unwrap())),
            "last_access" => crate::fmt::ots(u(p).map(|u| db.user.last_access_date.get(u).unwrap())),
            "type" => V::S(db.post_type.name.get(post_type.get(p).unwrap()).unwrap()),
            "type_id" => V::I(post_type_id.get(p).unwrap()),
            "owner_name" => crate::fmt::ostr(db.post.owner_display_name.get(p)),
            "editor_id" => crate::fmt::oint(db.post.last_editor_user_id.get(p)),
            "editor_name" => crate::fmt::ostr(db.post.last_editor_display_name.get(p)),
            "community" => crate::fmt::ots(db.post.community_owned_date.get(p)),
            "license" => V::S(db.post.content_license.get(p).unwrap()),
            _ => panic!("post_fields: unknown column {c}"),
        })
        .collect()
}

pub fn omax(v: i64, n: i64) -> V {
    if n == 0 { V::Null } else { V::I(v) }
}

pub fn type_fields(a: &TypeAgg, cols: &[&str]) -> Vec<V> {
    cols.iter()
        .map(|c| match *c {
            "name" => V::S(a.name),
            "type_id" => V::I(a.type_id),
            "n" => V::I(a.n),
            "score_sum" => nullable(a.score_sum, a.n),
            "score_avg" => avg(a.score_sum, a.n),
            "score_max" => omax(a.score_max, a.n),
            "score_min" => omax(a.score_min, a.n),
            "views_sum" => nullable(a.views_sum, a.views_n),
            "views_avg" => avg(a.views_sum, a.views_n),
            "views_max" => omax(a.views_max, a.views_n),
            "views_min" => omax(a.views_min, a.views_n),
            "answers_sum" => nullable(a.answers_sum, a.answers_n),
            "answers_avg" => avg(a.answers_sum, a.answers_n),
            "rep_sum" => nullable(a.rep_sum, a.rep_n),
            "rep_avg" => avg(a.rep_sum, a.rep_n),
            "comment_sum" => nullable(a.comment_sum, a.n),
            "comment_avg" => avg(a.comment_sum, a.n),
            "owner_n" => V::I(a.owner_n),
            "accepted_n" => V::I(a.accepted_n),
            "closed_n" => V::I(a.closed_n),
            "active_avg" => V::F(a.active_us as f64 / 1e6 / a.n as f64),
            "created_max" => V::T(a.created_max),
            "created_min" => V::T(a.created_min),
            "activity_max" => V::T(a.last_activity_max),
            _ => panic!("type_fields: unknown column {c}"),
        })
        .collect()
}

pub fn desc<T: Ord>(a: &T, b: &T) -> std::cmp::Ordering {
    b.cmp(a)
}

pub fn asc<T: Ord>(a: &T, b: &T) -> std::cmp::Ordering {
    a.cmp(b)
}

pub fn whole<D: Copy + Eq + std::hash::Hash, Q: Drive<R = D>>(q: Q) -> InvStream<Map<Q, impl Fn(D), ()>> {
    q.map(|_| ()).inv()
}

pub fn votes_of_type(db: &'static So, t: i64) -> DenseFold<Id<Post>, i64> {
    db.vote.with((&db.vote.vote_type_id).eq(t)).select(&db.vote.post).inv().dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1)
}

/// `CURRENT_DATE` in the oracle's session zone, America/New_York.
pub fn current_date() -> i64 {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_micros() as i64;
    crate::time::trunc_day(crate::time::utc_to_ny(now))
}

pub fn tag_list(t: Str) -> std::str::Split<'static, &'static str> {
    t[1..t.len() - 1].split("><")
}

pub fn tag_mentions(db: &'static So) -> MatSet<(Id<Post>, Id<Tag>)> {
    let elems: MatSet<Str> = (&db.post.tags_str).flat_map(tag_list).collect();
    let contains: HashIdx<Str, Id<Tag>> =
        (&elems).select_where((&db.tag.tag_name).inv(), |e: Str, n: Str| e.contains(n)).collect();
    db.post.select(Ident::<Post>::new().and((&db.post.tags_str).flat_map(tag_list).select(&contains))).collect()
}

pub fn min_some(a: Option<i64>, b: Option<i64>) -> Option<i64> {
    match (a, b) {
        (Some(x), Some(y)) => Some(x.min(y)),
        (x, None) | (None, x) => x,
    }
}

// --- users grouped, aggregating their posts --------------------------------

/// What SQL computes per user in
///
///   FROM Users u [LEFT] JOIN Posts p ON u.Id = p.OwnerUserId
///        [LEFT JOIN Comments c ON p.Id = c.PostId]
///        [LEFT JOIN Votes v ON p.Id = v.PostId]
///        [LEFT JOIN Badges b ON u.Id = b.UserId]
///
/// The post side and the badge side are independent, so the FROM is their
/// product. These fields are the post side only; `UserRow` multiplies by the
/// badge fan-out, which is what makes `AVG(p.Score)` need no correction at all
/// — the badge factor cancels between its sum and its count.
#[derive(Clone, Copy, Default)]
pub struct UserAgg {
    /// COUNT(DISTINCT p.Id).
    pub n: i64,
    /// COUNT(*): the joined rows. `prows` is COUNT(p.Id), which drops the one
    /// row a user with no posts keeps.
    pub rows: i64,
    pub prows: i64,
    /// COUNT(DISTINCT c.Id) / COUNT(c.Id), and the same for Votes.
    pub c: i64,
    pub v: i64,
    pub cx: i64,
    pub vx: i64,
    /// COUNT(b.Id), COUNT(cu.Id), COUNT(vu.Id) — the children joined on u.Id.
    pub bx: i64,
    pub cux: i64,
    pub vux: i64,
    /// SUM(vu.BountyAmount): the bounty on the votes the user *cast*, which is
    /// a different join from the bounty on the votes their posts received.
    pub ubounty_sum: i64,
    pub ubounty_n: i64,
    /// SUM(CASE WHEN v.VoteTypeId = 5 THEN 1 ELSE 0 END)
    pub fav: i64,
    /// SUM(u.UpVotes), SUM(u.DownVotes): the user's own counts, once per joined row.
    pub uup_sum: i64,
    pub udown_sum: i64,
    pub up: i64,
    pub down: i64,
    pub score_sum: i64,
    pub views_sum: i64,
    pub views_n: i64,
    pub questions: i64,
    pub answers: i64,
    /// SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END)
    pub accepted: i64,
    pub bounty_sum: i64,
    pub bounty_n: i64,
    pub created_max: i64,
    pub created_min: i64,
    pub activity_max: i64,
    pub score_max: i64,
}

pub struct UserRow {
    /// Comments the user wrote and votes they cast — the children joined on
    /// u.Id rather than p.Id, which multiply like Badges do.
    pub ncu: i64,
    pub nvu: i64,
    pub uid: i64,
    pub name: Str,
    pub rep: i64,
    pub ucreated: i64,
    pub last_access: i64,
    pub uviews: i64,
    pub uup: i64,
    pub udown: i64,
    pub nb: i64,
    pub agg: UserAgg,
}

/// One row per user with the post-side aggregates. `joins` names the children
/// SQL joined; `inner` is whether Posts was an inner join, which drops the
/// users with no posts.
/// A `WHERE` on the user. Like `PostWhere`, it is built into the relation the group
/// is taken over rather than applied to the result.
#[derive(Clone, Copy)]
pub enum UserWhere {
    All,
    RepGt(i64),
    CreatedGe(i64),
    CreatedLt(i64),
}

impl UserWhere {
    fn holds(self, rep: i64, created: i64) -> bool {
        match self {
            UserWhere::All => true,
            UserWhere::RepGt(r) => rep > r,
            UserWhere::CreatedGe(c) => created >= c,
            UserWhere::CreatedLt(c) => created < c,
        }
    }
}

pub fn user_base(db: &'static So, w: UserWhere) -> impl Drive<D = Id<User>, R = Id<User>> {
    let User { reputation, creation_date, .. } = &db.user;
    db.user.with(reputation.and(creation_date).filt(move |(r, c)| w.holds(r, c)))
}

pub fn users_with_counts(db: &'static So, joins: &str, inner: bool) -> Vec<UserRow> {
    users_where(db, joins, inner, UserWhere::All)
}

/// One row per user with the aggregates over their posts. `joins` names the
/// children: lowercase hangs off `p.Id`, uppercase off `u.Id`.
///
/// As in `group_posts`, the children are joined rather than counted. The post
/// side is `posts_of(db).select(<the post and its children>)`, which is
/// `LEFT JOIN Posts ... LEFT JOIN Comments ... LEFT JOIN Votes`, and the
/// `u.Id` children are `.and`ed beside it, so one user's row count is the
/// product SQL asks for. That product is the whole cost of these queries —
/// see notes/limitations.md, "No optimiser".
pub fn users_where(db: &'static So, joins: &str, inner: bool, w: UserWhere) -> Vec<UserRow> {
    let j = Joins::parse(&post_joins(joins));
    let (jb, jcu, jvu) =
        (joins.contains('b'), joins.contains('C'), joins.contains('V'));
    let Post {
        score,
        view_count,
        post_type_id,
        creation_date,
        last_activity_date,
        accepted_answer_id,
        ..
    } = &db.post;

    // One post and the children hanging off p.Id.
    let post_row = score
        .and(view_count.opt())
        .and(post_type_id)
        .and(creation_date)
        .and(last_activity_date)
        .and(accepted_answer_id.opt())
        .and(comments_of_if(db, j.c).opt())
        .and(
            votes_of_if(db, j.v)
                .select((&db.vote.vote_type_id).and((&db.vote.bounty_amount).opt()))
                .opt(),
        );
    let posts_side = posts_of(db).select(post_row).opt();
    let badges = badges_of_if(db, jb).opt();
    let ucs = comments_by_if(db, jcu).opt();
    let uvs = votes_by_if(db, jvu).select((&db.vote.bounty_amount).opt()).opt();

    let init = UserAgg {
        created_max: i64::MIN,
        created_min: i64::MAX,
        activity_max: i64::MIN,
        score_max: i64::MIN,
        ..UserAgg::default()
    };
    let agg = user_base(db, w)
        .group_by(Ident::<User>::new())
        .select((&db.user.up_votes).and(&db.user.down_votes).and(posts_side).and(badges).and(ucs).and(uvs))
        .fold(init, |a, (((((uv, dv), p), b), cu), vu)| {
            let mut a = UserAgg {
                rows: a.rows + 1,
                uup_sum: a.uup_sum + uv,
                udown_sum: a.udown_sum + dv,
                bx: a.bx + b.is_some() as i64,
                cux: a.cux + cu.is_some() as i64,
                vux: a.vux + vu.is_some() as i64,
                ubounty_sum: a.ubounty_sum + vu.flatten().unwrap_or(0),
                ubounty_n: a.ubounty_n + vu.flatten().is_some() as i64,
                ..a
            };
            if let Some((((((((sc, vw), ti), cd), la), ay), c1), v1)) = p {
                a.prows += 1;
                a.cx += c1.is_some() as i64;
                a.vx += v1.is_some() as i64;
                if let Some((vt, bounty)) = v1 {
                    a.up += (vt == 2) as i64;
                    a.down += (vt == 3) as i64;
                    a.fav += (vt == 5) as i64;
                    if let Some(x) = bounty {
                        a.bounty_sum += x;
                        a.bounty_n += 1;
                    }
                }
                a.score_sum += sc;
                a.views_sum += vw.unwrap_or(0);
                a.views_n += vw.is_some() as i64;
                a.questions += (ti == 1) as i64;
                a.answers += (ti == 2) as i64;
                a.accepted += ay.is_some() as i64;
                a.created_max = a.created_max.max(cd);
                a.created_min = a.created_min.min(cd);
                a.activity_max = a.activity_max.max(la);
                a.score_max = a.score_max.max(sc);
            }
            a
        });

    // COUNT(DISTINCT p.Id) and each post-side child's own count: one row per
    // post, which is the count SQL's DISTINCT undoes the fan-out to reach.
    let cc = comments_per_post(db);
    let vv = votes_per_post(db);
    let dis = (&db.post.owner_user)
        .inv()
        .select((&cc).and(&vv))
        .dense_fold_outer(db.user.id.n, (0i64, 0i64, 0i64), move |(n, c, v), (nc, nv)| {
            (n + 1, c + if j.c { nc } else { 0 }, v + if j.v { nv } else { 0 })
        });

    let bb = badges_per_user(db);
    let cu = comments_per_user(db);
    let vu = votes_per_user(db);
    let User { origid, display_name, reputation, creation_date, last_access_date, views, up_votes, down_votes, .. } =
        &db.user;
    let mut out = Vec::new();
    user_base(db, w)
        .select(
            origid
                .and(display_name)
                .and(reputation)
                .and(creation_date)
                .and(last_access_date)
                .and(views)
                .and(up_votes)
                .and(down_votes)
                .and(&bb)
                .and(&cu)
                .and(&vu)
                .and(&agg)
                .and(&dis),
        )
        .filt(move |x| !inner || (x.1).0 > 0)
        .drive(
            |_,
             ((((((((((((uid, name), rep), ucreated), last_access), uviews), uup), udown), nb), ncu), nvu), agg), (n, c, v))| {
                let agg = UserAgg { n, c, v, ..agg };
                {
                    out.push(UserRow {
                        ncu,
                        nvu,
                        uid,
                        name,
                        rep,
                        ucreated,
                        last_access,
                        uviews,
                        uup,
                        udown,
                        nb,
                        agg,
                    })
                }
            },
        );
    out
}

/// The post-side children of a user query: the lowercase letters. The
/// uppercase ones hang off `u.Id` and are read straight from `joins`.
fn post_joins(joins: &str) -> String {
    joins.chars().filter(|c| c.is_ascii_lowercase()).collect()
}

pub fn user_fields(r: &UserRow, _joins: &str, cols: &[&str]) -> Vec<V> {
    let a = r.agg;
    let mp = a.rows.max(1);
    cols.iter()
        .map(|c| match *c {
            "uid" => V::I(r.uid),
            "name" => V::S(r.name),
            "rep" => V::I(r.rep),
            "rep_avg" => V::F(r.rep as f64),
            "ucreated" => V::T(r.ucreated),
            "last_access" => V::T(r.last_access),
            "uviews" => V::I(r.uviews),
            "uup" => V::I(r.uup),
            "udown" => V::I(r.udown),
            "#n" => V::I(a.n),
            "#rows" => V::I(a.prows),
            "#c" => V::I(a.c),
            "#v" => V::I(a.v),
            "#b" => V::I(r.nb),
            "#cu" => V::I(r.ncu),
            "#vu" => V::I(r.nvu),
            "#cux" => V::I(a.cux),
            "#vux" => V::I(a.vux),
            "#cx" => V::I(a.cx),
            "#vx" => V::I(a.vx),
            "#bx" => V::I(a.bx),
            "#up" => V::I(a.up),
            "#down" => V::I(a.down),
            "#q" => V::I(a.questions),
            "#a" => V::I(a.answers),
            "#acc" => V::I(a.accepted),
            "#fav" => V::I(a.fav),
            "uup_sum" => V::I(a.uup_sum),
            "udown_sum" => V::I(a.udown_sum),
            "ubounty_sum" => nullable(a.ubounty_sum, a.ubounty_n),
            "score_sum" => nullable(a.score_sum, a.prows),
            "score_sum0" => V::I(a.score_sum),
            "score_avg" => avg(a.score_sum, a.prows),
            "score_avg0" => V::F(if a.prows == 0 { 0.0 } else { a.score_sum as f64 / a.prows as f64 }),
            "score_avg_all" => V::F(a.score_sum as f64 / mp as f64),
            "views_sum" => nullable(a.views_sum, a.views_n),
            "views_sum0" => V::I(a.views_sum),
            "views_avg" => avg(a.views_sum, a.views_n),
            "views_avg0" => V::F(if a.views_n == 0 { 0.0 } else { a.views_sum as f64 / a.views_n as f64 }),
            "bounty_sum" => nullable(a.bounty_sum, a.bounty_n),
            "bounty_avg" => avg(a.bounty_sum, a.bounty_n),
            "created_max" => if a.n == 0 { V::Null } else { V::T(a.created_max) },
            "created_min" => if a.n == 0 { V::Null } else { V::T(a.created_min) },
            "score_max" => if a.n == 0 { V::Null } else { V::I(a.score_max) },
            "views_avg_all" => V::F(a.views_sum as f64 / mp as f64),
            "activity_max" => if a.n == 0 { V::Null } else { V::T(a.activity_max) },
            _ => panic!("user_fields: unknown column {c}"),
        })
        .collect()
}

pub fn sort_users(v: &mut [UserRow], _joins: &str, by: &str) {
    let key = |r: &UserRow, w: &str| -> f64 {
        let a = r.agg;
        match w {
            "rep" => r.rep as f64,
            "uid" => r.uid as f64,
            "#n" => a.n as f64,
            "#rows" => a.prows as f64,
            "#c" => a.c as f64,
            "#cx" => a.cx as f64,
            "#v" => a.v as f64,
            "#vx" => a.vx as f64,
            "#b" => r.nb as f64,
            "#cu" => r.ncu as f64,
            "#vu" => r.nvu as f64,
            "#bx" => a.bx as f64,
            "#up" => a.up as f64,
            "#q" => a.questions as f64,
            "#a" => a.answers as f64,
            "#acc" => a.accepted as f64,
            "score_sum" => a.score_sum as f64,
            "score_avg" => {
                if a.prows == 0 { f64::NEG_INFINITY } else { a.score_sum as f64 / a.prows as f64 }
            }
            "views_sum" => a.views_sum as f64,
            "created_max" => a.created_max as f64,
            _ => panic!("sort_users: unknown key {w}"),
        }
    };
    let parts: Vec<&str> = by.split(',').collect();
    v.sort_by(|a, b| {
        let mut o = std::cmp::Ordering::Equal;
        for w in &parts {
            // a leading '+' is ASC; everything else in this corpus is DESC
            let (asc, w) = match w.strip_prefix('+') {
                Some(t) => (true, t),
                None => (false, *w),
            };
            let (x, y) = (key(a, w), key(b, w));
            o = o.then(if asc { x.total_cmp(&y) } else { y.total_cmp(&x) });
        }
        o
    });
}

/// Sort, cut at `n` (0 meaning no LIMIT) and print.
pub fn user_rows(db: &'static So, joins: &str, inner: bool, by: &str, n: usize, cols: &[&str]) -> String {
    render_users(db, users_with_counts(db, joins, inner), joins, by, n, cols)
}

/// Sort, cut at `n` (0 meaning no LIMIT) and print. A WHERE on u.Reputation is
/// on a grouped column, so it can be applied to the Vec first.
pub fn render_users(
    _db: &'static So,
    mut v: Vec<UserRow>,
    joins: &str,
    by: &str,
    n: usize,
    cols: &[&str],
) -> String {
    sort_users(&mut v, joins, by);
    let n = if n == 0 { v.len() } else { n };
    crate::fmt::rows(v.iter().take(n).map(|r| crate::fmt::row(user_fields(r, joins, cols))))
}

// --- posts with their children, more aggregates ----------------------------

/// What a GROUP BY over posts and their LEFT JOINed children computes, beyond
/// `Agg`: the bounty on the joined votes, the vote types, the latest vote and
/// history dates. Every field is taken over the joined rows, so it carries
/// the other children's fan-out exactly as SQL's does.
#[derive(Clone, Copy)]
pub struct Stats {
    pub rows: i64,
    pub cx: i64,
    pub vx: i64,
    pub ax: i64,
    pub bx: i64,
    pub hx: i64,
    pub up: i64,
    pub down: i64,
    pub bounty_n: i64,
    pub bounty_sum: i64,
    /// SUM(v.VoteTypeId), and SUM(vt.Id) through `JOIN VoteTypes vt`.
    pub vt_sum: i64,
    pub vtj_n: i64,
    pub vtj_sum: i64,
    pub vmax: i64,
    pub hmax: i64,
    pub hmin: i64,
    /// MAX(b.Date), MAX(b.Name), SUM(b.Class) over the owner's badges.
    pub bmax: i64,
    pub bname: Option<Str>,
    pub bclass: i64,
    /// History rows of type 10 and 11, and MAX(CASE WHEN type = 10 THEN ph.CreationDate END).
    pub h10: i64,
    pub h11: i64,
    pub h10max: i64,
    pub h11max: i64,
    /// MAX(c.CreationDate) and SUM(c.Score).
    pub cmax: i64,
    pub cscore: i64,
    /// Votes whose `LEFT JOIN VoteTypes` name is 'UpMod' / 'DownMod'.
    pub upn: i64,
    pub downn: i64,
    /// Votes whose joined vt.Id is 2 / 3, and history rows of type 4 or 5.
    pub upj: i64,
    pub downj: i64,
    pub h45: i64,
    /// Votes by VoteTypeId, for the types below 16.
    pub by_vt: [i64; 16],
    /// Votes named 'AcceptedByOriginator'; joined PostLinks rows.
    pub accn: i64,
    pub lx: i64,
    /// The owner's badges of Class 1, 2, 3 over the joined rows.
    pub bcls: [i64; 4],
}

/// Group `posts` by `key` and fold every joined row. `joins` is as for
/// `group_posts`; `vtypes`, when not empty, is the `AND v.VoteTypeId IN (...)`
/// on the Votes join.
pub fn group_stats<Q, K>(db: &'static So, posts: Q, key: K, joins: &str, vtypes: &'static [i64]) -> Vec<(ROf<K>, Stats)>
where
    Q: IntoQuery,
    Q::Q: Drive<D = Id<Post>, R = Id<Post>>,
    K: IntoQuery,
    K::Q: Probe<D = Id<Post>>,
    ROf<K>: Eq + std::hash::Hash + Copy,
{
    let mut out = Vec::new();
    stats_fold(db, posts, key, joins, vtypes).drive(|k, s| out.push((k, s)));
    out
}

/// The same fold, left as a relation so it can be joined to others on the key.
pub fn stats_fold<Q, K>(db: &'static So, posts: Q, key: K, joins: &str, vtypes: &'static [i64]) -> Fold<ROf<K>, Stats>
where
    Q: IntoQuery,
    Q::Q: Drive<D = Id<Post>, R = Id<Post>>,
    K: IntoQuery,
    K::Q: Probe<D = Id<Post>>,
    ROf<K>: Eq + std::hash::Hash + Copy,
{
    let j = Joins::parse(joins);
    let Vote { vote_type_id, vote_type, bounty_amount, creation_date, .. } = &db.vote;
    let cs = comments_of_if(db, j.c).select((&db.comment.creation_date).and(&db.comment.score)).opt();
    let vs = votes_of_if(db, j.v)
        .select(
            vote_type_id
                .filt(move |t| vtypes.is_empty() || vtypes.contains(&t))
                .and(vote_type.select((&db.vote_type.origid).and(&db.vote_type.name)).opt())
                .and(bounty_amount.opt())
                .and(creation_date),
        )
        .opt();
    let as_ = if j.typed { answers_of_if(db, true) } else { children_of_if(db, j.a) }.opt();
    let bs = (&db.post.owner_user).select(badges_of_if(db, j.b).select((&db.badge.date).and(&db.badge.name).and(&db.badge.class))).opt();
    let hs = history_of_if(db, j.h).select((&db.post_history.creation_date).and(&db.post_history.post_history_type_id)).opt();
    let init = Stats {
        rows: 0,
        cx: 0,
        vx: 0,
        ax: 0,
        bx: 0,
        hx: 0,
        up: 0,
        down: 0,
        bounty_n: 0,
        bounty_sum: 0,
        vt_sum: 0,
        vtj_n: 0,
        vtj_sum: 0,
        vmax: i64::MIN,
        hmax: i64::MIN,
        hmin: i64::MAX,
        bmax: i64::MIN,
        bname: None,
        bclass: 0,
        h10: 0,
        h11: 0,
        h10max: i64::MIN,
        h11max: i64::MIN,
        cmax: i64::MIN,
        cscore: 0,
        upn: 0,
        downn: 0,
        upj: 0,
        downj: 0,
        h45: 0,
        by_vt: [0; 16],
        accn: 0,
        lx: 0,
        bcls: [0; 4],
    };
    let ls = links_of_if(db, j.l).opt();
    posts
        .group_by(key)
        .select(cs.and(vs).and(as_).and(bs).and(hs).and(ls))
        .fold(init, |s, (((((c, v), a), b), h), l)| {
            let mut s = Stats {
                rows: s.rows + 1,
                lx: s.lx + l.is_some() as i64,
                cx: s.cx + c.is_some() as i64,
                vx: s.vx + v.is_some() as i64,
                ax: s.ax + a.is_some() as i64,
                bx: s.bx + b.is_some() as i64,
                hx: s.hx + h.is_some() as i64,
                hmax: h.map_or(s.hmax, |(d, _)| s.hmax.max(d)),
                hmin: h.map_or(s.hmin, |(d, _)| s.hmin.min(d)),
                h10: s.h10 + (h.map(|x| x.1) == Some(10)) as i64,
                h11: s.h11 + (h.map(|x| x.1) == Some(11)) as i64,
                h45: s.h45 + matches!(h.map(|x| x.1), Some(4) | Some(5)) as i64,
                h10max: match h {
                    Some((d, 10)) => s.h10max.max(d),
                    _ => s.h10max,
                },
                h11max: match h {
                    Some((d, 11)) => s.h11max.max(d),
                    _ => s.h11max,
                },
                cmax: c.map_or(s.cmax, |(d, _)| s.cmax.max(d)),
                cscore: s.cscore + c.map_or(0, |(_, x)| x),
                bmax: b.map_or(s.bmax, |((d, _), _)| s.bmax.max(d)),
                bname: match b {
                    Some(((_, n), _)) => Some(s.bname.map_or(n, |m| m.max(n))),
                    None => s.bname,
                },
                bclass: s.bclass + b.map_or(0, |(_, c)| c),
                ..s
            };
            if let Some((_, c)) = b {
                if (0..4).contains(&c) {
                    s.bcls[c as usize] += 1;
                }
            }
            if let Some((((t, tj), bounty), vd)) = v {
                s.up += (t == 2) as i64;
                s.down += (t == 3) as i64;
                if (0..16).contains(&t) {
                    s.by_vt[t as usize] += 1;
                }
                s.vt_sum += t;
                s.vtj_n += tj.is_some() as i64;
                s.vtj_sum += tj.map_or(0, |x| x.0);
                s.upn += (tj.map(|x| x.1) == Some("UpMod")) as i64;
                s.downn += (tj.map(|x| x.1) == Some("DownMod")) as i64;
                s.accn += (tj.map(|x| x.1) == Some("AcceptedByOriginator")) as i64;
                s.upj += (tj.map(|x| x.0) == Some(2)) as i64;
                s.downj += (tj.map(|x| x.0) == Some(3)) as i64;
                s.bounty_n += bounty.is_some() as i64;
                s.bounty_sum += bounty.unwrap_or(0);
                s.vmax = s.vmax.max(vd);
            }
            s
        })
}

/// One row per post: `GROUP BY p.Id, <the post's own columns>`.
pub fn post_stats<Q>(db: &'static So, posts: Q, joins: &str, vtypes: &'static [i64]) -> Vec<(Id<Post>, Stats)>
where
    Q: IntoQuery,
    Q::Q: Drive<D = Id<Post>, R = Id<Post>>,
{
    group_stats(db, posts, Ident::<Post>::new(), joins, vtypes)
}

pub fn stat_field(s: &Stats, c: &str) -> Option<V> {
    Some(match c {
        "#rows" => V::I(s.rows),
        "#cx" => V::I(s.cx),
        "#vx" => V::I(s.vx),
        "#ax" => V::I(s.ax),
        "#bx" => V::I(s.bx),
        "#hx" => V::I(s.hx),
        "#up" => V::I(s.up),
        "#down" => V::I(s.down),
        "bounty_sum" => nullable(s.bounty_sum, s.bounty_n),
        "bounty_avg" => avg(s.bounty_sum, s.bounty_n),
        "vt_avg" => avg(s.vt_sum, s.vx),
        "vtj_avg" => avg(s.vtj_sum, s.vtj_n),
        "vmax" => if s.vx == 0 { V::Null } else { V::T(s.vmax) },
        "hmax" => if s.hx == 0 { V::Null } else { V::T(s.hmax) },
        "hmin" => if s.hx == 0 { V::Null } else { V::T(s.hmin) },
        "bmax" => if s.bx == 0 { V::Null } else { V::T(s.bmax) },
        "bname" => crate::fmt::ostr(s.bname),
        "#h10" => V::I(s.h10),
        "#h11" => V::I(s.h11),
        "#h1011" => V::I(s.h10 + s.h11),
        "h10max" => if s.h10max == i64::MIN { V::Null } else { V::T(s.h10max) },
        "h11max" => if s.h11max == i64::MIN { V::Null } else { V::T(s.h11max) },
        "cmax" => if s.cx == 0 { V::Null } else { V::T(s.cmax) },
        "cscore" => nullable(s.cscore, s.cx),
        "#upn" => V::I(s.upn),
        "#downn" => V::I(s.downn),
        "up_frac" => V::F(s.up as f64 / s.rows as f64),
        "#upj" => V::I(s.upj),
        "#v1" => V::I(s.by_vt[1]),
        "#v6" => V::I(s.by_vt[6]),
        "#v4" => V::I(s.by_vt[4]),
        "#v10" => V::I(s.by_vt[10]),
        "#v11" => V::I(s.by_vt[11]),
        "#accn" => V::I(s.accn),
        "#lx" => V::I(s.lx),
        "#gold" => V::I(s.bcls[1]),
        "#silver" => V::I(s.bcls[2]),
        "#bronze" => V::I(s.bcls[3]),
        "#downj" => V::I(s.downj),
        "bclass_avg" => avg(s.bclass, s.bx),
        "bounty0_avg" => V::F(s.bounty_sum as f64 / s.rows as f64),
        "h10_frac" => V::F(s.h10 as f64 / s.rows as f64),
        "h11_frac" => V::F(s.h11 as f64 / s.rows as f64),
        "h45_one" => if s.h45 == 0 { V::Null } else { V::F(1.0) },
        "down_frac" => V::F(s.down as f64 / s.rows as f64),
        _ => return None,
    })
}

/// A post column or one of the aggregates.
pub fn stat_fields(db: &'static So, p: Id<Post>, s: &Stats, cols: &[&str]) -> Vec<V> {
    cols.iter()
        .map(|c| match stat_field(s, c) {
            Some(v) => v,
            None => match *c {
                // AVG(u.Reputation) over the joined rows of one post
                "rep_avg" => crate::fmt::ofloat(
                    db.post.owner_user.get(p).map(|u| db.user.reputation.get(u).unwrap() as f64),
                ),
                _ => post_fields(db, p, &[c]).pop().unwrap(),
            },
        })
        .collect()
}

/// Sort by `key` (smallest first), cut at `n` (0 meaning no LIMIT) and print.
pub fn stat_rows<T: Ord>(
    db: &'static So,
    mut v: Vec<(Id<Post>, Stats)>,
    key: impl Fn(Id<Post>, &Stats) -> T,
    n: usize,
    cols: &[&str],
) -> String {
    v.sort_by_key(|(p, s)| key(*p, s));
    let n = if n == 0 { v.len() } else { n };
    crate::fmt::rows(v.iter().take(n).map(|(p, s)| crate::fmt::row(stat_fields(db, *p, s, cols))))
}

/// `SELECT p.*`, in the table's column order.
pub const POST_STAR: &[&str] = &[
    "id", "type_id", "accepted", "parent", "created", "score", "views", "body", "owner_id", "owner_name", "editor_id",
    "editor_name", "edited", "activity", "title", "tags", "answers", "comments", "favorites", "closed", "community", "license",
];

// --- users grouped by a user column, LEFT JOIN Posts ------------------------

/// `FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY <key>`:
/// every field is over the joined rows, one per post or one with no post.
#[derive(Clone, Copy, Default)]
pub struct NameAgg {
    /// COUNT(*), and COUNT(p.Id).
    pub rows: i64,
    pub n: i64,
    /// Posts of type 1, 2, 3, and 4 or 5.
    pub q: i64,
    pub a: i64,
    pub t3: i64,
    pub t45: i64,
    pub t10: i64,
    pub t12: i64,
    /// Posts with ViewCount > 1000.
    pub popular: i64,
    pub created_max: i64,
    /// Posts whose `LEFT JOIN PostTypes` name is 'Question' / 'Answer'.
    pub qn: i64,
    pub an: i64,
    pub score_sum: i64,
    pub pos: i64,
    pub neg: i64,
    pub views_n: i64,
    pub views_sum: i64,
    /// SUM(u.Reputation) over the joined rows.
    pub rep_sum: i64,
    /// SUM(EXTRACT(EPOCH FROM p.CreationDate)) in exact microseconds. A float
    /// AVG of it has no single answer in its last digit, so the queries that
    /// take one are rewritten to `SUM(epoch_us(..))::DOUBLE / COUNT(..) / 1e6`.
    pub created_us: i128,
}

pub fn user_groups<K>(db: &'static So, key: K, w: UserWhere) -> Vec<(ROf<K>, NameAgg)>
where
    K: IntoQuery,
    K::Q: Probe<D = Id<User>>,
    ROf<K>: Eq + std::hash::Hash + Copy,
{
    let Post { post_type_id, post_type, score, view_count, creation_date, .. } = &db.post;
    let mut v = Vec::new();
    let post = post_type_id.and(score).and(view_count.opt()).and(creation_date).and(post_type.select(&db.post_type.name).opt());
    user_base(db, w)
        .group_by(key)
        .select((&db.user.reputation).and(posts_of(db).select(post).opt()))
        .fold(NameAgg::default(), |a, (rep, p)| {
            let mut a = NameAgg { rows: a.rows + 1, rep_sum: a.rep_sum + rep, ..a };
            if let Some(((((t, s), w), c), name)) = p {
                a.qn += (name == Some("Question")) as i64;
                a.an += (name == Some("Answer")) as i64;
                a.n += 1;
                a.q += (t == 1) as i64;
                a.a += (t == 2) as i64;
                a.t3 += (t == 3) as i64;
                a.t45 += (t == 4 || t == 5) as i64;
                a.t10 += (t == 10) as i64;
                a.t12 += (t == 12) as i64;
                a.popular += w.is_some_and(|x| x > 1000) as i64;
                a.created_max = if a.n == 1 { c } else { a.created_max.max(c) };
                a.score_sum += s;
                a.pos += (s > 0) as i64;
                a.neg += (s < 0) as i64;
                a.views_n += w.is_some() as i64;
                a.views_sum += w.unwrap_or(0);
                a.created_us += c as i128;
            }
            a
        })
        .drive(|k, a| v.push((k, a)));
    v
}

pub fn name_field(a: &NameAgg, c: &str) -> V {
    match c {
        "#n" => V::I(a.n),
        "#q" => V::I(a.q),
        "#a" => V::I(a.a),
        "#3" => V::I(a.t3),
        "#45" => V::I(a.t45),
        "#10" => V::I(a.t10),
        "#12" => V::I(a.t12),
        "#popular" => V::I(a.popular),
        "created_max" => if a.n == 0 { V::Null } else { V::T(a.created_max) },
        "views_avg" => avg(a.views_sum, a.views_n),
        "#qn" => V::I(a.qn),
        "#an" => V::I(a.an),
        "#pos" => V::I(a.pos),
        "#neg" => V::I(a.neg),
        "score_sum" => nullable(a.score_sum, a.n),
        "score_sum0" => V::I(a.score_sum),
        "score_avg" => avg(a.score_sum, a.n),
        "views_sum" => nullable(a.views_sum, a.views_n),
        "views_sum0" => V::I(a.views_sum),
        "rep_avg" => avg(a.rep_sum, a.rows),
        "created_avg" => if a.n == 0 { V::Null } else { V::F(a.created_us as f64 / a.n as f64 / 1e6) },
        _ => panic!("name_field: unknown column {c}"),
    }
}

/// `COUNT(DISTINCT x.Id)` per post, for `x` one of the post's children.
pub fn per_post_distinct<R>(db: &'static So, children: R) -> Fold<Id<Post>, i64>
where
    R: IntoQuery,
    R::Q: Probe<D = Id<Post>>,
    ROf<R>: Ord,
{
    db.post.group_by(Ident::<Post>::new()).select(children).count_distinct()
}

/// Users LEFT JOIN Posts LEFT JOIN Votes, grouped by `key`: COUNT(p.Id),
/// the questions and answers, the votes of type 2, 3, 2 or 3, and 5,
/// SUM(v.BountyAmount) as (n, sum), SUM(p.Score), MAX(p.CreationDate), and
/// SUM(u.Reputation) over the joined rows with the number of them.
pub fn user_vote_groups<K>(db: &'static So, key: K, w: UserWhere, vtypes: &'static [i64]) -> Vec<(ROf<K>, [i64; 13])>
where
    K: IntoQuery,
    K::Q: Probe<D = Id<User>>,
    ROf<K>: Eq + std::hash::Hash + Copy,
{
    let Post { post_type_id, score, creation_date, .. } = &db.post;
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let votes = votes_of(db).select(vote_type_id.filt(move |t| vtypes.is_empty() || vtypes.contains(&t)).and(bounty_amount.opt())).opt();
    let mut v = Vec::new();
    user_base(db, w)
        .group_by(key)
        .select((&db.user.reputation).and(posts_of(db).select(post_type_id.and(score).and(creation_date).and(votes)).opt()))
        .fold([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, i64::MIN, 0, 0], |a: [i64; 13], (rep, p)| {
            let mut a = a;
            a[11] += rep;
            a[12] += 1;
            if let Some((((t, s), c), x)) = p {
                let (vt, b) = match x {
                    Some((vt, b)) => (Some(vt), b),
                    None => (None, None),
                };
                a[0] += 1;
                a[1] += (t == 1) as i64;
                a[2] += (t == 2) as i64;
                a[3] += (vt == Some(2)) as i64;
                a[4] += (vt == Some(3)) as i64;
                a[5] += matches!(vt, Some(2) | Some(3)) as i64;
                a[6] += (vt == Some(5)) as i64;
                a[7] += b.is_some() as i64;
                a[8] += b.unwrap_or(0);
                a[9] += s;
                a[10] = a[10].max(c);
            }
            a
        })
        .drive(|k, a| v.push((k, a)));
    v
}

/// Per-post aggregates beside up to three `COUNT(DISTINCT ...)` per post
/// (`per_post_distinct`), joined on the post. `#d0`..`#d2` print them.
pub fn stats_with<Q>(db: &'static So, base: Q, joins: &str, vtypes: &'static [i64], d: &[&Fold<Id<Post>, i64>]) -> Vec<(Id<Post>, Stats, [i64; 3])>
where
    Q: Drive<D = Id<Post>, R = Id<Post>>,
{
    let f = stats_fold(db, base, Ident::<Post>::new(), joins, vtypes);
    let o = |x: Option<i64>| x.unwrap_or(0);
    let mut v = Vec::new();
    match d {
        [] => f.drive(|p, s| v.push((p, s, [0; 3]))),
        [a] => f.and(a.opt()).drive(|p, (s, x)| v.push((p, s, [o(x), 0, 0]))),
        [a, b] => f.and(a.opt()).and(b.opt()).drive(|p, ((s, x), y)| v.push((p, s, [o(x), o(y), 0]))),
        [a, b, c] => f.and(a.opt()).and(b.opt()).and(c.opt()).drive(|p, (((s, x), y), z)| v.push((p, s, [o(x), o(y), o(z)]))),
        _ => panic!("stats_with: at most three distinct counts"),
    }
    v
}

/// Sort by `key` (smallest first), cut at `n` (0 meaning no LIMIT) and print.
pub fn stats_rows<T: Ord>(db: &'static So, mut v: Vec<(Id<Post>, Stats, [i64; 3])>, key: impl Fn(Id<Post>, &Stats) -> T, n: usize, cols: &[&str]) -> String {
    v.sort_by_key(|(p, s, _)| key(*p, s));
    let n = if n == 0 { v.len() } else { n };
    crate::fmt::rows(v.iter().take(n).map(|(p, s, d)| {
        crate::fmt::row(
            cols.iter()
                .map(|c| match *c {
                    "#d0" => V::I(d[0]),
                    "#d1" => V::I(d[1]),
                    "#d2" => V::I(d[2]),
                    _ => stat_fields(db, *p, s, &[c]).pop().unwrap(),
                })
                .collect(),
        )
    }))
}

pub fn newest(db: &'static So, p: Id<Post>) -> std::cmp::Reverse<i64> {
    std::cmp::Reverse(db.post.creation_date.get(p).unwrap())
}

// --- users with their posts and everything joined to them -------------------

/// `FROM Users u LEFT JOIN Posts p ... [LEFT JOIN Comments c ON p.Id]
/// [LEFT JOIN Votes v ON p.Id] [LEFT JOIN PostHistory ph ON p.Id]
/// [LEFT JOIN Badges b ON u.Id]`, folded over the joined rows of each group.
#[derive(Clone, Copy)]
pub struct UStats {
    pub rows: i64,
    pub n: i64,
    pub q: i64,
    pub a: i64,
    pub t45: i64,
    pub t3: i64,
    pub t38: i64,
    pub score_sum: i64,
    pub pos: i64,
    pub views_n: i64,
    pub views_sum: i64,
    pub ans_n: i64,
    pub ans_sum: i64,
    pub cc_sum: i64,
    pub body_len: i64,
    pub pmax: i64,
    pub pmin: i64,
    pub cx: i64,
    pub cscore: i64,
    pub vx: i64,
    pub up: i64,
    pub down: i64,
    pub v4: i64,
    pub v7: i64,
    pub bounty_n: i64,
    pub bounty_sum: i64,
    pub hx: i64,
    /// History rows of type 10 to 13; MAX(p.LastActivityDate).
    pub hclose: i64,
    pub lamax: i64,
    pub bx: i64,
    pub bclass: i64,
    pub bmax: i64,
    pub rep_sum: i64,
    pub bcls: [i64; 4],
    /// MAX(c.CreationDate) over the joined comments.
    pub cmax: i64,
}

/// `joins`: `c`, `v`, `h` hang off each post, `b` off the user. `pf` filters
/// the joined posts on (PostTypeId, CreationDate) — a condition in the join,
/// so a user whose posts all fail it keeps one row with no post.
pub fn user_stats_fold<K>(db: &'static So, key: K, w: UserWhere, joins: &str, pf: fn(i64, i64) -> bool) -> Fold<ROf<K>, UStats>
where
    K: IntoQuery,
    K::Q: Probe<D = Id<User>>,
    ROf<K>: Eq + std::hash::Hash + Copy,
{
    user_stats_fold_v(db, key, w, joins, pf, &[])
}

/// The same with `AND v.VoteTypeId IN (vtypes)` on the Votes join.
pub fn user_stats_fold_v<K>(db: &'static So, key: K, w: UserWhere, joins: &str, pf: fn(i64, i64) -> bool, vtypes: &'static [i64]) -> Fold<ROf<K>, UStats>
where
    K: IntoQuery,
    K::Q: Probe<D = Id<User>>,
    ROf<K>: Eq + std::hash::Hash + Copy,
{
    let j = Joins::parse(joins);
    let Post { post_type_id, score, view_count, answer_count, comment_count, body, creation_date, last_activity_date, .. } = &db.post;
    let post = Ident::<Post>::new()
        .with(post_type_id.and(creation_date).filt(move |(t, c)| pf(t, c)))
        .select(
            post_type_id
                .and(score)
                .and(view_count.opt())
                .and(answer_count.opt())
                .and(comment_count)
                .and(body)
                .and(creation_date)
                .and(comments_of_if(db, j.c).select((&db.comment.score).and(&db.comment.creation_date)).opt())
                .and(
                    votes_of_if(db, j.v)
                        .select((&db.vote.vote_type_id).filt(move |t| vtypes.is_empty() || vtypes.contains(&t)).and((&db.vote.bounty_amount).opt()))
                        .opt(),
                )
                .and(history_of_if(db, j.h).select(&db.post_history.post_history_type_id).opt())
                .and(last_activity_date),
        );
    let badges = badges_of_if(db, j.b).select((&db.badge.class).and(&db.badge.date)).opt();
    let init = UStats {
        rows: 0,
        n: 0,
        q: 0,
        a: 0,
        t45: 0,
        t3: 0,
        t38: 0,
        score_sum: 0,
        pos: 0,
        views_n: 0,
        views_sum: 0,
        ans_n: 0,
        ans_sum: 0,
        cc_sum: 0,
        body_len: 0,
        pmax: i64::MIN,
        pmin: i64::MAX,
        cx: 0,
        cscore: 0,
        vx: 0,
        up: 0,
        down: 0,
        v4: 0,
        v7: 0,
        bounty_n: 0,
        bounty_sum: 0,
        hx: 0,
        hclose: 0,
        lamax: i64::MIN,
        bx: 0,
        bclass: 0,
        bmax: i64::MIN,
        rep_sum: 0,
        bcls: [0; 4],
        cmax: i64::MIN,
    };
    user_base(db, w)
        .group_by(key)
        .select((&db.user.reputation).and(posts_of(db).select(post).opt()).and(badges))
        .fold(init, |a, ((rep, p), b)| {
            let mut a = UStats { rows: a.rows + 1, rep_sum: a.rep_sum + rep, ..a };
            if let Some((cls, d)) = b {
                if (0..4).contains(&cls) {
                    a.bcls[cls as usize] += 1;
                }
                a.bx += 1;
                a.bclass += cls;
                a.bmax = a.bmax.max(d);
            }
            if let Some(((((((((((t, s), w), an), cc), body), cd), c), v), h), la)) = p {
                a.lamax = a.lamax.max(la);
                a.hclose += matches!(h, Some(10..=13)) as i64;
                a.n += 1;
                a.q += (t == 1) as i64;
                a.a += (t == 2) as i64;
                a.t45 += (t == 4 || t == 5) as i64;
                a.t3 += (t == 3) as i64;
                a.t38 += (3..=8).contains(&t) as i64;
                a.score_sum += s;
                a.pos += (s > 0) as i64;
                a.views_n += w.is_some() as i64;
                a.views_sum += w.unwrap_or(0);
                a.ans_n += an.is_some() as i64;
                a.ans_sum += an.unwrap_or(0);
                a.cc_sum += cc;
                a.body_len += body.chars().count() as i64;
                a.pmax = a.pmax.max(cd);
                a.pmin = a.pmin.min(cd);
                a.cx += c.is_some() as i64;
                a.cscore += c.map_or(0, |c| c.0);
                a.cmax = c.map_or(a.cmax, |c| a.cmax.max(c.1));
                a.hx += h.is_some() as i64;
                if let Some((vt, bounty)) = v {
                    a.vx += 1;
                    a.up += (vt == 2) as i64;
                    a.down += (vt == 3) as i64;
                    a.v4 += (vt == 4) as i64;
                    a.v7 += (vt == 7) as i64;
                    a.bounty_n += bounty.is_some() as i64;
                    a.bounty_sum += bounty.unwrap_or(0);
                }
            }
            a
        })
}

pub fn any_post(_: i64, _: i64) -> bool {
    true
}

/// COUNT(DISTINCT ...) of something reached from the user, per `key`.
pub fn user_distinct<K, R>(db: &'static So, key: K, w: UserWhere, r: R) -> Fold<ROf<K>, i64>
where
    K: IntoQuery,
    K::Q: Probe<D = Id<User>>,
    ROf<K>: Eq + std::hash::Hash + Copy,
    R: IntoQuery,
    R::Q: Probe<D = Id<User>>,
    ROf<R>: Ord,
{
    user_base(db, w).group_by(key).select(r).count_distinct()
}

pub fn ustat_field(a: &UStats, c: &str) -> V {
    let t = |x: i64| if a.n == 0 { V::Null } else { V::T(x) };
    match c {
        "#rows" => V::I(a.rows),
        "#n" => V::I(a.n),
        "#q" => V::I(a.q),
        "#a" => V::I(a.a),
        "#45" => V::I(a.t45),
        "#3" => V::I(a.t3),
        "#3to8" => V::I(a.t38),
        "#net" => V::I(a.up - a.down),
        "#pos" => V::I(a.pos),
        "score_sum" => nullable(a.score_sum, a.n),
        "score_sum0" => V::I(a.score_sum),
        "score_avg" => avg(a.score_sum, a.n),
        "views_sum" => nullable(a.views_sum, a.views_n),
        "views_sum0" => V::I(a.views_sum),
        "views_avg" => avg(a.views_sum, a.views_n),
        "answers_sum" => nullable(a.ans_sum, a.ans_n),
        "answers_avg" => avg(a.ans_sum, a.ans_n),
        "cc_avg" => avg(a.cc_sum, a.n),
        "len_avg0" => V::F(a.body_len as f64 / a.rows as f64),
        "created_max" => t(a.pmax),
        "created_min" => t(a.pmin),
        "#cx" => V::I(a.cx),
        "cscore_sum" => nullable(a.cscore, a.cx),
        "cscore_sum0" => V::I(a.cscore),
        "#vx" => V::I(a.vx),
        "#up" => V::I(a.up),
        "#down" => V::I(a.down),
        "#v4" => V::I(a.v4),
        "#v7" => V::I(a.v7),
        "#hclose" => V::I(a.hclose),
        "activity_max" => t(a.lamax),
        "bounty_sum" => nullable(a.bounty_sum, a.bounty_n),
        "bounty_sum0" => V::I(a.bounty_sum),
        "score_avg_rows" => V::F(a.score_sum as f64 / a.rows as f64),
        "cscore_avg_rows" => V::F(a.cscore as f64 / a.rows as f64),
        "#hx" => V::I(a.hx),
        "#bx" => V::I(a.bx),
        "bclass_sum" => nullable(a.bclass, a.bx),
        "bmax" => if a.bx == 0 { V::Null } else { V::T(a.bmax) },
        "rep_avg" => avg(a.rep_sum, a.rows),
        "#gold" => V::I(a.bcls[1]),
        "#silver" => V::I(a.bcls[2]),
        "#bronze" => V::I(a.bcls[3]),
        "cmax_or_epoch" => V::T(if a.cx == 0 { 0 } else { a.cmax }),
        _ => panic!("ustat_field: unknown column {c}"),
    }
}

pub fn user_col(db: &'static So, u: Id<User>, c: &str) -> V {
    let User { origid, display_name, reputation, creation_date, last_access_date, views, up_votes, down_votes, .. } = &db.user;
    match c {
        "uid" => V::I(origid.get(u).unwrap()),
        "name" => V::S(display_name.get(u).unwrap()),
        "rep" => V::I(reputation.get(u).unwrap()),
        "ucreated" => V::T(creation_date.get(u).unwrap()),
        "last_access" => V::T(last_access_date.get(u).unwrap()),
        "uviews" => V::I(views.get(u).unwrap()),
        "uup" => V::I(up_votes.get(u).unwrap()),
        "udown" => V::I(down_votes.get(u).unwrap()),
        _ => panic!("user_col: unknown column {c}"),
    }
}

/// One row per user: the joined-row fold beside up to four COUNT(DISTINCT ...).
pub fn users_stats_with(
    db: &'static So,
    w: UserWhere,
    joins: &str,
    pf: fn(i64, i64) -> bool,
    vtypes: &'static [i64],
    d: &[&Fold<Id<User>, i64>],
) -> Vec<(Id<User>, UStats, [i64; 4])> {
    let f = user_stats_fold_v(db, Ident::<User>::new(), w, joins, pf, vtypes);
    let o = |x: Option<i64>| x.unwrap_or(0);
    let mut v = Vec::new();
    match d {
        [] => f.drive(|u, s| v.push((u, s, [0; 4]))),
        [a] => f.and(a.opt()).drive(|u, (s, x)| v.push((u, s, [o(x), 0, 0, 0]))),
        [a, b] => f.and(a.opt()).and(b.opt()).drive(|u, ((s, x), y)| v.push((u, s, [o(x), o(y), 0, 0]))),
        [a, b, c] => f.and(a.opt()).and(b.opt()).and(c.opt()).drive(|u, (((s, x), y), z)| v.push((u, s, [o(x), o(y), o(z), 0]))),
        [a, b, c, e] => f.and(a.opt()).and(b.opt()).and(c.opt()).and(e.opt()).drive(|u, ((((s, x), y), z), q)| v.push((u, s, [o(x), o(y), o(z), o(q)]))),
        _ => panic!("users_stats_with: at most four distinct counts"),
    }
    v
}

/// Sort by `key` (smallest first), cut at `n` (0 meaning no LIMIT) and print
/// user columns, `ustat_field`s and `#d0`..`#d3`.
pub fn users_rows<T: Ord>(
    db: &'static So,
    mut v: Vec<(Id<User>, UStats, [i64; 4])>,
    key: impl Fn(Id<User>, &UStats, &[i64; 4]) -> T,
    n: usize,
    cols: &[&str],
) -> String {
    v.sort_by_key(|(u, s, d)| key(*u, s, d));
    let n = if n == 0 { v.len() } else { n };
    crate::fmt::rows(v.iter().take(n).map(|(u, s, d)| {
        crate::fmt::row(
            cols.iter()
                .map(|c| match *c {
                    "#d0" => V::I(d[0]),
                    "#d1" => V::I(d[1]),
                    "#d2" => V::I(d[2]),
                    "#d3" => V::I(d[3]),
                    "uid" | "name" | "rep" | "ucreated" | "last_access" | "uviews" | "uup" | "udown" => user_col(db, *u, c),
                    _ => ustat_field(s, c),
                })
                .collect(),
        )
    }))
}
