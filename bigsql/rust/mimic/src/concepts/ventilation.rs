use crate::concepts::oxygen_delivery::*;
use crate::concepts::ventilator_setting::*;
use crate::schema::*;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct Ventilation {
    pub stay_id: i64,
    pub starttime: Ts,
    pub endtime: Ts,
    pub ventilation_status: Str,
}

#[derive(Clone, Copy)]
struct Vs {
    stay_id: i64,
    charttime: Ts,
    status: Str,
}

#[derive(Clone, Copy)]
struct Vd {
    vs: Vs,
    lead: Option<Ts>,
    status_lag: Option<Str>,
}

#[derive(Clone, Copy)]
struct Vd1 {
    vd: Vd,
    new_event: i64,
}

const INVASIVE_MODE: [&str; 37] = [
    "(S) CMV",
    "APRV",
    "APRV/Biphasic+ApnPress",
    "APRV/Biphasic+ApnVol",
    "APV (cmv)",
    "Ambient",
    "Apnea Ventilation",
    "CMV",
    "CMV/ASSIST",
    "CMV/ASSIST/AutoFlow",
    "CMV/AutoFlow",
    "CPAP/PPS",
    "CPAP/PSV",
    "CPAP/PSV+Apn TCPL",
    "CPAP/PSV+ApnPres",
    "CPAP/PSV+ApnVol",
    "MMV",
    "MMV/AutoFlow",
    "MMV/PSV",
    "MMV/PSV/AutoFlow",
    "P-CMV",
    "PCV+",
    "PCV+/PSV",
    "PCV+Assist",
    "PRES/AC",
    "PRVC/AC",
    "PRVC/SIMV",
    "PSV/SBT",
    "SIMV",
    "SIMV/AutoFlow",
    "SIMV/PRES",
    "SIMV/PSV",
    "SIMV/PSV/AutoFlow",
    "SIMV/VOL",
    "SYNCHRON MASTER",
    "SYNCHRON SLAVE",
    "VOL/AC",
];

const INVASIVE_HAMILTON: [&str; 10] =
    ["APRV", "APV (cmv)", "Ambient", "(S) CMV", "P-CMV", "SIMV", "APV (simv)", "P-SIMV", "VS", "ASV"];

const SUPPLEMENTAL: [&str; 10] = [
    "Non-rebreather",
    "Face tent",
    "Aerosol-cool",
    "Venti mask ",
    "Medium conc mask ",
    "Ultrasonic neb",
    "Vapomist",
    "Oxymizer",
    "High flow neb",
    "Nasal cannula",
];

const H: i64 = 3600 * US;

fn hour_diff(a: Ts, b: Ts) -> i64 {
    b.div_euclid(H) - a.div_euclid(H)
}

fn is_in(x: Option<Str>, set: &[&str]) -> bool {
    x.is_some_and(|x| set.contains(&x))
}

fn status(v: Option<VentilatorSetting>, o: Option<OxygenDelivery>) -> Option<Str> {
    let mode = v.and_then(|v| v.ventilator_mode);
    let ham = v.and_then(|v| v.ventilator_mode_hamilton);
    let d = |i: usize| {
        o.and_then(|o| match i {
            1 => o.o2_delivery_device_1,
            2 => o.o2_delivery_device_2,
            3 => o.o2_delivery_device_3,
            _ => o.o2_delivery_device_4,
        })
    };
    let masks = ["Bipap mask ", "CPAP mask "];
    if is_in(d(1), &["Tracheostomy tube", "Trach mask "]) {
        Some("Tracheostomy")
    } else if is_in(d(1), &["Endotracheal tube"]) || is_in(mode, &INVASIVE_MODE) || is_in(ham, &INVASIVE_HAMILTON) {
        Some("InvasiveVent")
    } else if (1..=4).any(|i| is_in(d(i), &masks)) || is_in(ham, &["DuoPaP", "NIV", "NIV-ST"]) {
        Some("NonInvasiveVent")
    } else if is_in(d(1), &["High flow nasal cannula"]) {
        Some("HFNC")
    } else if is_in(d(1), &SUPPLEMENTAL) {
        Some("SupplementalOxygen")
    } else if is_in(d(1), &["None"]) {
        Some("None")
    } else {
        None
    }
}

pub fn ventilation(_db: &'static Db, vset: &[VentilatorSetting], od: &[OxygenDelivery]) -> Vec<Ventilation> {
    let vr = rel(vset.to_vec());
    let or = rel(od.to_vec());
    let tm_u = (&vr)
        .map(|v: VentilatorSetting| (v.stay_id, v.charttime))
        .inv()
        .map(|_| ())
        .union((&or).map(|o: OxygenDelivery| (o.stay_id, o.charttime)).inv().map(|_| ()))
        .fold((), |a, _| a);
    let tm = rel(drain(&tm_u).into_iter().map(|(k, _)| k).collect::<Vec<(i64, Ts)>>());
    let v_by: HashIdx<(i64, Ts), usize> = (&vr).map(|v: VentilatorSetting| (v.stay_id, v.charttime)).inv().collect();
    let o_by: HashIdx<(i64, Ts), usize> = (&or).map(|o: OxygenDelivery| (o.stay_id, o.charttime)).inv().collect();
    let joined = (&tm)
        .and((&tm).select(&v_by).select(&vr).opt())
        .and((&tm).select(&o_by).select(&or).opt())
        .map(|(((stay_id, charttime), v), o): (((i64, Ts), Option<VentilatorSetting>), Option<OxygenDelivery>)| {
            (stay_id, charttime, status(v, o))
        })
        .filt(|(_, _, s): (i64, Ts, Option<Str>)| s.is_some())
        .map(|(stay_id, charttime, s): (i64, Ts, Option<Str>)| Vs { stay_id, charttime, status: s.unwrap() });
    let vs = rel(drain(&joined).into_iter().map(|(_, x)| x).collect::<Vec<Vs>>());

    let by_ct = |a: &Ts, b: &Ts| a.cmp(b);
    let w = (&vs).map(|x: Vs| x.stay_id).inv().select(&vs).window(
        |g: &[(Ts, Vs)], out: &mut Vec<(Option<Ts>, Option<Str>)>| {
            let mut ld = Vec::new();
            lead(g, &mut ld);
            for (i, l) in ld.into_iter().enumerate() {
                out.push((l, (i > 0).then(|| g[i - 1].1.status)));
            }
        },
        |x: Vs| x.charttime,
        by_ct,
    );
    let vd = rel(drain(&w)
        .into_iter()
        .map(|(_, (vs, (lead, status_lag)))| Vd { vs, lead, status_lag })
        .collect::<Vec<Vd>>());

    let w = (&vd)
        .map(|x: Vd| (x.vs.stay_id, x.vs.status))
        .inv()
        .select(&vd)
        .window(lag, |x: Vd| x.vs.charttime, by_ct);
    let vd1 = rel(drain(&w)
        .into_iter()
        .map(|(_, (vd, ct_lag))| {
            let new_event = if vd.status_lag.is_none() {
                1
            } else if ct_lag.is_some_and(|l| hour_diff(l, vd.vs.charttime) >= 14) {
                1
            } else if vd.status_lag != Some(vd.vs.status) {
                1
            } else {
                0
            };
            Vd1 { vd, new_event }
        })
        .collect::<Vec<Vd1>>());

    let w = (&vd1).map(|x: Vd1| x.vd.vs.stay_id).inv().select(&vd1).window(
        |g: &[(Ts, Vd1)], out: &mut Vec<i64>| {
            let mut s = 0;
            for (_, r) in g {
                s += r.new_event;
                out.push(s);
            }
        },
        |x: Vd1| x.vd.vs.charttime,
        by_ct,
    );
    let vd2 = rel(drain(&w).into_iter().map(|(_, r)| r).collect::<Vec<(Vd1, i64)>>());

    let g = (&vd2)
        .map(|(x, seq): (Vd1, i64)| (x.vd.vs.stay_id, seq))
        .inv()
        .select(&vd2)
        .fold((i64::MAX, i64::MIN, i64::MIN, ""), |(lo, hi, end, st), (x, _): (Vd1, i64)| {
            let ct = x.vd.vs.charttime;
            let e = match x.vd.lead {
                Some(l) if hour_diff(ct, l) < 14 => l,
                _ => ct,
            };
            (lo.min(ct), hi.max(ct), end.max(e), st.max(x.vd.vs.status))
        });
    let having = (&g).filt(|(lo, hi, _, _): (Ts, Ts, Ts, Str)| lo != hi);
    drain(&having)
        .into_iter()
        .map(|((stay_id, _), (lo, _, end, st))| Ventilation { stay_id, starttime: lo, endtime: end, ventilation_status: st })
        .collect()
}

pub fn q(db: &'static Db) -> String {
    let vset = ventilator_setting(db);
    let od = oxygen_delivery(db);
    rows(ventilation(db, &vset, &od).into_iter().map(|v| {
        row(vec![V::I(v.stay_id), V::T(v.starttime), V::T(v.endtime), V::S(v.ventilation_status)])
    }))
}
