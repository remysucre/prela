use crate::concepts::icustay_times::*;
use crate::schema::*;
use harness::prelude::*;

const HOUR_US: i64 = 3600 * US;

#[derive(Clone, Copy)]
pub struct IcustayHourly {
    pub stay_id: i64,
    pub hr: i64,
    pub endtime: Ts,
}

pub fn icustay_hourly(_db: &'static Db, it: &[IcustayTimes]) -> Vec<IcustayHourly> {
    let r = rel(it.to_vec());
    let hours = (&r).flat_map(|t: IcustayTimes| {
        let span = match (t.intime_hr, t.outtime_hr) {
            (Some(a), Some(b)) => {
                let tr = a.div_euclid(HOUR_US) * HOUR_US;
                let endtime = if tr == a { a } else { tr + HOUR_US };
                let n = b.div_euclid(HOUR_US) - a.div_euclid(HOUR_US);
                Some((endtime, n))
            }
            _ => None,
        };
        span.into_iter().flat_map(move |(endtime, n)| {
            (-24..=n).map(move |hr| IcustayHourly { stay_id: t.stay_id, hr, endtime: endtime + hr * HOUR_US })
        })
    });
    drain(&hours).into_iter().map(|(_, h)| h).collect()
}

pub fn q(db: &'static Db) -> String {
    let it = crate::concepts::icustay_times::icustay_times(db);
    rows(icustay_hourly(db, &it).into_iter().map(|v| row(vec![V::I(v.stay_id), V::I(v.hr), V::T(v.endtime)])))
}
