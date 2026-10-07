use crate::schema::*;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct IntProviderClassification {
    pub provider_id: Str,
    pub prov_specialty: Option<Str>,
    pub provider_bucket: Str,
}

type P = IntProviderClassification;

fn right2(s: Str) -> (char, char) {
    let c: Vec<char> = format!("00{}", s.trim_matches(' ')).chars().collect();
    (c[c.len() - 2], c[c.len() - 1])
}

fn yes(s: Option<Str>) -> bool {
    s.is_some_and(|v| v.to_lowercase() == "yes")
}

pub fn int_provider_classification(db: &'static Db) -> Vec<P> {
    let p = &db.provider;
    let x = &db.medicare_provider_and_supplier_taxonomy_crosswalk;
    let a = &db.provider_specialty_assignment_codes;
    let xi: HashIdx<Str, _> = (&x.provider_taxonomy_code).map(|s: Str| s.trim_matches(' ')).inv().collect();
    let ai: HashIdx<(char, char), _> = (&a.specialty_code).map(right2).inv().collect();
    let bucket = (&a.primary_care_physician_step1).opt().and((&a.specialist_physician_step_2).opt()).and(&a.physician).map(
        |((pcp, spec), phys): ((Option<Str>, Option<Str>), i64)| {
            if yes(pcp) && phys == 1 {
                "pcp"
            } else if yes(spec) && phys == 1 {
                "specialist"
            } else if phys == 0 {
                "npp"
            } else {
                "unknown"
            }
        },
    );
    let mapped = (&p.id)
        .with((&p.entity_type_description).filt(|s: Str| s.trim_matches(' ').to_lowercase() == "individual"))
        .select(&p.npi)
        .and((&p.primary_specialty_description).opt())
        .and(
            (&p.primary_taxonomy_code)
                .map(|s: Str| s.trim_matches(' '))
                .select(&xi)
                .select(&x.medicare_specialty_code)
                .map(right2)
                .select(&ai)
                .select(&bucket),
        )
        .map(|((npi, spec), b): ((Str, Option<Str>), Str)| (npi, spec, b));
    let m = rel(drain(&mapped).into_iter().map(|(_, r)| r).collect::<Vec<_>>());
    let prio = |b: Str| match b {
        "pcp" => 1,
        "npp" => 2,
        "specialist" => 3,
        _ => 4,
    };
    let w = (&m)
        .map(|r: (Str, Option<Str>, Str)| r.0)
        .inv()
        .select(&m)
        .window(row_number, |r: (Str, Option<Str>, Str)| prio(r.2), |a: &i64, b: &i64| a.cmp(b));
    drain(&w)
        .into_iter()
        .filter(|(_, (_, rn))| *rn == 1)
        .map(|(_, ((provider_id, prov_specialty, provider_bucket), _))| P { provider_id, prov_specialty, provider_bucket })
        .collect()
}

pub fn fmt(v: &P) -> String {
    row(vec![V::S(v.provider_id), ostr(v.prov_specialty), V::S(v.provider_bucket)])
}

pub fn q(db: &'static Db) -> String {
    rows(int_provider_classification(db).iter().map(fmt))
}
