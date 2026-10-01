use harness::prelude::*;

pub struct Question {
    pub pid: Id<Post>,
    pub id: i64,
    pub score: i64,
    pub views: Option<i64>,
    pub created: i64,
    pub body: Str,
    pub display_name: Str,
    pub reputation: i64,
}

pub fn questions(db: &'static So) -> Vec<Question> {
    let Post { post_type_id, origid, score, view_count, creation_date, body, owner_user, .. } = &db.post;
    let User { display_name, reputation, .. } = &db.user;

    let mut v = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .select(origid.and(score).and(view_count.opt()).and(creation_date).and(body).and(owner_user.select(display_name.and(reputation))))
        .drive(|pid, (((((id, score), views), created), body), (display_name, reputation))| {
            v.push(Question { pid, id, score, views, created, body, display_name, reputation })
        });
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
    v.sort_by(|a, b| b.views.cmp(&a.views));
    v.truncate(10);
    v
}

/// `Posts JOIN PostTypes GROUP BY pt.Id, pt.Name`: [posts, score sum, views present, views sum].
pub fn type_id_aggs(db: &'static So) -> Vec<(Id<PostType>, [i64; 4])> {
    let Post { post_type, score, view_count, .. } = &db.post;
    let g = db.post.group_by(post_type).select(score.and(view_count.opt())).fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    top_n(drain(&g), |&(_, a)| std::cmp::Reverse(a[0]), 0)
}
