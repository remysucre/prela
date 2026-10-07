use crate::schema::*;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct Gcs {
    pub subject_id: i64,
    pub stay_id: i64,
    pub charttime: Ts,
    pub gcs: Option<f64>,
    pub gcs_motor: Option<f64>,
    pub gcs_verbal: Option<f64>,
    pub gcs_eyes: Option<f64>,
    pub gcs_unable: i64,
}

#[derive(Clone, Copy)]
struct Base {
    subject_id: i64,
    stay_id: i64,
    charttime: Ts,
    motor: Option<f64>,
    verbal: Option<f64>,
    eyes: Option<f64>,
    ett: i64,
}

fn max(a: Option<f64>, x: Option<f64>) -> Option<f64> {
    match (a, x) {
        (Some(a), Some(x)) => Some(a.max(x)),
        _ => a.or(x),
    }
}

pub fn gcs(db: &'static Db) -> Vec<Gcs> {
    let ce = &db.chart_event;
    let per = (&ce.id)
        .with((&ce.itemid).is_in([223900, 223901, 220739]))
        .group_by((&ce.subject_id).and(&ce.stay_id).and(&ce.charttime))
        .select((&ce.itemid).and((&ce.valuenum).opt()).and(&ce.value))
        .fold((None, None, None, 0i64), |(m, v, e, f), ((item, num), value)| {
            let ett = item == 223900 && value == "No Response-ETT";
            (
                max(m, if item == 223901 { num } else { None }),
                max(v, if ett { Some(0.0) } else if item == 223900 { num } else { None }),
                max(e, if item == 220739 { num } else { None }),
                f.max(ett as i64),
            )
        });
    let base = rel(drain(&per)
        .into_iter()
        .map(|(((subject_id, stay_id), charttime), (motor, verbal, eyes, ett))| Base {
            subject_id,
            stay_id,
            charttime,
            motor,
            verbal,
            eyes,
            ett,
        })
        .collect());
    let prev = (&base).map(|b: Base| b.stay_id).inv().select(&base).window(
        |g: &[(Ts, Base)], out: &mut Vec<Option<Base>>| {
            out.push(None);
            out.extend(g.windows(2).map(|w| Some(w[0].1)));
        },
        |b: Base| b.charttime,
        |a: &Ts, b: &Ts| a.cmp(b),
    );
    drain(&prev)
        .into_iter()
        .map(|(_, (b, p))| {
            let p = p.filter(|p| p.charttime > b.charttime - 6 * 3600 * US);
            let (pm, pv, pe) = p.map_or((None, None, None), |p| (p.motor, p.verbal, p.eyes));
            let gcs = if b.verbal == Some(0.0) || (b.verbal.is_none() && pv == Some(0.0)) {
                Some(15.0)
            } else if pv == Some(0.0) {
                Some((b.motor.unwrap_or(6.0) as f32 + b.verbal.unwrap_or(5.0) as f32 + b.eyes.unwrap_or(4.0) as f32) as f64)
            } else {
                Some(
                    (b.motor.or(pm).unwrap_or(6.0) as f32
                        + b.verbal.or(pv).unwrap_or(5.0) as f32
                        + b.eyes.or(pe).unwrap_or(4.0) as f32) as f64,
                )
            };
            Gcs {
                subject_id: b.subject_id,
                stay_id: b.stay_id,
                charttime: b.charttime,
                gcs,
                gcs_motor: b.motor.or(pm),
                gcs_verbal: b.verbal.or(pv),
                gcs_eyes: b.eyes.or(pe),
                gcs_unable: b.ett,
            }
        })
        .collect()
}

pub fn q(db: &'static Db) -> String {
    rows(gcs(db).into_iter().map(|g| {
        row(vec![
            V::I(g.subject_id),
            V::I(g.stay_id),
            V::T(g.charttime),
            ofloat(g.gcs),
            ofloat(g.gcs_motor),
            ofloat(g.gcs_verbal),
            ofloat(g.gcs_eyes),
            V::I(g.gcs_unable),
        ])
    }))
}
