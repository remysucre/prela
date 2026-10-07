use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::concepts::member_month::md5_hex;
use crate::schema::Db;
use harness::prelude::*;

type M = EncountersStgMedicalClaim;

pub fn leak(s: String) -> Str {
    Box::leak(s.into_boxed_str())
}

pub fn part(s: Option<&str>) -> String {
    match s {
        Some(s) => format!("V{}", s.replace('%', "%25").replace('|', "%7C")),
        None => "N".to_string(),
    }
}

pub fn surrogate(parts: &[Option<&str>]) -> Str {
    let s: Vec<String> = parts.iter().map(|p| part(*p)).collect();
    leak(md5_hex(s.join("|").as_bytes()))
}

pub fn date_text(d: Option<Date>) -> Option<String> {
    d.map(fmt_date)
}

pub fn stg(db: &'static Db) -> Vec<M> {
    let mc = crate::concepts::medical_claim::medical_claim(db);
    let il = crate::concepts::input_layer__medical_claim::input_layer__medical_claim(db, &mc);
    let st = crate::concepts::normalized_input__stg_medical_claim::normalized_input__stg_medical_claim(db, &il);
    let nmc = crate::concepts::normalized__medical_claim::normalized__medical_claim_from_stg(db, &st);
    let ne = crate::concepts::encounters__patient_data_source_id::normalized_eligibility(db);
    let pdsi = crate::concepts::encounters__patient_data_source_id::encounters__patient_data_source_id(db, &nmc, &ne);
    crate::concepts::encounters__stg_medical_claim::encounters__stg_medical_claim(db, &nmc, &pdsi)
}

#[derive(Clone, Copy)]
pub struct AnchorEvent {
    pub claim_id: Str,
    pub data_source: Str,
    pub tuva_last_run: Ts,
}

pub fn anchor_events(stg: &[M], sc2: &'static str, with_end: bool) -> Vec<AnchorEvent> {
    let m = rel(stg.to_vec());
    let sc = (&m)
        .filt(move |x: M| x.service_category_2 == sc2)
        .map(move |x: M| (x.claim_id, x.data_source, x.patient_data_source_id, x.start_date, if with_end { x.end_date } else { None }))
        .inv()
        .map(|_| ())
        .fold((), |a, _| a);
    let sc = rel(drain(&sc).into_iter().map(|(k, _)| k).collect::<Vec<_>>());
    let d = (&sc).map(|(c, ds, _, _, _)| (c, ds)).inv().map(|_| ()).fold((), |a, _| a);
    let run = ts(2026, 1, 1, 0, 0, 0);
    drain(&d)
        .into_iter()
        .map(|((claim_id, data_source), _)| AnchorEvent { claim_id, data_source, tuva_last_run: run })
        .collect()
}

pub fn fmt_anchor(v: &AnchorEvent) -> String {
    row(vec![V::S(v.claim_id), V::S(v.data_source), V::T(v.tuva_last_run)])
}

#[derive(Clone, Copy)]
pub struct GenEnc {
    pub patient_data_source_id: Str,
    pub data_source: Str,
    pub start_date: Option<Date>,
    pub claim_id: Str,
    pub old_encounter_id: Str,
}

pub fn generate_encounter_id(stg: &[M], anchors: &[(Str, Str)], label: &'static str) -> Vec<GenEnc> {
    let m = rel(stg.to_vec());
    let u = rel(anchors.to_vec());
    let ux: HashIdx<(Str, Str), usize> = (&u).inv().collect();
    let a = (&m)
        .and((&m).map(|x: M| (x.claim_id, x.data_source)).select(&ux))
        .map(|(x, _): (M, usize)| (x.patient_data_source_id, x.data_source, x.start_date, x.claim_id))
        .inv()
        .map(|_| ())
        .fold((), |a, _| a);
    drain(&a)
        .into_iter()
        .map(|((p, ds, sd, c), _)| GenEnc {
            patient_data_source_id: p,
            data_source: ds,
            start_date: sd,
            claim_id: c,
            old_encounter_id: surrogate(&[Some(label), Some(p), date_text(sd).as_deref()]),
        })
        .collect()
}

pub fn fmt_gen(v: &GenEnc) -> String {
    row(vec![
        V::S(v.patient_data_source_id),
        V::S(v.data_source),
        odate(v.start_date),
        V::S(v.claim_id),
        V::S(v.old_encounter_id),
    ])
}

#[derive(Clone, Copy)]
pub struct MatchClaim {
    pub patient_data_source_id: Str,
    pub data_source: Str,
    pub start_date: Date,
    pub claim_id: Str,
    pub claim_line_number: i64,
    pub old_encounter_id: Str,
}

pub fn match_claims_to_anchor(stg: &[M], gen_: &[GenEnc]) -> Vec<MatchClaim> {
    let m = rel(stg.to_vec());
    let g = rel(gen_.to_vec());
    let gx: HashIdx<(Str, Date), usize> =
        (&g).flat_map(|x: GenEnc| x.start_date.map(|s| (x.patient_data_source_id, s))).inv().collect();
    let j = (&m)
        .and((&m).flat_map(|x: M| x.start_date.map(|s| (x.patient_data_source_id, s))).select(&gx).select(&g))
        .map(|(x, u): (M, GenEnc)| {
            (x.patient_data_source_id, x.data_source, x.start_date.unwrap_or_default(), x.claim_id, x.claim_line_number, u.old_encounter_id)
        })
        .inv()
        .map(|_| ())
        .fold((), |a, _| a);
    drain(&j)
        .into_iter()
        .map(|((p, ds, sd, c, cl, e), _)| MatchClaim {
            patient_data_source_id: p,
            data_source: ds,
            start_date: sd,
            claim_id: c,
            claim_line_number: cl,
            old_encounter_id: e,
        })
        .collect()
}

pub fn fmt_match(v: &MatchClaim) -> String {
    row(vec![
        V::S(v.patient_data_source_id),
        V::S(v.data_source),
        V::D(v.start_date),
        V::S(v.claim_id),
        V::I(v.claim_line_number),
        V::S(v.old_encounter_id),
    ])
}

pub fn anchor_pairs(a: &[AnchorEvent]) -> Vec<(Str, Str)> {
    a.iter().map(|x| (x.claim_id, x.data_source)).collect()
}

pub fn q_anchor(db: &'static Db, sc2: &'static str) -> String {
    let s = stg(db);
    rows(anchor_events(&s, sc2, false).iter().map(fmt_anchor))
}
