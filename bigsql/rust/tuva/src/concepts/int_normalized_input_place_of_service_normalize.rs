use crate::concepts::normalized_input__stg_medical_claim::NormalizedInputStgMedicalClaim;
use crate::schema::{Db, PlaceOfService};
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct IntNormalizedInputPlaceOfServiceNormalize {
    pub claim_id: Str,
    pub claim_line_number: i64,
    pub data_source: Str,
    pub normalized_code: Option<Str>,
    pub normalized_description: Option<Str>,
    pub tuva_last_run: Ts,
}

pub fn lpad<const N: usize>(s: Str, pad: char) -> [char; N] {
    let cs: Vec<char> = s.chars().collect();
    let mut out = [pad; N];
    if cs.len() >= N {
        out.copy_from_slice(&cs[..N]);
    } else {
        out[N - cs.len()..].copy_from_slice(&cs);
    }
    out
}

pub fn exact<const N: usize>(s: Str) -> Option<[char; N]> {
    let cs: Vec<char> = s.chars().collect();
    cs.try_into().ok()
}

pub fn int_normalized_input_place_of_service_normalize(
    db: &'static Db,
    stg: &[NormalizedInputStgMedicalClaim],
) -> Vec<IntNormalizedInputPlaceOfServiceNormalize> {
    let pos = &db.place_of_service;
    let med = rel(stg.to_vec());
    let by_code: HashIdx<[char; 2], Id<PlaceOfService>> = (&pos.place_of_service_code)
        .filt(|c: Str| exact::<2>(c).is_some())
        .map(|c: Str| exact::<2>(c).unwrap())
        .inv()
        .collect();
    let hit = (&med)
        .filt(|m: NormalizedInputStgMedicalClaim| m.place_of_service_code.is_some())
        .map(|m: NormalizedInputStgMedicalClaim| lpad::<2>(m.place_of_service_code.unwrap(), '0'))
        .select(&by_code)
        .select((&pos.place_of_service_code).and(&pos.place_of_service_description));
    let out = (&med)
        .filt(|m: NormalizedInputStgMedicalClaim| m.claim_type == "professional")
        .map(|m: NormalizedInputStgMedicalClaim| ((m.claim_id, m.claim_line_number), m.data_source))
        .and((&hit).opt());
    let run = ts(2026, 1, 1, 0, 0, 0);
    drain(&out)
        .into_iter()
        .map(|(_, (((claim_id, claim_line_number), data_source), h))| IntNormalizedInputPlaceOfServiceNormalize {
            claim_id,
            claim_line_number,
            data_source,
            normalized_code: h.map(|x: (Str, Str)| x.0),
            normalized_description: h.map(|x: (Str, Str)| x.1),
            tuva_last_run: run,
        })
        .collect()
}

pub fn fmt(v: &IntNormalizedInputPlaceOfServiceNormalize) -> String {
    row(vec![
        V::S(v.claim_id),
        V::I(v.claim_line_number),
        V::S(v.data_source),
        ostr(v.normalized_code),
        ostr(v.normalized_description),
        V::T(v.tuva_last_run),
    ])
}

pub fn q(db: &'static Db) -> String {
    let mc = crate::concepts::medical_claim::medical_claim(db);
    let il = crate::concepts::input_layer__medical_claim::input_layer__medical_claim(db, &mc);
    let stg = crate::concepts::normalized_input__stg_medical_claim::normalized_input__stg_medical_claim(db, &il);
    rows(int_normalized_input_place_of_service_normalize(db, &stg).iter().map(fmt))
}
