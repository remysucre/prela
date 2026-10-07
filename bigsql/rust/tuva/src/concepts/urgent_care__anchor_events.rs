use crate::concepts::anchor_tmpl_b as k;
use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::schema::Db;
use harness::prelude::*;

type M = EncountersStgMedicalClaim;

#[derive(Clone, Copy)]
pub struct UrgentCareAnchorEvents {
    pub claim_id: Str,
    pub data_source: Str,
}

pub fn urgent_care__anchor_events(_db: &'static Db, stg: &[M]) -> Vec<UrgentCareAnchorEvents> {
    let m = rel(stg.to_vec());
    let d = (&m)
        .filt(|x: M| x.service_category_2 == "urgent care")
        .map(|x: M| (x.claim_id, x.data_source))
        .inv()
        .map(|_| ())
        .fold((), |a, _| a);
    drain(&d)
        .into_iter()
        .map(|((claim_id, data_source), _)| UrgentCareAnchorEvents { claim_id, data_source })
        .collect()
}

pub fn q(db: &'static Db) -> String {
    let s = k::stg(db);
    rows(urgent_care__anchor_events(db, &s).iter().map(|v| row(vec![V::S(v.claim_id), V::S(v.data_source)])))
}
