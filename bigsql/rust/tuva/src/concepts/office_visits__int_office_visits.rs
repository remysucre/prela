use crate::concepts::anchor_tmpl_b as k;
use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::schema::Db;
use harness::prelude::*;

type M = EncountersStgMedicalClaim;

#[derive(Clone, Copy)]
pub struct OfficeVisitsIntOfficeVisits {
    pub patient_data_source_id: Str,
    pub data_source: Str,
    pub start_date: Option<Date>,
    pub claim_id: Str,
    pub claim_line_number: i64,
    pub service_category_1: Str,
    pub service_category_2: Str,
    pub service_category_3: Str,
    pub old_encounter_id: Str,
}

#[derive(Clone, Copy)]
pub struct OfficeVisitSub {
    pub patient_data_source_id: Str,
    pub data_source: Str,
    pub start_date: Option<Date>,
    pub claim_id: Str,
    pub claim_line_number: i64,
    pub old_encounter_id: Str,
}

type Ov = OfficeVisitsIntOfficeVisits;

pub fn office_visits__int_office_visits(db: &'static Db, stg: &[M]) -> Vec<Ov> {
    let m = rel(stg.to_vec());
    let p = &db.int_combined_professional;
    let px: HashIdx<((Str, i64), Str), _> = (&p.id)
        .with((&p.service_category_1).filt(|s: Str| s == "office-based"))
        .select((&p.claim_id).and(&p.claim_line_number).and(&p.data_source))
        .inv()
        .collect();
    let a = (&m)
        .and((&m).map(|x: M| ((x.claim_id, x.claim_line_number), x.data_source)).select(&px))
        .map(|(x, _)| {
            (
                x.patient_data_source_id,
                x.data_source,
                x.start_date,
                x.claim_id,
                x.claim_line_number,
                (x.service_category_1, x.service_category_2, x.service_category_3),
            )
        })
        .inv()
        .map(|_| ())
        .fold((), |a, _| a);
    drain(&a)
        .into_iter()
        .map(|((pd, ds, sd, c, cl, (s1, s2, s3)), _)| Ov {
            patient_data_source_id: pd,
            data_source: ds,
            start_date: sd,
            claim_id: c,
            claim_line_number: cl,
            service_category_1: s1,
            service_category_2: s2,
            service_category_3: s3,
            old_encounter_id: k::surrogate(&[Some("office based"), Some(pd), k::date_text(sd).as_deref()]),
        })
        .collect()
}

pub fn by_service_category_2(ov: &[Ov], sc2: &'static str) -> Vec<OfficeVisitSub> {
    let o = rel(ov.to_vec());
    let d = (&o)
        .filt(move |x: Ov| x.service_category_2 == sc2)
        .map(|x: Ov| (x.patient_data_source_id, x.data_source, x.start_date, x.claim_id, x.claim_line_number, x.old_encounter_id))
        .inv()
        .map(|_| ())
        .fold((), |a, _| a);
    subs(drain(&d).into_iter().map(|(r, _)| r).collect())
}

pub fn subs(v: Vec<(Str, Str, Option<Date>, Str, i64, Str)>) -> Vec<OfficeVisitSub> {
    v.into_iter()
        .map(|(pd, ds, sd, c, cl, e)| OfficeVisitSub {
            patient_data_source_id: pd,
            data_source: ds,
            start_date: sd,
            claim_id: c,
            claim_line_number: cl,
            old_encounter_id: e,
        })
        .collect()
}

pub fn fmt_sub(v: &OfficeVisitSub) -> String {
    row(vec![
        V::S(v.patient_data_source_id),
        V::S(v.data_source),
        odate(v.start_date),
        V::S(v.claim_id),
        V::I(v.claim_line_number),
        V::S(v.old_encounter_id),
    ])
}

pub fn fmt(v: &Ov) -> String {
    row(vec![
        V::S(v.patient_data_source_id),
        V::S(v.data_source),
        odate(v.start_date),
        V::S(v.claim_id),
        V::I(v.claim_line_number),
        V::S(v.service_category_1),
        V::S(v.service_category_2),
        V::S(v.service_category_3),
        V::S(v.old_encounter_id),
    ])
}

pub fn q(db: &'static Db) -> String {
    let s = k::stg(db);
    rows(office_visits__int_office_visits(db, &s).iter().map(fmt))
}
