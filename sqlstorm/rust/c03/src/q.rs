use harness::prelude::*;

pub struct Question {
    pub pid: Id<Post>,
    pub id: i64,
    pub score: i64,
    pub views: Option<i64>,
    pub created: i64,
    pub display_name: Str,
}

pub fn questions(db: &'static So) -> Vec<Question> {
    let Post { post_type_id, origid, score, view_count, creation_date, owner_user, .. } = &db.post;
    let display_name = &db.user.display_name;

    let mut v = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .select(origid.and(score).and(view_count.opt()).and(creation_date).and(owner_user.select(display_name)))
        .drive(|pid, ((((id, score), views), created), display_name)| {
            v.push(Question { pid, id, score, views, created, display_name })
        });
    v
}

pub fn by_created(db: &'static So) -> Vec<Question> {
    let mut v = questions(db);
    v.sort_by(|a, b| b.created.cmp(&a.created));
    v.truncate(10);
    v
}
