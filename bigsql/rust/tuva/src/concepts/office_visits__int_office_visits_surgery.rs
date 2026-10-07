use crate::concepts::anchor_tmpl_b as k;
use crate::concepts::office_visits__int_office_visits as ov;
use crate::schema::Db;

pub type OfficeVisitsIntOfficeVisitsSurgery = ov::OfficeVisitSub;

pub fn office_visits__int_office_visits_surgery(_db: &'static Db, visits: &[ov::OfficeVisitsIntOfficeVisits]) -> Vec<OfficeVisitsIntOfficeVisitsSurgery> {
    ov::by_service_category_2(visits, "office-based surgery")
}

pub fn q(db: &'static Db) -> String {
    let s = k::stg(db);
    let v = ov::office_visits__int_office_visits(db, &s);
    harness::prelude::rows(office_visits__int_office_visits_surgery(db, &v).iter().map(ov::fmt_sub))
}
