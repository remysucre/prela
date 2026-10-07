use crate::concepts::anchor_tmpl_b as k;
use crate::concepts::office_visits__int_office_visits_union::OfficeVisitsIntOfficeVisitsUnion;
use crate::schema::Db;
use harness::prelude::*;

type U = OfficeVisitsIntOfficeVisitsUnion;
type E = (Str, Str, i64);

#[derive(Clone, Copy)]
pub struct OfficeVisitsIntOfficeVisitsEncounterRanking {
    pub old_encounter_id: Str,
    pub encounter_type: Str,
    pub priority_number: i64,
    pub relative_rank: i64,
}

pub fn office_visits__int_office_visits_encounter_ranking(_db: &'static Db, un: &[U]) -> Vec<OfficeVisitsIntOfficeVisitsEncounterRanking> {
    let u = rel(un.to_vec());
    let d = (&u).map(|x: U| (x.old_encounter_id, x.encounter_type, x.priority_number)).inv().map(|_| ()).fold((), |a, _| a);
    let d = rel(drain(&d).into_iter().map(|(e, _)| e).collect::<Vec<E>>());
    let w = (&d).map(|e: E| e.0).inv().select(&d).window(row_number, |e: E| e.2, |a: &i64, b: &i64| a.cmp(b));
    drain(&w)
        .into_iter()
        .map(|(_, ((oe, ty, n), rk))| OfficeVisitsIntOfficeVisitsEncounterRanking {
            old_encounter_id: oe,
            encounter_type: ty,
            priority_number: n,
            relative_rank: rk,
        })
        .collect()
}

pub fn fmt(v: &OfficeVisitsIntOfficeVisitsEncounterRanking) -> String {
    row(vec![V::S(v.old_encounter_id), V::S(v.encounter_type), V::I(v.priority_number), V::I(v.relative_rank)])
}

pub fn q(db: &'static Db) -> String {
    let s = k::stg(db);
    let u = crate::concepts::office_visits__int_office_visits_union::union_chain(db, &s);
    rows(office_visits__int_office_visits_encounter_ranking(db, &u).iter().map(fmt))
}
