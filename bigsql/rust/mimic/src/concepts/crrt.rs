use crate::schema::*;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct Crrt {
    pub stay_id: i64,
    pub charttime: Ts,
    pub crrt_mode: Option<Str>,
    pub access_pressure: Option<f64>,
    pub blood_flow: Option<f64>,
    pub citrate: Option<f64>,
    pub current_goal: Option<f64>,
    pub dialysate_fluid: Option<Str>,
    pub dialysate_rate: Option<f64>,
    pub effluent_pressure: Option<f64>,
    pub filter_pressure: Option<f64>,
    pub heparin_concentration: Option<Str>,
    pub heparin_dose: Option<f64>,
    pub hourly_patient_fluid_removal: Option<f64>,
    pub prefilter_replacement_rate: Option<f64>,
    pub postfilter_replacement_rate: Option<f64>,
    pub replacement_fluid: Option<Str>,
    pub replacement_rate: Option<f64>,
    pub return_pressure: Option<f64>,
    pub ultrafiltrate_output: Option<f64>,
    pub system_active: Option<i64>,
    pub clots: Option<i64>,
    pub clots_increasing: Option<i64>,
    pub clotted: Option<i64>,
}

const ITEMS: [i64; 19] = [
    227290, 224146, 224149, 224144, 228004, 225183, 225977, 224154, 224151, 224150, 225958, 224145, 224191, 228005,
    228006, 225976, 224153, 224152, 226457,
];

const NUM_ITEMS: [i64; 14] = [
    224149, 224144, 228004, 225183, 224154, 224151, 224150, 224145, 224191, 228005, 228006, 224153, 224152, 226457,
];

const STR_ITEMS: [i64; 4] = [227290, 225977, 225958, 225976];

#[derive(Clone, Copy)]
struct Acc {
    num: [Option<f64>; 14],
    s: [Option<Str>; 4],
    flags: [Option<i64>; 4],
}

fn mx<T: PartialOrd + Copy>(a: Option<T>, x: Option<T>) -> Option<T> {
    match (a, x) {
        (Some(a), Some(x)) => Some(if x > a { x } else { a }),
        (None, x) => x,
        (a, None) => a,
    }
}

pub fn crrt(db: &'static Db) -> Vec<Crrt> {
    let ce = &db.chart_event;
    let init = Acc { num: [None; 14], s: [None; 4], flags: [None; 4] };
    let per = (&ce.id)
        .with((&ce.itemid).is_in(ITEMS))
        .group_by((&ce.stay_id).and(&ce.charttime))
        .select((&ce.itemid).and((&ce.valuenum).opt()).and(&ce.value))
        .fold(init, |mut a, ((item, v), value)| {
            for (i, &it) in NUM_ITEMS.iter().enumerate() {
                if item == it {
                    a.num[i] = mx(a.num[i], v);
                }
            }
            for (i, &it) in STR_ITEMS.iter().enumerate() {
                if item == it {
                    a.s[i] = mx(a.s[i], Some(value));
                }
            }
            if item == 224146 {
                let f: [Option<i64>; 4] = [
                    match value {
                        "Active" | "Initiated" | "Reinitiated" | "New Filter" => Some(1),
                        "Recirculating" | "Discontinued" => Some(0),
                        _ => None,
                    },
                    match value {
                        "Clots Present" => Some(1),
                        "No Clot Present" => Some(0),
                        _ => None,
                    },
                    match value {
                        "Clots Increasing" | "Clot Increasing" => Some(1),
                        _ => None,
                    },
                    match value {
                        "Clotted" => Some(1),
                        _ => None,
                    },
                ];
                for (s, x) in a.flags.iter_mut().zip(f) {
                    *s = mx(*s, x);
                }
            }
            a
        });
    drain(&per)
        .into_iter()
        .map(|((stay_id, charttime), a)| Crrt {
            stay_id,
            charttime,
            crrt_mode: a.s[0],
            access_pressure: a.num[0],
            blood_flow: a.num[1],
            citrate: a.num[2],
            current_goal: a.num[3],
            dialysate_fluid: a.s[1],
            dialysate_rate: a.num[4],
            effluent_pressure: a.num[5],
            filter_pressure: a.num[6],
            heparin_concentration: a.s[2],
            heparin_dose: a.num[7],
            hourly_patient_fluid_removal: a.num[8],
            prefilter_replacement_rate: a.num[9],
            postfilter_replacement_rate: a.num[10],
            replacement_fluid: a.s[3],
            replacement_rate: a.num[11],
            return_pressure: a.num[12],
            ultrafiltrate_output: a.num[13],
            system_active: a.flags[0],
            clots: a.flags[1],
            clots_increasing: a.flags[2],
            clotted: a.flags[3],
        })
        .collect()
}

pub fn q(db: &'static Db) -> String {
    rows(crrt(db).into_iter().map(|v| {
        row(vec![
            V::I(v.stay_id),
            V::T(v.charttime),
            ostr(v.crrt_mode),
            ofloat(v.access_pressure),
            ofloat(v.blood_flow),
            ofloat(v.citrate),
            ofloat(v.current_goal),
            ostr(v.dialysate_fluid),
            ofloat(v.dialysate_rate),
            ofloat(v.effluent_pressure),
            ofloat(v.filter_pressure),
            ostr(v.heparin_concentration),
            ofloat(v.heparin_dose),
            ofloat(v.hourly_patient_fluid_removal),
            ofloat(v.prefilter_replacement_rate),
            ofloat(v.postfilter_replacement_rate),
            ostr(v.replacement_fluid),
            ofloat(v.replacement_rate),
            ofloat(v.return_pressure),
            ofloat(v.ultrafiltrate_output),
            oint(v.system_active),
            oint(v.clots),
            oint(v.clots_increasing),
            oint(v.clotted),
        ])
    }))
}
