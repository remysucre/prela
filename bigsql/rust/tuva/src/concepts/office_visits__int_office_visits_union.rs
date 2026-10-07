use crate::concepts::anchor_tmpl_b as k;
use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::concepts::office_visits__int_office_visits as ov;
use crate::concepts::office_visits__int_office_visits_radiology::OfficeVisitsIntOfficeVisitsRadiology;
use crate::schema::Db;
use harness::prelude::*;

type Ov = ov::OfficeVisitsIntOfficeVisits;
type Sub = ov::OfficeVisitSub;
type Rad = OfficeVisitsIntOfficeVisitsRadiology;
type U = (Str, i64, Str, Str, Str, i64);

#[derive(Clone, Copy)]
pub struct OfficeVisitsIntOfficeVisitsUnion {
    pub claim_id: Str,
    pub claim_line_number: i64,
    pub data_source: Str,
    pub old_encounter_id: Str,
    pub encounter_type: Str,
    pub priority_number: i64,
}

#[allow(clippy::too_many_arguments)]
pub fn office_visits__int_office_visits_union(
    _db: &'static Db,
    radiology: &[Rad],
    surgery: &[Sub],
    injections: &[Sub],
    ptotst: &[Sub],
    em: &[Sub],
    telehealth: &[Sub],
    visits: &[Ov],
) -> Vec<OfficeVisitsIntOfficeVisitsUnion> {
    let r = rel(radiology.to_vec());
    let s = rel(surgery.to_vec());
    let i = rel(injections.to_vec());
    let p = rel(ptotst.to_vec());
    let e = rel(em.to_vec());
    let t = rel(telehealth.to_vec());
    let o = rel(visits.to_vec());
    let sub = |ty: Str, n: i64| move |x: Sub| (x.claim_id, x.claim_line_number, x.data_source, x.old_encounter_id, ty, n);
    let d = (&r)
        .map(|x: Rad| (x.claim_id, x.claim_line_number, x.data_source, x.old_encounter_id, "office visit radiology", 0i64))
        .union((&s).map(sub("office visit surgery", 1)))
        .union((&i).map(sub("office visit injections", 2)))
        .union((&p).map(sub("office visit pt/ot/st", 3)))
        .union((&e).map(sub("office visit", 4)))
        .union((&t).map(sub("telehealth", 5)))
        .union((&o).map(|x: Ov| (x.claim_id, x.claim_line_number, x.data_source, x.old_encounter_id, "office visit - other", 9999i64)))
        .inv()
        .map(|_| ())
        .fold((), |a, _| a);
    drain(&d)
        .into_iter()
        .map(|((c, cl, ds, oe, ty, n), _): (U, ())| OfficeVisitsIntOfficeVisitsUnion {
            claim_id: c,
            claim_line_number: cl,
            data_source: ds,
            old_encounter_id: oe,
            encounter_type: ty,
            priority_number: n,
        })
        .collect()
}

pub fn union_chain(db: &'static Db, stg: &[EncountersStgMedicalClaim]) -> Vec<OfficeVisitsIntOfficeVisitsUnion> {
    let v = ov::office_visits__int_office_visits(db, stg);
    let r = crate::concepts::office_visits__int_office_visits_radiology::office_visits__int_office_visits_radiology(db, stg, &v);
    let s = crate::concepts::office_visits__int_office_visits_surgery::office_visits__int_office_visits_surgery(db, &v);
    let i = crate::concepts::office_visits__int_office_visits_injections::office_visits__int_office_visits_injections(db, stg, &v);
    let p = crate::concepts::office_visits__int_office_visits_ptotst::office_visits__int_office_visits_ptotst(db, &v);
    let e = crate::concepts::office_visits__int_office_visits_em::office_visits__int_office_visits_em(db, &v);
    let t = crate::concepts::office_visits__int_office_visits_telehealth::office_visits__int_office_visits_telehealth(db, &v);
    office_visits__int_office_visits_union(db, &r, &s, &i, &p, &e, &t, &v)
}

pub fn fmt(v: &OfficeVisitsIntOfficeVisitsUnion) -> String {
    row(vec![
        V::S(v.claim_id),
        V::I(v.claim_line_number),
        V::S(v.data_source),
        V::S(v.old_encounter_id),
        V::S(v.encounter_type),
        V::I(v.priority_number),
    ])
}

pub fn q(db: &'static Db) -> String {
    let s = k::stg(db);
    rows(union_chain(db, &s).iter().map(fmt))
}
