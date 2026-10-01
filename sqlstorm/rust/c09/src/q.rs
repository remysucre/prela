use harness::prelude::*;

pub struct Question {
    pub pid: Id<Post>,
    pub score: i64,
    pub views: Option<i64>,
    pub created: i64,
}

pub fn questions(db: &'static So) -> Vec<Question> {
    let Post { post_type_id, score, view_count, creation_date, owner_user, .. } = &db.post;

    let mut v = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .with(owner_user)
        .select(score.and(view_count.opt()).and(creation_date))
        .drive(|pid, ((score, views), created)| v.push(Question { pid, score, views, created }));
    v
}

fn top10(mut v: Vec<Question>, key: impl Fn(&Question, &Question) -> std::cmp::Ordering) -> Vec<Question> {
    v.sort_by(key);
    v.truncate(10);
    v
}

pub fn by_created(db: &'static So) -> Vec<Question> {
    top10(questions(db), |a, b| b.created.cmp(&a.created))
}

pub fn by_score(db: &'static So) -> Vec<Question> {
    top10(questions(db), |a, b| b.score.cmp(&a.score))
}

pub fn by_views(db: &'static So) -> Vec<Question> {
    top10(questions(db), |a, b| b.views.cmp(&a.views))
}
