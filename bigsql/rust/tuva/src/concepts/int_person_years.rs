use crate::concepts::member_month::MemberMonth;
use crate::schema::*;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct IntPersonYears {
    pub person_id: Str,
    pub data_source: Str,
    pub performance_year: i64,
}

pub fn int_person_years(_db: &'static Db, mm: &[MemberMonth]) -> Vec<IntPersonYears> {
    let m = rel(mm.to_vec());
    let g = (&m)
        .map(|x: MemberMonth| (x.person_id, x.data_source, &x.year_month[..4]))
        .inv()
        .map(|_| ())
        .fold((), |a, _| a);
    drain(&g)
        .into_iter()
        .map(|((person_id, data_source, y), _)| IntPersonYears { person_id, data_source, performance_year: y.parse().unwrap() })
        .collect()
}

pub fn fmt(v: &IntPersonYears) -> String {
    row(vec![V::S(v.person_id), V::S(v.data_source), V::I(v.performance_year)])
}

pub fn member_month_chain(db: &'static Db) -> Vec<MemberMonth> {
    let el = crate::concepts::eligibility::eligibility(db);
    let il = crate::concepts::input_layer__eligibility::input_layer__eligibility(db, &el);
    let ec = crate::concepts::int_eligibility_casting::int_eligibility_casting(db, &il);
    let dn = crate::concepts::int_eligibility_dates_normalized::int_eligibility_dates_normalized(db, &ec);
    let sn = crate::concepts::int_eligibility_state_normalized::int_eligibility_state_normalized(db, &ec);
    let ne = crate::concepts::normalized__eligibility::normalized__eligibility(db, &ec, &dn, &sn);
    let sp = crate::concepts::member_month__month_spine::member_month__month_spine(db);
    crate::concepts::member_month::member_month(db, &ne, &sp)
}

pub fn q(db: &'static Db) -> String {
    let mm = member_month_chain(db);
    rows(int_person_years(db, &mm).iter().map(fmt))
}
