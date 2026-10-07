use crate::concepts::normalized_input__stg_medical_claim::NormalizedInputStgMedicalClaim;
use crate::schema::{Db, Provider};
use harness::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Name {
    Person(Str, Str),
    Org(Str),
}

#[derive(Clone, Copy)]
pub struct IntNormalizedInputMedicalNpiNormalize {
    pub claim_id: Str,
    pub claim_line_number: i64,
    pub claim_type: Str,
    pub data_source: Str,
    pub normalized_rendering_npi: Option<Str>,
    pub normalized_rendering_name: Option<Name>,
    pub normalized_billing_npi: Option<Str>,
    pub normalized_billing_name: Option<Name>,
    pub normalized_facility_npi: Option<Str>,
    pub normalized_facility_name: Option<Name>,
    pub tuva_last_run: Ts,
}

type Prov = (Str, Option<Name>);
type PRow = ((((Str, Option<Str>), Option<Str>), Option<Str>), Option<Str>);
type Base = (((Str, i64), (Str, Str)), ((Option<Prov>, Option<Prov>), Option<Prov>));

fn name(((((npi, etc), last), first), org): PRow) -> Prov {
    let n = if etc == Some("1") {
        match (last, first) {
            (Some(l), Some(f)) => Some(Name::Person(l, f)),
            _ => None,
        }
    } else {
        org.map(Name::Org)
    };
    (npi, n)
}

fn fmt_name(n: Option<Name>) -> V {
    match n {
        Some(Name::Person(l, f)) => V::Owned(format!("{l}, {f}")),
        Some(Name::Org(o)) => V::S(o),
        None => V::Null,
    }
}

pub fn int_normalized_input_medical_npi_normalize(
    db: &'static Db,
    stg: &[NormalizedInputStgMedicalClaim],
) -> Vec<IntNormalizedInputMedicalNpiNormalize> {
    let p = &db.provider;
    let med = rel(stg.to_vec());
    let by_npi: HashIdx<Str, Id<Provider>> = (&p.npi).inv().collect();
    let prov = (&p.npi)
        .and((&p.entity_type_code).opt())
        .and((&p.provider_last_name).opt())
        .and((&p.provider_first_name).opt())
        .and((&p.provider_organization_name).opt());
    let rend = (&med)
        .map(|m: NormalizedInputStgMedicalClaim| m.rendering_npi)
        .select(&by_npi)
        .select(&prov)
        .map(name);
    let bill = (&med)
        .filt(|m: NormalizedInputStgMedicalClaim| m.billing_npi.is_some())
        .map(|m: NormalizedInputStgMedicalClaim| m.billing_npi.unwrap())
        .select(&by_npi)
        .select(&prov)
        .map(name);
    let fac = (&med)
        .filt(|m: NormalizedInputStgMedicalClaim| m.claim_type == "institutional" && m.facility_npi.is_some())
        .map(|m: NormalizedInputStgMedicalClaim| m.facility_npi.unwrap())
        .select(&by_npi)
        .with((&p.entity_type_description).filt(|d: Str| d == "Organization"))
        .select(&prov)
        .map(name);
    let base_q = (&med)
        .map(|m: NormalizedInputStgMedicalClaim| ((m.claim_id, m.claim_line_number), (m.claim_type, m.data_source)))
        .and((&rend).opt().and((&bill).opt()).and((&fac).opt()))
        .inv()
        .map(|_| ())
        .fold((), |a, _| a);
    let base = rel(drain(&base_q).into_iter().map(|(b, _)| b).collect::<Vec<Base>>());
    let ranked = (&base)
        .filt(|(_, (_, f)): Base| f.is_some())
        .map(|(((c, _), (_, d)), _): Base| (d, c))
        .inv()
        .select(&base)
        .window(row_number, |(((_, l), _), _): Base| l, |a: &i64, b: &i64| a.cmp(b));
    let first: HashIdx<(Str, Str), Prov> = (&ranked)
        .filt(|(_, rn): (Base, i64)| rn == 1)
        .map(|((_, (_, f)), _): (Base, i64)| f.unwrap())
        .collect();
    let out = (&base).and((&base).map(|(((c, _), (_, d)), _): Base| (d, c)).select(&first).opt());
    let run = ts(2026, 1, 1, 0, 0, 0);
    drain(&out)
        .into_iter()
        .map(|(_, ((((claim_id, claim_line_number), (claim_type, data_source)), ((r, b), _)), f))| {
            IntNormalizedInputMedicalNpiNormalize {
                claim_id,
                claim_line_number,
                claim_type,
                data_source,
                normalized_rendering_npi: r.map(|x| x.0),
                normalized_rendering_name: r.and_then(|x| x.1),
                normalized_billing_npi: b.map(|x| x.0),
                normalized_billing_name: b.and_then(|x| x.1),
                normalized_facility_npi: f.map(|x: Prov| x.0),
                normalized_facility_name: f.and_then(|x: Prov| x.1),
                tuva_last_run: run,
            }
        })
        .collect()
}

pub fn fmt(v: &IntNormalizedInputMedicalNpiNormalize) -> String {
    row(vec![
        V::S(v.claim_id),
        V::I(v.claim_line_number),
        V::S(v.claim_type),
        V::S(v.data_source),
        ostr(v.normalized_rendering_npi),
        fmt_name(v.normalized_rendering_name),
        ostr(v.normalized_billing_npi),
        fmt_name(v.normalized_billing_name),
        ostr(v.normalized_facility_npi),
        fmt_name(v.normalized_facility_name),
        V::T(v.tuva_last_run),
    ])
}

pub fn q(db: &'static Db) -> String {
    let mc = crate::concepts::medical_claim::medical_claim(db);
    let il = crate::concepts::input_layer__medical_claim::input_layer__medical_claim(db, &mc);
    let stg = crate::concepts::normalized_input__stg_medical_claim::normalized_input__stg_medical_claim(db, &il);
    rows(int_normalized_input_medical_npi_normalize(db, &stg).iter().map(fmt))
}
