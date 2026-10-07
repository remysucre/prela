use crate::schema::*;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct Age {
    pub subject_id: i64,
    pub hadm_id: i64,
    pub admittime: Ts,
    pub anchor_age: i64,
    pub anchor_year: i64,
    pub age: i64,
}

pub fn age(db: &'static Db) -> Vec<Age> {
    let ad = &db.admission;
    let pa = &db.patient;
    let q = (&ad.subject_id)
        .and(&ad.hadm_id)
        .and(&ad.admittime)
        .and((&ad.subject).select((&pa.anchor_age).and(&pa.anchor_year)));
    drain(q)
        .into_iter()
        .map(|(_, (((subject_id, hadm_id), admittime), (anchor_age, anchor_year)))| Age {
            subject_id,
            hadm_id,
            admittime,
            anchor_age,
            anchor_year,
            age: anchor_age + year(admittime) - anchor_year,
        })
        .collect()
}

pub fn q(db: &'static Db) -> String {
    rows(age(db).into_iter().map(|a| {
        row(vec![
            V::I(a.subject_id),
            V::I(a.hadm_id),
            V::T(a.admittime),
            V::I(a.anchor_age),
            V::I(a.anchor_year),
            V::I(a.age),
        ])
    }))
}
