use crate::concepts::anchor_tmpl_b as k;
use crate::concepts::office_visits__int_office_visits_encounter_ranking::OfficeVisitsIntOfficeVisitsEncounterRanking;
use crate::concepts::office_visits__int_office_visits_union::OfficeVisitsIntOfficeVisitsUnion;
use crate::schema::Db;
use harness::prelude::*;

type U = OfficeVisitsIntOfficeVisitsUnion;
type R = OfficeVisitsIntOfficeVisitsEncounterRanking;

#[derive(Clone, Copy)]
pub struct OfficeVisitsIntOfficeVisitsClaimLine {
    pub claim_id: Str,
    pub claim_line_number: i64,
    pub data_source: Str,
    pub old_encounter_id: Str,
    pub encounter_type: Str,
}

pub fn office_visits__int_office_visits_claim_line(_db: &'static Db, un: &[U], ranking: &[R]) -> Vec<OfficeVisitsIntOfficeVisitsClaimLine> {
    let u = rel(un.to_vec());
    let r = rel(ranking.to_vec());
    let x: HashIdx<Str, usize> = (&r).filt(|x: R| x.relative_rank == 1).map(|x: R| x.old_encounter_id).inv().collect();
    let j = (&u).and((&u).map(|y: U| y.old_encounter_id).select(&x).select(&r));
    drain(&j)
        .into_iter()
        .map(|(_, (y, z))| OfficeVisitsIntOfficeVisitsClaimLine {
            claim_id: y.claim_id,
            claim_line_number: y.claim_line_number,
            data_source: y.data_source,
            old_encounter_id: y.old_encounter_id,
            encounter_type: z.encounter_type,
        })
        .collect()
}

pub fn fmt(v: &OfficeVisitsIntOfficeVisitsClaimLine) -> String {
    row(vec![V::S(v.claim_id), V::I(v.claim_line_number), V::S(v.data_source), V::S(v.old_encounter_id), V::S(v.encounter_type)])
}

pub fn q(db: &'static Db) -> String {
    let s = k::stg(db);
    let u = crate::concepts::office_visits__int_office_visits_union::union_chain(db, &s);
    let r = crate::concepts::office_visits__int_office_visits_encounter_ranking::office_visits__int_office_visits_encounter_ranking(db, &u);
    rows(office_visits__int_office_visits_claim_line(db, &u, &r).iter().map(fmt))
}
