use crate::concepts::member_month::md5_hex;
use crate::concepts::normalized__eligibility::NormalizedEligibility;
use crate::concepts::normalized__medical_claim::NormalizedMedicalClaim;
use crate::schema::Db;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct EncountersPatientDataSourceId {
    pub person_id: Str,
    pub data_source: Str,
    pub patient_data_source_id: Str,
}

fn leak(s: String) -> Str {
    Box::leak(s.into_boxed_str())
}

fn part(s: &str) -> String {
    format!("V{}", s.replace('%', "%25").replace('|', "%7C"))
}

pub fn encounters__patient_data_source_id(
    _db: &'static Db,
    nmc: &[NormalizedMedicalClaim],
    ne: &[NormalizedEligibility],
) -> Vec<EncountersPatientDataSourceId> {
    let m = rel(nmc.to_vec());
    let e = rel(ne.to_vec());
    let u = (&m)
        .map(|x: NormalizedMedicalClaim| (x.person_id, x.data_source))
        .union((&e).map(|x: NormalizedEligibility| (x.person_id, x.data_source)))
        .inv()
        .map(|_| ())
        .fold((), |a, _| a);
    drain(&u)
        .into_iter()
        .map(|((person_id, data_source), _)| EncountersPatientDataSourceId {
            person_id,
            data_source,
            patient_data_source_id: leak(md5_hex(
                format!("{}|{}|{}", part("patient data source"), part(person_id), part(data_source)).as_bytes(),
            )),
        })
        .collect()
}

pub fn fmt(v: &EncountersPatientDataSourceId) -> String {
    row(vec![V::S(v.person_id), V::S(v.data_source), V::S(v.patient_data_source_id)])
}

pub fn q(db: &'static Db) -> String {
    let mc = crate::concepts::medical_claim::medical_claim(db);
    let il = crate::concepts::input_layer__medical_claim::input_layer__medical_claim(db, &mc);
    let stg = crate::concepts::normalized_input__stg_medical_claim::normalized_input__stg_medical_claim(db, &il);
    let nmc = crate::concepts::normalized__medical_claim::normalized__medical_claim_from_stg(db, &stg);
    let ne = normalized_eligibility(db);
    rows(encounters__patient_data_source_id(db, &nmc, &ne).iter().map(fmt))
}

pub fn normalized_eligibility(db: &'static Db) -> Vec<NormalizedEligibility> {
    let el = crate::concepts::eligibility::eligibility(db);
    let il = crate::concepts::input_layer__eligibility::input_layer__eligibility(db, &el);
    let ec = crate::concepts::int_eligibility_casting::int_eligibility_casting(db, &il);
    let dn = crate::concepts::int_eligibility_dates_normalized::int_eligibility_dates_normalized(db, &ec);
    let sn = crate::concepts::int_eligibility_state_normalized::int_eligibility_state_normalized(db, &ec);
    crate::concepts::normalized__eligibility::normalized__eligibility(db, &ec, &dn, &sn)
}
