use crate::concepts::normalized__eligibility::NormalizedEligibility;
use crate::concepts::normalized__medical_claim::NormalizedMedicalClaim;
use crate::concepts::encounters__combined_claim_line_crosswalk::EncountersCombinedClaimLineCrosswalk;
use crate::concepts::encounters__orphaned_claims::EncountersOrphanedClaims;
use crate::schema::*;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct StgMedicalClaim {
    pub claim_id: Str,
    pub claim_line_number: i64,
    pub person_id: Str,
    pub claim_start_date: Option<Date>,
    pub claim_end_date: Option<Date>,
    pub allowed_amount: Option<f64>,
    pub paid_amount: Option<f64>,
    pub rendering_npi: Option<Str>,
    pub hcpcs_code: Option<Str>,
    pub data_source: Str,
    pub encounter_id: Str,
}

type M = NormalizedMedicalClaim;
type X = EncountersCombinedClaimLineCrosswalk;
type O = EncountersOrphanedClaims;
type K = ((Str, i64), Str);
type Enc = (K, Str);

pub fn stg_medical_claim(db: &'static Db, nmc: &[M], xw: &[X], orph: &[O]) -> Vec<StgMedicalClaim> {
    let xr = rel(xw.to_vec());
    let or = rel(orph.to_vec());
    let a = (&xr)
        .filt(|e: X| e.claim_line_attribution_number == 1)
        .map(|e: X| -> Enc { (((e.claim_id, e.claim_line_number), e.data_source), e.encounter_id) });
    let b = (&or).map(|e: O| -> Enc { (((e.claim_id, e.claim_line_number), e.data_source), e.encounter_id) });
    let all = rel(drain(a.union(b)).into_iter().map(|(_, r)| r).collect::<Vec<Enc>>());
    let ax: HashIdx<K, usize> = (&all).map(|e: Enc| e.0).inv().collect();
    let g = &db.service_category_grouper;
    let gx: HashIdx<K, _> = (&g.id)
        .with((&g.duplicate_row_number).filt(|x: i64| x == 1))
        .select((&g.claim_id).and(&g.claim_line_number).and(&g.data_source))
        .inv()
        .collect();
    let m = rel(nmc.to_vec());
    let key = |x: M| -> K { ((x.claim_id, x.claim_line_number), x.data_source) };
    let j = (&m)
        .and((&m).map(key).select(&gx))
        .and((&m).map(key).select(&ax).select(&all))
        .map(|((x, _), (_, enc)): ((M, _), Enc)| StgMedicalClaim {
            claim_id: x.claim_id,
            claim_line_number: x.claim_line_number,
            person_id: x.person_id,
            claim_start_date: x.claim_start_date,
            claim_end_date: x.claim_end_date,
            allowed_amount: x.allowed_amount,
            paid_amount: x.paid_amount,
            rendering_npi: x.rendering_npi,
            hcpcs_code: x.hcpcs_code,
            data_source: x.data_source,
            encounter_id: enc,
        });
    drain(&j).into_iter().map(|(_, r)| r).collect()
}

pub fn fmt(v: &StgMedicalClaim) -> String {
    row(vec![
        V::S(v.claim_id),
        V::I(v.claim_line_number),
        V::S(v.person_id),
        odate(v.claim_start_date),
        odate(v.claim_end_date),
        ofloat(v.allowed_amount),
        ofloat(v.paid_amount),
        ostr(v.rendering_npi),
        ostr(v.hcpcs_code),
        V::S(v.data_source),
        V::S(v.encounter_id),
    ])
}

pub struct Upstream {
    pub nmc: Vec<M>,
    pub ne: Vec<NormalizedEligibility>,
    pub xw: Vec<X>,
    pub orph: Vec<O>,
}

pub fn upstream(db: &'static Db) -> Upstream {
    let mc = crate::concepts::medical_claim::medical_claim(db);
    let il = crate::concepts::input_layer__medical_claim::input_layer__medical_claim(db, &mc);
    let s = crate::concepts::normalized_input__stg_medical_claim::normalized_input__stg_medical_claim(db, &il);
    let nmc = crate::concepts::normalized__medical_claim::normalized__medical_claim_from_stg(db, &s);
    let ne = crate::concepts::encounters__patient_data_source_id::normalized_eligibility(db);
    let pdsi = crate::concepts::encounters__patient_data_source_id::encounters__patient_data_source_id(db, &nmc, &ne);
    let stg = crate::concepts::encounters__stg_medical_claim::encounters__stg_medical_claim(db, &nmc, &pdsi);
    let xw = crate::concepts::encounters__combined_claim_line_crosswalk::chain(db, &stg);
    let orph = crate::concepts::encounters__orphaned_claims::encounters__orphaned_claims(db, &stg, &xw);
    Upstream { nmc, ne, xw, orph }
}

pub fn q(db: &'static Db) -> String {
    let u = upstream(db);
    rows(stg_medical_claim(db, &u.nmc, &u.xw, &u.orph).iter().map(fmt))
}
