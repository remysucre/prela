use crate::schema::Db;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct MemberMonthMonthSpine {
    pub year_month_int: i64,
    pub year: i64,
    pub month: i64,
    pub first_day_of_month: Date,
    pub last_day_of_month: Date,
}

impl MemberMonthMonthSpine {
    pub fn year_month(&self) -> String {
        format!("{:04}{:02}", self.year, self.month)
    }
}

pub fn member_month__month_spine(_db: &'static Db) -> Vec<MemberMonthMonthSpine> {
    let digits = || rel((0i64..10).collect::<Vec<_>>());
    let (ones, tens, hundreds, thousands) = (digits(), digits(), digits(), digits());
    let base = date(1900, 1, 1);
    let q = (&ones)
        .cross(&tens)
        .cross(&hundreds)
        .cross(&thousands)
        .map(|(((o, t), h), th): (((i64, i64), i64), i64)| o + t * 10 + h * 100 + th * 1000)
        .filt(|m: i64| m < 2412)
        .map(|m: i64| {
            let first = add_months(base, m);
            let (y, mo) = (year(first), month(first));
            MemberMonthMonthSpine {
                year_month_int: y * 100 + mo,
                year: y,
                month: mo,
                first_day_of_month: first,
                last_day_of_month: add_days(add_months(first, 1), -1),
            }
        });
    drain(&q).into_iter().map(|(_, r)| r).collect()
}

pub fn q(db: &'static Db) -> String {
    rows(member_month__month_spine(db).iter().map(|v| {
        row(vec![
            V::Owned(v.year_month()),
            V::I(v.year_month_int),
            V::I(v.year),
            V::I(v.month),
            V::D(v.first_day_of_month),
            V::D(v.last_day_of_month),
        ])
    }))
}
