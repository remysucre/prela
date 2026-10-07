use crate::concepts::urine_output::*;
use crate::concepts::weight_durations::*;
use crate::schema::*;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct UrineOutputRate {
    pub stay_id: i64,
    pub charttime: Ts,
    pub weight: Option<f64>,
    pub uo: f64,
    pub urineoutput_6hr: Option<f64>,
    pub urineoutput_12hr: Option<f64>,
    pub urineoutput_24hr: Option<f64>,
    pub uo_mlkghr_6hr: Option<f64>,
    pub uo_mlkghr_12hr: Option<f64>,
    pub uo_mlkghr_24hr: Option<f64>,
    pub uo_tm_6hr: Option<f64>,
    pub uo_tm_12hr: Option<f64>,
    pub uo_tm_24hr: Option<f64>,
}

#[derive(Clone, Copy)]
struct UoTm {
    stay_id: i64,
    charttime: Ts,
    tm: i64,
    uo: f64,
}

#[derive(Clone, Copy)]
struct Stg {
    stay_id: i64,
    charttime: Ts,
    uo: f64,
    sum: [Option<f64>; 3],
    tm: [Option<f64>; 3],
}

#[derive(Clone, Copy)]
struct Acc {
    uo: [((f64, f64), i64); 3],
    tm: [(i64, i64); 3],
}

const MIN_US: i64 = 60 * US;
const HOUR_US: i64 = 3600 * US;
const HRS: [i64; 3] = [5, 11, i64::MAX];

fn diff(a: Ts, b: Ts, unit: i64) -> i64 {
    b.div_euclid(unit) - a.div_euclid(unit)
}

pub fn urine_output_rate(db: &'static Db, uo: &[UrineOutput], wd: &[WeightDurations]) -> Vec<UrineOutputRate> {
    let ie = &db.icu_stay;
    let ce = &db.chart_event;
    let key = (&ce.charttime)
        .and((&ce.stay).select((&ie.intime).and(&ie.outtime).and(&ie.stay_id)))
        .filt(|(t, ((i, o), _)): (Ts, ((Ts, Ts), i64))| t > add_months(i, -1) && t < add_months(o, 1))
        .map(|(_, (_, s)): (Ts, ((Ts, Ts), i64))| s);
    let tm = (&ce.id)
        .with((&ce.itemid).eq(220045))
        .group_by(key)
        .select(&ce.charttime)
        .fold((i64::MAX, i64::MIN), |(lo, hi), t| (lo.min(t), hi.max(t)));
    let uor = rel(uo.to_vec());
    let j = (&uor).and((&uor).map(|u: UrineOutput| u.stay_id).select(&tm));
    let j = rel(drain(&j).into_iter().map(|(_, x)| x).collect::<Vec<(UrineOutput, (Ts, Ts))>>());
    let lg = (&j)
        .map(|(u, _): (UrineOutput, (Ts, Ts))| u.stay_id)
        .inv()
        .select(&j)
        .window(lag, |(u, _): (UrineOutput, (Ts, Ts))| u.charttime, |a: &Ts, b: &Ts| a.cmp(b));
    let ut = rel(drain(&lg)
        .into_iter()
        .map(|(_, ((u, (intime_hr, _)), prev))| UoTm {
            stay_id: u.stay_id,
            charttime: u.charttime,
            tm: diff(prev.unwrap_or(intime_hr), u.charttime, MIN_US),
            uo: u.urineoutput,
        })
        .collect());
    let by: HashIdx<i64, usize> = (&ut).map(|s: UoTm| s.stay_id).inv().collect();
    let init = Acc { uo: [((0.0, 0.0), 0); 3], tm: [(0, 0); 3] };
    let sums = (&ut)
        .and((&ut).map(|s: UoTm| s.stay_id).select(&by).select(&ut))
        .filt(|(io, s): (UoTm, UoTm)| io.charttime >= s.charttime && io.charttime <= s.charttime + 23 * HOUR_US)
        .fold(init, |mut a, (io, s): (UoTm, UoTm)| {
            let h = diff(s.charttime, io.charttime, HOUR_US);
            for i in 0..3 {
                if h <= HRS[i] {
                    a.uo[i] = (kahan(a.uo[i].0, s.uo), a.uo[i].1 + 1);
                    a.tm[i] = (a.tm[i].0 + s.tm, a.tm[i].1 + 1);
                }
            }
            a
        });
    let stg = rel(drain((&ut).and((&sums).opt()))
        .into_iter()
        .map(|(_, (io, a))| {
            let a = a.unwrap_or(init);
            Stg {
                stay_id: io.stay_id,
                charttime: io.charttime,
                uo: io.uo,
                sum: a.uo.map(|(s, n)| (n > 0).then_some(s.0)),
                tm: a.tm.map(|(s, n)| (n > 0).then(|| round_dec(s as f64 / 60.0, 6))),
            }
        })
        .collect());
    let wr = rel(wd.to_vec());
    let wby: HashIdx<i64, usize> = (&wr).map(|w: WeightDurations| w.stay_id).inv().collect();
    let hit: HashIdx<usize, f64> = (&stg)
        .and((&stg).map(|r: Stg| r.stay_id).select(&wby).select(&wr))
        .filt(|(r, w): (Stg, WeightDurations)| r.charttime > w.starttime && r.charttime <= w.endtime && w.weight > 0.0)
        .map(|(_, w): (Stg, WeightDurations)| w.weight)
        .collect();
    drain((&stg).and((&hit).opt()))
        .into_iter()
        .map(|(_, (r, weight))| {
            let rate = |i: usize, lim: f64| match (r.sum[i], weight, r.tm[i]) {
                (Some(u), Some(w), Some(t)) if t >= lim => Some(round_dec(u / w / t, 4)),
                _ => None,
            };
            UrineOutputRate {
                stay_id: r.stay_id,
                charttime: r.charttime,
                weight,
                uo: r.uo,
                urineoutput_6hr: r.sum[0],
                urineoutput_12hr: r.sum[1],
                urineoutput_24hr: r.sum[2],
                uo_mlkghr_6hr: rate(0, 6.0),
                uo_mlkghr_12hr: rate(1, 12.0),
                uo_mlkghr_24hr: rate(2, 24.0),
                uo_tm_6hr: r.tm[0].map(|t| round_dec(t, 2)),
                uo_tm_12hr: r.tm[1].map(|t| round_dec(t, 2)),
                uo_tm_24hr: r.tm[2].map(|t| round_dec(t, 2)),
            }
        })
        .collect()
}

pub fn q(db: &'static Db) -> String {
    let uo = crate::concepts::urine_output::urine_output(db);
    let wd = crate::concepts::weight_durations::weight_durations(db);
    rows(urine_output_rate(db, &uo, &wd).into_iter().map(|k| {
        row(vec![
            V::I(k.stay_id),
            V::T(k.charttime),
            ofloat(k.weight),
            V::F(k.uo),
            ofloat(k.urineoutput_6hr),
            ofloat(k.urineoutput_12hr),
            ofloat(k.urineoutput_24hr),
            ofloat(k.uo_mlkghr_6hr),
            ofloat(k.uo_mlkghr_12hr),
            ofloat(k.uo_mlkghr_24hr),
            ofloat(k.uo_tm_6hr),
            ofloat(k.uo_tm_12hr),
            ofloat(k.uo_tm_24hr),
        ])
    }))
}
