use crate::schema::*;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct Bg {
    pub subject_id: i64,
    pub hadm_id: Option<i64>,
    pub charttime: Ts,
    pub specimen: Option<Str>,
    pub so2: Option<f64>,
    pub po2: Option<f64>,
    pub pco2: Option<f64>,
    pub fio2_chartevents: Option<f64>,
    pub fio2: Option<f64>,
    pub aado2: Option<f64>,
    pub aado2_calc: Option<f64>,
    pub pao2fio2ratio: Option<f64>,
    pub ph: Option<f64>,
    pub baseexcess: Option<f64>,
    pub bicarbonate: Option<f64>,
    pub totalco2: Option<f64>,
    pub hematocrit: Option<f64>,
    pub hemoglobin: Option<f64>,
    pub carboxyhemoglobin: Option<f64>,
    pub methemoglobin: Option<f64>,
    pub chloride: Option<f64>,
    pub calcium: Option<f64>,
    pub temperature: Option<f64>,
    pub potassium: Option<f64>,
    pub sodium: Option<f64>,
    pub lactate: Option<f64>,
    pub glucose: Option<f64>,
}

const ITEMS: [i64; 25] = [
    52033, 50801, 50802, 50803, 50804, 50805, 50806, 50807, 50808, 50809, 50810, 50811, 50813,
    50814, 50815, 50816, 50817, 50818, 50819, 50820, 50821, 50822, 50823, 50824, 50825,
];

const NUM: [i64; 20] = [
    50817, 50821, 50818, 50816, 50801, 50820, 50802, 50803, 50804, 50810, 50811, 50805, 50814,
    50806, 50808, 50825, 50822, 50824, 50813, 50809,
];

const HOUR_US: i64 = 3600 * US;

#[derive(Clone, Copy)]
struct Lab {
    subject_id: i64,
    hadm_id: Option<i64>,
    charttime: Ts,
    specimen: Option<Str>,
    num: [Option<f64>; 20],
    spo2: Option<f64>,
}

#[derive(Clone, Copy)]
struct Spo2 {
    subject_id: i64,
    charttime: Ts,
    spo2: Option<f64>,
}

#[derive(Clone, Copy)]
struct Fio2 {
    subject_id: i64,
    charttime: Ts,
    fio2: Option<f64>,
}

fn omax<T: PartialOrd + Copy>(a: Option<T>, x: Option<T>) -> Option<T> {
    match (a, x) {
        (Some(a), Some(x)) => Some(if x > a { x } else { a }),
        (a, None) => a,
        (None, x) => x,
    }
}

fn clean(item: i64, v: Option<f64>) -> Option<f64> {
    let v = v?;
    match item {
        50809 | 50813 => (v <= 10000.0).then_some(v),
        50810 | 50817 => (v <= 100.0).then_some(v),
        50816 => {
            if v > 20.0 && v <= 100.0 {
                Some(v)
            } else if v > 0.2 && v <= 1.0 {
                Some(v * 100.0)
            } else {
                None
            }
        }
        _ => Some(v),
    }
}

pub fn bg(db: &'static Db) -> Vec<Bg> {
    let le = &db.lab_event;
    let init = Lab {
        subject_id: i64::MIN,
        hadm_id: None,
        charttime: i64::MIN,
        specimen: None,
        num: [None; 20],
        spo2: None,
    };
    let per = (&le.id)
        .with((&le.itemid).is_in(ITEMS))
        .group_by(&le.specimen_id)
        .select(
            (&le.subject_id)
                .and((&le.hadm_id).opt())
                .and(&le.charttime)
                .and(&le.itemid)
                .and((&le.value).opt())
                .and((&le.valuenum).opt()),
        )
        .fold(
            init,
            |mut a, (((((subject_id, hadm_id), charttime), item), value), v)| {
                a.subject_id = a.subject_id.max(subject_id);
                a.hadm_id = omax(a.hadm_id, hadm_id);
                a.charttime = a.charttime.max(charttime);
                if item == 52033 {
                    a.specimen = omax(a.specimen, value);
                }
                if let Some(i) = NUM.iter().position(|&x| x == item) {
                    a.num[i] = omax(a.num[i], clean(item, v));
                }
                a
            },
        );
    let labs: Vec<Lab> = drain((&per).filt(|a: Lab| a.num[1].is_some()))
        .into_iter()
        .map(|(_, a)| a)
        .collect();

    let ce = &db.chart_event;
    let spo2_per = (&ce.id)
        .with((&ce.itemid).is_in([220277]))
        .with((&ce.valuenum).filt(|v| v > 0.0 && v <= 100.0))
        .group_by((&ce.subject_id).and(&ce.charttime))
        .select(&ce.valuenum)
        .fold(((0.0, 0.0), 0i64), |a: ((f64, f64), i64), v: f64| {
            (kahan(a.0, v), a.1 + 1)
        });
    let spo2s: Vec<Spo2> = drain(&spo2_per)
        .into_iter()
        .map(|((subject_id, charttime), a)| Spo2 {
            subject_id,
            charttime,
            spo2: (a.1 > 0).then(|| a.0 .0 / a.1 as f64),
        })
        .collect();

    let l = rel(labs);
    let sp = rel(spo2s);
    let sp_by_subject: HashIdx<i64, usize> = (&sp).map(|x: Spo2| x.subject_id).inv().collect();
    let last_spo2 = (&l)
        .and(
            (&l).map(|x: Lab| x.subject_id)
                .select(&sp_by_subject)
                .select(&sp),
        )
        .filt(|(x, s): (Lab, Spo2)| {
            s.charttime >= x.charttime - 2 * HOUR_US && s.charttime <= x.charttime
        })
        .window(
            row_number,
            |(_, s): (Lab, Spo2)| s.charttime,
            |a: &Ts, b: &Ts| b.cmp(a),
        );
    let first_spo2 = (&last_spo2).filt(|(_, rn)| rn == 1);
    let mut stg2 = Vec::new();
    (&l).and((&first_spo2).opt()).drive(|_, (x, m)| {
        stg2.push(Lab {
            spo2: m.and_then(|((_, s), _)| s.spo2),
            ..x
        });
    });

    let fio2_per = (&ce.id)
        .with((&ce.itemid).is_in([223835]))
        .with((&ce.valuenum).filt(|v| v > 0.0 && v <= 100.0))
        .group_by((&ce.subject_id).and(&ce.charttime))
        .select(&ce.valuenum)
        .fold(None, |a, v: f64| {
            let x = if v as f32 > 0.2f32 && v <= 1.0 {
                Some((v as f32 * 100f32) as f64)
            } else if v >= 20.0 && v <= 100.0 {
                Some(v)
            } else {
                None
            };
            omax(a, x)
        });
    let fio2s: Vec<Fio2> = drain(&fio2_per)
        .into_iter()
        .map(|((subject_id, charttime), fio2)| Fio2 {
            subject_id,
            charttime,
            fio2,
        })
        .collect();

    let b = rel(stg2);
    let f = rel(fio2s);
    let by_subject: HashIdx<i64, usize> = (&f).map(|x: Fio2| x.subject_id).inv().collect();
    let last = (&b)
        .and(
            (&b).map(|x: Lab| x.subject_id)
                .select(&by_subject)
                .select(&f),
        )
        .filt(|(x, s): (Lab, Fio2)| {
            s.fio2.is_some_and(|v| v > 0.0)
                && s.charttime >= x.charttime - 4 * HOUR_US
                && s.charttime <= x.charttime
        })
        .window(
            row_number,
            |(_, s): (Lab, Fio2)| s.charttime,
            |a: &Ts, b: &Ts| b.cmp(a),
        );
    let first = (&last).filt(|(_, rn)| rn == 1);

    let mut out = Vec::new();
    (&b).and((&first).opt()).drive(|_, (x, m)| {
        let fio2_chartevents = m.and_then(|((_, s), _)| s.fio2);
        let n = x.num;
        let (po2, pco2, fio2) = (n[1], n[2], n[3]);
        let aado2_calc = match (po2, pco2) {
            (Some(po2), Some(pco2)) => match (fio2, fio2_chartevents) {
                (Some(f), _) => Some((f / 100.0) * 713.0 - pco2 / 0.8 - po2),
                (None, Some(f)) => Some(((f as f32 / 100f32) * 713f32) as f64 - pco2 / 0.8 - po2),
                _ => None,
            },
            _ => None,
        };
        let pao2fio2ratio = po2.and_then(|po2| match (fio2, fio2_chartevents) {
            (Some(f), _) => Some(100.0 * po2 / f),
            (None, Some(f)) => Some(100.0 * po2 / f),
            _ => None,
        });
        out.push(Bg {
            subject_id: x.subject_id,
            hadm_id: x.hadm_id,
            charttime: x.charttime,
            specimen: x.specimen,
            so2: n[0],
            po2,
            pco2,
            fio2_chartevents,
            fio2,
            aado2: n[4],
            aado2_calc: aado2_calc.map(|v| round_dec(v, 4)),
            pao2fio2ratio,
            ph: n[5],
            baseexcess: n[6],
            bicarbonate: n[7],
            totalco2: n[8],
            hematocrit: n[9],
            hemoglobin: n[10],
            carboxyhemoglobin: n[11],
            methemoglobin: n[12],
            chloride: n[13],
            calcium: n[14],
            temperature: n[15],
            potassium: n[16],
            sodium: n[17],
            lactate: n[18],
            glucose: n[19],
        })
    });
    out
}

pub fn q(db: &'static Db) -> String {
    rows(bg(db).into_iter().map(|v| {
        row(vec![
            V::I(v.subject_id),
            oint(v.hadm_id),
            V::T(v.charttime),
            ostr(v.specimen),
            ofloat(v.so2),
            ofloat(v.po2),
            ofloat(v.pco2),
            ofloat(v.fio2_chartevents),
            ofloat(v.fio2),
            ofloat(v.aado2),
            ofloat(v.aado2_calc),
            ofloat(v.pao2fio2ratio),
            ofloat(v.ph),
            ofloat(v.baseexcess),
            ofloat(v.bicarbonate),
            ofloat(v.totalco2),
            ofloat(v.hematocrit),
            ofloat(v.hemoglobin),
            ofloat(v.carboxyhemoglobin),
            ofloat(v.methemoglobin),
            ofloat(v.chloride),
            ofloat(v.calcium),
            ofloat(v.temperature),
            ofloat(v.potassium),
            ofloat(v.sodium),
            ofloat(v.lactate),
            ofloat(v.glucose),
        ])
    }))
}
