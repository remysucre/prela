use crate::concepts::encounters__prof_and_lower_priority::EncountersProfAndLowerPriority as Plp;
use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim as Stg;
use crate::concepts::member_month::md5_hex;
use crate::schema::Db;
use harness::prelude::*;
use std::cmp::Reverse;

#[derive(Clone, Copy)]
pub struct GenerateEncounterId {
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

#[derive(Clone, Copy)]
pub struct StartEndDates {
    pub encounter_id: Str,
    pub anchor_claim_id: Option<Str>,
    pub encounter_start_date: Option<Date>,
    pub encounter_end_date: Option<Date>,
}

#[derive(Clone, Copy)]
pub struct ProfClaims {
    pub encounter_id: Str,
    pub encounter_start_date: Date,
    pub encounter_end_date: Date,
    pub claim_id: Str,
    pub claim_line_number: i64,
    pub data_source: Str,
    pub claim_attribution_number: i64,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Base {
    pdsi: Str,
    claim: Str,
    s: Option<Date>,
    e: Option<Date>,
    npi: Option<Str>,
    ddc: Option<Str>,
}

#[derive(Clone, Copy)]
struct Merge {
    pdsi: Str,
    claim_a: Str,
    rn_a: i64,
    rn_b: i64,
}

type A = (Base, i64);
type C = (A, i64);
type D = (C, Option<i64>);
type E = (D, Option<Str>);
type J = (Str, Str, Option<Str>, Option<Date>, Option<Date>);
type P = ((Str, i64, Str), (Str, Date, Date, Option<Str>));

fn leak(s: String) -> Str {
    Box::leak(s.into_boxed_str())
}

fn part(s: &str) -> String {
    format!("V{}", s.replace('%', "%25").replace('|', "%7C"))
}

fn omin<T: Ord>(a: Option<T>, b: Option<T>) -> Option<T> {
    match (a, b) {
        (Some(x), Some(y)) => Some(x.min(y)),
        (x, None) => x,
        (None, y) => y,
    }
}

fn omax<T: Ord>(a: Option<T>, b: Option<T>) -> Option<T> {
    match (a, b) {
        (Some(x), Some(y)) => Some(x.max(y)),
        (x, None) => x,
        (None, y) => y,
    }
}

fn nl<T>(o: Option<T>) -> (bool, Option<T>) {
    (o.is_none(), o)
}

fn nl_desc<T>(o: Option<T>) -> (bool, Reverse<Option<T>>) {
    (o.is_none(), Reverse(o))
}

fn cmp3<T: Copy, U: Copy>(a: Option<T>, b: Option<U>, f: impl Fn(T, U) -> bool) -> bool {
    match (a, b) {
        (Some(x), Some(y)) => f(x, y),
        _ => false,
    }
}

fn merge_flag(a: Base, b: Base) -> bool {
    let npi = cmp3(a.npi, b.npi, |x, y| x == y);
    let d30 = a.ddc == Some("30");
    (cmp3(a.e, b.e, |x, y| x == y) && npi)
        || (cmp3(a.e, b.s, |x, y| x + DAY_US == y) && npi && d30)
        || (cmp3(a.s, b.s, |x, y| x == y) && npi && d30)
        || (cmp3(a.e, b.e, |x, y| x != y) && cmp3(a.e, b.s, |x, y| x > y) && npi)
}

pub fn generate_encounter_id(stg: &[Stg], category: &'static str, label: &'static str) -> Vec<GenerateEncounterId> {
    let m = rel(stg.to_vec());
    let cse = (&m)
        .map(|x: Stg| (x.claim_id, x.patient_data_source_id))
        .inv()
        .select(&m)
        .fold((None, None), |(s, e): (Option<Date>, Option<Date>), x: Stg| (omin(s, x.start_date), omax(e, x.end_date)));
    let base_q = (&m)
        .filt(move |x: Stg| x.service_category_2 == category && x.claim_type == "institutional")
        .and((&m).map(|x: Stg| (x.claim_id, x.patient_data_source_id)).select(&cse))
        .map(|(x, (s, e)): (Stg, (Option<Date>, Option<Date>))| Base {
            pdsi: x.patient_data_source_id,
            claim: x.claim_id,
            s,
            e,
            npi: x.facility_npi,
            ddc: x.discharge_disposition_code,
        })
        .inv()
        .map(|_| ())
        .fold((), |a, _| a);
    let b = rel(drain(&base_q).into_iter().map(|(x, _)| x).collect::<Vec<Base>>());

    let rn = (&b)
        .map(|x: Base| x.pdsi)
        .inv()
        .select(&b)
        .window(rank, |x: Base| (nl(x.e), nl(x.s), x.claim), |p, q| p.cmp(q));
    let a = rel(drain(&rn).into_iter().map(|(_, r)| r).collect::<Vec<A>>());
    let a_by_p: HashIdx<Str, usize> = (&a).map(|(x, _): A| x.pdsi).inv().collect();

    let merges: Vec<Merge> = drain(
        &(&a)
            .and((&a).map(|(x, _): A| x.pdsi).select(&a_by_p).select(&a))
            .filt(|((x, rx), (y, ry)): (A, A)| rx < ry && x.claim != y.claim)
            .filt(|((x, _), (y, _)): (A, A)| merge_flag(x, y))
            .map(|((x, rx), (y, ry)): (A, A)| (x.pdsi, x.claim, y.claim, rx, ry)),
    )
    .into_iter()
    .map(|(_, (pdsi, claim_a, _claim_b, rn_a, rn_b))| Merge { pdsi, claim_a, rn_a, rn_b })
    .collect();
    let mg = rel(merges);
    let bb = (&mg).map(|x: Merge| (x.pdsi, x.claim_a)).inv().map(|_| ()).fold((), |a, _| a);
    let mg_by_p: HashIdx<Str, usize> = (&mg).map(|x: Merge| x.pdsi).inv().collect();
    let cc = (&a)
        .and((&a).map(|(x, _): A| x.pdsi).select(&mg_by_p).select(&mg))
        .filt(|((_, r), y): (A, Merge)| y.rn_a < r && y.rn_b > r)
        .map(|((x, _), _): (A, Merge)| (x.pdsi, x.claim))
        .inv()
        .map(|_| ())
        .fold((), |a, _| a);

    let cf = (&a)
        .and((&a).map(|(x, _): A| (x.pdsi, x.claim)).select(&bb).opt())
        .and((&a).map(|(x, _): A| (x.pdsi, x.claim)).select(&cc).opt())
        .map(|((x, b1), c1): ((A, Option<()>), Option<()>)| (x, if b1.is_none() && c1.is_none() { 1i64 } else { 0 }));
    let c = rel(drain(&cf).into_iter().map(|(_, r)| r).collect::<Vec<C>>());
    let c_by_p: HashIdx<Str, usize> = (&c).map(|((x, _), _): C| x.pdsi).inv().collect();

    let min_close = (&c)
        .map(|((x, _), _): C| (x.pdsi, x.claim))
        .inv()
        .select((&c).and((&c).map(|((x, _), _): C| x.pdsi).select(&c_by_p).select(&c)))
        .filt(|(((_, rx), _), ((_, ry), fy)): (C, C)| rx <= ry && fy == 1)
        .map(|(_, ((_, ry), _)): (C, C)| ry)
        .fold(i64::MAX, |acc: i64, r: i64| acc.min(r));
    let dq = (&c)
        .and((&c).map(|((x, _), _): C| (x.pdsi, x.claim)).select(&min_close).opt())
        .map(|(x, mcr): (C, Option<i64>)| (x, mcr));
    let d = rel(drain(&dq).into_iter().map(|(_, r)| r).collect::<Vec<D>>());
    let d_by_rn: HashIdx<(Str, i64), usize> = (&d).map(|(((x, r), _), _): D| (x.pdsi, r)).inv().collect();
    let eq = (&d).and(
        (&d)
            .flat_map(|(((x, _), _), mcr): D| mcr.map(|r| (x.pdsi, r)))
            .select(&d_by_rn)
            .select(&d)
            .map(|(((y, _), _), _): D| y.claim)
            .opt(),
    );
    let e = rel(drain(&eq).into_iter().map(|(_, r)| r).collect::<Vec<E>>());

    let w = (&e)
        .map(|((((x, _), _), _), enc): E| (x.pdsi, enc))
        .inv()
        .select(&e)
        .window(row_number, |((((x, _), _), _), _): E| (nl(x.s), nl(x.e), x.claim), |p, q| p.cmp(q))
        .window(
            row_number,
            |(((((x, _), _), _), _), _): (E, i64)| (nl_desc(x.s), nl_desc(x.e), Reverse(x.claim)),
            |p, q| p.cmp(q),
        );
    let lab = part(label);
    drain(&w)
        .into_iter()
        .map(|(_, ((((((x, _), close), mcr), enc), n1), n2))| GenerateEncounterId {
            patient_data_source_id: x.pdsi,
            claim_id: x.claim,
            start_date: x.s,
            end_date: x.e,
            discharge_disposition_code: x.ddc,
            facility_npi: x.npi,
            encounter_claim_number: n1,
            encounter_claim_number_desc: n2,
            close_flag: close,
            min_closing_row: mcr,
            anchor_claim_id: enc,
            encounter_id: leak(md5_hex(
                format!("{}|{}|{}", lab, part(x.pdsi), enc.map(part).unwrap_or_else(|| "N".to_string())).as_bytes(),
            )),
        })
        .collect()
}

pub fn start_end_dates(enc_ids: &[GenerateEncounterId]) -> Vec<StartEndDates> {
    let g = rel(enc_ids.to_vec());
    let f = (&g).map(|x: GenerateEncounterId| x.encounter_id).inv().select(&g).fold(
        (None, None, None),
        |(a, s, e): (Option<Str>, Option<Date>, Option<Date>), x: GenerateEncounterId| {
            (omin(a, x.anchor_claim_id), omin(s, x.start_date), omax(e, x.end_date))
        },
    );
    drain(&f)
        .into_iter()
        .map(|(encounter_id, (anchor_claim_id, encounter_start_date, encounter_end_date))| StartEndDates {
            encounter_id,
            anchor_claim_id,
            encounter_start_date,
            encounter_end_date,
        })
        .collect()
}

pub fn prof_claims(stg: &[Stg], plp: &[Plp], enc_ids: &[GenerateEncounterId], sed: &[StartEndDates]) -> Vec<ProfClaims> {
    let g = rel(enc_ids.to_vec());
    let sd = rel(sed.to_vec());
    let sd_by_enc: HashIdx<Str, usize> = (&sd).map(|x: StartEndDates| x.encounter_id).inv().collect();
    let jq = (&g)
        .filt(|x: GenerateEncounterId| x.encounter_claim_number == 1)
        .map(|x: GenerateEncounterId| (x.encounter_id, x.patient_data_source_id))
        .and((&g).map(|x: GenerateEncounterId| x.encounter_id).select(&sd_by_enc).select(&sd))
        .map(|((enc, pdsi), dat): ((Str, Str), StartEndDates)| (enc, pdsi, dat.anchor_claim_id, dat.encounter_end_date, dat.encounter_start_date));
    let j = rel(drain(&jq).into_iter().map(|(_, r)| r).collect::<Vec<J>>());
    let j_by_p: HashIdx<Str, usize> = (&j).map(|x: J| x.1).inv().collect();

    let m = rel(stg.to_vec());
    let p = rel(plp.to_vec());
    let p_idx: HashIdx<(Str, i64, Str), usize> =
        (&p).map(|x: Plp| (x.claim_id, x.claim_line_number, x.data_source)).inv().collect();
    let pq = (&m)
        .and((&m).map(|x: Stg| (x.claim_id, x.claim_line_number, x.data_source)).select(&p_idx).select(&p))
        .and((&m).map(|x: Stg| x.patient_data_source_id).select(&j_by_p).select(&j))
        .filt(|((x, _), (_, _, _, e, s)): ((Stg, Plp), J)| {
            matches!((x.start_date, s, e), (Some(d), Some(s), Some(e)) if d >= s && d <= e)
        })
        .map(|((x, _), (enc, _, anchor, e, s)): ((Stg, Plp), J)| {
            ((x.claim_id, x.claim_line_number, x.data_source), (enc, s.unwrap(), e.unwrap(), anchor))
        });
    let pr = rel(drain(&pq).into_iter().map(|(_, r)| r).collect::<Vec<P>>());
    let w = (&pr)
        .map(|(k, _): P| k)
        .inv()
        .select(&pr)
        .window(row_number, |(_, (_, _, _, anchor)): P| nl(anchor), |a, b| a.cmp(b));
    drain(&w)
        .into_iter()
        .map(|(_, (((claim_id, claim_line_number, data_source), (encounter_id, s, e, _)), n))| ProfClaims {
            encounter_id,
            encounter_start_date: s,
            encounter_end_date: e,
            claim_id,
            claim_line_number,
            data_source,
            claim_attribution_number: n,
        })
        .collect()
}

pub fn fmt_gen(v: &GenerateEncounterId) -> String {
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

pub fn fmt_sed(v: &StartEndDates) -> String {
    row(vec![V::S(v.encounter_id), ostr(v.anchor_claim_id), odate(v.encounter_start_date), odate(v.encounter_end_date)])
}

pub fn fmt_prof(v: &ProfClaims) -> String {
    row(vec![
        V::S(v.encounter_id),
        odate(Some(v.encounter_start_date)),
        odate(Some(v.encounter_end_date)),
        V::S(v.claim_id),
        V::I(v.claim_line_number),
        V::S(v.data_source),
        V::I(v.claim_attribution_number),
    ])
}

pub fn stg(db: &'static Db) -> Vec<Stg> {
    let mc = crate::concepts::medical_claim::medical_claim(db);
    let il = crate::concepts::input_layer__medical_claim::input_layer__medical_claim(db, &mc);
    let st = crate::concepts::normalized_input__stg_medical_claim::normalized_input__stg_medical_claim(db, &il);
    let nmc = crate::concepts::normalized__medical_claim::normalized__medical_claim_from_stg(db, &st);
    let ne = crate::concepts::encounters__patient_data_source_id::normalized_eligibility(db);
    let pdsi = crate::concepts::encounters__patient_data_source_id::encounters__patient_data_source_id(db, &nmc, &ne);
    crate::concepts::encounters__stg_medical_claim::encounters__stg_medical_claim(db, &nmc, &pdsi)
}

pub fn plp(db: &'static Db) -> Vec<Plp> {
    let prof = crate::concepts::encounters__stg_professional::encounters__stg_professional(db);
    crate::concepts::encounters__prof_and_lower_priority::encounters__prof_and_lower_priority(db, &prof)
}
