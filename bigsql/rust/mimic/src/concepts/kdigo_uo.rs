use crate::concepts::urine_output::*;
use crate::concepts::weight_durations::*;
use crate::schema::*;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct KdigoUo {
    pub stay_id: i64,
    pub charttime: Ts,
    pub weight: Option<f64>,
    pub urineoutput_6hr: f64,
    pub urineoutput_12hr: f64,
    pub urineoutput_24hr: f64,
    pub uo_rt_6hr: Option<f64>,
    pub uo_rt_12hr: Option<f64>,
    pub uo_rt_24hr: Option<f64>,
    pub uo_tm_6hr: f64,
    pub uo_tm_12hr: f64,
    pub uo_tm_24hr: f64,
}

#[derive(Clone, Copy)]
struct S1 {
    stay_id: i64,
    charttime: Ts,
    secs: i64,
    hrs: f64,
    uo: f64,
}

#[derive(Clone, Copy)]
struct S2 {
    stay_id: i64,
    charttime: Ts,
    uo: [f64; 3],
    tm: [f64; 3],
}

const SPAN: [i64; 3] = [21600, 43200, 86400];

fn secs(t: Ts) -> i64 {
    t.div_euclid(US)
}

pub fn kdigo_uo(db: &'static Db, uo: &[UrineOutput], wd: &[WeightDurations]) -> Vec<KdigoUo> {
    let ie = &db.icu_stay;
    let by_sid: HashIdx<i64, Id<IcuStay>> = (&ie.stay_id).inv().collect();
    let uor = rel(uo.to_vec());
    let j = (&uor).and((&uor).map(|u: UrineOutput| u.stay_id).select(&by_sid).select(&ie.intime));
    let j = rel(drain(&j).into_iter().map(|(_, x)| x).collect::<Vec<(UrineOutput, Ts)>>());
    let lg = (&j)
        .map(|(u, _): (UrineOutput, Ts)| u.stay_id)
        .inv()
        .select(&j)
        .window(lag, |(u, _): (UrineOutput, Ts)| u.charttime, |a: &Ts, b: &Ts| a.cmp(b));
    let s1 = rel(drain(&lg)
        .into_iter()
        .map(|(_, ((u, intime), prev))| S1 {
            stay_id: u.stay_id,
            charttime: u.charttime,
            secs: secs(u.charttime) - secs(intime),
            hrs: prev.map_or(1.0, |p| (secs(u.charttime) - secs(p)) as f64 / 3600.0),
            uo: u.urineoutput,
        })
        .collect());
    let by: HashIdx<i64, usize> = (&s1).map(|s: S1| s.stay_id).inv().collect();
    let init = ([(0.0, 0.0); 3], [(0.0, 0.0); 3]);
    let sums = (&s1)
        .and((&s1).map(|s: S1| s.stay_id).select(&by).select(&s1))
        .filt(|(a, b): (S1, S1)| b.secs <= a.secs && b.secs >= a.secs - SPAN[2])
        .fold(init, |mut acc, (a, b): (S1, S1)| {
            for i in 0..3 {
                if b.secs >= a.secs - SPAN[i] {
                    acc.0[i] = kahan(acc.0[i], b.uo);
                    acc.1[i] = kahan(acc.1[i], b.hrs);
                }
            }
            acc
        });
    let s2 = rel(drain((&s1).and(&sums))
        .into_iter()
        .map(|(_, (s, (u, t)))| S2 {
            stay_id: s.stay_id,
            charttime: s.charttime,
            uo: [u[0].0, u[1].0, u[2].0],
            tm: [round_dec(t[0].0, 6), round_dec(t[1].0, 6), round_dec(t[2].0, 6)],
        })
        .collect());
    let wr = rel(wd.to_vec());
    let wby: HashIdx<i64, usize> = (&wr).map(|w: WeightDurations| w.stay_id).inv().collect();
    let hit: HashIdx<usize, f64> = (&s2)
        .and((&s2).map(|r: S2| r.stay_id).select(&wby).select(&wr))
        .filt(|(r, w): (S2, WeightDurations)| r.charttime >= w.starttime && r.charttime < w.endtime)
        .map(|(_, w): (S2, WeightDurations)| w.weight)
        .collect();
    drain((&s2).and((&hit).opt()))
        .into_iter()
        .map(|(_, (r, weight))| {
            let rt = |i: usize, ok: bool| {
                if ok { weight.map(|w| round_dec(r.uo[i] / w / r.tm[i], 4)) } else { None }
            };
            KdigoUo {
                stay_id: r.stay_id,
                charttime: r.charttime,
                weight,
                urineoutput_6hr: r.uo[0],
                urineoutput_12hr: r.uo[1],
                urineoutput_24hr: r.uo[2],
                uo_rt_6hr: rt(0, r.tm[0] >= 6.0 && r.tm[0] < 12.0),
                uo_rt_12hr: rt(1, r.tm[1] >= 12.0),
                uo_rt_24hr: rt(2, r.tm[2] >= 24.0),
                uo_tm_6hr: r.tm[0],
                uo_tm_12hr: r.tm[1],
                uo_tm_24hr: r.tm[2],
            }
        })
        .collect()
}

pub fn q(db: &'static Db) -> String {
    let uo = crate::concepts::urine_output::urine_output(db);
    let wd = crate::concepts::weight_durations::weight_durations(db);
    rows(kdigo_uo(db, &uo, &wd).into_iter().map(|k| {
        row(vec![
            V::I(k.stay_id),
            V::T(k.charttime),
            ofloat(k.weight),
            V::F(k.urineoutput_6hr),
            V::F(k.urineoutput_12hr),
            V::F(k.urineoutput_24hr),
            ofloat(k.uo_rt_6hr),
            ofloat(k.uo_rt_12hr),
            ofloat(k.uo_rt_24hr),
            V::F(k.uo_tm_6hr),
            V::F(k.uo_tm_12hr),
            V::F(k.uo_tm_24hr),
        ])
    }))
}
