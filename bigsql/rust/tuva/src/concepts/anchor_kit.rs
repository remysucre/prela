use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::concepts::member_month::md5_hex;
use crate::schema::Db;
use harness::prelude::*;

type M = EncountersStgMedicalClaim;

#[derive(Clone, Copy)]
pub struct ClaimAnchor {
    pub claim_id: Str,
    pub data_source: Str,
    pub tuva_last_run: Ts,
}

#[derive(Clone, Copy)]
pub struct ClaimEncounter {
    pub patient_data_source_id: Str,
    pub data_source: Str,
    pub start_date: Option<Date>,
    pub claim_id: Str,
    pub old_encounter_id: Str,
}

#[derive(Clone, Copy)]
pub struct ClaimMatch {
    pub patient_data_source_id: Str,
    pub data_source: Str,
    pub start_date: Date,
    pub claim_id: Str,
    pub claim_line_number: i64,
    pub old_encounter_id: Str,
}

pub fn leak(s: String) -> Str {
    Box::leak(s.into_boxed_str())
}

fn part(s: Option<&str>) -> String {
    match s {
        None => "N".to_string(),
        Some(s) => format!("V{}", s.replace('%', "%25").replace('|', "%7C")),
    }
}

pub fn surrogate_key(parts: &[Option<&str>]) -> Str {
    let s: Vec<String> = parts.iter().map(|p| part(*p)).collect();
    leak(md5_hex(s.join("|").as_bytes()))
}

pub fn date_text(d: Option<Date>) -> Option<String> {
    d.map(fmt_date)
}

pub fn stg(db: &'static Db) -> Vec<M> {
    let mc = crate::concepts::medical_claim::medical_claim(db);
    let il = crate::concepts::input_layer__medical_claim::input_layer__medical_claim(db, &mc);
    let s = crate::concepts::normalized_input__stg_medical_claim::normalized_input__stg_medical_claim(db, &il);
    let nmc = crate::concepts::normalized__medical_claim::normalized__medical_claim_from_stg(db, &s);
    let ne = crate::concepts::encounters__patient_data_source_id::normalized_eligibility(db);
    let pdsi = crate::concepts::encounters__patient_data_source_id::encounters__patient_data_source_id(db, &nmc, &ne);
    crate::concepts::encounters__stg_medical_claim::encounters__stg_medical_claim(db, &nmc, &pdsi)
}

pub fn claim_anchor_events(stg: &[M], keep: impl Fn(M) -> bool) -> Vec<ClaimAnchor> {
    let m = rel(stg.to_vec());
    let sc = (&m)
        .filt(keep)
        .map(|x: M| (x.claim_id, x.data_source, x.patient_data_source_id, x.start_date))
        .inv()
        .map(|_| ())
        .fold((), |a, _| a);
    let sc = rel(drain(&sc).into_iter().map(|(k, _)| k).collect());
    let run = ts(2026, 1, 1, 0, 0, 0);
    let u = (&sc)
        .map(|(c, d, _, _): (Str, Str, Str, Option<Date>)| (c, d))
        .inv()
        .map(|_| ())
        .fold((), |a, _| a);
    drain(&u)
        .into_iter()
        .map(|((claim_id, data_source), _)| ClaimAnchor { claim_id, data_source, tuva_last_run: run })
        .collect()
}

pub fn claim_generate_encounter_id(stg: &[M], anchors: &[ClaimAnchor], label: &str) -> Vec<ClaimEncounter> {
    let m = rel(stg.to_vec());
    let a = rel(anchors.to_vec());
    let ax: HashIdx<(Str, Str), usize> = (&a).map(|x: ClaimAnchor| (x.claim_id, x.data_source)).inv().collect();
    let anchor = (&m)
        .and((&m).map(|x: M| (x.claim_id, x.data_source)).select(&ax))
        .map(|(x, _): (M, usize)| (x.patient_data_source_id, x.data_source, x.start_date, x.claim_id))
        .inv()
        .map(|_| ())
        .fold((), |a, _| a);
    drain(&anchor)
        .into_iter()
        .map(|((patient_data_source_id, data_source, start_date, claim_id), _)| ClaimEncounter {
            patient_data_source_id,
            data_source,
            start_date,
            claim_id,
            old_encounter_id: surrogate_key(&[
                Some(label),
                Some(patient_data_source_id),
                date_text(start_date).as_deref(),
            ]),
        })
        .collect()
}

pub fn match_claims_to_anchor<G: Copy>(
    stg: &[M],
    enc: &[G],
    key: impl Fn(G) -> Option<(Str, Date)>,
    id: impl Fn(G) -> Str,
    sc2: Option<&'static str>,
) -> Vec<ClaimMatch> {
    let m = rel(stg.to_vec());
    let g = rel(enc.to_vec());
    let gx: HashIdx<(Str, Date), usize> = (&g).flat_map(key).inv().collect();
    let j = (&m)
        .filt(move |x: M| sc2.is_none_or(|s| x.service_category_2 == s))
        .and(
            (&m).flat_map(|x: M| x.start_date.map(|d| (x.patient_data_source_id, d)))
                .select(&gx)
                .select(&g),
        )
        .map(move |(x, u): (M, G)| {
            (x.patient_data_source_id, x.data_source, x.start_date.unwrap(), x.claim_id, x.claim_line_number, id(u))
        })
        .inv()
        .map(|_| ())
        .fold((), |a, _| a);
    drain(&j)
        .into_iter()
        .map(|((patient_data_source_id, data_source, start_date, claim_id, claim_line_number, old_encounter_id), _)| {
            ClaimMatch { patient_data_source_id, data_source, start_date, claim_id, claim_line_number, old_encounter_id }
        })
        .collect()
}

pub fn fmt_anchor(v: &ClaimAnchor) -> String {
    row(vec![V::S(v.claim_id), V::S(v.data_source), V::T(v.tuva_last_run)])
}

pub fn fmt_encounter(v: &ClaimEncounter) -> String {
    row(vec![
        V::S(v.patient_data_source_id),
        V::S(v.data_source),
        odate(v.start_date),
        V::S(v.claim_id),
        V::S(v.old_encounter_id),
    ])
}

pub fn fmt_match(v: &ClaimMatch) -> String {
    row(vec![
        V::S(v.patient_data_source_id),
        V::S(v.data_source),
        V::D(v.start_date),
        V::S(v.claim_id),
        V::I(v.claim_line_number),
        V::S(v.old_encounter_id),
    ])
}

pub fn claim_enc_key(x: ClaimEncounter) -> Option<(Str, Date)> {
    x.start_date.map(|d| (x.patient_data_source_id, d))
}

pub fn claim_enc_id(x: ClaimEncounter) -> Str {
    x.old_encounter_id
}
