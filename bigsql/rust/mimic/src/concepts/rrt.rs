use crate::schema::*;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct Rrt {
    pub stay_id: i64,
    pub charttime: Ts,
    pub dialysis_present: i64,
    pub dialysis_active: i64,
    pub dialysis_type: Option<Str>,
}

const ITEMS: [i64; 48] = [
    226118, 227357, 225725, 226499, 224154, 225810, 227639, 225183, 227438, 224191, 225806, 225807, 228004, 228005,
    228006, 224144, 224145, 224149, 224150, 224151, 224152, 224153, 224404, 224406, 226457, 225959, 224135, 224139,
    224146, 225323, 225740, 225776, 225951, 225952, 225953, 225954, 225956, 225958, 225961, 225963, 225965, 225976,
    225977, 227124, 227290, 227638, 227640, 227753,
];

const PRESENT: [i64; 48] = [
    226118, 227357, 225725, 226499, 224154, 225810, 225959, 227639, 225183, 227438, 224191, 225806, 225807, 228004,
    228005, 228006, 224144, 224145, 224149, 224150, 224151, 224152, 224153, 224404, 224406, 226457, 224135, 224139,
    224146, 225323, 225740, 225776, 225951, 225952, 225953, 225954, 225956, 225958, 225961, 225963, 225965, 225976,
    225977, 227124, 227290, 227638, 227640, 227753,
];

const ACTIVE: [i64; 14] = [
    226499, 224154, 225183, 227438, 224191, 225806, 225807, 228004, 228005, 228006, 224144, 224145, 224153, 226457,
];

const PERITONEAL: [i64; 14] = [
    225810, 225806, 225807, 225810, 227639, 225959, 225951, 225952, 225961, 225953, 225963, 225965, 227638, 227640,
];

const PROC_ITEMS: [i64; 8] = [225441, 225802, 225803, 225805, 224270, 225809, 225955, 225436];

type Mv = (((i64, Ts), Ts), ((i64, i64), Option<Str>));
type Stg = ((i64, Ts), ((i64, i64), Option<Str>));

pub fn rrt(db: &'static Db) -> Vec<Rrt> {
    let ce = &db.chart_event;
    let ie = &db.input_event;
    let pe = &db.procedure_event;

    let ie_q = (&ie.id)
        .with((&ie.itemid).is_in([227536, 227525]))
        .with((&ie.amount).filt(|a| a > 0.0))
        .select((&ie.stay_id).and(&ie.starttime).and(&ie.endtime))
        .map(|k: ((i64, Ts), Ts)| -> Mv { (k, ((1, 1), Some("CRRT"))) });
    let pe_q = (&pe.id)
        .with((&pe.itemid).is_in(PROC_ITEMS))
        .select((&pe.stay_id).and(&pe.starttime).and(&pe.endtime).and(&pe.itemid))
        .map(|(k, item): (((i64, Ts), Ts), i64)| -> Mv {
            let active = if item != 224270 && item != 225436 { 1 } else { 0 };
            let ty = match item {
                225441 => Some("IHD"),
                225802 => Some("CRRT"),
                225803 => Some("CVVHD"),
                225805 => Some("Peritoneal"),
                225809 => Some("CVVHDF"),
                225955 => Some("SCUF"),
                _ => None,
            };
            (k, ((1, active), ty))
        });
    let mv_u = (&ie_q).inv().map(|_| ()).union((&pe_q).inv().map(|_| ())).fold((), |a, _| a);
    let mv: Vec<Mv> = drain(&mv_u).into_iter().map(|(m, _)| m).collect();
    let mvr = rel(mv);

    let ce_q = (&ce.id)
        .with((&ce.itemid).is_in(ITEMS))
        .select((&ce.stay_id).and(&ce.charttime).and(&ce.itemid).and(&ce.value))
        .map(|(((stay, t), item), value): (((i64, Ts), i64), Str)| {
            let present = if PRESENT.contains(&item) { 1 } else { 0 };
            let active = if (item == 225965 && value == "In use") || ACTIVE.contains(&item) { 1 } else { 0 };
            let ty = if item == 227290 {
                Some(value)
            } else if PERITONEAL.contains(&item) {
                Some("Peritoneal")
            } else if item == 226499 {
                Some("IHD")
            } else {
                None
            };
            ((stay, t), ((present, active), ty))
        })
        .filt(|(_, ((present, _), _)): Stg| present == 1);
    let stg_u = (&ce_q)
        .inv()
        .map(|_| ())
        .union((&mvr).map(|((st, _), v): Mv| -> Stg { (st, v) }).inv().map(|_| ()))
        .fold((), |a, _| a);
    let stg: Vec<Stg> = drain(&stg_u).into_iter().map(|(s, _)| s).collect();
    let sr = rel(stg);

    let mv_by: HashIdx<i64, usize> = (&mvr).map(|(((s, _), _), _): Mv| s).inv().collect();
    let hit: HashIdx<usize, Mv> = (&sr)
        .and((&sr).map(|((s, _), _): Stg| s).select(&mv_by).select(&mvr))
        .filt(|(((_, t), _), (((_, a), b), _)): (Stg, Mv)| t >= a && t <= b)
        .map(|(_, m): (Stg, Mv)| m)
        .collect();
    let out = (&sr).and((&hit).opt());
    drain(&out)
        .into_iter()
        .map(|(_, (((stay_id, charttime), ((p, a), ty)), m))| match m {
            Some((_, ((mp, ma), mty))) => Rrt {
                stay_id,
                charttime,
                dialysis_present: mp,
                dialysis_active: ma,
                dialysis_type: mty.or(ty),
            },
            None => Rrt { stay_id, charttime, dialysis_present: p, dialysis_active: a, dialysis_type: ty },
        })
        .collect()
}

pub fn q(db: &'static Db) -> String {
    rows(rrt(db).into_iter().map(|v| {
        row(vec![
            V::I(v.stay_id),
            V::T(v.charttime),
            V::I(v.dialysis_present),
            V::I(v.dialysis_active),
            ostr(v.dialysis_type),
        ])
    }))
}
