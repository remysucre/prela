use crate::concepts::eligibility::Eligibility;
use crate::concepts::input_layer__eligibility::input_layer__eligibility;
use crate::concepts::input_layer__medical_claim::input_layer__medical_claim;
use crate::concepts::medical_claim::MedicalClaim;
use crate::schema::Db;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct MedicalClaimLineFlags {
    pub claim_id: Str,
    pub claim_line_number: i64,
    pub data_source: Str,
    pub claim_line_number_not_positive: Option<i64>,
    pub claim_type_null: Option<i64>,
    pub claim_type_invalid: Option<i64>,
    pub institutional_indicators_present_for_professional_claim: Option<i64>,
    pub person_id_null: Option<i64>,
    pub in_network_flag_invalid: Option<i64>,
    pub claim_start_date_null: Option<i64>,
    pub claim_end_date_null: Option<i64>,
    pub claim_line_start_date_null: Option<i64>,
    pub claim_line_end_date_null: Option<i64>,
    pub claim_start_date_out_of_reasonable_range: Option<i64>,
    pub claim_end_date_out_of_reasonable_range: Option<i64>,
    pub claim_line_start_date_out_of_reasonable_range: Option<i64>,
    pub claim_line_end_date_out_of_reasonable_range: Option<i64>,
    pub claim_start_after_claim_end: Option<i64>,
    pub claim_line_start_after_claim_line_end: Option<i64>,
    pub admission_date_after_discharge_date: Option<i64>,
    pub admission_date_out_of_reasonable_range: Option<i64>,
    pub admission_date_null_for_inpatient_claim: Option<i64>,
    pub discharge_date_null_for_inpatient_claim: Option<i64>,
    pub discharge_date_out_of_reasonable_range: Option<i64>,
    pub paid_date_null: Option<i64>,
    pub paid_date_out_of_reasonable_range: Option<i64>,
    pub paid_date_before_claim_end_date: Option<i64>,
    pub procedure_date_1_outside_supported_date_range: Option<i64>,
    pub procedure_date_2_outside_supported_date_range: Option<i64>,
    pub procedure_date_3_outside_supported_date_range: Option<i64>,
    pub procedure_date_4_outside_supported_date_range: Option<i64>,
    pub procedure_date_5_outside_supported_date_range: Option<i64>,
    pub procedure_date_6_outside_supported_date_range: Option<i64>,
    pub procedure_date_7_outside_supported_date_range: Option<i64>,
    pub procedure_date_8_outside_supported_date_range: Option<i64>,
    pub procedure_date_9_outside_supported_date_range: Option<i64>,
    pub procedure_date_10_outside_supported_date_range: Option<i64>,
    pub procedure_date_11_outside_supported_date_range: Option<i64>,
    pub procedure_date_12_outside_supported_date_range: Option<i64>,
    pub procedure_date_13_outside_supported_date_range: Option<i64>,
    pub procedure_date_14_outside_supported_date_range: Option<i64>,
    pub procedure_date_15_outside_supported_date_range: Option<i64>,
    pub procedure_date_16_outside_supported_date_range: Option<i64>,
    pub procedure_date_17_outside_supported_date_range: Option<i64>,
    pub procedure_date_18_outside_supported_date_range: Option<i64>,
    pub procedure_date_19_outside_supported_date_range: Option<i64>,
    pub procedure_date_20_outside_supported_date_range: Option<i64>,
    pub procedure_date_21_outside_supported_date_range: Option<i64>,
    pub procedure_date_22_outside_supported_date_range: Option<i64>,
    pub procedure_date_23_outside_supported_date_range: Option<i64>,
    pub procedure_date_24_outside_supported_date_range: Option<i64>,
    pub procedure_date_25_outside_supported_date_range: Option<i64>,
    pub file_date_outside_supported_date_range: Option<i64>,
    pub ingest_datetime_out_of_reasonable_range: Option<i64>,
    pub paid_amount_null: Option<i64>,
    pub paid_amount_lt_zero: Option<i64>,
    pub allowed_amount_null: Option<i64>,
    pub allowed_amount_lt_zero: Option<i64>,
    pub paid_amount_gt_allowed_amount: Option<i64>,
    pub admit_source_code_invalid: Option<i64>,
    pub admit_type_code_invalid: Option<i64>,
    pub discharge_disposition_code_invalid: Option<i64>,
    pub place_of_service_code_invalid: Option<i64>,
    pub bill_type_code_invalid: Option<i64>,
    pub revenue_center_code_invalid: Option<i64>,
    pub place_of_service_code_null_for_professional_claim: Option<i64>,
    pub place_of_service_code_present_for_institutional_claim: Option<i64>,
    pub bill_type_code_null_for_institutional_claim: Option<i64>,
    pub revenue_center_code_null_for_institutional_claim: Option<i64>,
    pub hcpcs_code_null_for_professional_claim: Option<i64>,
    pub rendering_npi_invalid: Option<i64>,
    pub billing_npi_invalid: Option<i64>,
    pub facility_npi_invalid: Option<i64>,
    pub rendering_npi_null: Option<i64>,
    pub billing_npi_null: Option<i64>,
    pub facility_npi_null_for_inpatient_claim: Option<i64>,
    pub drg_code_type_null_when_drg_code_present: Option<i64>,
    pub drg_code_type_invalid: Option<i64>,
    pub drg_code_invalid: Option<i64>,
    pub drg_code_null_for_acute_inpatient_claim: Option<i64>,
    pub diagnosis_code_1_null: Option<i64>,
    pub diagnosis_code_type_null_when_diagnosis_code_present: Option<i64>,
    pub diagnosis_code_type_invalid: Option<i64>,
    pub diagnosis_code_1_invalid: Option<i64>,
    pub diagnosis_code_2_to_25_invalid: Option<i64>,
    pub procedure_code_type_null_when_procedure_code_present: Option<i64>,
    pub procedure_code_type_invalid: Option<i64>,
    pub procedure_code_1_to_25_invalid: Option<i64>,
    pub no_matching_eligibility_span: Option<i64>,
}

type K5 = (Str, Str, Str, Str, Str);
type K7 = (K5, (i64, i64));
type K3 = (Str, i64, Str);
type Er = (K5, Date, Date);

#[derive(Clone, Copy)]
struct Code {
    key: K3,
    pos: i64,
    ty: Str,
    code: Str,
}

struct Lk {
    elig: bool,
    dx: Option<(i64, i64)>,
    px: Option<i64>,
    admit_source: bool,
    admit_type: bool,
    discharge_disposition: bool,
    place_of_service: bool,
    bill_type: bool,
    revenue_center: bool,
    rendering: bool,
    billing: bool,
    facility: bool,
    ms_drg: bool,
    apr_drg: bool,
}

const IP: [&str; 26] = [
    "11", "12", "15", "16", "17", "18", "21", "22", "25", "26", "27", "28", "31", "41", "42", "45", "46", "47", "48", "61",
    "62", "65", "66", "67", "68", "82",
];
const DRG_TYPES: [&str; 2] = ["ms-drg", "apr-drg"];
const DX_TYPES: [&str; 2] = ["icd-10-cm", "icd-9-cm"];
const PX_TYPES: [&str; 2] = ["icd-10-pcs", "icd-9-pcs"];

fn d2000() -> Date {
    date(2000, 1, 1)
}

fn today() -> Date {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_micros() as i64;
    trunc_day(now)
}

fn w(c: bool, x: bool) -> Option<i64> {
    c.then_some(x as i64)
}

fn oor(d: Option<Date>, today: Date) -> Option<i64> {
    d.map(|d| (d < d2000() || d > today) as i64)
}

fn gt2(a: Option<Date>, b: Option<Date>) -> Option<i64> {
    a.zip(b).map(|(a, b)| (a > b) as i64)
}

fn sub2(s: Str) -> &'static str {
    match s.char_indices().nth(2) {
        Some((i, _)) => &s[..i],
        None => s,
    }
}

fn nodot(s: Str) -> Str {
    if s.contains('.') { Box::leak(s.replace('.', "").into_boxed_str()) } else { s }
}

fn dx(r: &MedicalClaim) -> [Option<Str>; 25] {
    [r.diagnosis_code_1, r.diagnosis_code_2, r.diagnosis_code_3, r.diagnosis_code_4, r.diagnosis_code_5, r.diagnosis_code_6, r.diagnosis_code_7, r.diagnosis_code_8, r.diagnosis_code_9, r.diagnosis_code_10, r.diagnosis_code_11, r.diagnosis_code_12, r.diagnosis_code_13, r.diagnosis_code_14, r.diagnosis_code_15, r.diagnosis_code_16, r.diagnosis_code_17, r.diagnosis_code_18, r.diagnosis_code_19, r.diagnosis_code_20, r.diagnosis_code_21, r.diagnosis_code_22, r.diagnosis_code_23, r.diagnosis_code_24, r.diagnosis_code_25]
}

fn px(r: &MedicalClaim) -> [Option<Str>; 25] {
    [r.procedure_code_1, r.procedure_code_2, r.procedure_code_3, r.procedure_code_4, r.procedure_code_5, r.procedure_code_6, r.procedure_code_7, r.procedure_code_8, r.procedure_code_9, r.procedure_code_10, r.procedure_code_11, r.procedure_code_12, r.procedure_code_13, r.procedure_code_14, r.procedure_code_15, r.procedure_code_16, r.procedure_code_17, r.procedure_code_18, r.procedure_code_19, r.procedure_code_20, r.procedure_code_21, r.procedure_code_22, r.procedure_code_23, r.procedure_code_24, r.procedure_code_25]
}

fn pdates(r: &MedicalClaim) -> [Option<Date>; 25] {
    [r.procedure_date_1, r.procedure_date_2, r.procedure_date_3, r.procedure_date_4, r.procedure_date_5, r.procedure_date_6, r.procedure_date_7, r.procedure_date_8, r.procedure_date_9, r.procedure_date_10, r.procedure_date_11, r.procedure_date_12, r.procedure_date_13, r.procedure_date_14, r.procedure_date_15, r.procedure_date_16, r.procedure_date_17, r.procedure_date_18, r.procedure_date_19, r.procedure_date_20, r.procedure_date_21, r.procedure_date_22, r.procedure_date_23, r.procedure_date_24, r.procedure_date_25]
}

fn k5(r: &MedicalClaim) -> K5 {
    (r.person_id, r.member_id, r.payer, r.plan, r.data_source)
}

fn k3(r: &MedicalClaim) -> K3 {
    (r.claim_id, r.claim_line_number, r.data_source)
}

fn ym(d: Date) -> (i64, i64) {
    (year(d), month(d))
}

fn claim_date(r: &MedicalClaim) -> Option<Date> {
    r.claim_line_start_date.or(Some(r.claim_start_date)).or(r.admission_date)
}

fn elig_cond(r: &MedicalClaim, today: Date) -> bool {
    claim_date(r).is_some_and(|d| {
        d >= d2000() && d <= today && d >= date(1900, 1, 1) && d <= date(2100, 12, 31) && d <= date(2026, 1, 1)
    })
}

fn codes(r: &MedicalClaim, ty: Option<Str>, cs: [Option<Str>; 25], types: [&str; 2]) -> Vec<Code> {
    let mut v = Vec::new();
    if let Some(ty) = ty
        && types.contains(&ty)
    {
        for (i, c) in cs.into_iter().enumerate() {
            if let Some(c) = c {
                v.push(Code { key: k3(r), pos: i as i64 + 1, ty, code: nodot(c) });
            }
        }
    }
    v
}

fn flags(r: &MedicalClaim, l: Lk, today: Date) -> MedicalClaimLineFlags {
    let ct = r.claim_type;
    let inst = ct == "institutional";
    let prof = ct == "professional";
    let inpat = inst && r.bill_type_code.is_some_and(|b| IP.contains(&sub2(b)));
    let acute = inst && r.bill_type_code.is_some_and(|b| ["11", "12"].contains(&sub2(b)));
    let (dx, px, pd) = (dx(r), px(r), pdates(r));
    let dxt_ok = r.diagnosis_code_type.is_some_and(|t| DX_TYPES.contains(&t));
    let pxt_ok = r.procedure_code_type.is_some_and(|t| PX_TYPES.contains(&t));
    MedicalClaimLineFlags {
        claim_id: r.claim_id,
        claim_line_number: r.claim_line_number,
        data_source: r.data_source,
        claim_line_number_not_positive: w(true, r.claim_line_number <= 0),
        claim_type_null: w(true, false),
        claim_type_invalid: w(true, !["professional", "institutional", "undetermined"].contains(&ct)),
        institutional_indicators_present_for_professional_claim: w(prof, r.bill_type_code.is_some() || r.drg_code.is_some() || r.admit_type_code.is_some() || r.admit_source_code.is_some() || r.discharge_disposition_code.is_some() || r.revenue_center_code.is_some()),
        person_id_null: w(true, false),
        in_network_flag_invalid: w(true, !["0", "1"].contains(&r.in_network_flag.to_string().trim())),
        claim_start_date_null: w(true, false),
        claim_end_date_null: w(true, false),
        claim_line_start_date_null: w(true, r.claim_line_start_date.is_none()),
        claim_line_end_date_null: w(true, r.claim_line_end_date.is_none()),
        claim_start_date_out_of_reasonable_range: oor(Some(r.claim_start_date), today),
        claim_end_date_out_of_reasonable_range: oor(Some(r.claim_end_date), today),
        claim_line_start_date_out_of_reasonable_range: oor(r.claim_line_start_date, today),
        claim_line_end_date_out_of_reasonable_range: oor(r.claim_line_end_date, today),
        claim_start_after_claim_end: gt2(Some(r.claim_start_date), Some(r.claim_end_date)),
        claim_line_start_after_claim_line_end: gt2(r.claim_line_start_date, r.claim_line_end_date),
        admission_date_after_discharge_date: gt2(r.admission_date, r.discharge_date),
        admission_date_out_of_reasonable_range: oor(r.admission_date, today),
        admission_date_null_for_inpatient_claim: w(inpat, r.admission_date.is_none()),
        discharge_date_null_for_inpatient_claim: w(inpat, r.discharge_date.is_none()),
        discharge_date_out_of_reasonable_range: oor(r.discharge_date, today),
        paid_date_null: w(true, r.paid_date.is_none()),
        paid_date_out_of_reasonable_range: oor(r.paid_date, today),
        paid_date_before_claim_end_date: r.paid_date.map(|p| (p < r.claim_end_date) as i64),
        procedure_date_1_outside_supported_date_range: oor(pd[0], today),
        procedure_date_2_outside_supported_date_range: oor(pd[1], today),
        procedure_date_3_outside_supported_date_range: oor(pd[2], today),
        procedure_date_4_outside_supported_date_range: oor(pd[3], today),
        procedure_date_5_outside_supported_date_range: oor(pd[4], today),
        procedure_date_6_outside_supported_date_range: oor(pd[5], today),
        procedure_date_7_outside_supported_date_range: oor(pd[6], today),
        procedure_date_8_outside_supported_date_range: oor(pd[7], today),
        procedure_date_9_outside_supported_date_range: oor(pd[8], today),
        procedure_date_10_outside_supported_date_range: oor(pd[9], today),
        procedure_date_11_outside_supported_date_range: oor(pd[10], today),
        procedure_date_12_outside_supported_date_range: oor(pd[11], today),
        procedure_date_13_outside_supported_date_range: oor(pd[12], today),
        procedure_date_14_outside_supported_date_range: oor(pd[13], today),
        procedure_date_15_outside_supported_date_range: oor(pd[14], today),
        procedure_date_16_outside_supported_date_range: oor(pd[15], today),
        procedure_date_17_outside_supported_date_range: oor(pd[16], today),
        procedure_date_18_outside_supported_date_range: oor(pd[17], today),
        procedure_date_19_outside_supported_date_range: oor(pd[18], today),
        procedure_date_20_outside_supported_date_range: oor(pd[19], today),
        procedure_date_21_outside_supported_date_range: oor(pd[20], today),
        procedure_date_22_outside_supported_date_range: oor(pd[21], today),
        procedure_date_23_outside_supported_date_range: oor(pd[22], today),
        procedure_date_24_outside_supported_date_range: oor(pd[23], today),
        procedure_date_25_outside_supported_date_range: oor(pd[24], today),
        file_date_outside_supported_date_range: oor(Some(r.file_date), today),
        ingest_datetime_out_of_reasonable_range: oor(Some(trunc_day(r.ingest_datetime)), today),
        paid_amount_null: w(true, r.paid_amount.is_none()),
        paid_amount_lt_zero: r.paid_amount.map(|x| (x < 0.0) as i64),
        allowed_amount_null: w(true, r.allowed_amount.is_none()),
        allowed_amount_lt_zero: r.allowed_amount.map(|x| (x < 0.0) as i64),
        paid_amount_gt_allowed_amount: r.paid_amount.zip(r.allowed_amount).map(|(p, a)| (p > a) as i64),
        admit_source_code_invalid: w(inst && r.admit_source_code.is_some(), !l.admit_source),
        admit_type_code_invalid: w(inst && r.admit_type_code.is_some(), !l.admit_type),
        discharge_disposition_code_invalid: w(inst && r.discharge_disposition_code.is_some(), !l.discharge_disposition),
        place_of_service_code_invalid: w(prof && r.place_of_service_code.is_some(), !l.place_of_service),
        bill_type_code_invalid: w(inst && r.bill_type_code.is_some(), !l.bill_type),
        revenue_center_code_invalid: w(inst && r.revenue_center_code.is_some(), !l.revenue_center),
        place_of_service_code_null_for_professional_claim: w(prof, r.place_of_service_code.is_none()),
        place_of_service_code_present_for_institutional_claim: w(inst, r.place_of_service_code.is_some()),
        bill_type_code_null_for_institutional_claim: w(inst, r.bill_type_code.is_none()),
        revenue_center_code_null_for_institutional_claim: w(inst, r.revenue_center_code.is_none()),
        hcpcs_code_null_for_professional_claim: w(prof, r.hcpcs_code.is_none()),
        rendering_npi_invalid: w(true, !l.rendering),
        billing_npi_invalid: w(r.billing_npi.is_some(), !l.billing),
        facility_npi_invalid: w(r.facility_npi.is_some(), !l.facility),
        rendering_npi_null: w(true, false),
        billing_npi_null: w(true, r.billing_npi.is_none()),
        facility_npi_null_for_inpatient_claim: w(inpat, r.facility_npi.is_none()),
        drg_code_type_null_when_drg_code_present: w(r.drg_code.is_some(), false),
        drg_code_type_invalid: w(true, !DRG_TYPES.contains(&r.drg_code_type)),
        drg_code_invalid: w(inst && r.drg_code.is_some() && DRG_TYPES.contains(&r.drg_code_type), !l.ms_drg && !l.apr_drg),
        drg_code_null_for_acute_inpatient_claim: w(acute, r.drg_code.is_none()),
        diagnosis_code_1_null: w(true, dx[0].is_none()),
        diagnosis_code_type_null_when_diagnosis_code_present: w(dx.iter().any(|c| c.is_some()), r.diagnosis_code_type.is_none()),
        diagnosis_code_type_invalid: w(r.diagnosis_code_type.is_some(), !dxt_ok),
        diagnosis_code_1_invalid: w(dx[0].is_some() && dxt_ok, l.dx.map_or(0, |f| f.0) == 1),
        diagnosis_code_2_to_25_invalid: w(dx[1..].iter().any(|c| c.is_some()) && dxt_ok, l.dx.map_or(0, |f| f.1) == 1),
        procedure_code_type_null_when_procedure_code_present: w(px.iter().any(|c| c.is_some()), r.procedure_code_type.is_none()),
        procedure_code_type_invalid: w(r.procedure_code_type.is_some(), !pxt_ok),
        procedure_code_1_to_25_invalid: w(px.iter().any(|c| c.is_some()) && pxt_ok, l.px.unwrap_or(0) == 1),
        no_matching_eligibility_span: w(elig_cond(r, today), !l.elig),
    }
}

pub fn medical_claim_line_flags(db: &'static Db, el: &[Eligibility], mc: &[MedicalClaim]) -> Vec<MedicalClaimLineFlags> {
    let today = today();
    let eff_cap = date(2026, 1, 1);
    let er = rel(el
        .iter()
        .map(|e| {
            let end = e.enrollment_end_date;
            let eff = if end == date(9999, 12, 31) || end > eff_cap { eff_cap } else { end };
            ((e.person_id, e.member_id, e.payer, e.plan, e.data_source), e.enrollment_start_date, eff)
        })
        .collect::<Vec<Er>>());
    let src = rel(mc.iter().collect::<Vec<&MedicalClaim>>());

    let smm = (&src)
        .filt(|r: &MedicalClaim| elig_cond(r, today))
        .map(|r: &MedicalClaim| (k5(r), ym(claim_date(r).unwrap())))
        .inv()
        .fold((), |a, _| a);
    let smm = rel(drain(&smm).into_iter().map(|(k, _)| k).collect::<Vec<K7>>());
    let e_by: HashIdx<K5, usize> = (&er).map(|e: Er| e.0).inv().collect();
    let mm = (&smm)
        .and((&smm).map(|k: K7| k.0).select(&e_by).select(&er))
        .filt(|((_, (y, m)), (_, s, e)): (K7, Er)| {
            let ((sy, sm), (ey, em)) = (ym(s), ym(e));
            (y > sy || (y == sy && m >= sm)) && (y < ey || (y == ey && m <= em))
        })
        .map(|(k, _): (K7, Er)| k)
        .inv()
        .fold((), |a, _| a);

    let i10: HashIdx<Str, _> = (&db.icd10_cm.icd_10_cm).map(nodot).inv().collect();
    let i9: HashIdx<Str, _> = (&db.icd9_cm.icd_9_cm).map(nodot).inv().collect();
    let dxr = rel(drain((&src).flat_map(|r: &MedicalClaim| codes(r, r.diagnosis_code_type, dx(r), DX_TYPES)))
        .into_iter()
        .map(|(_, c)| c)
        .collect::<Vec<Code>>());
    let l10 = (&dxr).filt(|c: Code| c.ty == "icd-10-cm").map(|c: Code| c.code).select(&i10).opt();
    let l9 = (&dxr).filt(|c: Code| c.ty == "icd-9-cm").map(|c: Code| c.code).select(&i9).opt();
    let dxf = (&dxr).map(|c: Code| c.key).inv().select((&dxr).and(l10).and(l9)).fold((0i64, 0i64), |(a1, a2), ((c, x10), x9)| {
        let bad = (c.ty == "icd-10-cm" && x10.is_none()) || (c.ty == "icd-9-cm" && x9.is_none());
        let v1 = (c.pos == 1 && bad) as i64;
        let v2 = (c.pos > 1 && bad) as i64;
        (a1.max(v1), a2.max(v2))
    });

    let p10: HashIdx<Str, _> = (&db.icd10_pcs.icd_10_pcs).map(nodot).inv().collect();
    let p9: HashIdx<Str, _> = (&db.icd9_pcs.icd_9_pcs).map(nodot).inv().collect();
    let pxr = rel(drain((&src).flat_map(|r: &MedicalClaim| codes(r, r.procedure_code_type, px(r), PX_TYPES)))
        .into_iter()
        .map(|(_, c)| c)
        .collect::<Vec<Code>>());
    let q10 = (&pxr).filt(|c: Code| c.ty == "icd-10-pcs").map(|c: Code| c.code).select(&p10).opt();
    let q9 = (&pxr).filt(|c: Code| c.ty == "icd-9-pcs").map(|c: Code| c.code).select(&p9).opt();
    let pxf = (&pxr).map(|c: Code| c.key).inv().select((&pxr).and(q10).and(q9)).fold(0i64, |a, ((c, x10), x9)| {
        let bad = (c.ty == "icd-10-pcs" && x10.is_none()) || (c.ty == "icd-9-pcs" && x9.is_none());
        a.max(bad as i64)
    });

    let asx: HashIdx<Str, _> = (&db.admit_source.admit_source_code).inv().collect();
    let atx: HashIdx<Str, _> = (&db.admit_type.admit_type_code).inv().collect();
    let ddx: HashIdx<Str, _> = (&db.discharge_disposition.discharge_disposition_code).inv().collect();
    let posx: HashIdx<Str, _> = (&db.place_of_service.place_of_service_code).inv().collect();
    let btx: HashIdx<Str, _> = (&db.bill_type.bill_type_code).inv().collect();
    let rcx: HashIdx<Str, _> = (&db.revenue_center.revenue_center_code).inv().collect();
    let npx: HashIdx<Str, _> = (&db.provider.npi).inv().collect();
    let msx: HashIdx<Str, _> = (&db.ms_drg.ms_drg_code).inv().collect();
    let aprx: HashIdx<Str, _> = (&db.apr_drg.apr_drg_code).inv().collect();

    let j = (&src)
        .and((&src).flat_map(|r: &MedicalClaim| claim_date(r).map(|d| (k5(r), ym(d)))).select(&mm).opt())
        .and((&src).map(|r: &MedicalClaim| k3(r)).select(&dxf).opt())
        .and((&src).map(|r: &MedicalClaim| k3(r)).select(&pxf).opt())
        .and((&src).flat_map(|r: &MedicalClaim| r.admit_source_code).select(&asx).opt())
        .and((&src).flat_map(|r: &MedicalClaim| r.admit_type_code).select(&atx).opt())
        .and((&src).flat_map(|r: &MedicalClaim| r.discharge_disposition_code).select(&ddx).opt())
        .and((&src).flat_map(|r: &MedicalClaim| r.place_of_service_code).select(&posx).opt())
        .and((&src).flat_map(|r: &MedicalClaim| r.bill_type_code).select(&btx).opt())
        .and((&src).flat_map(|r: &MedicalClaim| r.revenue_center_code).select(&rcx).opt())
        .and((&src).map(|r: &MedicalClaim| r.rendering_npi).select(&npx).opt())
        .and((&src).flat_map(|r: &MedicalClaim| r.billing_npi).select(&npx).opt())
        .and((&src).flat_map(|r: &MedicalClaim| r.facility_npi).select(&npx).opt())
        .and((&src).filt(|r: &MedicalClaim| r.drg_code_type == "ms-drg").flat_map(|r: &MedicalClaim| r.drg_code).select(&msx).opt())
        .and((&src).filt(|r: &MedicalClaim| r.drg_code_type == "apr-drg").flat_map(|r: &MedicalClaim| r.drg_code).select(&aprx).opt());
    drain(&j)
        .into_iter()
        .map(|(_, ((((((((((((((r, elig), dxl), pxl), asrc), atyp), ddisp), pos), bt), rc), rn), bn), fac), ms), apr))| {
            let l = Lk {
                elig: elig.is_some(),
                dx: dxl,
                px: pxl,
                admit_source: asrc.is_some(),
                admit_type: atyp.is_some(),
                discharge_disposition: ddisp.is_some(),
                place_of_service: pos.is_some(),
                bill_type: bt.is_some(),
                revenue_center: rc.is_some(),
                rendering: rn.is_some(),
                billing: bn.is_some(),
                facility: fac.is_some(),
                ms_drg: ms.is_some(),
                apr_drg: apr.is_some(),
            };
            flags(r, l, today)
        })
        .collect()
}

pub fn q(db: &'static Db) -> String {
    let el = crate::concepts::eligibility::eligibility(db);
    let il_el = input_layer__eligibility(db, &el);
    let mc = crate::concepts::medical_claim::medical_claim(db);
    let il_mc = input_layer__medical_claim(db, &mc);
    rows(medical_claim_line_flags(db, &il_el, &il_mc).into_iter().map(|v| {
        row(vec![
            V::S(v.claim_id),
            V::I(v.claim_line_number),
            V::S(v.data_source),
            oint(v.claim_line_number_not_positive),
            oint(v.claim_type_null),
            oint(v.claim_type_invalid),
            oint(v.institutional_indicators_present_for_professional_claim),
            oint(v.person_id_null),
            oint(v.in_network_flag_invalid),
            oint(v.claim_start_date_null),
            oint(v.claim_end_date_null),
            oint(v.claim_line_start_date_null),
            oint(v.claim_line_end_date_null),
            oint(v.claim_start_date_out_of_reasonable_range),
            oint(v.claim_end_date_out_of_reasonable_range),
            oint(v.claim_line_start_date_out_of_reasonable_range),
            oint(v.claim_line_end_date_out_of_reasonable_range),
            oint(v.claim_start_after_claim_end),
            oint(v.claim_line_start_after_claim_line_end),
            oint(v.admission_date_after_discharge_date),
            oint(v.admission_date_out_of_reasonable_range),
            oint(v.admission_date_null_for_inpatient_claim),
            oint(v.discharge_date_null_for_inpatient_claim),
            oint(v.discharge_date_out_of_reasonable_range),
            oint(v.paid_date_null),
            oint(v.paid_date_out_of_reasonable_range),
            oint(v.paid_date_before_claim_end_date),
            oint(v.procedure_date_1_outside_supported_date_range),
            oint(v.procedure_date_2_outside_supported_date_range),
            oint(v.procedure_date_3_outside_supported_date_range),
            oint(v.procedure_date_4_outside_supported_date_range),
            oint(v.procedure_date_5_outside_supported_date_range),
            oint(v.procedure_date_6_outside_supported_date_range),
            oint(v.procedure_date_7_outside_supported_date_range),
            oint(v.procedure_date_8_outside_supported_date_range),
            oint(v.procedure_date_9_outside_supported_date_range),
            oint(v.procedure_date_10_outside_supported_date_range),
            oint(v.procedure_date_11_outside_supported_date_range),
            oint(v.procedure_date_12_outside_supported_date_range),
            oint(v.procedure_date_13_outside_supported_date_range),
            oint(v.procedure_date_14_outside_supported_date_range),
            oint(v.procedure_date_15_outside_supported_date_range),
            oint(v.procedure_date_16_outside_supported_date_range),
            oint(v.procedure_date_17_outside_supported_date_range),
            oint(v.procedure_date_18_outside_supported_date_range),
            oint(v.procedure_date_19_outside_supported_date_range),
            oint(v.procedure_date_20_outside_supported_date_range),
            oint(v.procedure_date_21_outside_supported_date_range),
            oint(v.procedure_date_22_outside_supported_date_range),
            oint(v.procedure_date_23_outside_supported_date_range),
            oint(v.procedure_date_24_outside_supported_date_range),
            oint(v.procedure_date_25_outside_supported_date_range),
            oint(v.file_date_outside_supported_date_range),
            oint(v.ingest_datetime_out_of_reasonable_range),
            oint(v.paid_amount_null),
            oint(v.paid_amount_lt_zero),
            oint(v.allowed_amount_null),
            oint(v.allowed_amount_lt_zero),
            oint(v.paid_amount_gt_allowed_amount),
            oint(v.admit_source_code_invalid),
            oint(v.admit_type_code_invalid),
            oint(v.discharge_disposition_code_invalid),
            oint(v.place_of_service_code_invalid),
            oint(v.bill_type_code_invalid),
            oint(v.revenue_center_code_invalid),
            oint(v.place_of_service_code_null_for_professional_claim),
            oint(v.place_of_service_code_present_for_institutional_claim),
            oint(v.bill_type_code_null_for_institutional_claim),
            oint(v.revenue_center_code_null_for_institutional_claim),
            oint(v.hcpcs_code_null_for_professional_claim),
            oint(v.rendering_npi_invalid),
            oint(v.billing_npi_invalid),
            oint(v.facility_npi_invalid),
            oint(v.rendering_npi_null),
            oint(v.billing_npi_null),
            oint(v.facility_npi_null_for_inpatient_claim),
            oint(v.drg_code_type_null_when_drg_code_present),
            oint(v.drg_code_type_invalid),
            oint(v.drg_code_invalid),
            oint(v.drg_code_null_for_acute_inpatient_claim),
            oint(v.diagnosis_code_1_null),
            oint(v.diagnosis_code_type_null_when_diagnosis_code_present),
            oint(v.diagnosis_code_type_invalid),
            oint(v.diagnosis_code_1_invalid),
            oint(v.diagnosis_code_2_to_25_invalid),
            oint(v.procedure_code_type_null_when_procedure_code_present),
            oint(v.procedure_code_type_invalid),
            oint(v.procedure_code_1_to_25_invalid),
            oint(v.no_matching_eligibility_span),
        ])
    }))
}
