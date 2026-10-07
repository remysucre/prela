use crate::concepts::anchor_tmpl_b as k;
use crate::concepts::office_visits__int_office_visits as ov;
use crate::schema::Db;

pub type OfficeVisitsIntOfficeVisitsPtotst = ov::OfficeVisitSub;

pub fn office_visits__int_office_visits_ptotst(_db: &'static Db, visits: &[ov::OfficeVisitsIntOfficeVisits]) -> Vec<OfficeVisitsIntOfficeVisitsPtotst> {
    ov::by_service_category_2(visits, "office-based pt/ot/st")
}

pub fn q(db: &'static Db) -> String {
    let s = k::stg(db);
    let v = ov::office_visits__int_office_visits(db, &s);
    harness::prelude::rows(office_visits__int_office_visits_ptotst(db, &v).iter().map(ov::fmt_sub))
}
