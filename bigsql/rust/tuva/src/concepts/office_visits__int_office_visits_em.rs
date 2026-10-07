use crate::concepts::anchor_tmpl_b as k;
use crate::concepts::office_visits__int_office_visits as ov;
use crate::schema::Db;

pub type OfficeVisitsIntOfficeVisitsEm = ov::OfficeVisitSub;

pub fn office_visits__int_office_visits_em(_db: &'static Db, visits: &[ov::OfficeVisitsIntOfficeVisits]) -> Vec<OfficeVisitsIntOfficeVisitsEm> {
    ov::by_service_category_2(visits, "office-based visit")
}

pub fn q(db: &'static Db) -> String {
    let s = k::stg(db);
    let v = ov::office_visits__int_office_visits(db, &s);
    harness::prelude::rows(office_visits__int_office_visits_em(db, &v).iter().map(ov::fmt_sub))
}
