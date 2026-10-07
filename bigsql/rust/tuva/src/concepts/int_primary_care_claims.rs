use crate::concepts::member_month::MemberMonth;
use crate::concepts::int_provider_classification::IntProviderClassification;
use crate::schema::*;
use crate::concepts::stg_medical_claim::StgMedicalClaim;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct IntPrimaryCareClaims {
    pub person_id: Str,
    pub claim_id: Str,
    pub claim_line_number: i64,
    pub encounter_id: Str,
    pub claim_start_date: Date,
    pub claim_end_date: Option<Date>,
    pub claim_year: i64,
    pub claim_month: i64,
    pub claim_year_month: Str,
    pub claim_year_month_int: i64,
    pub allowed_amount: f64,
    pub provider_id: Str,
    pub hcpcs_code: Str,
    pub data_source: Str,
    pub provider_bucket: Str,
    pub prov_specialty: Option<Str>,
}

#[derive(Clone, Copy)]
pub struct ClaimMonth {
    pub person_id: Str,
    pub claim_id: Str,
    pub claim_line_number: i64,
    pub encounter_id: Str,
    pub claim_start_date: Date,
    pub claim_end_date: Option<Date>,
    pub claim_year_month: Str,
    pub claim_year_month_int: i64,
    pub allowed_amount: f64,
    pub provider_id: Option<Str>,
    pub hcpcs_code: Option<Str>,
    pub data_source: Str,
}

type S = StgMedicalClaim;
type C = IntProviderClassification;
type MmKey = (Str, Str, Str);

fn leak(s: String) -> Str {
    Box::leak(s.into_boxed_str())
}

pub fn member_months(mm: &[MemberMonth]) -> Vec<MmKey> {
    let m = rel(mm.to_vec());
    let g = (&m).map(|x: MemberMonth| (x.person_id, x.year_month, x.data_source)).inv().map(|_| ()).fold((), |a, _| a);
    drain(&g).into_iter().map(|(k, _)| k).collect()
}

pub fn eligible_claim_months(stg: &[S], mms: &[MmKey]) -> Vec<ClaimMonth> {
    let s = rel(stg.to_vec());
    let mr = rel(mms.to_vec());
    let mx: HashIdx<MmKey, usize> = (&mr).map(|k: MmKey| k).inv().collect();
    let cm = (&s).filt(|x: S| x.claim_start_date.is_some()).map(|x: S| {
        let d = x.claim_start_date.unwrap();
        let ymi = year(d) * 100 + month(d);
        ClaimMonth {
            person_id: x.person_id,
            claim_id: x.claim_id,
            claim_line_number: x.claim_line_number,
            encounter_id: x.encounter_id,
            claim_start_date: d,
            claim_end_date: x.claim_end_date,
            claim_year_month: leak(format!("{ymi:06}")),
            claim_year_month_int: ymi,
            allowed_amount: x.allowed_amount.filter(|&a| a != 0.0).or(x.paid_amount).unwrap_or(0.0),
            provider_id: x.rendering_npi,
            hcpcs_code: x.hcpcs_code,
            data_source: x.data_source,
        }
    });
    let cm = rel(drain(&cm).into_iter().map(|(_, r)| r).collect::<Vec<ClaimMonth>>());
    let e = (&cm).and((&cm).map(|c: ClaimMonth| (c.person_id, c.claim_year_month, c.data_source)).select(&mx)).map(|(c, _): (ClaimMonth, usize)| c);
    drain(&e).into_iter().map(|(_, r)| r).collect()
}

pub fn individual_providers(db: &'static Db) -> HashIdx<Str, Id<Provider>> {
    let p = &db.provider;
    (&p.id)
        .with((&p.entity_type_description).filt(|s: Str| s.trim_matches(' ').to_lowercase() == "individual"))
        .select(&p.npi)
        .inv()
        .collect()
}

pub fn int_primary_care_claims(db: &'static Db, stg: &[S], mm: &[MemberMonth], pc: &[C]) -> Vec<IntPrimaryCareClaims> {
    let mms = member_months(mm);
    let e = rel(eligible_claim_months(stg, &mms));
    let hx: HashIdx<Str, _> = (&db.primary_care_hcpcs_codes.hcpcs_code).inv().collect();
    let pr = rel(pc.to_vec());
    let px: HashIdx<Str, usize> = (&pr).map(|c: C| c.provider_id).inv().collect();
    let sp = individual_providers(db);
    let p = &db.provider;
    let base = (&e)
        .and((&e).flat_map(|c: ClaimMonth| c.hcpcs_code).select(&hx))
        .map(|(c, _)| c);
    let base = rel(drain(&base).into_iter().map(|(_, r)| r).collect::<Vec<ClaimMonth>>());
    let w = (&base)
        .and((&base).flat_map(|c: ClaimMonth| c.provider_id).select(&px).select(&pr).opt())
        .and((&base).flat_map(|c: ClaimMonth| c.provider_id).select(&sp).select((&p.primary_specialty_description).opt()))
        .map(|((c, pc), spec): ((ClaimMonth, Option<C>), Option<Str>)| IntPrimaryCareClaims {
            person_id: c.person_id,
            claim_id: c.claim_id,
            claim_line_number: c.claim_line_number,
            encounter_id: c.encounter_id,
            claim_start_date: c.claim_start_date,
            claim_end_date: c.claim_end_date,
            claim_year: year(c.claim_start_date),
            claim_month: month(c.claim_start_date),
            claim_year_month: c.claim_year_month,
            claim_year_month_int: c.claim_year_month_int,
            allowed_amount: c.allowed_amount,
            provider_id: c.provider_id.unwrap(),
            hcpcs_code: c.hcpcs_code.unwrap(),
            data_source: c.data_source,
            provider_bucket: pc.map_or("other_individual", |x| x.provider_bucket),
            prov_specialty: pc.and_then(|x| x.prov_specialty).or(spec),
        });
    drain(&w).into_iter().map(|(_, r)| r).collect()
}

pub fn fmt(v: &IntPrimaryCareClaims) -> String {
    row(vec![
        V::S(v.person_id),
        V::S(v.claim_id),
        V::I(v.claim_line_number),
        V::S(v.encounter_id),
        V::D(v.claim_start_date),
        odate(v.claim_end_date),
        V::I(v.claim_year),
        V::I(v.claim_month),
        V::S(v.claim_year_month),
        V::I(v.claim_year_month_int),
        V::F(v.allowed_amount),
        V::S(v.provider_id),
        V::S(v.hcpcs_code),
        V::S(v.data_source),
        V::S(v.provider_bucket),
        ostr(v.prov_specialty),
    ])
}

pub fn q(db: &'static Db) -> String {
    let u = crate::concepts::stg_medical_claim::upstream(db);
    let sp = crate::concepts::member_month__month_spine::member_month__month_spine(db);
    let mm = crate::concepts::member_month::member_month(db, &u.ne, &sp);
    let stg = crate::concepts::stg_medical_claim::stg_medical_claim(db, &u.nmc, &u.xw, &u.orph);
    let pc = crate::concepts::int_provider_classification::int_provider_classification(db);
    rows(int_primary_care_claims(db, &stg, &mm, &pc).iter().map(fmt))
}
