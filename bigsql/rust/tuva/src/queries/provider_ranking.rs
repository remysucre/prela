use crate::concepts::member_month::MemberMonth;
use crate::concepts::member_month__month_spine::MemberMonthMonthSpine;
use crate::concepts::int_person_years::IntPersonYears;
use crate::concepts::int_primary_care_claims::{ClaimMonth, IntPrimaryCareClaims};
use crate::concepts::int_provider_classification::IntProviderClassification;
use crate::schema::*;
use crate::concepts::stg_medical_claim::StgMedicalClaim;
use harness::prelude::*;
use std::cmp::Reverse;

#[derive(Clone, Copy)]
pub struct ProviderRanking {
    pub person_id: Str,
    pub data_source: Str,
    pub performance_year: Option<i64>,
    pub as_of_date: Option<Date>,
    pub provider_id: Str,
    pub provider_bucket: Str,
    pub prov_specialty: Option<Str>,
    pub step: i64,
    pub step_description: Str,
    pub allowed_amount: f64,
    pub visits: i64,
    pub scope: Str,
    pub lookback_start_date: Option<Date>,
    pub lookback_end_date: Option<Date>,
    pub ranking: i64,
    pub attribution_key: Str,
    pub tuva_last_run: Ts,
}

#[derive(Clone, Copy)]
struct Cl {
    person_id: Str,
    data_source: Str,
    provider_id: Str,
    provider_bucket: Str,
    prov_specialty: Option<Str>,
    encounter_id: Str,
    ymi: i64,
    claim_year: i64,
    claim_end_date: Option<Date>,
    allowed_amount: f64,
}

type Cal = (i64, Date, Date);
type Mo = (Str, i64, Date, Date);
type Par = (Str, Date);
type GKey = (Str, Str, i64, Str, Str, Option<Str>, i64);
type Contrib = (GKey, (f64, Str));
type Agg = (GKey, (f64, i64));
type MmKey = (Str, Str, Str);
type C = IntProviderClassification;

fn leak(s: String) -> Str {
    Box::leak(s.into_boxed_str())
}

fn step_description(s: i64) -> Str {
    match s {
        1 => "12-month PCP/NPP primary-care HCPCS",
        2 => "12-month specialist primary-care HCPCS",
        3 => "24-month PCP/NPP primary-care HCPCS",
        4 => "24-month primary-care HCPCS (any classification)",
        5 => "24-month any rendering NPI",
        _ => "Unknown",
    }
}

fn vec_of<Q: Drive>(q: Q) -> Vec<Q::R> {
    drain(q).into_iter().map(|(_, r)| r).collect()
}

fn pcp_npp(b: Str) -> bool {
    b == "pcp" || b == "npp"
}

fn months(cal: &VecRel<usize, Cal>, params: &VecRel<usize, Par>, back: i64) -> Vec<Mo> {
    let m = cal
        .cross(params)
        .filt(move |((_, first, last), (_, asof)): (Cal, Par)| last >= add_months(asof, back) && first <= asof)
        .map(|((ymi, first, last), (ds, _)): (Cal, Par)| -> Mo { (ds, ymi, first, last) })
        .inv()
        .map(|_| ())
        .fold((), |a, _| a);
    drain(&m).into_iter().map(|(k, _)| k).collect()
}

fn aggregate(parts: Vec<Contrib>) -> Vec<Agg> {
    let r = rel(parts);
    let g = (&r).map(|c: Contrib| c.0).inv().select(&r).buf_fold(|vs| {
        let s: i128 = vs.iter().map(|c: &Contrib| (c.1.0 * 1e6).round() as i128).sum();
        let mut e: Vec<Str> = vs.iter().map(|c| c.1.1).collect();
        e.sort_unstable();
        e.dedup();
        (s as f64 / 1e6, e.len() as i64)
    });
    drain(&g)
}

fn first_step(aggs: Vec<Agg>) -> Vec<Agg> {
    let r = rel(aggs);
    let w = (&r)
        .map(|((p, d, y, pr, _, _, _), _): Agg| (p, d, y, pr))
        .inv()
        .select(&r)
        .window(row_number, |a: Agg| a.0.6, |a: &i64, b: &i64| a.cmp(b));
    drain(&w).into_iter().filter(|(_, (_, rn))| *rn == 1).map(|(_, (a, _))| a).collect()
}

type RKey = (i64, Reverse<i64>, Reverse<i64>, Str);

fn rkey(((_, _, _, pr, _, _, st), (al, v)): Agg) -> RKey {
    (st, Reverse(fkey(al)), Reverse(v), pr)
}

#[allow(clippy::too_many_arguments)]
pub fn provider_ranking(
    db: &'static Db,
    mm: &[MemberMonth],
    spine: &[MemberMonthMonthSpine],
    pcc: &[IntPrimaryCareClaims],
    py: &[IntPersonYears],
    stg: &[StgMedicalClaim],
    pc: &[C],
) -> Vec<ProviderRanking> {
    let run = ts(2026, 1, 1, 0, 0, 0);
    let sp = rel(spine.to_vec());
    let cal = (&sp)
        .map(|s: MemberMonthMonthSpine| -> Cal { (s.year_month_int, s.first_day_of_month, s.last_day_of_month) })
        .inv()
        .map(|_| ())
        .fold((), |a, _| a);
    let cal = rel(drain(&cal).into_iter().map(|(k, _)| k).collect::<Vec<Cal>>());
    let calx: HashIdx<i64, usize> = (&cal).map(|c: Cal| c.0).inv().collect();

    let pcr = rel(pcc.to_vec());
    let bounds = (&pcr)
        .map(|c: IntPrimaryCareClaims| c.data_source)
        .inv()
        .select(&pcr)
        .fold(None, |a: Option<Date>, c: IntPrimaryCareClaims| match (a, c.claim_end_date) {
            (Some(x), Some(y)) => Some(x.max(y)),
            (x, y) => x.or(y),
        });
    let asof = date(2018, 12, 31);
    let params = rel(drain(&bounds).into_iter().map(|(ds, _)| -> Par { (ds, asof) }).collect::<Vec<Par>>());
    let parx: HashIdx<Str, usize> = (&params).map(|p: Par| p.0).inv().collect();

    let m12 = rel(months(&cal, &params, -11));
    let m24 = rel(months(&cal, &params, -23));
    let m12x: HashIdx<(Str, i64), usize> = (&m12).map(|m: Mo| (m.0, m.1)).inv().collect();
    let m24x: HashIdx<(Str, i64), usize> = (&m24).map(|m: Mo| (m.0, m.1)).inv().collect();
    let lb = |m: &VecRel<usize, Mo>| {
        let g = m.map(|x: Mo| x.0).inv().select(m).fold(i64::MAX, |a, x: Mo| a.min(x.2));
        rel(drain(&g))
    };
    let (lb12, lb24) = (lb(&m12), lb(&m24));
    let lb24x: HashIdx<Str, usize> = (&lb24).map(|x: (Str, Date)| x.0).inv().collect();
    let bounds = rel(vec_of((&lb12).and((&lb12).map(|x: (Str, Date)| x.0).select(&lb24x).select(&lb24)).map(
        |((ds, a), (_, b)): ((Str, Date), (Str, Date))| (ds, a, b),
    )));
    let lbx: HashIdx<Str, usize> = (&bounds).map(|x: (Str, Date, Date)| x.0).inv().collect();

    let m12s = rel(vec_of((&m12).map(|m: Mo| (m.0, leak(m.1.to_string())))));
    let m12sx: HashIdx<(Str, Str), usize> = (&m12s).inv().collect();
    let mms = crate::concepts::int_primary_care_claims::member_months(mm);
    let mmr = rel(mms.clone());
    let ec = (&mmr)
        .and((&mmr).map(|(_, ym, ds): MmKey| (ds, ym)).select(&m12sx))
        .map(|((p, _, ds), _): (MmKey, usize)| (p, ds))
        .inv()
        .map(|_| ())
        .fold((), |a, _| a);
    let ec = rel(drain(&ec).into_iter().map(|(k, _)| k).collect::<Vec<(Str, Str)>>());
    let ecx: HashIdx<(Str, Str), usize> = (&ec).inv().collect();

    let to_cl = |c: IntPrimaryCareClaims| Cl {
        person_id: c.person_id,
        data_source: c.data_source,
        provider_id: c.provider_id,
        provider_bucket: c.provider_bucket,
        prov_specialty: c.prov_specialty,
        encounter_id: c.encounter_id,
        ymi: c.claim_year_month_int,
        claim_year: c.claim_year,
        claim_end_date: c.claim_end_date,
        allowed_amount: c.allowed_amount,
    };
    let pcl = rel(vec_of((&pcr).map(to_cl)));

    let eac = rel(crate::concepts::int_primary_care_claims::eligible_claim_months(stg, &mms));
    let spx = crate::concepts::int_primary_care_claims::individual_providers(db);
    let pr = rel(pc.to_vec());
    let px: HashIdx<Str, usize> = (&pr).map(|c: C| c.provider_id).inv().collect();
    let p = &db.provider;
    let arc = (&eac)
        .and((&eac).flat_map(|c: ClaimMonth| c.provider_id).select(&spx).select((&p.primary_specialty_description).opt()))
        .and((&eac).flat_map(|c: ClaimMonth| c.provider_id).select(&px).select(&pr).opt())
        .map(|((c, spec), pc): ((ClaimMonth, Option<Str>), Option<C>)| Cl {
            person_id: c.person_id,
            data_source: c.data_source,
            provider_id: c.provider_id.unwrap(),
            provider_bucket: pc.map_or("other_individual", |x| x.provider_bucket),
            prov_specialty: pc.and_then(|x| x.prov_specialty).or(spec),
            encounter_id: c.encounter_id,
            ymi: c.claim_year_month_int,
            claim_year: 0,
            claim_end_date: c.claim_end_date,
            allowed_amount: c.allowed_amount,
        });
    let arc = rel(vec_of(&arc));

    let windowed = |src: &VecRel<usize, Cl>, mx: &HashIdx<(Str, i64), usize>| {
        rel(vec_of(
            src.and(src.map(|c: Cl| (c.data_source, c.ymi)).select(mx))
                .and(src.map(|c: Cl| c.data_source).select(&parx).select(&params))
                .filt(|((c, _), (_, a)): ((Cl, usize), Par)| c.claim_end_date.is_some_and(|e| e <= a))
                .map(|((c, _), _): ((Cl, usize), Par)| c),
        ))
    };
    let c12 = windowed(&pcl, &m12x);
    let c24 = windowed(&pcl, &m24x);
    let a24 = windowed(&arc, &m24x);

    let contrib = |src: &VecRel<usize, Cl>, step: i64, keep: fn(Str) -> bool| {
        rel(vec_of(src.filt(move |c: Cl| keep(c.provider_bucket)).map(move |c: Cl| -> Contrib {
            ((c.person_id, c.data_source, 0, c.provider_id, c.provider_bucket, c.prov_specialty, step), (c.allowed_amount, c.encounter_id))
        })))
    };
    let (s1, s2, s3, s4, s5) = (
        contrib(&c12, 1, pcp_npp),
        contrib(&c12, 2, |b| b == "specialist"),
        contrib(&c24, 3, pcp_npp),
        contrib(&c24, 4, |_| true),
        contrib(&a24, 5, |_| true),
    );
    let cur = vec_of((&s1).union(&s2).union(&s3).union(&s4).union(&s5));
    let cur_u = rel(first_step(aggregate(cur)));
    let cs = (&cur_u)
        .and((&cur_u).map(|a: Agg| (a.0.0, a.0.1)).select(&ecx))
        .and((&cur_u).map(|a: Agg| a.0.1).select(&parx).select(&params))
        .and((&cur_u).map(|a: Agg| a.0.1).select(&lbx).select(&bounds))
        .map(|(((a, _), (_, asof)), (_, l12, l24)): (((Agg, usize), Par), (Str, Date, Date))| (a, asof, l12, l24));
    let cs = vec_of(&cs);

    let pyr = rel(py.to_vec());
    type Pc = (IntPersonYears, Cl);
    let join_year = |src: &VecRel<usize, Cl>| {
        let ix: HashIdx<(Str, Str, i64), usize> = src.map(|c: Cl| (c.person_id, c.data_source, c.claim_year)).inv().collect();
        rel(vec_of((&pyr).and((&pyr).map(|y: IntPersonYears| (y.person_id, y.data_source, y.performance_year)).select(&ix).select(src))))
    };
    let join_pd = |src: &VecRel<usize, Cl>| {
        let ix: HashIdx<(Str, Str), usize> = src.map(|c: Cl| (c.person_id, c.data_source)).inv().collect();
        rel(vec_of((&pyr).and((&pyr).map(|y: IntPersonYears| (y.person_id, y.data_source)).select(&ix).select(src))))
    };
    let in_two = |(y, c): Pc| c.ymi >= (y.performance_year - 1) * 100 + 1 && c.ymi <= y.performance_year * 100 + 12;
    let ycontrib = |src: VecRel<usize, Pc>, step: i64, keep: &dyn Fn(Pc) -> bool| {
        rel(vec_of((&src).filt(|x: Pc| keep(x)).map(move |(y, c): Pc| -> Contrib {
            (
                (y.person_id, y.data_source, y.performance_year, c.provider_id, c.provider_bucket, c.prov_specialty, step),
                (c.allowed_amount, c.encounter_id),
            )
        })))
    };
    let (y1, y2, y3, y4, y5) = (
        ycontrib(join_year(&pcl), 1, &|x: Pc| pcp_npp(x.1.provider_bucket)),
        ycontrib(join_year(&pcl), 2, &|x: Pc| x.1.provider_bucket == "specialist"),
        ycontrib(join_pd(&pcl), 3, &|x: Pc| in_two(x) && pcp_npp(x.1.provider_bucket)),
        ycontrib(join_pd(&pcl), 4, &|x: Pc| in_two(x)),
        ycontrib(join_pd(&arc), 5, &|x: Pc| in_two(x)),
    );
    let yr = vec_of((&y1).union(&y2).union(&y3).union(&y4).union(&y5));
    let yu = rel(first_step(aggregate(yr)));
    type Yj = (Agg, Option<Cal>, Option<Cal>, Option<Cal>);
    let look = |f: fn(i64) -> i64| (&yu).map(move |a: Agg| f(a.0.2)).select(&calx).select(&cal).opt();
    let yj = rel(vec_of(
        (&yu)
            .and(look(|y| y * 100 + 1))
            .and(look(|y| (y - 1) * 100 + 1))
            .and(look(|y| y * 100 + 12))
            .map(|(((a, curr), prev), end): (((Agg, Option<Cal>), Option<Cal>), Option<Cal>)| -> Yj { (a, curr, prev, end) }),
    ));
    let yearly = (&yj)
        .map(|x: Yj| (x.0.0.0, x.0.0.1, x.0.0.2))
        .inv()
        .select(&yj)
        .window(rank, |x: Yj| rkey(x.0), |a: &RKey, b: &RKey| a.cmp(b))
        .map(|((a, curr, prev, end), rk): (Yj, i64)| {
            let ((person, ds, pyear, prov, bucket, spec, step), (al, v)) = a;
            let start = if (3..=5).contains(&step) { prev.or(curr).map(|c| c.1) } else { curr.map(|c| c.1) };
            ProviderRanking {
                person_id: person,
                data_source: ds,
                performance_year: Some(pyear),
                as_of_date: None,
                provider_id: prov,
                provider_bucket: bucket,
                prov_specialty: spec,
                step,
                step_description: step_description(step),
                allowed_amount: al,
                visits: v,
                scope: "yearly",
                lookback_start_date: start,
                lookback_end_date: end.map(|c| c.2),
                ranking: rk,
                attribution_key: leak(format!("yearly|{ds}|{pyear}|{person}")),
                tuva_last_run: run,
            }
        });
    let mut out = vec_of(&yearly);

    let csr2 = rel(cs);
    type Cs = (Agg, Date, Date, Date);
    let cs_ranked = (&csr2)
        .map(|x: Cs| (x.0.0.0, x.0.0.1))
        .inv()
        .select(&csr2)
        .window(
            rank,
            |x: Cs| rkey(x.0),
            |a: &RKey, b: &RKey| a.cmp(b),
        )
        .map(|((((person, ds, _, prov, bucket, spec, step), (al, v)), asof, l12, l24), rk): (Cs, i64)| ProviderRanking {
            person_id: person,
            data_source: ds,
            performance_year: None,
            as_of_date: Some(asof),
            provider_id: prov,
            provider_bucket: bucket,
            prov_specialty: spec,
            step,
            step_description: step_description(step),
            allowed_amount: al,
            visits: v,
            scope: "current",
            lookback_start_date: Some(if step == 1 || step == 2 { l12 } else { l24 }),
            lookback_end_date: Some(asof),
            ranking: rk,
            attribution_key: leak(format!("current|{ds}|{}|{person}", fmt_date(asof).replace('-', ""))),
            tuva_last_run: run,
        });
    out.extend(vec_of(&cs_ranked));
    out
}

pub fn fmt(v: &ProviderRanking) -> String {
    row(vec![
        V::S(v.person_id),
        V::S(v.data_source),
        oint(v.performance_year),
        odate(v.as_of_date),
        V::S(v.provider_id),
        V::S(v.provider_bucket),
        ostr(v.prov_specialty),
        V::I(v.step),
        V::S(v.step_description),
        V::F(v.allowed_amount),
        V::I(v.visits),
        V::S(v.scope),
        odate(v.lookback_start_date),
        odate(v.lookback_end_date),
        V::I(v.ranking),
        V::S(v.attribution_key),
        V::T(v.tuva_last_run),
    ])
}

pub fn q(db: &'static Db) -> String {
    let u = crate::concepts::stg_medical_claim::upstream(db);
    let spine = crate::concepts::member_month__month_spine::member_month__month_spine(db);
    let mm = crate::concepts::member_month::member_month(db, &u.ne, &spine);
    let stg = crate::concepts::stg_medical_claim::stg_medical_claim(db, &u.nmc, &u.xw, &u.orph);
    let pc = crate::concepts::int_provider_classification::int_provider_classification(db);
    let pcc = crate::concepts::int_primary_care_claims::int_primary_care_claims(db, &stg, &mm, &pc);
    let py = crate::concepts::int_person_years::int_person_years(db, &mm);
    rows(provider_ranking(db, &mm, &spine, &pcc, &py, &stg, &pc).iter().map(fmt))
}
