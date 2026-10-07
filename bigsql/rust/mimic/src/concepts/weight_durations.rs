use crate::schema::*;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct WeightDurations {
    pub stay_id: i64,
    pub starttime: Ts,
    pub endtime: Ts,
    pub weight: f64,
    pub weight_type: Str,
}

#[derive(Clone, Copy)]
struct W {
    stay_id: i64,
    charttime: Ts,
    admit: bool,
    weight: f64,
    intime: Ts,
    outtime: Ts,
}

#[derive(Clone, Copy)]
struct S {
    stay_id: i64,
    starttime: Ts,
    weight: f64,
    admit: bool,
    outtime: Ts,
}

const H2: i64 = 2 * 3600 * US;

fn kind(admit: bool) -> Str {
    if admit { "admit" } else { "daily" }
}

pub fn weight_durations(db: &'static Db) -> Vec<WeightDurations> {
    let ce = &db.chart_event;
    let ie = &db.icu_stay;
    let stg = (&ce.id)
        .with((&ce.itemid).is_in([226512, 224639]))
        .with((&ce.valuenum).filt(|v| v > 0.0 && v < 1500.0))
        .select(
            (&ce.stay_id)
                .and(&ce.charttime)
                .and(&ce.itemid)
                .and(&ce.valuenum)
                .and((&ce.stay).select((&ie.intime).and(&ie.outtime))),
        )
        .map(|((((stay_id, charttime), item), v), (intime, outtime))| W {
            stay_id,
            charttime,
            admit: item == 226512,
            weight: round_dec(v, 3),
            intime,
            outtime,
        });
    let stg = rel(drain(stg).into_iter().map(|(_, w)| w).collect());
    let rn = (&stg)
        .map(|w: W| (w.stay_id, w.admit))
        .inv()
        .select(&stg)
        .window(row_number, |w: W| w.charttime, |a: &Ts, b: &Ts| a.cmp(b));
    let stg2 = rel(drain(&rn)
        .into_iter()
        .map(|(_, (w, rn))| S {
            stay_id: w.stay_id,
            starttime: if w.admit && rn == 1 { w.intime - H2 } else { w.charttime },
            weight: w.weight,
            admit: w.admit,
            outtime: w.outtime,
        })
        .collect());
    let stg3 = (&stg2).map(|s: S| s.stay_id).inv().select(&stg2).window(lead, |s: S| s.starttime, |a: &Ts, b: &Ts| a.cmp(b));
    let stg3 = rel(drain(&stg3)
        .into_iter()
        .map(|(_, (s, next))| (s, next.unwrap_or(s.outtime + H2)))
        .collect::<Vec<(S, Ts)>>());
    let w1 = (&stg3)
        .map(|(s, _): (S, Ts)| s.stay_id)
        .inv()
        .select(&stg3)
        .window(lead, |(s, _): (S, Ts)| s.starttime, |a: &Ts, b: &Ts| a.cmp(b));
    let wt1 = rel(drain(&w1)
        .into_iter()
        .map(|(_, ((s, end), _))| WeightDurations {
            stay_id: s.stay_id,
            starttime: s.starttime,
            endtime: end,
            weight: s.weight,
            weight_type: kind(s.admit),
        })
        .collect::<Vec<WeightDurations>>());
    let rn1 = (&wt1)
        .map(|w: WeightDurations| w.stay_id)
        .inv()
        .select(&wt1)
        .window(row_number, |w: WeightDurations| w.starttime, |a: &Ts, b: &Ts| a.cmp(b));
    let firsts = rel(drain((&rn1).filt(|(_, rn): (WeightDurations, i64)| rn == 1))
        .into_iter()
        .map(|(_, (w, _))| w)
        .collect::<Vec<WeightDurations>>());
    let by_sid: HashIdx<i64, Id<IcuStay>> = (&ie.stay_id).inv().collect();
    let fix = (&firsts)
        .and((&firsts).map(|w: WeightDurations| w.stay_id).select(&by_sid).select(&ie.intime))
        .filt(|(w, intime): (WeightDurations, Ts)| intime < w.starttime)
        .map(|(w, intime): (WeightDurations, Ts)| WeightDurations { starttime: intime - H2, endtime: w.starttime, ..w });
    let mut out: Vec<WeightDurations> = drain(&wt1).into_iter().map(|(_, w)| w).collect();
    out.extend(drain(&fix).into_iter().map(|(_, w)| w));
    out
}

pub fn q(db: &'static Db) -> String {
    rows(weight_durations(db).into_iter().map(|w| {
        row(vec![V::I(w.stay_id), V::T(w.starttime), V::T(w.endtime), V::F(w.weight), V::S(w.weight_type)])
    }))
}
