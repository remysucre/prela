use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::concepts::member_month::md5_hex;
use crate::schema::Db;
use harness::prelude::*;
use std::cmp::Ordering;

#[derive(Clone, Copy)]
pub struct AcuteInpatientGenerateEncounterId {
    pub patient_data_source_id: Str,
    pub claim_id: Str,
    pub start_date: Option<Date>,
    pub end_date: Option<Date>,
    pub discharge_disposition_code: Option<Str>,
    pub facility_npi: Option<Str>,
    pub encounter_claim_number: i64,
    pub encounter_claim_number_desc: i64,
    pub close_flag: i64,
    pub min_closing_row: Option<i64>,
    pub anchor_claim_id: Option<Str>,
    pub encounter_id: Str,
}

type S = EncountersStgMedicalClaim;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Base {
    pdsi: Str,
    claim_id: Str,
    start: Option<Date>,
    end: Option<Date>,
    npi: Option<Str>,
    ddc: Option<Str>,
}

#[derive(Clone, Copy)]
struct Row {
    b: Base,
    row_num: i64,
}

#[derive(Clone, Copy)]
struct Merge {
    pdsi: Str,
    claim_id_a: Str,
    row_num_a: i64,
    row_num_b: i64,
}

#[derive(Clone, Copy)]
struct Cf {
    r: Row,
    close_flag: i64,
}

#[derive(Clone, Copy)]
struct Amc {
    c: Cf,
    min_closing_row: Option<i64>,
}

#[derive(Clone, Copy)]
struct Enc {
    a: Amc,
    encounter_id: Option<Str>,
}

fn leak(s: String) -> Str {
    Box::leak(s.into_boxed_str())
}

fn part(s: &str) -> String {
    format!("V{}", s.replace('%', "%25").replace('|', "%7C"))
}

pub fn surrogate(kind: &str, pdsi: Str, enc: Option<Str>) -> Str {
    let e = enc.map(part).unwrap_or_else(|| "N".to_string());
    leak(md5_hex(format!("{}|{}|{}", part(kind), part(pdsi), e).as_bytes()))
}

pub fn asc_nl<T: Ord>(a: &Option<T>, b: &Option<T>) -> Ordering {
    match (a, b) {
        (Some(x), Some(y)) => x.cmp(y),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

pub fn desc_nl<T: Ord>(a: &Option<T>, b: &Option<T>) -> Ordering {
    match (a, b) {
        (Some(x), Some(y)) => y.cmp(x),
        _ => asc_nl(a, b),
    }
}

fn min_opt<T: Ord>(a: Option<T>, b: Option<T>) -> Option<T> {
    match (a, b) {
        (Some(x), Some(y)) => Some(x.min(y)),
        (x, None) => x,
        (None, y) => y,
    }
}

fn max_opt<T: Ord>(a: Option<T>, b: Option<T>) -> Option<T> {
    match (a, b) {
        (Some(x), Some(y)) => Some(x.max(y)),
        (x, None) => x,
        (None, y) => y,
    }
}

fn eq<T: PartialEq>(a: Option<T>, b: Option<T>) -> bool {
    matches!((a, b), (Some(x), Some(y)) if x == y)
}

fn merge_flag(a: Base, b: Base) -> i64 {
    let fac = eq(a.npi, b.npi);
    let d30 = a.ddc == Some("30");
    let c1 = eq(a.end, b.end) && fac;
    let c2 = eq(a.end.map(|e| e + DAY_US), b.start) && fac && d30;
    let c3 = eq(a.start, b.start) && fac && d30;
    let c4 = matches!((a.end, b.end), (Some(x), Some(y)) if x != y)
        && matches!((a.end, b.start), (Some(x), Some(y)) if x > y)
        && fac;
    (c1 || c2 || c3 || c4) as i64
}

type Key3 = (Option<Date>, Option<Date>, Str);

fn cmp3(a: &Key3, b: &Key3) -> Ordering {
    asc_nl(&a.0, &b.0).then(asc_nl(&a.1, &b.1)).then(a.2.cmp(b.2))
}

fn cmp3_desc(a: &Key3, b: &Key3) -> Ordering {
    desc_nl(&a.0, &b.0).then(desc_nl(&a.1, &b.1)).then(b.2.cmp(a.2))
}

pub fn acute_inpatient__generate_encounter_id(_db: &'static Db, stg: &[S]) -> Vec<AcuteInpatientGenerateEncounterId> {
    let m = rel(stg.to_vec());
    let cse = (&m)
        .map(|x: S| (x.claim_id, x.patient_data_source_id))
        .inv()
        .select(&m)
        .fold((None, None), |(s, e): (Option<Date>, Option<Date>), x: S| (min_opt(s, x.start_date), max_opt(e, x.end_date)));
    let base = (&m)
        .filt(|x: S| x.service_category_2 == "acute inpatient" && x.claim_type == "institutional")
        .and((&m).map(|x: S| (x.claim_id, x.patient_data_source_id)).select(&cse))
        .map(|(x, (s, e)): (S, (Option<Date>, Option<Date>))| Base {
            pdsi: x.patient_data_source_id,
            claim_id: x.claim_id,
            start: s,
            end: e,
            npi: x.facility_npi,
            ddc: x.discharge_disposition_code,
        })
        .inv()
        .map(|_| ())
        .fold((), |a, _| a);
    let base = rel(drain(&base).into_iter().map(|(b, _)| b).collect::<Vec<Base>>());

    let rn = (&base)
        .map(|b: Base| b.pdsi)
        .inv()
        .select(&base)
        .window(rank, |b: Base| (b.end, b.start, b.claim_id), cmp3);
    let arn = rel(drain(&rn).into_iter().map(|(_, (b, row_num))| Row { b, row_num }).collect::<Vec<Row>>());
    let by_p: HashIdx<Str, usize> = (&arn).map(|r: Row| r.b.pdsi).inv().collect();

    let merges = (&arn)
        .and((&arn).map(|r: Row| r.b.pdsi).select(&by_p).select(&arn))
        .filt(|(a, b): (Row, Row)| a.row_num < b.row_num && a.b.claim_id != b.b.claim_id)
        .filt(|(a, b): (Row, Row)| merge_flag(a.b, b.b) == 1)
        .map(|(a, b): (Row, Row)| Merge {
            pdsi: a.b.pdsi,
            claim_id_a: a.b.claim_id,
            row_num_a: a.row_num,
            row_num_b: b.row_num,
        });
    let mr = rel(drain(&merges).into_iter().map(|(_, x)| x).collect::<Vec<Merge>>());

    let bb = (&mr).map(|x: Merge| (x.pdsi, x.claim_id_a)).inv().map(|_| ()).fold((), |a, _| a);
    let mr_by_p: HashIdx<Str, usize> = (&mr).map(|x: Merge| x.pdsi).inv().collect();
    let cc = (&arn)
        .and((&arn).map(|r: Row| r.b.pdsi).select(&mr_by_p).select(&mr))
        .filt(|(a, x): (Row, Merge)| x.row_num_a < a.row_num && x.row_num_b > a.row_num)
        .map(|(a, _): (Row, Merge)| (a.b.pdsi, a.b.claim_id))
        .inv()
        .map(|_| ())
        .fold((), |a, _| a);

    let cf = (&arn)
        .and((&arn).map(|r: Row| (r.b.pdsi, r.b.claim_id)).select(&bb).opt())
        .and((&arn).map(|r: Row| (r.b.pdsi, r.b.claim_id)).select(&cc).opt())
        .map(|((r, b), c): ((Row, Option<()>), Option<()>)| Cf {
            r,
            close_flag: (b.is_none() && c.is_none()) as i64,
        });
    let cf = rel(drain(&cf).into_iter().map(|(_, x)| x).collect::<Vec<Cf>>());

    let closes_by_p: HashIdx<Str, usize> =
        (&cf).filt(|c: Cf| c.close_flag == 1).map(|c: Cf| c.r.b.pdsi).inv().collect();
    let later = (&cf)
        .and((&cf).map(|c: Cf| c.r.b.pdsi).select(&closes_by_p).select(&cf))
        .filt(|(a, b): (Cf, Cf)| a.r.row_num <= b.r.row_num)
        .map(|(_, b): (Cf, Cf)| b.r.row_num);
    let min_close = (&cf)
        .map(|c: Cf| (c.r.b.pdsi, c.r.b.claim_id))
        .inv()
        .select(&later)
        .fold(i64::MAX, |a, x: i64| a.min(x));

    let amc = (&cf)
        .and((&cf).map(|c: Cf| (c.r.b.pdsi, c.r.b.claim_id)).select(&min_close).opt())
        .map(|(c, m): (Cf, Option<i64>)| Amc { c, min_closing_row: m });
    let amc = rel(drain(&amc).into_iter().map(|(_, x)| x).collect::<Vec<Amc>>());
    let by_row: HashIdx<(Str, i64), usize> = (&amc).map(|a: Amc| (a.c.r.b.pdsi, a.c.r.row_num)).inv().collect();
    let enc = (&amc)
        .and(
            (&amc)
                .flat_map(|a: Amc| a.min_closing_row.map(|m| (a.c.r.b.pdsi, m)))
                .select(&by_row)
                .select(&amc)
                .opt(),
        )
        .map(|(a, b): (Amc, Option<Amc>)| Enc {
            a,
            encounter_id: b.map(|b| b.c.r.b.claim_id),
        });
    let enc = rel(drain(&enc).into_iter().map(|(_, x)| x).collect::<Vec<Enc>>());

    let key = |e: Enc| (e.a.c.r.b.start, e.a.c.r.b.end, e.a.c.r.b.claim_id);
    let w = (&enc)
        .map(|e: Enc| (e.a.c.r.b.pdsi, e.encounter_id))
        .inv()
        .select(&enc)
        .window(row_number, key, cmp3)
        .window(row_number, move |(e, _): (Enc, i64)| key(e), cmp3_desc);
    drain(&w)
        .into_iter()
        .map(|(_, ((e, n), nd))| {
            let b = e.a.c.r.b;
            AcuteInpatientGenerateEncounterId {
                patient_data_source_id: b.pdsi,
                claim_id: b.claim_id,
                start_date: b.start,
                end_date: b.end,
                discharge_disposition_code: b.ddc,
                facility_npi: b.npi,
                encounter_claim_number: n,
                encounter_claim_number_desc: nd,
                close_flag: e.a.c.close_flag,
                min_closing_row: e.a.min_closing_row,
                anchor_claim_id: e.encounter_id,
                encounter_id: surrogate("acute inpatient", b.pdsi, e.encounter_id),
            }
        })
        .collect()
}

pub fn fmt(v: &AcuteInpatientGenerateEncounterId) -> String {
    row(vec![
        V::S(v.patient_data_source_id),
        V::S(v.claim_id),
        odate(v.start_date),
        odate(v.end_date),
        ostr(v.discharge_disposition_code),
        ostr(v.facility_npi),
        V::I(v.encounter_claim_number),
        V::I(v.encounter_claim_number_desc),
        V::I(v.close_flag),
        oint(v.min_closing_row),
        ostr(v.anchor_claim_id),
        V::S(v.encounter_id),
    ])
}

pub fn stg_medical_claim(db: &'static Db) -> Vec<S> {
    let mc = crate::concepts::medical_claim::medical_claim(db);
    let il = crate::concepts::input_layer__medical_claim::input_layer__medical_claim(db, &mc);
    let stg = crate::concepts::normalized_input__stg_medical_claim::normalized_input__stg_medical_claim(db, &il);
    let nmc = crate::concepts::normalized__medical_claim::normalized__medical_claim_from_stg(db, &stg);
    let ne = crate::concepts::encounters__patient_data_source_id::normalized_eligibility(db);
    let pdsi = crate::concepts::encounters__patient_data_source_id::encounters__patient_data_source_id(db, &nmc, &ne);
    crate::concepts::encounters__stg_medical_claim::encounters__stg_medical_claim(db, &nmc, &pdsi)
}

pub fn q(db: &'static Db) -> String {
    let stg = stg_medical_claim(db);
    rows(acute_inpatient__generate_encounter_id(db, &stg).iter().map(fmt))
}
