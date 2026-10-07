use crate::concepts as c;
use crate::concepts::anchor_kit::{surrogate_key, ClaimMatch};
use crate::concepts::anchor_tmpl_b::MatchClaim;
use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::concepts::inpatient_common::ProfClaims;
use crate::schema::Db;
use harness::prelude::*;
use std::cmp::Ordering;

type M = EncountersStgMedicalClaim;
type AP = c::acute_inpatient__prof_claims::AcuteInpatientProfClaims;
type EP = c::emergency_department__prof_claims::EmergencyDepartmentProfClaims;
type OV = c::office_visits__int_office_visits_claim_line::OfficeVisitsIntOfficeVisitsClaimLine;
type ASC = c::asc__match_claims_to_anchor::AscMatchClaimsToAnchor;
type RAD = c::outpatient_radiology__match_claims_to_anchor::OutpatientRadiologyMatchClaimsToAnchor;
type INST = c::encounters__int_institutional_claim_lines::EncountersIntInstitutionalClaimLines;
type K3 = (Str, i64, Str);

#[derive(Clone, Copy)]
pub struct EncountersCombinedClaimLineCrosswalk {
    pub claim_id: Str,
    pub claim_line_number: i64,
    pub data_source: Str,
    pub old_encounter_id: Str,
    pub encounter_id: Str,
    pub encounter_type: Str,
    pub encounter_group: Str,
    pub priority_number: i64,
    pub anchor_claim_id: Option<Str>,
    pub claim_line_attribution_number: i64,
}

#[derive(Clone, Copy)]
struct Cand {
    claim_id: Str,
    claim_line_number: i64,
    data_source: Str,
    encounter_id: Str,
    encounter_type: Str,
    encounter_group: Str,
    priority_number: i64,
    anchor_claim_id: Option<Str>,
}

fn cand(
    k: (Str, i64, Str, Str),
    encounter_type: Str,
    encounter_group: Str,
    priority_number: i64,
) -> Cand {
    Cand {
        claim_id: k.0,
        claim_line_number: k.1,
        data_source: k.2,
        encounter_id: k.3,
        encounter_type,
        encounter_group,
        priority_number,
        anchor_claim_id: None,
    }
}

fn pc(t: &'static str, g: &'static str, p: i64) -> impl Fn(ProfClaims) -> Cand + Copy {
    move |x: ProfClaims| cand((x.claim_id, x.claim_line_number, x.data_source, x.encounter_id), t, g, p)
}

fn mc(t: &'static str, g: &'static str, p: i64) -> impl Fn(MatchClaim) -> Cand + Copy {
    move |x: MatchClaim| cand((x.claim_id, x.claim_line_number, x.data_source, x.old_encounter_id), t, g, p)
}

fn km(t: &'static str, g: &'static str, p: i64) -> impl Fn(ClaimMatch) -> Cand + Copy {
    move |x: ClaimMatch| cand((x.claim_id, x.claim_line_number, x.data_source, x.old_encounter_id), t, g, p)
}

fn first(x: ProfClaims) -> bool {
    x.claim_attribution_number == 1
}

type Ord5 = (i64, i64, i64, Str, Str);

#[allow(clippy::too_many_arguments)]
pub fn encounters__combined_claim_line_crosswalk(
    _db: &'static Db,
    stg: &[M],
    acute: &[AP],
    ed: &[EP],
    psych: &[ProfClaims],
    rehab: &[ProfClaims],
    long_term: &[ProfClaims],
    snf: &[ProfClaims],
    hospice: &[ProfClaims],
    substance_use: &[ProfClaims],
    office: &[OV],
    urgent_care: &[MatchClaim],
    op_psych: &[MatchClaim],
    op_rehab: &[MatchClaim],
    asc: &[ASC],
    dialysis: &[MatchClaim],
    op_hospice: &[MatchClaim],
    home_health: &[MatchClaim],
    op_surgery: &[ClaimMatch],
    op_injections: &[ClaimMatch],
    op_ptotst: &[ClaimMatch],
    op_substance_use: &[ClaimMatch],
    op_radiology: &[RAD],
    op_clinic: &[ClaimMatch],
    inst: &[INST],
    lab: &[ClaimMatch],
    dme: &[ClaimMatch],
    ambulance: &[ClaimMatch],
) -> Vec<EncountersCombinedClaimLineCrosswalk> {
    let m = rel(stg.to_vec());
    let und: HashIdx<K3, usize> = (&m)
        .filt(|x: M| x.claim_type == "undetermined")
        .map(|x: M| (x.claim_id, x.claim_line_number, x.data_source))
        .inv()
        .collect();

    let ra = rel(acute.to_vec());
    let re = rel(ed.to_vec());
    let rp = rel(psych.to_vec());
    let rr = rel(rehab.to_vec());
    let rl = rel(long_term.to_vec());
    let rs = rel(snf.to_vec());
    let rh = rel(hospice.to_vec());
    let rsu = rel(substance_use.to_vec());
    let ro = rel(office.to_vec());
    let ruc = rel(urgent_care.to_vec());
    let rop = rel(op_psych.to_vec());
    let ror = rel(op_rehab.to_vec());
    let rasc = rel(asc.to_vec());
    let rdi = rel(dialysis.to_vec());
    let roh = rel(op_hospice.to_vec());
    let rhh = rel(home_health.to_vec());
    let rsg = rel(op_surgery.to_vec());
    let rin = rel(op_injections.to_vec());
    let rpt = rel(op_ptotst.to_vec());
    let rosu = rel(op_substance_use.to_vec());
    let rrad = rel(op_radiology.to_vec());
    let rcl = rel(op_clinic.to_vec());
    let rinst = rel(inst.to_vec());
    let rlab = rel(lab.to_vec());
    let rdme = rel(dme.to_vec());
    let ramb = rel(ambulance.to_vec());

    let acute_first = |x: AP| x.claim_attribution_number == 1;
    let u = (&ra).filt(acute_first)
        .map(|x: AP| cand((x.claim_id, x.claim_line_number, x.data_source, x.encounter_id), "acute inpatient", "inpatient", 0))
        .union((&ra).filt(acute_first).map(|x: AP| {
            cand((x.claim_id, x.claim_line_number, x.data_source, x.encounter_id), "emergency department", "outpatient", 1)
        }))
        .union((&re).filt(|x: EP| x.claim_attribution_number == 1).map(|x: EP| {
            cand((x.claim_id, x.claim_line_number, x.data_source, x.encounter_id), "emergency department", "outpatient", 1)
        }))
        .union((&rp).filt(first).map(pc("inpatient psych", "inpatient", 2)))
        .union((&rr).filt(first).map(pc("inpatient rehabilitation", "inpatient", 3)))
        .union((&rl).filt(first).map(pc("inpatient long term acute care", "inpatient", 4)))
        .union((&rs).filt(first).map(pc("inpatient skilled nursing", "inpatient", 5)))
        .union((&rh).filt(first).map(pc("inpatient hospice", "inpatient", 7)))
        .union((&rsu).filt(first).map(pc("inpatient substance use", "inpatient", 6)))
        .union((&ro).filt(|x: OV| x.encounter_type == "office visit radiology").map(|x: OV| {
            cand((x.claim_id, x.claim_line_number, x.data_source, x.old_encounter_id), x.encounter_type, "office based", 7)
        }))
        .union((&ro).filt(|x: OV| x.encounter_type != "office visit radiology").map(|x: OV| {
            cand((x.claim_id, x.claim_line_number, x.data_source, x.old_encounter_id), x.encounter_type, "office based", 8)
        }))
        .union((&ruc).map(mc("urgent care", "outpatient", 9)))
        .union((&rop).map(mc("outpatient psych", "outpatient", 10)))
        .union((&ror).map(mc("outpatient rehabilitation", "outpatient", 11)))
        .union((&rasc).map(|x: ASC| {
            cand((x.claim_id, x.claim_line_number, x.data_source, x.old_encounter_id), "ambulatory surgery center", "outpatient", 12)
        }))
        .union((&rdi).map(mc("dialysis", "outpatient", 13)))
        .union((&roh).map(mc("outpatient hospice", "outpatient", 14)))
        .union((&rhh).map(mc("home health", "outpatient", 15)))
        .union((&rsg).map(km("outpatient surgery", "outpatient", 16)))
        .union((&rin).map(km("outpatient injections", "outpatient", 17)))
        .union((&rpt).map(km("outpatient pt/ot/st", "outpatient", 18)))
        .union((&rosu).map(km("outpatient substance use", "outpatient", 19)))
        .union((&rrad).map(|x: RAD| {
            cand((x.claim_id, x.claim_line_number, x.data_source, x.old_encounter_id), "outpatient radiology", "outpatient", 20)
        }))
        .union((&rcl).map(km("outpatient hospital or clinic", "outpatient", 999)))
        .union((&rinst).map(|x: INST| Cand {
            claim_id: x.claim_id,
            claim_line_number: x.claim_line_number,
            data_source: x.data_source,
            encounter_id: x.encounter_id,
            encounter_type: x.encounter_type,
            encounter_group: x.encounter_group,
            priority_number: x.priority_number,
            anchor_claim_id: x.anchor_claim_id,
        }))
        .union((&rlab).map(km("lab - orphaned", "other", 1000000)))
        .union((&rdme).map(km("dme - orphaned", "other", 1000001)))
        .union((&ramb).map(km("ambulance - orphaned", "other", 1000002)));
    let cands = rel(drain(u).into_iter().map(|(_, x)| x).collect::<Vec<Cand>>());

    let order = |x: Cand| -> Ord5 {
        (
            x.priority_number,
            if x.encounter_type == "inpatient hospice" { 0 } else { 1 },
            if Some(x.claim_id) == x.anchor_claim_id { 1 } else { 99 },
            x.encounter_type,
            x.encounter_id,
        )
    };
    let w = (&cands)
        .map(|x: Cand| (x.claim_id, x.claim_line_number, x.data_source))
        .inv()
        .minus(&und)
        .select(&cands)
        .window(row_number, order, |a: &Ord5, b: &Ord5| -> Ordering { a.cmp(b) });
    drain(&w)
        .into_iter()
        .map(|(_, (x, n)): (K3, (Cand, i64))| EncountersCombinedClaimLineCrosswalk {
            claim_id: x.claim_id,
            claim_line_number: x.claim_line_number,
            data_source: x.data_source,
            old_encounter_id: x.encounter_id,
            encounter_id: surrogate_key(&[Some(x.encounter_type), Some(x.encounter_id)]),
            encounter_type: x.encounter_type,
            encounter_group: x.encounter_group,
            priority_number: x.priority_number,
            anchor_claim_id: x.anchor_claim_id,
            claim_line_attribution_number: n,
        })
        .collect()
}

pub fn fmt(v: &EncountersCombinedClaimLineCrosswalk) -> String {
    row(vec![
        V::S(v.claim_id),
        V::I(v.claim_line_number),
        V::S(v.data_source),
        V::S(v.old_encounter_id),
        V::S(v.encounter_id),
        V::S(v.encounter_type),
        V::S(v.encounter_group),
        V::I(v.priority_number),
        ostr(v.anchor_claim_id),
        V::I(v.claim_line_attribution_number),
    ])
}

pub fn chain(db: &'static Db, stg: &[M]) -> Vec<EncountersCombinedClaimLineCrosswalk> {
    let prof = c::encounters__stg_professional::encounters__stg_professional(db);
    let plp = c::encounters__prof_and_lower_priority::encounters__prof_and_lower_priority(db, &prof);
    let op_inst = c::encounters__stg_outpatient_institutional::encounters__stg_outpatient_institutional(db);

    let acute_g = c::acute_inpatient__generate_encounter_id::acute_inpatient__generate_encounter_id(db, stg);
    let acute_d = c::acute_inpatient__start_end_dates::acute_inpatient__start_end_dates(db, &acute_g);
    let acute = c::acute_inpatient__prof_claims::acute_inpatient__prof_claims(db, stg, &plp, &acute_g, &acute_d);

    let ed_pre = c::emergency_department__generate_encounter_id_pre_sort::emergency_department__generate_encounter_id_pre_sort(db, stg);
    let ed_g = c::emergency_department__generate_encounter_id::emergency_department__generate_encounter_id(db, &ed_pre);
    let ed_d = c::emergency_department__start_end_dates::emergency_department__start_end_dates(db, &ed_g);
    let ed = c::emergency_department__prof_claims::emergency_department__prof_claims(db, stg, &prof, &op_inst, &ed_g, &ed_d);

    let psych_g = c::inpatient_psych__generate_encounter_id::inpatient_psych__generate_encounter_id(db, stg);
    let psych_d = c::inpatient_psych__start_end_dates::inpatient_psych__start_end_dates(db, &psych_g);
    let psych = c::inpatient_psych__prof_claims::inpatient_psych__prof_claims(db, stg, &plp, &psych_g, &psych_d);

    let rehab_g = c::inpatient_rehab__generate_encounter_id::inpatient_rehab__generate_encounter_id(db, stg);
    let rehab_d = c::inpatient_rehab__start_end_dates::inpatient_rehab__start_end_dates(db, &rehab_g);
    let rehab = c::inpatient_rehab__prof_claims::inpatient_rehab__prof_claims(db, stg, &plp, &rehab_g, &rehab_d);

    let lt_g = c::inpatient_long_term__generate_encounter_id::inpatient_long_term__generate_encounter_id(db, stg);
    let lt_d = c::inpatient_long_term__start_end_dates::inpatient_long_term__start_end_dates(db, &lt_g);
    let long_term = c::inpatient_long_term__prof_claims::inpatient_long_term__prof_claims(db, stg, &plp, &lt_g, &lt_d);

    let snf_g = c::inpatient_snf__generate_encounter_id::inpatient_snf__generate_encounter_id(db, stg);
    let snf_d = c::inpatient_snf__start_end_dates::inpatient_snf__start_end_dates(db, &snf_g);
    let snf = c::inpatient_snf__prof_claims::inpatient_snf__prof_claims(db, stg, &plp, &snf_g, &snf_d);

    let hos_g = c::inpatient_hospice__generate_encounter_id::inpatient_hospice__generate_encounter_id(db, stg);
    let hos_d = c::inpatient_hospice__start_end_dates::inpatient_hospice__start_end_dates(db, &hos_g);
    let hospice = c::inpatient_hospice__prof_claims::inpatient_hospice__prof_claims(db, stg, &plp, &hos_g, &hos_d);

    let su_g = c::inpatient_substance_use__generate_encounter_id::inpatient_substance_use__generate_encounter_id(db, stg);
    let su_d = c::inpatient_substance_use__start_end_dates::inpatient_substance_use__start_end_dates(db, &su_g);
    let substance_use =
        c::inpatient_substance_use__prof_claims::inpatient_substance_use__prof_claims(db, stg, &plp, &su_g, &su_d);

    let ov_u = c::office_visits__int_office_visits_union::union_chain(db, stg);
    let ov_r = c::office_visits__int_office_visits_encounter_ranking::office_visits__int_office_visits_encounter_ranking(db, &ov_u);
    let office = c::office_visits__int_office_visits_claim_line::office_visits__int_office_visits_claim_line(db, &ov_u, &ov_r);

    let uc_a = c::urgent_care__anchor_events::urgent_care__anchor_events(db, stg);
    let uc_g = c::urgent_care__generate_encounter_id::urgent_care__generate_encounter_id(db, stg, &uc_a);
    let urgent_care = c::urgent_care__match_claims_to_anchor::urgent_care__match_claims_to_anchor(db, stg, &uc_g);

    let opp_a = c::outpatient_psych__anchor_events::outpatient_psych__anchor_events(db, stg);
    let opp_g = c::outpatient_psych__generate_encounter_id::outpatient_psych__generate_encounter_id(db, stg, &opp_a);
    let op_psych = c::outpatient_psych__match_claims_to_anchor::outpatient_psych__match_claims_to_anchor(db, stg, &opp_g);

    let opr_a = c::outpatient_rehab__anchor_events::outpatient_rehab__anchor_events(db, stg);
    let opr_g = c::outpatient_rehab__generate_encounter_id::outpatient_rehab__generate_encounter_id(db, stg, &opr_a);
    let op_rehab = c::outpatient_rehab__match_claims_to_anchor::outpatient_rehab__match_claims_to_anchor(db, stg, &opr_g);

    let asc_a = c::asc__anchor_events::asc__anchor_events(db, stg);
    let asc_g = c::asc__generate_encounter_id::asc__generate_encounter_id(db, stg, &asc_a);
    let asc_d = c::asc__start_end_dates::asc__start_end_dates(db, &asc_g);
    let asc = c::asc__match_claims_to_anchor::asc__match_claims_to_anchor(db, stg, &asc_d);

    let di_a = c::dialysis__anchor_events::dialysis__anchor_events(db, stg);
    let di_g = c::dialysis__generate_encounter_id::dialysis__generate_encounter_id(db, stg, &di_a);
    let dialysis = c::dialysis__match_claims_to_anchor::dialysis__match_claims_to_anchor(db, stg, &di_g);

    let oh_a = c::outpatient_hospice__anchor_events::outpatient_hospice__anchor_events(db, stg);
    let oh_g = c::outpatient_hospice__generate_encounter_id::outpatient_hospice__generate_encounter_id(db, stg, &oh_a);
    let op_hospice = c::outpatient_hospice__match_claims_to_anchor::outpatient_hospice__match_claims_to_anchor(db, stg, &oh_g);

    let hh_a = c::home_health__anchor_events::home_health__anchor_events(db, stg);
    let hh_g = c::home_health__generate_encounter_id::home_health__generate_encounter_id(db, stg, &hh_a);
    let home_health = c::home_health__match_claims_to_anchor::home_health__match_claims_to_anchor(db, stg, &hh_g);

    let sg_a = c::outpatient_surgery__anchor_events::outpatient_surgery__anchor_events(db, stg);
    let sg_g = c::outpatient_surgery__generate_encounter_id::outpatient_surgery__generate_encounter_id(db, stg, &sg_a);
    let op_surgery = c::outpatient_surgery__match_claims_to_anchor::outpatient_surgery__match_claims_to_anchor(db, stg, &sg_g);

    let in_a = c::outpatient_injections__anchor_events::outpatient_injections__anchor_events(db, stg, &op_inst);
    let in_g = c::outpatient_injections__generate_encounter_id::outpatient_injections__generate_encounter_id(db, &in_a);
    let op_injections =
        c::outpatient_injections__match_claims_to_anchor::outpatient_injections__match_claims_to_anchor(db, stg, &in_g);

    let pt_a = c::outpatient_ptotst__anchor_events::outpatient_ptotst__anchor_events(db, stg);
    let pt_g = c::outpatient_ptotst__generate_encounter_id::outpatient_ptotst__generate_encounter_id(db, stg, &pt_a);
    let op_ptotst = c::outpatient_ptotst__match_claims_to_anchor::outpatient_ptotst__match_claims_to_anchor(db, stg, &pt_g);

    let osu_a = c::outpatient_substance_use__anchor_events::outpatient_substance_use__anchor_events(db, stg);
    let osu_g = c::outpatient_substance_use__generate_encounter_id::outpatient_substance_use__generate_encounter_id(db, stg, &osu_a);
    let op_substance_use =
        c::outpatient_substance_use__match_claims_to_anchor::outpatient_substance_use__match_claims_to_anchor(db, stg, &osu_g);

    let rad_a = c::outpatient_radiology__anchor_events::outpatient_radiology__anchor_events(db, stg);
    let rad_g = c::outpatient_radiology__generate_encounter_id::outpatient_radiology__generate_encounter_id(db, &rad_a);
    let op_radiology =
        c::outpatient_radiology__match_claims_to_anchor::outpatient_radiology__match_claims_to_anchor(db, stg, &rad_g);

    let cl_a = c::outpatient_hospital_or_clinic__anchor_events::outpatient_hospital_or_clinic__anchor_events(db, stg);
    let cl_g = c::outpatient_hospital_or_clinic__generate_encounter_id::outpatient_hospital_or_clinic__generate_encounter_id(db, stg, &cl_a);
    let op_clinic = c::outpatient_hospital_or_clinic__match_claims_to_anchor::outpatient_hospital_or_clinic__match_claims_to_anchor(db, stg, &cl_g);

    let inst = c::encounters__int_institutional_claim_lines::encounters__int_institutional_claim_lines(
        db, stg, &acute_g, &ed_g, &hos_g, &psych_g, &rehab_g, &lt_g, &snf_g, &su_g,
    );

    let lab_a = c::lab__anchor_events::lab__anchor_events(db, stg);
    let lab_g = c::lab__generate_encounter_id::lab__generate_encounter_id(db, stg, &lab_a);
    let lab = c::lab__match_claims_to_anchor::lab__match_claims_to_anchor(db, stg, &lab_g);

    let dme_a = c::dme__anchor_events::dme__anchor_events(db, stg);
    let dme_g = c::dme__generate_encounter_id::dme__generate_encounter_id(db, stg, &dme_a);
    let dme = c::dme__match_claims_to_anchor::dme__match_claims_to_anchor(db, stg, &dme_g);

    let amb_a = c::ambulance__anchor_events::ambulance__anchor_events(db, stg);
    let amb_g = c::ambulance__generate_encounter_id::ambulance__generate_encounter_id(db, stg, &amb_a);
    let ambulance = c::ambulance__match_claims_to_anchor::ambulance__match_claims_to_anchor(db, stg, &amb_g);

    encounters__combined_claim_line_crosswalk(
        db,
        stg,
        &acute,
        &ed,
        &psych,
        &rehab,
        &long_term,
        &snf,
        &hospice,
        &substance_use,
        &office,
        &urgent_care,
        &op_psych,
        &op_rehab,
        &asc,
        &dialysis,
        &op_hospice,
        &home_health,
        &op_surgery,
        &op_injections,
        &op_ptotst,
        &op_substance_use,
        &op_radiology,
        &op_clinic,
        &inst,
        &lab,
        &dme,
        &ambulance,
    )
}

pub fn q(db: &'static Db) -> String {
    let stg = c::anchor_kit::stg(db);
    rows(chain(db, &stg).iter().map(fmt))
}
