use crate::concepts::int_normalized_input_admit_source_final::IntNormalizedInputAdmitSourceFinal as Asf;
use crate::concepts::int_normalized_input_admit_type_final::IntNormalizedInputAdmitTypeFinal as Atf;
use crate::concepts::int_normalized_input_bill_type_final::IntNormalizedInputBillTypeFinal as Btf;
use crate::concepts::int_normalized_input_discharge_disposition_final::IntNormalizedInputDischargeDispositionFinal as Ddf;
use crate::concepts::int_normalized_input_drg_final::IntNormalizedInputDrgFinal as Drg;
use crate::concepts::int_normalized_input_medical_claim_date_normalize::IntNormalizedInputMedicalClaimDateNormalize as Cld;
use crate::concepts::int_normalized_input_medical_date_aggregation::IntNormalizedInputMedicalDateAggregation as Dag;
use crate::concepts::int_normalized_input_medical_npi_normalize::{IntNormalizedInputMedicalNpiNormalize as Npi, Name};
use crate::concepts::int_normalized_input_place_of_service_normalize::IntNormalizedInputPlaceOfServiceNormalize as Pos;
use crate::concepts::int_normalized_input_procedure_code_final::IntNormalizedInputProcedureCodeFinal as Pxc;
use crate::concepts::int_normalized_input_procedure_date_final::IntNormalizedInputProcedureDateFinal as Pxd;
use crate::concepts::int_normalized_input_revenue_center_normalize::IntNormalizedInputRevenueCenterNormalize as Rev;
use crate::concepts::int_normalized_input_undetermined_claim_type::IntNormalizedInputUndeterminedClaimType as Und;
use crate::concepts::normalized_input__stg_medical_claim::NormalizedInputStgMedicalClaim as Stg;
use crate::schema::Db;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct NormalizedMedicalClaim {
    pub medical_claim_id: Str,
    pub claim_id: Str,
    pub claim_line_number: i64,
    pub claim_type: Str,
    pub person_id: Str,
    pub member_id: Str,
    pub payer: Str,
    pub plan: Str,
    pub claim_start_date: Option<Date>,
    pub claim_end_date: Option<Date>,
    pub claim_line_start_date: Option<Date>,
    pub claim_line_end_date: Option<Date>,
    pub admission_date: Option<Date>,
    pub discharge_date: Option<Date>,
    pub admit_source_code: Option<Str>,
    pub admit_source_description: Option<Str>,
    pub admit_type_code: Option<Str>,
    pub admit_type_description: Option<Str>,
    pub discharge_disposition_code: Option<Str>,
    pub discharge_disposition_description: Option<Str>,
    pub place_of_service_code: Option<Str>,
    pub place_of_service_description: Option<Str>,
    pub bill_type_code: Option<Str>,
    pub bill_type_description: Option<Str>,
    pub drg_code_type: Str,
    pub drg_code: Option<Str>,
    pub drg_description: Option<Str>,
    pub revenue_center_code: Option<Str>,
    pub revenue_center_description: Option<Str>,
    pub service_unit_quantity: Option<i64>,
    pub hcpcs_code: Option<Str>,
    pub hcpcs_modifier_1: Option<Str>,
    pub hcpcs_modifier_2: Option<Str>,
    pub hcpcs_modifier_3: Option<Str>,
    pub hcpcs_modifier_4: Option<Str>,
    pub hcpcs_modifier_5: Option<Str>,
    pub rendering_npi: Option<Str>,
    pub rendering_tin: Option<Str>,
    pub rendering_name: Option<Str>,
    pub billing_npi: Option<Str>,
    pub billing_tin: Option<Str>,
    pub billing_name: Option<Str>,
    pub facility_npi: Option<Str>,
    pub facility_name: Option<Str>,
    pub paid_date: Option<Date>,
    pub paid_amount: Option<f64>,
    pub allowed_amount: Option<f64>,
    pub charge_amount: f64,
    pub coinsurance_amount: Option<f64>,
    pub copayment_amount: Option<f64>,
    pub deductible_amount: Option<f64>,
    pub total_cost_amount: Option<f64>,
    pub diagnosis_code_type: Option<Str>,
    pub diagnosis_code_1: Option<Str>,
    pub diagnosis_code_2: Option<Str>,
    pub diagnosis_code_3: Option<Str>,
    pub diagnosis_code_4: Option<Str>,
    pub diagnosis_code_5: Option<Str>,
    pub diagnosis_code_6: Option<Str>,
    pub diagnosis_code_7: Option<Str>,
    pub diagnosis_code_8: Option<Str>,
    pub diagnosis_code_9: Option<Str>,
    pub diagnosis_code_10: Option<Str>,
    pub diagnosis_code_11: Option<Str>,
    pub diagnosis_code_12: Option<Str>,
    pub diagnosis_code_13: Option<Str>,
    pub diagnosis_code_14: Option<Str>,
    pub diagnosis_code_15: Option<Str>,
    pub diagnosis_code_16: Option<Str>,
    pub diagnosis_code_17: Option<Str>,
    pub diagnosis_code_18: Option<Str>,
    pub diagnosis_code_19: Option<Str>,
    pub diagnosis_code_20: Option<Str>,
    pub diagnosis_code_21: Option<Str>,
    pub diagnosis_code_22: Option<Str>,
    pub diagnosis_code_23: Option<Str>,
    pub diagnosis_code_24: Option<Str>,
    pub diagnosis_code_25: Option<Str>,
    pub diagnosis_poa_1: Option<Str>,
    pub diagnosis_poa_2: Option<Str>,
    pub diagnosis_poa_3: Option<Str>,
    pub diagnosis_poa_4: Option<Str>,
    pub diagnosis_poa_5: Option<Str>,
    pub diagnosis_poa_6: Option<Str>,
    pub diagnosis_poa_7: Option<Str>,
    pub diagnosis_poa_8: Option<Str>,
    pub diagnosis_poa_9: Option<Str>,
    pub diagnosis_poa_10: Option<Str>,
    pub diagnosis_poa_11: Option<Str>,
    pub diagnosis_poa_12: Option<Str>,
    pub diagnosis_poa_13: Option<Str>,
    pub diagnosis_poa_14: Option<Str>,
    pub diagnosis_poa_15: Option<Str>,
    pub diagnosis_poa_16: Option<Str>,
    pub diagnosis_poa_17: Option<Str>,
    pub diagnosis_poa_18: Option<Str>,
    pub diagnosis_poa_19: Option<Str>,
    pub diagnosis_poa_20: Option<Str>,
    pub diagnosis_poa_21: Option<Str>,
    pub diagnosis_poa_22: Option<Str>,
    pub diagnosis_poa_23: Option<Str>,
    pub diagnosis_poa_24: Option<Str>,
    pub diagnosis_poa_25: Option<Str>,
    pub procedure_code_type: Option<Str>,
    pub procedure_code_1: Option<Str>,
    pub procedure_code_2: Option<Str>,
    pub procedure_code_3: Option<Str>,
    pub procedure_code_4: Option<Str>,
    pub procedure_code_5: Option<Str>,
    pub procedure_code_6: Option<Str>,
    pub procedure_code_7: Option<Str>,
    pub procedure_code_8: Option<Str>,
    pub procedure_code_9: Option<Str>,
    pub procedure_code_10: Option<Str>,
    pub procedure_code_11: Option<Str>,
    pub procedure_code_12: Option<Str>,
    pub procedure_code_13: Option<Str>,
    pub procedure_code_14: Option<Str>,
    pub procedure_code_15: Option<Str>,
    pub procedure_code_16: Option<Str>,
    pub procedure_code_17: Option<Str>,
    pub procedure_code_18: Option<Str>,
    pub procedure_code_19: Option<Str>,
    pub procedure_code_20: Option<Str>,
    pub procedure_code_21: Option<Str>,
    pub procedure_code_22: Option<Str>,
    pub procedure_code_23: Option<Str>,
    pub procedure_code_24: Option<Str>,
    pub procedure_code_25: Option<Str>,
    pub procedure_date_1: Option<Date>,
    pub procedure_date_2: Option<Date>,
    pub procedure_date_3: Option<Date>,
    pub procedure_date_4: Option<Date>,
    pub procedure_date_5: Option<Date>,
    pub procedure_date_6: Option<Date>,
    pub procedure_date_7: Option<Date>,
    pub procedure_date_8: Option<Date>,
    pub procedure_date_9: Option<Date>,
    pub procedure_date_10: Option<Date>,
    pub procedure_date_11: Option<Date>,
    pub procedure_date_12: Option<Date>,
    pub procedure_date_13: Option<Date>,
    pub procedure_date_14: Option<Date>,
    pub procedure_date_15: Option<Date>,
    pub procedure_date_16: Option<Date>,
    pub procedure_date_17: Option<Date>,
    pub procedure_date_18: Option<Date>,
    pub procedure_date_19: Option<Date>,
    pub procedure_date_20: Option<Date>,
    pub procedure_date_21: Option<Date>,
    pub procedure_date_22: Option<Date>,
    pub procedure_date_23: Option<Date>,
    pub procedure_date_24: Option<Date>,
    pub procedure_date_25: Option<Date>,
    pub data_source: Str,
    pub in_network_flag: i64,
    pub file_date: Date,
    pub ingest_datetime: Ts,
    pub file_name: Str,
    pub tuva_last_run: Ts,
}

fn leak(s: String) -> Str {
    Box::leak(s.into_boxed_str())
}

fn dec6(x: f64) -> f64 {
    (x * 1e6).round_ties_even() as f32 as f64 / 1e6
}

fn nodot(s: Str) -> Str {
    if s.contains('.') { leak(s.replace('.', "")) } else { s }
}

fn name(n: Name) -> Str {
    match n {
        Name::Person(l, f) => leak(format!("{l}, {f}")),
        Name::Org(o) => o,
    }
}

macro_rules! by2 {
    ($v:expr, $t:ty) => {{
        let r = rel($v.to_vec());
        let i: HashIdx<(Str, Str), usize> = (&r).map(|x: $t| (x.claim_id, x.data_source)).inv().collect();
        (r, i)
    }};
}

macro_rules! by3 {
    ($v:expr, $t:ty) => {{
        let r = rel($v.to_vec());
        let i: HashIdx<(Str, i64, Str), usize> = (&r).map(|x: $t| (x.claim_id, x.claim_line_number, x.data_source)).inv().collect();
        (r, i)
    }};
}

#[allow(clippy::too_many_arguments)]
pub fn normalized__medical_claim(
    _db: &'static Db,
    stg: &[Stg],
    asf: &[Asf],
    atf: &[Atf],
    btf: &[Btf],
    cld: &[Cld],
    dag: &[Dag],
    npi: &[Npi],
    ddf: &[Ddf],
    drg: &[Drg],
    pos: &[Pos],
    pxc: &[Pxc],
    pxd: &[Pxd],
    rev: &[Rev],
    und: &[Und],
) -> Vec<NormalizedMedicalClaim> {
    let med = rel(stg.to_vec());
    let k2 = |m: Stg| (m.claim_id, m.data_source);
    let k3 = |m: Stg| (m.claim_id, m.claim_line_number, m.data_source);
    let (asr, asx) = by2!(asf, Asf);
    let (atr, atx) = by2!(atf, Atf);
    let (btr, btx) = by2!(btf, Btf);
    let (clr, clx) = by3!(cld, Cld);
    let (dgr, dgx) = by2!(dag, Dag);
    let (npr, npx) = by3!(npi, Npi);
    let (ddr, ddx) = by2!(ddf, Ddf);
    let (drr, drx) = by2!(drg, Drg);
    let (por, pox) = by3!(pos, Pos);
    let (pcr, pcx) = by2!(pxc, Pxc);
    let (pdr, pdx) = by2!(pxd, Pxd);
    let (rvr, rvx) = by3!(rev, Rev);
    let (unr, unx) = by3!(und, Und);
    let j = (&med)
        .and((&med).map(k2).select(&asx).select(&asr).opt())
        .and((&med).map(k2).select(&atx).select(&atr).opt())
        .and((&med).map(k2).select(&btx).select(&btr).opt())
        .and((&med).map(k3).select(&clx).select(&clr).opt())
        .and((&med).map(k2).select(&dgx).select(&dgr).opt())
        .and((&med).map(k3).select(&npx).select(&npr).opt())
        .and((&med).map(k2).select(&ddx).select(&ddr).opt())
        .and((&med).map(k2).select(&drx).select(&drr).opt())
        .and((&med).map(k3).select(&pox).select(&por).opt())
        .and((&med).map(k2).select(&pcx).select(&pcr).opt())
        .and((&med).map(k2).select(&pdx).select(&pdr).opt())
        .and((&med).map(k3).select(&rvx).select(&rvr).opt())
        .and((&med).map(k3).select(&unx).select(&unr).opt());
    let run = ts(2026, 1, 1, 0, 0, 0);
    drain(&j)
        .into_iter()
        .map(|(_, (((((((((((((m, asf), atf), btf), cl), d), npi), ddf), drg), pos), pxc), pxd), rev), u))| {
            NormalizedMedicalClaim {
                medical_claim_id: leak(format!("{}-{}-{}", m.claim_id, m.claim_line_number, m.data_source)),
                claim_id: m.claim_id,
                claim_line_number: m.claim_line_number,
                claim_type: m.claim_type,
                person_id: m.person_id,
                member_id: m.member_id,
                payer: m.payer,
                plan: m.plan,
                claim_start_date: d.map(|x| x.minimum_claim_start_date).or(u.map(|x| x.claim_start_date)),
                claim_end_date: d.map(|x| x.maximum_claim_end_date).or(u.map(|x| x.claim_start_date)),
                claim_line_start_date: cl.and_then(|x| x.normalized_claim_line_start_date).or(u.and_then(|x| x.claim_line_start_date)),
                claim_line_end_date: cl.and_then(|x| x.normalized_claim_line_end_date).or(u.and_then(|x| x.claim_line_end_date)),
                admission_date: d.and_then(|x| x.minimum_admission_date).or(u.and_then(|x| x.admission_date)),
                discharge_date: d.and_then(|x| x.maximum_discharge_date).or(u.and_then(|x| x.discharge_date)),
                admit_source_code: asf.map(|x| x.normalized_code).or(u.and_then(|x| x.admit_source_code)),
                admit_source_description: asf.map(|x| x.normalized_description).or(u.and_then(|x| x.admit_source_description)),
                admit_type_code: atf.map(|x| x.normalized_code).or(u.and_then(|x| x.admit_type_code)),
                admit_type_description: atf.map(|x| x.normalized_description).or(u.and_then(|x| x.admit_type_description)),
                discharge_disposition_code: ddf.map(|x| x.normalized_code).or(u.and_then(|x| x.discharge_disposition_code)),
                discharge_disposition_description: ddf.map(|x| x.normalized_description).or(u.and_then(|x| x.discharge_disposition_description)),
                place_of_service_code: pos.and_then(|x| x.normalized_code).or(u.and_then(|x| x.place_of_service_code)),
                place_of_service_description: pos.and_then(|x| x.normalized_description).or(u.and_then(|x| x.place_of_service_description)),
                bill_type_code: btf.map(|x| x.normalized_code).or(u.and_then(|x| x.bill_type_code)),
                bill_type_description: btf.map(|x| x.normalized_description).or(u.and_then(|x| x.bill_type_description)),
                drg_code_type: m.drg_code_type,
                drg_code: drg.and_then(|x| x.normalized_code).or(u.and_then(|x| x.drg_code)),
                drg_description: drg.and_then(|x| x.normalized_description).or(u.and_then(|x| x.drg_description)),
                revenue_center_code: rev.and_then(|x| x.normalized_code).or(u.and_then(|x| x.revenue_center_code)),
                revenue_center_description: rev.and_then(|x| x.normalized_description).or(u.and_then(|x| x.revenue_center_description)),
                service_unit_quantity: m.service_unit_quantity,
                hcpcs_code: m.hcpcs_code,
                hcpcs_modifier_1: m.hcpcs_modifier_1,
                hcpcs_modifier_2: m.hcpcs_modifier_2,
                hcpcs_modifier_3: m.hcpcs_modifier_3,
                hcpcs_modifier_4: m.hcpcs_modifier_4,
                hcpcs_modifier_5: m.hcpcs_modifier_5,
                rendering_npi: npi.and_then(|x| x.normalized_rendering_npi).or(u.map(|x| x.rendering_npi)),
                rendering_tin: m.rendering_tin,
                rendering_name: npi.and_then(|x| x.normalized_rendering_name).map(name).or(u.and_then(|x| x.rendering_name)),
                billing_npi: npi.and_then(|x| x.normalized_billing_npi).or(u.and_then(|x| x.billing_npi)),
                billing_tin: m.billing_tin,
                billing_name: npi.and_then(|x| x.normalized_billing_name).map(name).or(u.and_then(|x| x.billing_name)),
                facility_npi: npi.and_then(|x| x.normalized_facility_npi).or(u.and_then(|x| x.facility_npi)),
                facility_name: npi.and_then(|x| x.normalized_facility_name).map(name).or(u.and_then(|x| x.facility_name)),
                paid_date: m.paid_date,
                paid_amount: m.paid_amount.map(dec6),
                allowed_amount: m.allowed_amount.map(dec6),
                charge_amount: dec6(m.charge_amount),
                coinsurance_amount: m.coinsurance_amount.map(dec6),
                copayment_amount: m.copayment_amount.map(dec6),
                deductible_amount: m.deductible_amount.map(dec6),
                total_cost_amount: m.total_cost_amount.map(dec6),
                diagnosis_code_type: m.diagnosis_code_type,
                diagnosis_code_1: m.diagnosis_code_1.map(nodot),
                diagnosis_code_2: m.diagnosis_code_2.map(nodot),
                diagnosis_code_3: m.diagnosis_code_3.map(nodot),
                diagnosis_code_4: m.diagnosis_code_4.map(nodot),
                diagnosis_code_5: m.diagnosis_code_5.map(nodot),
                diagnosis_code_6: m.diagnosis_code_6.map(nodot),
                diagnosis_code_7: m.diagnosis_code_7.map(nodot),
                diagnosis_code_8: m.diagnosis_code_8.map(nodot),
                diagnosis_code_9: m.diagnosis_code_9.map(nodot),
                diagnosis_code_10: m.diagnosis_code_10.map(nodot),
                diagnosis_code_11: m.diagnosis_code_11.map(nodot),
                diagnosis_code_12: m.diagnosis_code_12.map(nodot),
                diagnosis_code_13: m.diagnosis_code_13.map(nodot),
                diagnosis_code_14: m.diagnosis_code_14.map(nodot),
                diagnosis_code_15: m.diagnosis_code_15.map(nodot),
                diagnosis_code_16: m.diagnosis_code_16.map(nodot),
                diagnosis_code_17: m.diagnosis_code_17.map(nodot),
                diagnosis_code_18: m.diagnosis_code_18.map(nodot),
                diagnosis_code_19: m.diagnosis_code_19.map(nodot),
                diagnosis_code_20: m.diagnosis_code_20.map(nodot),
                diagnosis_code_21: m.diagnosis_code_21.map(nodot),
                diagnosis_code_22: m.diagnosis_code_22.map(nodot),
                diagnosis_code_23: m.diagnosis_code_23.map(nodot),
                diagnosis_code_24: m.diagnosis_code_24.map(nodot),
                diagnosis_code_25: m.diagnosis_code_25.map(nodot),
                diagnosis_poa_1: m.diagnosis_poa_1,
                diagnosis_poa_2: m.diagnosis_poa_2,
                diagnosis_poa_3: m.diagnosis_poa_3,
                diagnosis_poa_4: m.diagnosis_poa_4,
                diagnosis_poa_5: m.diagnosis_poa_5,
                diagnosis_poa_6: m.diagnosis_poa_6,
                diagnosis_poa_7: m.diagnosis_poa_7,
                diagnosis_poa_8: m.diagnosis_poa_8,
                diagnosis_poa_9: m.diagnosis_poa_9,
                diagnosis_poa_10: m.diagnosis_poa_10,
                diagnosis_poa_11: m.diagnosis_poa_11,
                diagnosis_poa_12: m.diagnosis_poa_12,
                diagnosis_poa_13: m.diagnosis_poa_13,
                diagnosis_poa_14: m.diagnosis_poa_14,
                diagnosis_poa_15: m.diagnosis_poa_15,
                diagnosis_poa_16: m.diagnosis_poa_16,
                diagnosis_poa_17: m.diagnosis_poa_17,
                diagnosis_poa_18: m.diagnosis_poa_18,
                diagnosis_poa_19: m.diagnosis_poa_19,
                diagnosis_poa_20: m.diagnosis_poa_20,
                diagnosis_poa_21: m.diagnosis_poa_21,
                diagnosis_poa_22: m.diagnosis_poa_22,
                diagnosis_poa_23: m.diagnosis_poa_23,
                diagnosis_poa_24: m.diagnosis_poa_24,
                diagnosis_poa_25: m.diagnosis_poa_25,
                procedure_code_type: m.procedure_code_type,
                procedure_code_1: pxc.and_then(|x| x.procedure_code_1).or(u.and_then(|x| x.procedure_code_1)),
                procedure_code_2: pxc.and_then(|x| x.procedure_code_2).or(u.and_then(|x| x.procedure_code_2)),
                procedure_code_3: pxc.and_then(|x| x.procedure_code_3).or(u.and_then(|x| x.procedure_code_3)),
                procedure_code_4: pxc.and_then(|x| x.procedure_code_4).or(u.and_then(|x| x.procedure_code_4)),
                procedure_code_5: pxc.and_then(|x| x.procedure_code_5).or(u.and_then(|x| x.procedure_code_5)),
                procedure_code_6: pxc.and_then(|x| x.procedure_code_6).or(u.and_then(|x| x.procedure_code_6)),
                procedure_code_7: pxc.and_then(|x| x.procedure_code_7).or(u.and_then(|x| x.procedure_code_7)),
                procedure_code_8: pxc.and_then(|x| x.procedure_code_8).or(u.and_then(|x| x.procedure_code_8)),
                procedure_code_9: pxc.and_then(|x| x.procedure_code_9).or(u.and_then(|x| x.procedure_code_9)),
                procedure_code_10: pxc.and_then(|x| x.procedure_code_10).or(u.and_then(|x| x.procedure_code_10)),
                procedure_code_11: pxc.and_then(|x| x.procedure_code_11).or(u.and_then(|x| x.procedure_code_11)),
                procedure_code_12: pxc.and_then(|x| x.procedure_code_12).or(u.and_then(|x| x.procedure_code_12)),
                procedure_code_13: pxc.and_then(|x| x.procedure_code_13).or(u.and_then(|x| x.procedure_code_13)),
                procedure_code_14: pxc.and_then(|x| x.procedure_code_14).or(u.and_then(|x| x.procedure_code_14)),
                procedure_code_15: pxc.and_then(|x| x.procedure_code_15).or(u.and_then(|x| x.procedure_code_15)),
                procedure_code_16: pxc.and_then(|x| x.procedure_code_16).or(u.and_then(|x| x.procedure_code_16)),
                procedure_code_17: pxc.and_then(|x| x.procedure_code_17).or(u.and_then(|x| x.procedure_code_17)),
                procedure_code_18: pxc.and_then(|x| x.procedure_code_18).or(u.and_then(|x| x.procedure_code_18)),
                procedure_code_19: pxc.and_then(|x| x.procedure_code_19).or(u.and_then(|x| x.procedure_code_19)),
                procedure_code_20: pxc.and_then(|x| x.procedure_code_20).or(u.and_then(|x| x.procedure_code_20)),
                procedure_code_21: pxc.and_then(|x| x.procedure_code_21).or(u.and_then(|x| x.procedure_code_21)),
                procedure_code_22: pxc.and_then(|x| x.procedure_code_22).or(u.and_then(|x| x.procedure_code_22)),
                procedure_code_23: pxc.and_then(|x| x.procedure_code_23).or(u.and_then(|x| x.procedure_code_23)),
                procedure_code_24: pxc.and_then(|x| x.procedure_code_24).or(u.and_then(|x| x.procedure_code_24)),
                procedure_code_25: pxc.and_then(|x| x.procedure_code_25).or(u.and_then(|x| x.procedure_code_25)),
                procedure_date_1: pxd.and_then(|x| x.procedure_date_1).or(u.and_then(|x| x.procedure_date_1)),
                procedure_date_2: pxd.and_then(|x| x.procedure_date_2).or(u.and_then(|x| x.procedure_date_2)),
                procedure_date_3: pxd.and_then(|x| x.procedure_date_3).or(u.and_then(|x| x.procedure_date_3)),
                procedure_date_4: pxd.and_then(|x| x.procedure_date_4).or(u.and_then(|x| x.procedure_date_4)),
                procedure_date_5: pxd.and_then(|x| x.procedure_date_5).or(u.and_then(|x| x.procedure_date_5)),
                procedure_date_6: pxd.and_then(|x| x.procedure_date_6).or(u.and_then(|x| x.procedure_date_6)),
                procedure_date_7: pxd.and_then(|x| x.procedure_date_7).or(u.and_then(|x| x.procedure_date_7)),
                procedure_date_8: pxd.and_then(|x| x.procedure_date_8).or(u.and_then(|x| x.procedure_date_8)),
                procedure_date_9: pxd.and_then(|x| x.procedure_date_9).or(u.and_then(|x| x.procedure_date_9)),
                procedure_date_10: pxd.and_then(|x| x.procedure_date_10).or(u.and_then(|x| x.procedure_date_10)),
                procedure_date_11: pxd.and_then(|x| x.procedure_date_11).or(u.and_then(|x| x.procedure_date_11)),
                procedure_date_12: pxd.and_then(|x| x.procedure_date_12).or(u.and_then(|x| x.procedure_date_12)),
                procedure_date_13: pxd.and_then(|x| x.procedure_date_13).or(u.and_then(|x| x.procedure_date_13)),
                procedure_date_14: pxd.and_then(|x| x.procedure_date_14).or(u.and_then(|x| x.procedure_date_14)),
                procedure_date_15: pxd.and_then(|x| x.procedure_date_15).or(u.and_then(|x| x.procedure_date_15)),
                procedure_date_16: pxd.and_then(|x| x.procedure_date_16).or(u.and_then(|x| x.procedure_date_16)),
                procedure_date_17: pxd.and_then(|x| x.procedure_date_17).or(u.and_then(|x| x.procedure_date_17)),
                procedure_date_18: pxd.and_then(|x| x.procedure_date_18).or(u.and_then(|x| x.procedure_date_18)),
                procedure_date_19: pxd.and_then(|x| x.procedure_date_19).or(u.and_then(|x| x.procedure_date_19)),
                procedure_date_20: pxd.and_then(|x| x.procedure_date_20).or(u.and_then(|x| x.procedure_date_20)),
                procedure_date_21: pxd.and_then(|x| x.procedure_date_21).or(u.and_then(|x| x.procedure_date_21)),
                procedure_date_22: pxd.and_then(|x| x.procedure_date_22).or(u.and_then(|x| x.procedure_date_22)),
                procedure_date_23: pxd.and_then(|x| x.procedure_date_23).or(u.and_then(|x| x.procedure_date_23)),
                procedure_date_24: pxd.and_then(|x| x.procedure_date_24).or(u.and_then(|x| x.procedure_date_24)),
                procedure_date_25: pxd.and_then(|x| x.procedure_date_25).or(u.and_then(|x| x.procedure_date_25)),
                data_source: m.data_source,
                in_network_flag: m.in_network_flag,
                file_date: m.file_date,
                ingest_datetime: m.ingest_datetime,
                file_name: m.file_name,
                tuva_last_run: run,
            }
        })
        .collect()
}

pub fn fmt(v: &NormalizedMedicalClaim) -> String {
    row(vec![
        V::S(v.medical_claim_id),
        V::S(v.claim_id),
        V::I(v.claim_line_number),
        V::S(v.claim_type),
        V::S(v.person_id),
        V::S(v.member_id),
        V::S(v.payer),
        V::S(v.plan),
        odate(v.claim_start_date),
        odate(v.claim_end_date),
        odate(v.claim_line_start_date),
        odate(v.claim_line_end_date),
        odate(v.admission_date),
        odate(v.discharge_date),
        ostr(v.admit_source_code),
        ostr(v.admit_source_description),
        ostr(v.admit_type_code),
        ostr(v.admit_type_description),
        ostr(v.discharge_disposition_code),
        ostr(v.discharge_disposition_description),
        ostr(v.place_of_service_code),
        ostr(v.place_of_service_description),
        ostr(v.bill_type_code),
        ostr(v.bill_type_description),
        V::S(v.drg_code_type),
        ostr(v.drg_code),
        ostr(v.drg_description),
        ostr(v.revenue_center_code),
        ostr(v.revenue_center_description),
        v.service_unit_quantity.map_or(V::Null, |x| V::F(x as f64)),
        ostr(v.hcpcs_code),
        ostr(v.hcpcs_modifier_1),
        ostr(v.hcpcs_modifier_2),
        ostr(v.hcpcs_modifier_3),
        ostr(v.hcpcs_modifier_4),
        ostr(v.hcpcs_modifier_5),
        ostr(v.rendering_npi),
        ostr(v.rendering_tin),
        ostr(v.rendering_name),
        ostr(v.billing_npi),
        ostr(v.billing_tin),
        ostr(v.billing_name),
        ostr(v.facility_npi),
        ostr(v.facility_name),
        odate(v.paid_date),
        ofloat(v.paid_amount),
        ofloat(v.allowed_amount),
        V::F(v.charge_amount),
        ofloat(v.coinsurance_amount),
        ofloat(v.copayment_amount),
        ofloat(v.deductible_amount),
        ofloat(v.total_cost_amount),
        ostr(v.diagnosis_code_type),
        ostr(v.diagnosis_code_1),
        ostr(v.diagnosis_code_2),
        ostr(v.diagnosis_code_3),
        ostr(v.diagnosis_code_4),
        ostr(v.diagnosis_code_5),
        ostr(v.diagnosis_code_6),
        ostr(v.diagnosis_code_7),
        ostr(v.diagnosis_code_8),
        ostr(v.diagnosis_code_9),
        ostr(v.diagnosis_code_10),
        ostr(v.diagnosis_code_11),
        ostr(v.diagnosis_code_12),
        ostr(v.diagnosis_code_13),
        ostr(v.diagnosis_code_14),
        ostr(v.diagnosis_code_15),
        ostr(v.diagnosis_code_16),
        ostr(v.diagnosis_code_17),
        ostr(v.diagnosis_code_18),
        ostr(v.diagnosis_code_19),
        ostr(v.diagnosis_code_20),
        ostr(v.diagnosis_code_21),
        ostr(v.diagnosis_code_22),
        ostr(v.diagnosis_code_23),
        ostr(v.diagnosis_code_24),
        ostr(v.diagnosis_code_25),
        ostr(v.diagnosis_poa_1),
        ostr(v.diagnosis_poa_2),
        ostr(v.diagnosis_poa_3),
        ostr(v.diagnosis_poa_4),
        ostr(v.diagnosis_poa_5),
        ostr(v.diagnosis_poa_6),
        ostr(v.diagnosis_poa_7),
        ostr(v.diagnosis_poa_8),
        ostr(v.diagnosis_poa_9),
        ostr(v.diagnosis_poa_10),
        ostr(v.diagnosis_poa_11),
        ostr(v.diagnosis_poa_12),
        ostr(v.diagnosis_poa_13),
        ostr(v.diagnosis_poa_14),
        ostr(v.diagnosis_poa_15),
        ostr(v.diagnosis_poa_16),
        ostr(v.diagnosis_poa_17),
        ostr(v.diagnosis_poa_18),
        ostr(v.diagnosis_poa_19),
        ostr(v.diagnosis_poa_20),
        ostr(v.diagnosis_poa_21),
        ostr(v.diagnosis_poa_22),
        ostr(v.diagnosis_poa_23),
        ostr(v.diagnosis_poa_24),
        ostr(v.diagnosis_poa_25),
        ostr(v.procedure_code_type),
        ostr(v.procedure_code_1),
        ostr(v.procedure_code_2),
        ostr(v.procedure_code_3),
        ostr(v.procedure_code_4),
        ostr(v.procedure_code_5),
        ostr(v.procedure_code_6),
        ostr(v.procedure_code_7),
        ostr(v.procedure_code_8),
        ostr(v.procedure_code_9),
        ostr(v.procedure_code_10),
        ostr(v.procedure_code_11),
        ostr(v.procedure_code_12),
        ostr(v.procedure_code_13),
        ostr(v.procedure_code_14),
        ostr(v.procedure_code_15),
        ostr(v.procedure_code_16),
        ostr(v.procedure_code_17),
        ostr(v.procedure_code_18),
        ostr(v.procedure_code_19),
        ostr(v.procedure_code_20),
        ostr(v.procedure_code_21),
        ostr(v.procedure_code_22),
        ostr(v.procedure_code_23),
        ostr(v.procedure_code_24),
        ostr(v.procedure_code_25),
        odate(v.procedure_date_1),
        odate(v.procedure_date_2),
        odate(v.procedure_date_3),
        odate(v.procedure_date_4),
        odate(v.procedure_date_5),
        odate(v.procedure_date_6),
        odate(v.procedure_date_7),
        odate(v.procedure_date_8),
        odate(v.procedure_date_9),
        odate(v.procedure_date_10),
        odate(v.procedure_date_11),
        odate(v.procedure_date_12),
        odate(v.procedure_date_13),
        odate(v.procedure_date_14),
        odate(v.procedure_date_15),
        odate(v.procedure_date_16),
        odate(v.procedure_date_17),
        odate(v.procedure_date_18),
        odate(v.procedure_date_19),
        odate(v.procedure_date_20),
        odate(v.procedure_date_21),
        odate(v.procedure_date_22),
        odate(v.procedure_date_23),
        odate(v.procedure_date_24),
        odate(v.procedure_date_25),
        V::S(v.data_source),
        V::I(v.in_network_flag),
        V::D(v.file_date),
        V::T(v.ingest_datetime),
        V::S(v.file_name),
        V::T(v.tuva_last_run),
    ])
}

pub fn q(db: &'static Db) -> String {
    let mc = crate::concepts::medical_claim::medical_claim(db);
    let il = crate::concepts::input_layer__medical_claim::input_layer__medical_claim(db, &mc);
    let stg = crate::concepts::normalized_input__stg_medical_claim::normalized_input__stg_medical_claim(db, &il);
    rows(normalized__medical_claim_from_stg(db, &stg).iter().map(fmt))
}

pub fn normalized__medical_claim_from_stg(db: &'static Db, stg: &[Stg]) -> Vec<NormalizedMedicalClaim> {
    let asv = crate::concepts::int_normalized_input_admit_source_voting::int_normalized_input_admit_source_voting(db, stg);
    let asf = crate::concepts::int_normalized_input_admit_source_final::int_normalized_input_admit_source_final(db, &asv);
    let atv = crate::concepts::int_normalized_input_admit_type_normalize_voting::int_normalized_input_admit_type_normalize_voting(db, stg);
    let atf = crate::concepts::int_normalized_input_admit_type_final::int_normalized_input_admit_type_final(db, &atv);
    let btv = crate::concepts::int_normalized_input_bill_type_voting::int_normalized_input_bill_type_voting(db, stg);
    let btf = crate::concepts::int_normalized_input_bill_type_final::int_normalized_input_bill_type_final(db, &btv);
    let cld = crate::concepts::int_normalized_input_medical_claim_date_normalize::int_normalized_input_medical_claim_date_normalize(db, stg);
    let dag = crate::concepts::int_normalized_input_medical_date_aggregation::int_normalized_input_medical_date_aggregation(db, &cld);
    let npi = crate::concepts::int_normalized_input_medical_npi_normalize::int_normalized_input_medical_npi_normalize(db, stg);
    let ddv = crate::concepts::int_normalized_input_discharge_disposition_voting::int_normalized_input_discharge_disposition_voting(db, stg);
    let ddf = crate::concepts::int_normalized_input_discharge_disposition_final::int_normalized_input_discharge_disposition_final(db, &ddv);
    let drv = crate::concepts::int_normalized_input_drg_voting::int_normalized_input_drg_voting(db, stg);
    let drg = crate::concepts::int_normalized_input_drg_final::int_normalized_input_drg_final(db, &drv);
    let pos = crate::concepts::int_normalized_input_place_of_service_normalize::int_normalized_input_place_of_service_normalize(db, stg);
    let pci = crate::concepts::int_normalized_input_procedure_code_intermediate::int_normalized_input_procedure_code_intermediate(db, stg);
    let pxc = crate::concepts::int_normalized_input_procedure_code_final::int_normalized_input_procedure_code_final(db, &pci);
    let pdn = crate::concepts::int_normalized_input_procedure_date_normalize::int_normalized_input_procedure_date_normalize(db, stg);
    let pdv = crate::concepts::int_normalized_input_procedure_date_voting::int_normalized_input_procedure_date_voting(db, &pdn);
    let pxd = crate::concepts::int_normalized_input_procedure_date_final::int_normalized_input_procedure_date_final(db, &pdv);
    let rev = crate::concepts::int_normalized_input_revenue_center_normalize::int_normalized_input_revenue_center_normalize(db, stg);
    let und = crate::concepts::int_normalized_input_undetermined_claim_type::int_normalized_input_undetermined_claim_type(db, stg);
    normalized__medical_claim(db, stg, &asf, &atf, &btf, &cld, &dag, &npi, &ddf, &drg, &pos, &pxc, &pxd, &rev, &und)
}
