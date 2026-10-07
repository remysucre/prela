use crate::concepts::int_eligibility_casting::IntEligibilityCasting;
use crate::concepts::int_eligibility_dates_normalized::IntEligibilityDatesNormalized;
use crate::concepts::int_eligibility_state_normalized::IntEligibilityStateNormalized;
use crate::schema::Db;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct NormalizedEligibility {
    pub eligibility_id: Option<Str>,
    pub person_id: Str,
    pub member_id: Str,
    pub subscriber_id: Option<Str>,
    pub subscriber_relation: Option<Str>,
    pub enrollment_start_date: Option<Date>,
    pub enrollment_end_date: Option<Date>,
    pub payer: Str,
    pub payer_type: Str,
    pub plan: Str,
    pub first_name: Option<Str>,
    pub middle_name: Option<Str>,
    pub last_name: Option<Str>,
    pub name_suffix: Option<Str>,
    pub social_security_number: Option<Str>,
    pub address: Option<Str>,
    pub city: Option<Str>,
    pub state: Str,
    pub zip_code: Option<Str>,
    pub phone: Option<Str>,
    pub email: Option<Str>,
    pub ethnicity: Option<Str>,
    pub sex: Str,
    pub race: Str,
    pub birth_date: Option<Date>,
    pub death_date: Option<Date>,
    pub death_flag: i64,
    pub original_reason_entitlement_code: Str,
    pub dual_status_code: Option<Str>,
    pub medicare_status_code: Str,
    pub enrollment_status: Option<Str>,
    pub hospice_flag: i64,
    pub institutional_snp_flag: i64,
    pub medicaid_indicator: Option<i64>,
    pub long_term_institutional_flag: i64,
    pub part_d_raf_type: Option<Str>,
    pub low_income_subsidy_indicator: Option<i64>,
    pub metal_level: Option<Str>,
    pub csr_indicator: Option<i64>,
    pub enrollment_duration_months: Option<i64>,
    pub esrd_status: Option<i64>,
    pub transplant_duration_months: Option<i64>,
    pub group_id: Str,
    pub group_name: Str,
    pub fips_state_code: Option<Str>,
    pub normalized_state_name: Option<Str>,
    pub fips_state_abbreviation: Option<Str>,
    pub file_date: Date,
    pub file_name: Str,
    pub ingest_datetime: Ts,
    pub tuva_last_run: Ts,
    pub data_source: Str,
}

fn leak(s: String) -> Str {
    Box::leak(s.into_boxed_str())
}

type Dn = IntEligibilityDatesNormalized;
type Sn = IntEligibilityStateNormalized;

pub fn normalized__eligibility(
    _db: &'static Db,
    ec: &[IntEligibilityCasting],
    dn: &[Dn],
    sn: &[Sn],
) -> Vec<NormalizedEligibility> {
    let src = rel(ec.to_vec());
    let dr = rel(dn.to_vec());
    let sr = rel(sn.to_vec());
    let d_by: HashIdx<Str, usize> = (&dr).map(|d: Dn| d.person_id_key).inv().collect();
    let s_by: HashIdx<Str, usize> = (&sr).map(|s: Sn| s.person_id_key).inv().collect();
    let key = (&src).map(|e: IntEligibilityCasting| e.person_id_key);
    let j = (&src)
        .and((&key).select((&d_by).select(&dr)).opt())
        .and((&key).select((&s_by).select(&sr)).opt());
    drain(&j)
        .into_iter()
        .map(|(_, ((e, d), s))| {
            let start = d.map(|d| d.normalized_enrollment_start_date);
            NormalizedEligibility {
                eligibility_id: start.map(|st| {
                    leak(format!(
                        "{}-{}-{}-{}-{}-{}",
                        e.person_id,
                        e.member_id,
                        fmt_date(st),
                        e.payer,
                        e.plan,
                        e.data_source
                    ))
                }),
                person_id: e.person_id,
                member_id: e.member_id,
                subscriber_id: e.subscriber_id,
                subscriber_relation: e.subscriber_relation,
                enrollment_start_date: start,
                enrollment_end_date: d.and_then(|d| d.normalized_enrollment_end_date),
                payer: e.payer,
                payer_type: e.payer_type,
                plan: e.plan,
                first_name: e.first_name,
                middle_name: e.middle_name,
                last_name: e.last_name,
                name_suffix: e.name_suffix,
                social_security_number: e.social_security_number,
                address: e.address,
                city: e.city,
                state: e.state,
                zip_code: e.zip_code,
                phone: e.phone,
                email: e.email,
                ethnicity: e.ethnicity,
                sex: e.sex,
                race: e.race,
                birth_date: d.map(|d| d.normalized_birth_date),
                death_date: d.and_then(|d| d.normalized_death_date),
                death_flag: e.death_flag,
                original_reason_entitlement_code: e.original_reason_entitlement_code,
                dual_status_code: e.dual_status_code,
                medicare_status_code: e.medicare_status_code,
                enrollment_status: e.enrollment_status,
                hospice_flag: e.hospice_flag,
                institutional_snp_flag: e.institutional_snp_flag,
                medicaid_indicator: e.medicaid_indicator,
                long_term_institutional_flag: e.long_term_institutional_flag,
                part_d_raf_type: e.part_d_raf_type,
                low_income_subsidy_indicator: e.low_income_subsidy_indicator,
                metal_level: e.metal_level,
                csr_indicator: e.csr_indicator,
                enrollment_duration_months: e.enrollment_duration_months,
                esrd_status: e.esrd_status,
                transplant_duration_months: e.transplant_duration_months,
                group_id: e.group_id,
                group_name: e.group_name,
                fips_state_code: s.and_then(|s| s.fips_state_code),
                normalized_state_name: s.and_then(|s| s.normalized_state_name),
                fips_state_abbreviation: s.and_then(|s| s.fips_state_abbreviation),
                file_date: e.file_date,
                file_name: e.file_name,
                ingest_datetime: e.ingest_datetime,
                tuva_last_run: e.tuva_last_run,
                data_source: e.data_source,
            }
        })
        .collect()
}

pub fn fmt(v: &NormalizedEligibility) -> String {
    row(vec![
        ostr(v.eligibility_id),
        V::S(v.person_id),
        V::S(v.member_id),
        ostr(v.subscriber_id),
        ostr(v.subscriber_relation),
        odate(v.enrollment_start_date),
        odate(v.enrollment_end_date),
        V::S(v.payer),
        V::S(v.payer_type),
        V::S(v.plan),
        ostr(v.first_name),
        ostr(v.middle_name),
        ostr(v.last_name),
        ostr(v.name_suffix),
        ostr(v.social_security_number),
        ostr(v.address),
        ostr(v.city),
        V::S(v.state),
        ostr(v.zip_code),
        ostr(v.phone),
        ostr(v.email),
        ostr(v.ethnicity),
        V::S(v.sex),
        V::S(v.race),
        odate(v.birth_date),
        odate(v.death_date),
        V::I(v.death_flag),
        V::S(v.original_reason_entitlement_code),
        ostr(v.dual_status_code),
        V::S(v.medicare_status_code),
        ostr(v.enrollment_status),
        V::I(v.hospice_flag),
        V::I(v.institutional_snp_flag),
        oint(v.medicaid_indicator),
        V::I(v.long_term_institutional_flag),
        ostr(v.part_d_raf_type),
        oint(v.low_income_subsidy_indicator),
        ostr(v.metal_level),
        oint(v.csr_indicator),
        oint(v.enrollment_duration_months),
        oint(v.esrd_status),
        oint(v.transplant_duration_months),
        V::S(v.group_id),
        V::S(v.group_name),
        ostr(v.fips_state_code),
        ostr(v.normalized_state_name),
        ostr(v.fips_state_abbreviation),
        V::D(v.file_date),
        V::S(v.file_name),
        V::T(v.ingest_datetime),
        V::T(v.tuva_last_run),
        V::S(v.data_source),
    ])
}

pub fn q(db: &'static Db) -> String {
    let el = crate::concepts::eligibility::eligibility(db);
    let il = crate::concepts::input_layer__eligibility::input_layer__eligibility(db, &el);
    let ec = crate::concepts::int_eligibility_casting::int_eligibility_casting(db, &il);
    let dn = crate::concepts::int_eligibility_dates_normalized::int_eligibility_dates_normalized(db, &ec);
    let sn = crate::concepts::int_eligibility_state_normalized::int_eligibility_state_normalized(db, &ec);
    rows(normalized__eligibility(db, &ec, &dn, &sn).iter().map(fmt))
}
