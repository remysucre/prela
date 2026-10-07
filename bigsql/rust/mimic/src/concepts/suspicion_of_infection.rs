use crate::concepts::antibiotic::*;
use crate::schema::*;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct SuspicionOfInfection {
    pub subject_id: i64,
    pub stay_id: Option<i64>,
    pub hadm_id: i64,
    pub ab_id: i64,
    pub antibiotic: Str,
    pub antibiotic_time: Ts,
    pub suspected_infection: i64,
    pub suspected_infection_time: Option<Ts>,
    pub culture_time: Option<Ts>,
    pub specimen: Option<Str>,
    pub positive_culture: Option<i64>,
}

#[derive(Clone, Copy)]
struct Me {
    micro_specimen_id: i64,
    subject_id: i64,
    chartdate: Ts,
    charttime: Option<Ts>,
    spec_type_desc: Str,
    positiveculture: i64,
}

type Ab = (Antibiotic, i64);
type Seq = ((Ts, (bool, Ts)), (i64, i64));

fn seq(m: Me) -> Seq {
    ((m.chartdate, (m.charttime.is_none(), m.charttime.unwrap_or(0))), (-m.positiveculture, m.micro_specimen_id))
}

fn first(
    ar: &VecRel<usize, Ab>,
    mr: &VecRel<usize, Me>,
    by: &HashIdx<i64, usize>,
    hit: impl Fn(Ts, Ts, Me) -> bool,
) -> Window<usize, Me, i64> {
    ar.and(ar.map(|(a, _): Ab| a.subject_id).select(by).select(mr))
        .filt(move |((a, _), m): (Ab, Me)| hit(a.starttime, trunc_day(a.starttime), m))
        .map(|(_, m): (Ab, Me)| m)
        .window(row_number, seq, |a: &Seq, b: &Seq| a.cmp(b))
}

pub fn suspicion_of_infection(db: &'static Db, abx: &[Antibiotic]) -> Vec<SuspicionOfInfection> {
    let mb = &db.microbiology_event;
    let ab0 = rel(abx.to_vec());
    let w = (&ab0).map(|a: Antibiotic| a.subject_id).inv().select(&ab0).window(
        |s: &[(_, Antibiotic)], out: &mut Vec<i64>| out.extend(1..=s.len() as i64),
        |a: Antibiotic| (a.starttime, a.stoptime, a.antibiotic, a.hadm_id, a.stay_id),
        |x, y| x.cmp(y),
    );
    let ar: VecRel<usize, Ab> = rel(drain(&w).into_iter().map(|(_, r)| r).collect());

    let pos = (&mb.org_name).opt().and((&mb.org_itemid).opt()).map(|(n, i): (Option<Str>, Option<i64>)| match (n, i) {
        (Some(n), Some(i)) if i != 90856 && i != 90760 && n != "" && n != "CANCELLED" => 1i64,
        _ => 0,
    });
    let me = (&mb.id)
        .group_by(&mb.micro_specimen_id)
        .select((&mb.subject_id).and(&mb.chartdate).and((&mb.charttime).opt()).and(&mb.spec_type_desc).and(pos))
        .fold(
            (i64::MIN, i64::MIN, None, "", 0i64),
            |(s, d, t, sp, p): (i64, Ts, Option<Ts>, Str, i64), ((((s2, d2), t2), sp2), p2)| {
                (s.max(s2), d.max(d2), t.max(t2), sp.max(sp2), p.max(p2))
            },
        );
    let mr = rel(drain(&me)
        .into_iter()
        .map(|(micro_specimen_id, (subject_id, chartdate, charttime, spec_type_desc, positiveculture))| Me {
            micro_specimen_id,
            subject_id,
            chartdate: trunc_day(chartdate),
            charttime,
            spec_type_desc,
            positiveculture,
        })
        .collect());
    let by: HashIdx<i64, usize> = (&mr).map(|m: Me| m.subject_id).inv().collect();

    let h72 = 72 * 3600 * US;
    let me2ab = first(&ar, &mr, &by, move |t, d, m| match m.charttime {
        Some(c) => t > c && t <= c + h72,
        None => d >= m.chartdate && d <= m.chartdate + 3 * DAY_US,
    });
    let h24 = 24 * 3600 * US;
    let ab2me = first(&ar, &mr, &by, move |t, d, m| match m.charttime {
        Some(c) => t >= c - h24 && t < c,
        None => d >= m.chartdate - DAY_US && d <= m.chartdate,
    });

    let seq1 = |(_, rn): (Me, i64)| rn == 1;
    drain((&ar).and((&ab2me).filt(seq1).opt()).and((&me2ab).filt(seq1).opt()))
        .into_iter()
        .map(|(_, (((a, ab_id), n24), l72))| {
            let n24 = n24.map(|(m, _)| m);
            let l72 = l72.map(|(m, _)| m);
            let ct = |m: Me| m.charttime.unwrap_or(m.chartdate);
            let any = l72.is_some() || n24.is_some();
            SuspicionOfInfection {
                subject_id: a.subject_id,
                stay_id: a.stay_id,
                hadm_id: a.hadm_id,
                ab_id,
                antibiotic: a.antibiotic,
                antibiotic_time: a.starttime,
                suspected_infection: any as i64,
                suspected_infection_time: any.then(|| l72.map(ct).unwrap_or(a.starttime)),
                culture_time: l72.or(n24).map(ct),
                specimen: l72.or(n24).map(|m| m.spec_type_desc),
                positive_culture: l72.or(n24).map(|m| m.positiveculture),
            }
        })
        .collect()
}

pub fn q(db: &'static Db) -> String {
    let abx = crate::concepts::antibiotic::antibiotic(db);
    rows(suspicion_of_infection(db, &abx).into_iter().map(|v| {
        row(vec![
            V::I(v.subject_id),
            oint(v.stay_id),
            V::I(v.hadm_id),
            V::I(v.ab_id),
            V::S(v.antibiotic),
            V::T(v.antibiotic_time),
            V::I(v.suspected_infection),
            ots(v.suspected_infection_time),
            ots(v.culture_time),
            ostr(v.specimen),
            oint(v.positive_culture),
        ])
    }))
}
