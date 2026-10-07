use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::concepts::encounters__stg_outpatient_institutional::EncountersStgOutpatientInstitutional;
use crate::concepts::encounters__stg_professional::EncountersStgProfessional;
use crate::concepts::emergency_department__generate_encounter_id::EmergencyDepartmentGenerateEncounterId;
use crate::concepts::emergency_department__generate_encounter_id_pre_sort::asc_nl;
use crate::concepts::emergency_department__start_end_dates::EmergencyDepartmentStartEndDates;
use crate::schema::Db;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct EmergencyDepartmentProfClaims {
    pub encounter_id: Str,
    pub encounter_start_date: Option<Date>,
    pub encounter_end_date: Option<Date>,
    pub claim_id: Str,
    pub claim_line_number: i64,
    pub data_source: Str,
    pub claim_attribution_number: i64,
}

type G = EmergencyDepartmentGenerateEncounterId;
type D = EmergencyDepartmentStartEndDates;
type M = EncountersStgMedicalClaim;
type Pr = EncountersStgProfessional;
type I = EncountersStgOutpatientInstitutional;

#[derive(Clone, Copy)]
struct Jfc {
    f: G,
    end: Option<Date>,
    start: Option<Date>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Ip {
    encounter_id: Str,
    start: Option<Date>,
    end: Option<Date>,
    original_anchor_claim: Option<Str>,
    claim_id: Str,
    claim_line_number: i64,
    data_source: Str,
}

fn between(x: Option<Date>, lo: Option<Date>, hi: Option<Date>) -> bool {
    matches!((x, lo, hi), (Some(x), Some(l), Some(h)) if x >= l && x <= h)
}

fn ip(d: Jfc, claim_id: Str, claim_line_number: i64, data_source: Str) -> Ip {
    Ip {
        encounter_id: d.f.encounter_id,
        start: d.start,
        end: d.end,
        original_anchor_claim: d.f.original_anchor_claim,
        claim_id,
        claim_line_number,
        data_source,
    }
}

pub fn emergency_department__prof_claims(
    _db: &'static Db,
    stg: &[M],
    prof: &[Pr],
    inst: &[I],
    genc: &[G],
    sed: &[D],
) -> Vec<EmergencyDepartmentProfClaims> {
    let g = rel(genc.to_vec());
    let d = rel(sed.to_vec());
    let dx: HashIdx<Str, usize> = (&d).map(|x: D| x.encounter_id).inv().collect();
    let jfc = (&g)
        .filt(|x: G| x.encounter_claim_number == 1)
        .and((&g).map(|x: G| x.encounter_id).select(&dx).select(&d))
        .map(|(f, x): (G, D)| Jfc {
            f,
            end: x.encounter_end_date,
            start: x.encounter_start_date,
        });
    let jfc = rel(drain(&jfc).into_iter().map(|(_, x)| x).collect::<Vec<Jfc>>());
    let jx: HashIdx<Str, usize> = (&jfc).map(|x: Jfc| x.f.patient_data_source_id).inv().collect();
    let jx2: HashIdx<Str, usize> = (&jfc).map(|x: Jfc| x.f.patient_data_source_id).inv().collect();

    let m = rel(stg.to_vec());
    let p = rel(prof.to_vec());
    let i = rel(inst.to_vec());
    let px: HashIdx<(Str, Str), usize> = (&p).map(|x: Pr| (x.claim_line_id, x.data_source)).inv().collect();
    let ix: HashIdx<(Str, Str), usize> = (&i).map(|x: I| (x.claim_id, x.data_source)).inv().collect();
    let a = (&m)
        .and((&m).map(|x: M| (x.claim_line_id, x.data_source)).select(&px).select(&p))
        .and((&m).map(|x: M| x.patient_data_source_id).select(&jx).select(&jfc))
        .filt(|((x, _), d): ((M, Pr), Jfc)| between(x.start_date, d.start, d.end))
        .map(|((_, pr), d): ((M, Pr), Jfc)| ip(d, pr.claim_id, pr.claim_line_number, pr.data_source));
    let b = (&m)
        .and((&m).map(|x: M| (x.claim_id, x.data_source)).select(&ix).select(&i))
        .and((&m).map(|x: M| x.patient_data_source_id).select(&jx2).select(&jfc))
        .filt(|((x, _), d): ((M, I), Jfc)| between(x.start_date, d.start, d.end) && d.f.claim_id != x.claim_id)
        .map(|((x, _), d): ((M, I), Jfc)| ip(d, x.claim_id, x.claim_line_number, x.data_source));
    let u = rel(drain(a.union(b)).into_iter().map(|(_, x)| x).collect::<Vec<Ip>>());
    let w = (&u)
        .map(|x: Ip| (x.claim_id, x.claim_line_number, x.data_source))
        .inv()
        .select(&u)
        .window(
            row_number,
            |x: Ip| (x.original_anchor_claim, x.encounter_id),
            |a: &(Option<Str>, Str), b: &(Option<Str>, Str)| asc_nl(&a.0, &b.0).then(a.1.cmp(b.1)),
        )
        .map(|(x, n): (Ip, i64)| (x.encounter_id, x.start, x.end, x.claim_id, x.claim_line_number, x.data_source, n))
        .inv()
        .map(|_| ())
        .fold((), |a, _| a);
    drain(&w)
        .into_iter()
        .map(|((encounter_id, s, e, claim_id, claim_line_number, data_source, n), _)| EmergencyDepartmentProfClaims {
            encounter_id,
            encounter_start_date: s,
            encounter_end_date: e,
            claim_id,
            claim_line_number,
            data_source,
            claim_attribution_number: n,
        })
        .collect()
}

pub fn fmt(v: &EmergencyDepartmentProfClaims) -> String {
    row(vec![
        V::S(v.encounter_id),
        odate(v.encounter_start_date),
        odate(v.encounter_end_date),
        V::S(v.claim_id),
        V::I(v.claim_line_number),
        V::S(v.data_source),
        V::I(v.claim_attribution_number),
    ])
}

pub fn q(db: &'static Db) -> String {
    let stg = crate::concepts::emergency_department__generate_encounter_id_pre_sort::stg_medical_claim(db);
    let pre = crate::concepts::emergency_department__generate_encounter_id_pre_sort::emergency_department__generate_encounter_id_pre_sort(db, &stg);
    let genc = crate::concepts::emergency_department__generate_encounter_id::emergency_department__generate_encounter_id(db, &pre);
    let sed = crate::concepts::emergency_department__start_end_dates::emergency_department__start_end_dates(db, &genc);
    let prof = crate::concepts::encounters__stg_professional::encounters__stg_professional(db);
    let inst = crate::concepts::encounters__stg_outpatient_institutional::encounters__stg_outpatient_institutional(db);
    rows(emergency_department__prof_claims(db, &stg, &prof, &inst, &genc, &sed).iter().map(fmt))
}
