use crate::schema::*;
use harness::prelude::*;
use std::cmp::Ordering;

#[derive(Clone, Copy)]
pub struct OxygenDelivery {
    pub subject_id: i64,
    pub stay_id: i64,
    pub charttime: Ts,
    pub o2_flow: Option<f64>,
    pub o2_flow_additional: Option<f64>,
    pub o2_delivery_device_1: Option<Str>,
    pub o2_delivery_device_2: Option<Str>,
    pub o2_delivery_device_3: Option<Str>,
    pub o2_delivery_device_4: Option<Str>,
}

#[derive(Clone, Copy)]
struct Flow {
    subject_id: i64,
    charttime: Ts,
    stay_id: i64,
    itemid: i64,
    valuenum: Option<f64>,
}

fn desc_nulls_last<T: PartialOrd>(a: &Option<T>, b: &Option<T>) -> Ordering {
    match (a, b) {
        (Some(a), Some(b)) => b.partial_cmp(a).unwrap_or(Ordering::Equal),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn omax(a: Option<f64>, x: Option<f64>) -> Option<f64> {
    match (a, x) {
        (Some(a), Some(x)) => Some(a.max(x)),
        (a, None) => a,
        (None, x) => x,
    }
}

pub fn oxygen_delivery(db: &'static Db) -> Vec<OxygenDelivery> {
    let ce = &db.chart_event;
    let flows = (&ce.id)
        .with((&ce.itemid).is_in([223834, 227582, 227287]))
        .group_by(
            (&ce.subject_id)
                .and(&ce.charttime)
                .and((&ce.itemid).map(|i: i64| if i == 227582 { 223834 } else { i })),
        )
        .select((&ce.stay_id).and((&ce.valuenum).opt()).and((&ce.storetime).opt()))
        .window(
            row_number,
            |((_, v), st): ((i64, Option<f64>), Option<Ts>)| (st, v),
            |a: &(Option<Ts>, Option<f64>), b: &(Option<Ts>, Option<f64>)| {
                desc_nulls_last(&a.0, &b.0).then_with(|| desc_nulls_last(&a.1, &b.1))
            },
        );
    let mut win: Vec<Flow> = Vec::new();
    (&flows).filt(|(_, rn)| rn == 1).drive(|((subject_id, charttime), itemid), (((stay_id, valuenum), _), _)| {
        win.push(Flow { subject_id, charttime, stay_id, itemid, valuenum })
    });

    let devices = (&ce.id)
        .with((&ce.itemid).is_in([226732]))
        .group_by((&ce.subject_id).and(&ce.charttime))
        .select(((&ce.storetime).opt()).and(&ce.value))
        .window(
            row_number,
            |x: (Option<Ts>, Str)| x,
            |a: &(Option<Ts>, Str), b: &(Option<Ts>, Str)| desc_nulls_last(&a.0, &b.0).then_with(|| b.1.cmp(a.1)),
        );

    let n = win.len();
    let w = rel(win);
    let key = (&w).map(|f: Flow| (f.subject_id, f.charttime));
    let stg = (&w).and((&w).map(|f: Flow| (f.subject_id, f.charttime)).select((&devices).opt()));
    let init = (i64::MIN, None, None, [None::<Str>; 4]);
    let per = Universe::new(n).group_by(key).select(stg).fold(
        init,
        |(s, a, b, mut d), (f, dev): (Flow, Option<((Option<Ts>, Str), i64)>)| {
            if let Some(((_, value), rn)) = dev {
                if (1..=4).contains(&rn) {
                    let x = &mut d[(rn - 1) as usize];
                    *x = Some(x.map_or(value, |x: Str| x.max(value)));
                }
            }
            match f.itemid {
                223834 => (s.max(f.stay_id), omax(a, f.valuenum), b, d),
                227287 => (s.max(f.stay_id), a, omax(b, f.valuenum), d),
                _ => (s.max(f.stay_id), a, b, d),
            }
        },
    );
    drain(&per)
        .into_iter()
        .map(|((subject_id, charttime), (stay_id, o2_flow, o2_flow_additional, d))| OxygenDelivery {
            subject_id,
            stay_id,
            charttime,
            o2_flow,
            o2_flow_additional,
            o2_delivery_device_1: d[0],
            o2_delivery_device_2: d[1],
            o2_delivery_device_3: d[2],
            o2_delivery_device_4: d[3],
        })
        .collect()
}

pub fn q(db: &'static Db) -> String {
    rows(oxygen_delivery(db).into_iter().map(|v| {
        row(vec![
            V::I(v.subject_id),
            V::I(v.stay_id),
            V::T(v.charttime),
            ofloat(v.o2_flow),
            ofloat(v.o2_flow_additional),
            ostr(v.o2_delivery_device_1),
            ostr(v.o2_delivery_device_2),
            ostr(v.o2_delivery_device_3),
            ostr(v.o2_delivery_device_4),
        ])
    }))
}
