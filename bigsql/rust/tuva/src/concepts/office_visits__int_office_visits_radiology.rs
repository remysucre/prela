use crate::concepts::anchor_tmpl_b as k;
use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::concepts::office_visits__int_office_visits as ov;
use crate::schema::Db;
use harness::prelude::*;

type M = EncountersStgMedicalClaim;
type Ov = ov::OfficeVisitsIntOfficeVisits;

#[derive(Clone, Copy)]
pub struct OfficeVisitsIntOfficeVisitsRadiology {
    pub patient_data_source_id: Str,
    pub data_source: Str,
    pub start_date: Option<Date>,
    pub claim_id: Str,
    pub claim_line_number: i64,
    pub hcpcs_code: Option<Str>,
    pub old_encounter_id: Str,
}

pub fn office_visits__int_office_visits_radiology(db: &'static Db, stg: &[M], visits: &[Ov]) -> Vec<OfficeVisitsIntOfficeVisitsRadiology> {
    let o = rel(visits.to_vec());
    let m = rel(stg.to_vec());
    let r = &db.service_category_office_based_radiology;
    let mx: HashIdx<(Str, i64, Str), usize> = (&m).map(|x: M| (x.claim_id, x.claim_line_number, x.data_source)).inv().collect();
    let rx: HashIdx<((Str, i64), Str), _> = (&r.claim_id).and(&r.claim_line_number).and(&r.data_source).inv().collect();
    let om = (&o).and((&o).map(|x: Ov| (x.claim_id, x.claim_line_number, x.data_source)).select(&mx).select(&m));
    let d = (&om)
        .and((&om).map(|(_, x): (Ov, M)| ((x.claim_id, x.claim_line_number), x.data_source)).select(&rx))
        .map(|((v, x), _)| (v.patient_data_source_id, v.data_source, v.start_date, v.claim_id, v.claim_line_number, x.hcpcs_code))
        .inv()
        .map(|_| ())
        .fold((), |a, _| a);
    drain(&d)
        .into_iter()
        .map(|((pd, ds, sd, c, cl, h), _)| OfficeVisitsIntOfficeVisitsRadiology {
            patient_data_source_id: pd,
            data_source: ds,
            start_date: sd,
            claim_id: c,
            claim_line_number: cl,
            hcpcs_code: h,
            old_encounter_id: k::surrogate(&[Some("office visit radiology"), Some(pd), k::date_text(sd).as_deref(), h]),
        })
        .collect()
}

pub fn fmt(v: &OfficeVisitsIntOfficeVisitsRadiology) -> String {
    row(vec![
        V::S(v.patient_data_source_id),
        V::S(v.data_source),
        odate(v.start_date),
        V::S(v.claim_id),
        V::I(v.claim_line_number),
        ostr(v.hcpcs_code),
        V::S(v.old_encounter_id),
    ])
}

pub fn q(db: &'static Db) -> String {
    let s = k::stg(db);
    let v = ov::office_visits__int_office_visits(db, &s);
    rows(office_visits__int_office_visits_radiology(db, &s, &v).iter().map(fmt))
}
