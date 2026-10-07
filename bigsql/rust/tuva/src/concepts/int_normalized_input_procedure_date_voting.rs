use crate::concepts::int_normalized_input_procedure_date_normalize::IntNormalizedInputProcedureDateNormalize;
use crate::schema::Db;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct IntNormalizedInputProcedureDateVoting {
    pub claim_id: Str,
    pub data_source: Str,
    pub column_name: Str,
    pub normalized_code: Option<Date>,
    pub occurrence_count: i64,
    pub next_occurrence_count: i64,
    pub occurrence_row_count: i64,
    pub tuva_last_run: Ts,
}

type Norm = IntNormalizedInputProcedureDateNormalize;
type K3 = ((Str, Str), Str);
type J = (Norm, i64);

fn k3(n: Norm) -> K3 {
    ((n.claim_id, n.data_source), n.procedure_column)
}

fn vote(g: &[(i64, J)], out: &mut Vec<(i64, i64)>) {
    for i in 0..g.len() {
        out.push((g.get(i + 1).map_or(0, |n| n.0), i as i64 + 1));
    }
}

pub fn int_normalized_input_procedure_date_voting(_db: &'static Db, norm: &[Norm]) -> Vec<IntNormalizedInputProcedureDateVoting> {
    let nr = rel(norm.to_vec());
    let distinct_count = (&nr).map(k3).inv().fold(0i64, |a, _| a + 1);
    let joined = rel(drain((&nr).and((&nr).map(k3).select(&distinct_count))).into_iter().map(|(_, j)| j).collect::<Vec<J>>());
    let ranked = (&joined)
        .map(|(n, _): J| k3(n))
        .inv()
        .select(&joined)
        .window(vote, |(n, _): J| n.procedure_date_occurrence_count, |a: &i64, b: &i64| b.cmp(a));
    let run = ts(2026, 1, 1, 0, 0, 0);
    drain(&ranked)
        .into_iter()
        .map(|(_, ((n, _), (next, rn)))| IntNormalizedInputProcedureDateVoting {
            claim_id: n.claim_id,
            data_source: n.data_source,
            column_name: n.procedure_column,
            normalized_code: n.procedure_date,
            occurrence_count: n.procedure_date_occurrence_count,
            next_occurrence_count: next,
            occurrence_row_count: rn,
            tuva_last_run: run,
        })
        .collect()
}

pub fn fmt(v: &IntNormalizedInputProcedureDateVoting) -> String {
    row(vec![
        V::S(v.claim_id),
        V::S(v.data_source),
        V::S(v.column_name),
        odate(v.normalized_code),
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
    let norm = crate::concepts::int_normalized_input_procedure_date_normalize::int_normalized_input_procedure_date_normalize(db, &stg);
    rows(int_normalized_input_procedure_date_voting(db, &norm).iter().map(fmt))
}
