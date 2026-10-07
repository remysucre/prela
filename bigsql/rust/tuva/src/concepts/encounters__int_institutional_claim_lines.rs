use crate::concepts::acute_inpatient__generate_encounter_id::*;
use crate::concepts::emergency_department__generate_encounter_id::*;
use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::concepts::inpatient_common::GenerateEncounterId;
use crate::schema::Db;
use harness::prelude::*;

type M = EncountersStgMedicalClaim;
type A = AcuteInpatientGenerateEncounterId;
type E = EmergencyDepartmentGenerateEncounterId;
type G = GenerateEncounterId;

#[derive(Clone, Copy)]
pub struct EncountersIntInstitutionalClaimLines {
    pub claim_id: Str,
    pub claim_line_number: i64,
    pub data_source: Str,
    pub encounter_id: Str,
    pub encounter_type: Str,
    pub encounter_group: Str,
    pub priority_number: i64,
    pub anchor_claim_id: Option<Str>,
}

#[derive(Clone, Copy)]
struct U {
    claim_id: Str,
    patient_data_source_id: Str,
    encounter_id: Str,
    encounter_type: Str,
    encounter_group: Str,
    priority_number: i64,
    anchor_claim_id: Option<Str>,
}

fn tag(t: &'static str, p: i64) -> impl Fn(G) -> U + Copy {
    move |x: G| U {
        claim_id: x.claim_id,
        patient_data_source_id: x.patient_data_source_id,
        encounter_id: x.encounter_id,
        encounter_type: t,
        encounter_group: "inpatient",
        priority_number: p,
        anchor_claim_id: None,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn encounters__int_institutional_claim_lines(
    _db: &'static Db,
    stg: &[M],
    acute: &[A],
    ed: &[E],
    hospice: &[G],
    psych: &[G],
    rehab: &[G],
    long_term: &[G],
    snf: &[G],
    substance_use: &[G],
) -> Vec<EncountersIntInstitutionalClaimLines> {
    let a = rel(acute.to_vec());
    let e = rel(ed.to_vec());
    let ua = (&a).map(|x: A| U {
        claim_id: x.claim_id,
        patient_data_source_id: x.patient_data_source_id,
        encounter_id: x.encounter_id,
        encounter_type: "acute inpatient",
        encounter_group: "inpatient",
        priority_number: 0,
        anchor_claim_id: None,
    });
    let ue = (&e).map(|x: E| U {
        claim_id: x.claim_id,
        patient_data_source_id: x.patient_data_source_id,
        encounter_id: x.encounter_id,
        encounter_type: "emergency department",
        encounter_group: "outpatient",
        priority_number: 1,
        anchor_claim_id: x.original_anchor_claim,
    });
    let h = rel(hospice.to_vec());
    let p = rel(psych.to_vec());
    let r = rel(rehab.to_vec());
    let l = rel(long_term.to_vec());
    let s = rel(snf.to_vec());
    let su = rel(substance_use.to_vec());
    let u = rel(drain(
        ua.union(ue)
            .union((&h).map(tag("inpatient hospice", 1)))
            .union((&p).map(tag("inpatient psych", 2)))
            .union((&r).map(tag("inpatient rehabilitation", 3)))
            .union((&l).map(tag("inpatient long term acute care", 4)))
            .union((&s).map(tag("inpatient skilled nursing", 5)))
            .union((&su).map(tag("inpatient substance use", 6))),
    )
    .into_iter()
    .map(|(_, x)| x)
    .collect::<Vec<U>>());
    let m = rel(stg.to_vec());
    let mx: HashIdx<(Str, Str), usize> = (&m)
        .map(|x: M| (x.claim_id, x.patient_data_source_id))
        .inv()
        .collect();
    let j = (&u).and(
        (&u).map(|x: U| (x.claim_id, x.patient_data_source_id))
            .select(&mx)
            .select(&m),
    );
    drain(&j)
        .into_iter()
        .map(
            |(_, (x, med)): (usize, (U, M))| EncountersIntInstitutionalClaimLines {
                claim_id: x.claim_id,
                claim_line_number: med.claim_line_number,
                data_source: med.data_source,
                encounter_id: x.encounter_id,
                encounter_type: x.encounter_type,
                encounter_group: x.encounter_group,
                priority_number: x.priority_number,
                anchor_claim_id: x.anchor_claim_id,
            },
        )
        .collect()
}

pub fn fmt(v: &EncountersIntInstitutionalClaimLines) -> String {
    row(vec![
        V::S(v.claim_id),
        V::I(v.claim_line_number),
        V::S(v.data_source),
        V::S(v.encounter_id),
        V::S(v.encounter_type),
        V::S(v.encounter_group),
        V::I(v.priority_number),
        ostr(v.anchor_claim_id),
    ])
}

pub fn q(db: &'static Db) -> String {
    let stg = stg_medical_claim(db);
    let acute = acute_inpatient__generate_encounter_id(db, &stg);
    let pre = crate::concepts::emergency_department__generate_encounter_id_pre_sort::emergency_department__generate_encounter_id_pre_sort(db, &stg);
    let ed = emergency_department__generate_encounter_id(db, &pre);
    let hospice = crate::concepts::inpatient_hospice__generate_encounter_id::inpatient_hospice__generate_encounter_id(db, &stg);
    let psych = crate::concepts::inpatient_psych__generate_encounter_id::inpatient_psych__generate_encounter_id(db, &stg);
    let rehab = crate::concepts::inpatient_rehab__generate_encounter_id::inpatient_rehab__generate_encounter_id(db, &stg);
    let long_term = crate::concepts::inpatient_long_term__generate_encounter_id::inpatient_long_term__generate_encounter_id(db, &stg);
    let snf =
        crate::concepts::inpatient_snf__generate_encounter_id::inpatient_snf__generate_encounter_id(
            db, &stg,
        );
    let substance_use = crate::concepts::inpatient_substance_use__generate_encounter_id::inpatient_substance_use__generate_encounter_id(db, &stg);
    rows(
        encounters__int_institutional_claim_lines(
            db,
            &stg,
            &acute,
            &ed,
            &hospice,
            &psych,
            &rehab,
            &long_term,
            &snf,
            &substance_use,
        )
        .iter()
        .map(fmt),
    )
}
