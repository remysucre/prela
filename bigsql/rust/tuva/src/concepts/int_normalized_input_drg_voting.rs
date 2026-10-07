use crate::concepts::normalized_input__stg_medical_claim::NormalizedInputStgMedicalClaim;
use crate::schema::{AprDrg, Db, MsDrg};
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct IntNormalizedInputDrgVoting {
    pub claim_id: Str,
    pub data_source: Str,
    pub column_name: Str,
    pub normalized_code: Option<Str>,
    pub normalized_description: Option<Str>,
    pub occurrence_count: i64,
    pub next_occurrence_count: i64,
    pub occurrence_row_count: i64,
    pub tuva_last_run: Ts,
}

type Dc = (((Str, Str), (Option<Str>, Option<Str>)), i64);

fn vote(g: &[(i64, Dc)], out: &mut Vec<(i64, i64)>) {
    for i in 0..g.len() {
        out.push((g.get(i + 1).map_or(0, |n| n.0), i as i64 + 1));
    }
}

pub fn int_normalized_input_drg_voting(
    db: &'static Db,
    stg: &[NormalizedInputStgMedicalClaim],
) -> Vec<IntNormalizedInputDrgVoting> {
    let ms = &db.ms_drg;
    let apr = &db.apr_drg;
    let med = rel(stg.to_vec());
    let ms_by: HashIdx<Str, Id<MsDrg>> = (&ms.ms_drg_code).inv().collect();
    let apr_by: HashIdx<Str, Id<AprDrg>> = (&apr.apr_drg_code).inv().collect();
    let ms_hit = (&med)
        .filt(|m: NormalizedInputStgMedicalClaim| m.drg_code_type == "ms-drg" && m.drg_code.is_some())
        .map(|m: NormalizedInputStgMedicalClaim| m.drg_code.unwrap())
        .select(&ms_by)
        .select((&ms.ms_drg_code).and(&ms.ms_drg_description));
    let apr_hit = (&med)
        .filt(|m: NormalizedInputStgMedicalClaim| m.drg_code_type == "apr-drg" && m.drg_code.is_some())
        .map(|m: NormalizedInputStgMedicalClaim| m.drg_code.unwrap())
        .select(&apr_by)
        .select((&apr.apr_drg_code).and(&apr.apr_drg_description));
    let normalize = (&med)
        .filt(|m: NormalizedInputStgMedicalClaim| m.claim_type == "institutional")
        .map(|m: NormalizedInputStgMedicalClaim| (m.claim_id, m.data_source))
        .and((&ms_hit).opt())
        .and((&apr_hit).opt())
        .map(|((k, m), a): (((Str, Str), Option<(Str, Str)>), Option<(Str, Str)>)| {
            (k, (m.map(|x| x.0).or(a.map(|x| x.0)), m.map(|x| x.1).or(a.map(|x| x.1))))
        })
        .filt(|(_, (code, _)): ((Str, Str), (Option<Str>, Option<Str>))| code.is_some());
    let counts = (&normalize).inv().fold(0i64, |a, _| a + 1);
    let dc = rel(drain(&counts));
    let ranked = (&dc)
        .map(|((k, _), _): Dc| k)
        .inv()
        .select(&dc)
        .window(vote, |(_, n): Dc| n, |a: &i64, b: &i64| b.cmp(a));
    let run = ts(2026, 1, 1, 0, 0, 0);
    drain(&ranked)
        .into_iter()
        .map(|((claim_id, data_source), ((((_, _), (code, desc)), n), (next, rn)))| {
            IntNormalizedInputDrgVoting {
                claim_id,
                data_source,
                column_name: "drg_code",
                normalized_code: code,
                normalized_description: desc,
                occurrence_count: n,
                next_occurrence_count: next,
                occurrence_row_count: rn,
                tuva_last_run: run,
            }
        })
        .collect()
}

pub fn fmt(v: &IntNormalizedInputDrgVoting) -> String {
    row(vec![
        V::S(v.claim_id),
        V::S(v.data_source),
        V::S(v.column_name),
        ostr(v.normalized_code),
        ostr(v.normalized_description),
        V::I(v.occurrence_count),
        V::I(v.next_occurrence_count),
        V::I(v.occurrence_row_count),
        V::T(v.tuva_last_run),
    ])
}

pub fn q(db: &'static Db) -> String {
    let mc = crate::concepts::medical_claim::medical_claim(db);
    let il = crate::concepts::input_layer__medical_claim::input_layer__medical_claim(db, &mc);
    let stg = crate::concepts::normalized_input__stg_medical_claim::normalized_input__stg_medical_claim(db, &il);
    rows(int_normalized_input_drg_voting(db, &stg).iter().map(fmt))
}
