use crate::concepts::anchor_tmpl_b as k;
use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::concepts::office_visits__int_office_visits as ov;
use crate::schema::Db;
use harness::prelude::*;

type M = EncountersStgMedicalClaim;
type Ov = ov::OfficeVisitsIntOfficeVisits;

pub type OfficeVisitsIntOfficeVisitsInjections = ov::OfficeVisitSub;

pub fn office_visits__int_office_visits_injections(_db: &'static Db, stg: &[M], visits: &[Ov]) -> Vec<OfficeVisitsIntOfficeVisitsInjections> {
    let o = rel(visits.to_vec());
    let m = rel(stg.to_vec());
    let mx: HashIdx<(Str, i64, Str), usize> = (&m).map(|x: M| (x.claim_id, x.claim_line_number, x.data_source)).inv().collect();
    let d = (&o)
        .and((&o).map(|x: Ov| (x.claim_id, x.claim_line_number, x.data_source)).select(&mx).select(&m))
        .filt(|(_, x): (Ov, M)| x.hcpcs_code.is_some_and(|h| h.chars().next() == Some('J')))
        .map(|(x, _): (Ov, M)| (x.patient_data_source_id, x.data_source, x.start_date, x.claim_id, x.claim_line_number, x.old_encounter_id))
        .inv()
        .map(|_| ())
        .fold((), |a, _| a);
    ov::subs(drain(&d).into_iter().map(|(r, _)| r).collect())
}

pub fn q(db: &'static Db) -> String {
    let s = k::stg(db);
    let v = ov::office_visits__int_office_visits(db, &s);
    rows(office_visits__int_office_visits_injections(db, &s, &v).iter().map(ov::fmt_sub))
}
