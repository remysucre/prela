use harness::prelude::*;

pub struct Question {
    pub pid: Id<Post>,
    pub uid: Id<User>,
    pub id: i64,
    pub score: i64,
    pub views: Option<i64>,
    pub created: i64,
    pub display_name: Str,
    pub reputation: i64,
}

pub fn questions(db: &'static So) -> Vec<Question> {
    let Post { post_type_id, origid, score, view_count, creation_date, owner_user, .. } = &db.post;
    let User { display_name, reputation, .. } = &db.user;

    let mut v = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .select(origid.and(score).and(view_count.opt()).and(creation_date).and(owner_user).and(owner_user.select(display_name.and(reputation))))
        .drive(|pid, (((((id, score), views), created), uid), (display_name, reputation))| {
            v.push(Question { pid, uid, id, score, views, created, display_name, reputation })
        });
    v
}

pub fn by_created(db: &'static So) -> Vec<Question> {
    top_n(questions(db), |q| std::cmp::Reverse(q.created), 10)
}
