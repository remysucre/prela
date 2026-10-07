-- Tuva provider_ranking, compiled by dbt with every upstream model ephemeral (service_category and core__stg_claims_encounter stay tables)


-- Build a comprehensive provider ranking that shows all potential providers
-- a beneficiary could be attributed to, along with the earliest step each
-- provider qualifies for. Final assignment models can then pick rank = 1.

with  __dbt__cte__eligibility as (
select
    person_id
    , member_id
    , subscriber_id
    , subscriber_relation
    , enrollment_start_date
    , enrollment_end_date
    , payer
    , payer_type
    , plan
    , first_name
    , middle_name
    , last_name
    , name_suffix
    , social_security_number
    , address
    , city
    , state
    , zip_code
    , phone
    , email
    , ethnicity
    , sex
    , race
    , birth_date
    , death_date
    , death_flag
    , original_reason_entitlement_code
    , dual_status_code
    , medicare_status_code
    , enrollment_status
    , hospice_flag
    , institutional_snp_flag
    , medicaid_indicator
    , long_term_institutional_flag
    , part_d_raf_type
    , low_income_subsidy_indicator
    , metal_level
    , csr_indicator
    , enrollment_duration_months
    , esrd_status
    , transplant_duration_months
    , group_id
    , group_name
    , 'claims_' || cast(person_id as TEXT) as x_temp_record_origin
    , person_id as x_temp_person_id
    , first_name as x_temp_first_name
    , 'eligibility' as x_tuva_test_extension
    , 'eligibility' as ext_tuva_test_extension
    , 'extension-first-name' as x_first_name
    , 'extension-first-name' as ext_first_name
    , file_date
    , file_name
    , ingest_datetime
    , data_source
from "synthetic_data"."eligibility"
),  __dbt__cte__input_layer__eligibility as (

select *
from __dbt__cte__eligibility
),  __dbt__cte__int_eligibility_casting as (


with eligibility_source as (
    select
        elig.*
        , case
            when cast(elig.enrollment_end_date as date) = cast('9999-12-31' as date)
                then cast(null as date)
            else cast(elig.enrollment_end_date as date)
          end as _canonical_enrollment_end_date
    from __dbt__cte__input_layer__eligibility as elig
)

select
      cast(elig.person_id as TEXT) as person_id
    , (elig.person_id || coalesce(cast(elig.member_id as TEXT),'') || coalesce(elig.data_source,'') || coalesce(elig.payer,'') || coalesce(elig.plan,'') || coalesce(cast(elig.enrollment_start_date as TEXT),'') || coalesce(cast(elig._canonical_enrollment_end_date as TEXT),'')) as person_id_key
    , cast(elig.member_id as TEXT) as member_id
    , cast(elig.subscriber_id as TEXT) as subscriber_id
    , cast(elig.subscriber_relation as TEXT) as subscriber_relation
    , cast(elig.enrollment_start_date as date) as enrollment_start_date
    , elig._canonical_enrollment_end_date as enrollment_end_date
    , cast(elig.payer as TEXT) as payer
    , cast(elig.payer_type as TEXT) as payer_type
    , cast(elig.plan as TEXT) as plan
    , cast(elig.first_name as TEXT) as first_name
    , cast(elig.middle_name as TEXT) as middle_name
    , cast(elig.last_name as TEXT) as last_name
    , cast(elig.name_suffix as TEXT) as name_suffix
    , cast(elig.social_security_number as TEXT) as social_security_number
    , cast(elig.address as TEXT) as address
    , cast(elig.city as TEXT) as city
    , cast(elig.state as TEXT) as state
    , cast(elig.zip_code as TEXT) as zip_code
    , cast(elig.phone as TEXT) as phone
    , cast(elig.email as TEXT) as email
    , cast(elig.ethnicity as TEXT) as ethnicity
    , cast(elig.sex as TEXT) as sex
    , cast(elig.race as TEXT) as race
    , cast(elig.birth_date as date) as birth_date
    , cast(elig.death_date as date) as death_date
    , cast(elig.death_flag as integer) as death_flag
    , cast(elig.original_reason_entitlement_code as TEXT) as original_reason_entitlement_code
    , cast(elig.dual_status_code as TEXT) as dual_status_code
    , cast(elig.medicare_status_code as TEXT) as medicare_status_code
    , cast(elig.enrollment_status as TEXT) as enrollment_status
    , cast(elig.hospice_flag as integer) as hospice_flag
    , cast(elig.institutional_snp_flag as integer) as institutional_snp_flag
    , cast(elig.medicaid_indicator as integer) as medicaid_indicator
    , cast(elig.long_term_institutional_flag as integer) as long_term_institutional_flag
    , cast(elig.part_d_raf_type as TEXT) as part_d_raf_type
    , cast(elig.low_income_subsidy_indicator as integer) as low_income_subsidy_indicator
    , cast(elig.metal_level as TEXT) as metal_level
    , cast(elig.csr_indicator as integer) as csr_indicator
    , cast(elig.enrollment_duration_months as integer) as enrollment_duration_months
    , cast(elig.esrd_status as integer) as esrd_status
    , cast(elig.transplant_duration_months as integer) as transplant_duration_months
    , cast(elig.group_id as TEXT) as group_id
    , cast(elig.group_name as TEXT) as group_name
    
    , cast(elig.file_name as TEXT) as file_name
    , cast(elig.file_date as date) as file_date
    , cast(elig.ingest_datetime as timestamp) as ingest_datetime
    , cast('2026-01-01 00:00:00' as timestamp) as tuva_last_run
    , cast(elig.data_source as TEXT) as data_source
from eligibility_source as elig
),  __dbt__cte__int_eligibility_dates_normalized as (



select distinct
    elig.person_id
  , elig.person_id_key
  , elig.birth_date as normalized_birth_date
  , elig.death_date as normalized_death_date
  , elig.enrollment_start_date as normalized_enrollment_start_date
  , elig.enrollment_end_date as normalized_enrollment_end_date
from __dbt__cte__int_eligibility_casting as elig
),  __dbt__cte__int_eligibility_state_normalized as (



select distinct
    elig.person_id
    , elig.person_id_key
    , ansi.ansi_fips_state_name as normalized_state_name
    , ansi.ansi_fips_state_code as fips_state_code
    , ansi.ansi_fips_state_abbreviation as fips_state_abbreviation
from __dbt__cte__int_eligibility_casting as elig
left outer join "terminology"."ansi_fips_state" as ansi
  on (
       trim(lower(elig.state)) = trim(lower(ansi.ansi_fips_state_abbreviation))
    or trim(lower(elig.state)) = trim(lower(ansi.ansi_fips_state_code))
    or trim(lower(elig.state)) = trim(lower(ansi.ansi_fips_state_name))
  )
),  __dbt__cte__normalized__eligibility as (


select
      (elig.person_id || '-' || elig.member_id || '-' || date_norm.normalized_enrollment_start_date || '-' || elig.payer || '-' || elig.plan || '-' || elig.data_source) as eligibility_id
    , elig.person_id
    , elig.member_id
    , elig.subscriber_id
    , elig.subscriber_relation
    , date_norm.normalized_enrollment_start_date as enrollment_start_date
    , date_norm.normalized_enrollment_end_date as enrollment_end_date
    , elig.payer
    , elig.payer_type
    , elig.plan
    , elig.first_name
    , elig.middle_name
    , elig.last_name
    , elig.name_suffix
    , elig.social_security_number
    , elig.address
    , elig.city
    , elig.state
    , elig.zip_code
    , elig.phone
    , elig.email
    , elig.ethnicity
    , elig.sex
    , elig.race
    , date_norm.normalized_birth_date as birth_date
    , date_norm.normalized_death_date as death_date
    , elig.death_flag
    , elig.original_reason_entitlement_code
    , elig.dual_status_code
    , elig.medicare_status_code
    , elig.enrollment_status
    , elig.hospice_flag
    , elig.institutional_snp_flag
    , elig.medicaid_indicator
    , elig.long_term_institutional_flag
    , elig.part_d_raf_type
    , elig.low_income_subsidy_indicator
    , elig.metal_level
    , elig.csr_indicator
    , elig.enrollment_duration_months
    , elig.esrd_status
    , elig.transplant_duration_months
    , elig.group_id
    , elig.group_name
    , state_norm.fips_state_code
    , state_norm.normalized_state_name
    , state_norm.fips_state_abbreviation
    
    , elig.file_date
    , elig.file_name
    , elig.ingest_datetime
    , elig.tuva_last_run
    , elig.data_source
from __dbt__cte__int_eligibility_casting as elig
left outer join __dbt__cte__int_eligibility_dates_normalized as date_norm
  on elig.person_id_key = date_norm.person_id_key
left outer join __dbt__cte__int_eligibility_state_normalized as state_norm
  on elig.person_id_key = state_norm.person_id_key
),  __dbt__cte__member_month__month_spine as (


-- Complete package-owned month spine for Tuva's supported date range.
-- 1900-01 through 2100-12 is 2,412 consecutive months. The inline digit
-- relations avoid recursive and nested CTEs so the model remains portable.


select
    strftime('%Y%m', cast(month_starts.first_day_of_month as date)) as year_month
  , cast(strftime('%Y%m', cast(month_starts.first_day_of_month as date)) as integer) as year_month_int
  , cast(date_part('year', month_starts.first_day_of_month) as integer) as year
  , cast(date_part('month', month_starts.first_day_of_month) as integer) as month
  , month_starts.first_day_of_month
  , cast(
      

    (

    (month_starts.first_day_of_month + cast(1 as bigint) * interval 1 month) + cast(-1 as bigint) * interval 1 day)
      as date
    ) as last_day_of_month
from (
  select
    cast(
      

    (cast('1900-01-01' as date) + cast(generated_months.month_offset as bigint) * interval 1 month)
      as date
    ) as first_day_of_month
  from (
    select cast(
        ones.digit
        + (tens.digit * 10)
        + (hundreds.digit * 100)
        + (thousands.digit * 1000)
      as integer) as month_offset
    from (
  select 0 as digit
  union all select 1
  union all select 2
  union all select 3
  union all select 4
  union all select 5
  union all select 6
  union all select 7
  union all select 8
  union all select 9
) as ones
    cross join (
  select 0 as digit
  union all select 1
  union all select 2
  union all select 3
  union all select 4
  union all select 5
  union all select 6
  union all select 7
  union all select 8
  union all select 9
) as tens
    cross join (
  select 0 as digit
  union all select 1
  union all select 2
  union all select 3
  union all select 4
  union all select 5
  union all select 6
  union all select 7
  union all select 8
  union all select 9
) as hundreds
    cross join (
  select 0 as digit
  union all select 1
  union all select 2
  union all select 3
  union all select 4
  union all select 5
  union all select 6
  union all select 7
  union all select 8
  union all select 9
) as thousands
  ) as generated_months
  where generated_months.month_offset < 2412
) as month_starts
),  __dbt__cte__member_month as (


with stg_eligibility as (
  select
    person_id
    , member_id
    , payer
    , plan
    , enrollment_start_date
    , enrollment_end_date
    , tuva_last_run
    , data_source
  from __dbt__cte__normalized__eligibility as elig
)

, eligibility_with_effective_end_date as (
  select
    *
    , case
        when enrollment_end_date is null
          or enrollment_end_date > cast(tuva_last_run as date)
          then cast(tuva_last_run as date)
        else enrollment_end_date
      end as effective_enrollment_end_date
  from stg_eligibility
)

, month_start_and_end_dates as (
  select
      year_month
    , first_day_of_month as month_start_date
    , last_day_of_month as month_end_date
  from __dbt__cte__member_month__month_spine
)

, joined as (
select distinct
  a.person_id
  , a.member_id
  , b.year_month
  , a.payer
  , a.plan
  , a.tuva_last_run
  , a.data_source
from eligibility_with_effective_end_date as a
inner join month_start_and_end_dates as b
  on a.enrollment_start_date <= b.month_end_date
  and a.effective_enrollment_end_date >= b.month_start_date
  and a.enrollment_start_date <= a.effective_enrollment_end_date
)

select
  cast(
    md5(cast(coalesce(cast(person_id as TEXT), '_dbt_utils_surrogate_key_null_') || '-' || coalesce(cast(member_id as TEXT), '_dbt_utils_surrogate_key_null_') || '-' || coalesce(cast(year_month as TEXT), '_dbt_utils_surrogate_key_null_') || '-' || coalesce(cast(payer as TEXT), '_dbt_utils_surrogate_key_null_') || '-' || coalesce(cast(plan as TEXT), '_dbt_utils_surrogate_key_null_') || '-' || coalesce(cast(data_source as TEXT), '_dbt_utils_surrogate_key_null_') as TEXT))
    as TEXT
  ) as member_month_id
, *
from joined
),  __dbt__cte__medical_claim as (
select
    claim_id
    , claim_line_number
    , claim_type
    , person_id
    , member_id
    , payer
    , plan
    , claim_start_date
    , claim_end_date
    , claim_line_start_date
    , claim_line_end_date
    , admission_date
    , discharge_date
    , admit_source_code
    , admit_type_code
    , discharge_disposition_code
    , place_of_service_code
    , bill_type_code
    , drg_code_type
    , drg_code
    , revenue_center_code
    , service_unit_quantity
    , hcpcs_code
    , hcpcs_modifier_1
    , hcpcs_modifier_2
    , hcpcs_modifier_3
    , hcpcs_modifier_4
    , hcpcs_modifier_5
    , rendering_npi
    , rendering_tin
    , billing_npi
    , billing_tin
    , facility_npi
    , paid_date
    , paid_amount
    , allowed_amount
    , charge_amount
    , coinsurance_amount
    , copayment_amount
    , deductible_amount
    , total_cost_amount
    , diagnosis_code_type
    , diagnosis_code_1
    , diagnosis_code_2
    , diagnosis_code_3
    , diagnosis_code_4
    , diagnosis_code_5
    , diagnosis_code_6
    , diagnosis_code_7
    , diagnosis_code_8
    , diagnosis_code_9
    , diagnosis_code_10
    , diagnosis_code_11
    , diagnosis_code_12
    , diagnosis_code_13
    , diagnosis_code_14
    , diagnosis_code_15
    , diagnosis_code_16
    , diagnosis_code_17
    , diagnosis_code_18
    , diagnosis_code_19
    , diagnosis_code_20
    , diagnosis_code_21
    , diagnosis_code_22
    , diagnosis_code_23
    , diagnosis_code_24
    , diagnosis_code_25
    , diagnosis_poa_1
    , diagnosis_poa_2
    , diagnosis_poa_3
    , diagnosis_poa_4
    , diagnosis_poa_5
    , diagnosis_poa_6
    , diagnosis_poa_7
    , diagnosis_poa_8
    , diagnosis_poa_9
    , diagnosis_poa_10
    , diagnosis_poa_11
    , diagnosis_poa_12
    , diagnosis_poa_13
    , diagnosis_poa_14
    , diagnosis_poa_15
    , diagnosis_poa_16
    , diagnosis_poa_17
    , diagnosis_poa_18
    , diagnosis_poa_19
    , diagnosis_poa_20
    , diagnosis_poa_21
    , diagnosis_poa_22
    , diagnosis_poa_23
    , diagnosis_poa_24
    , diagnosis_poa_25
    , procedure_code_type
    , procedure_code_1
    , procedure_code_2
    , procedure_code_3
    , procedure_code_4
    , procedure_code_5
    , procedure_code_6
    , procedure_code_7
    , procedure_code_8
    , procedure_code_9
    , procedure_code_10
    , procedure_code_11
    , procedure_code_12
    , procedure_code_13
    , procedure_code_14
    , procedure_code_15
    , procedure_code_16
    , procedure_code_17
    , procedure_code_18
    , procedure_code_19
    , procedure_code_20
    , procedure_code_21
    , procedure_code_22
    , procedure_code_23
    , procedure_code_24
    , procedure_code_25
    , procedure_date_1
    , procedure_date_2
    , procedure_date_3
    , procedure_date_4
    , procedure_date_5
    , procedure_date_6
    , procedure_date_7
    , procedure_date_8
    , procedure_date_9
    , procedure_date_10
    , procedure_date_11
    , procedure_date_12
    , procedure_date_13
    , procedure_date_14
    , procedure_date_15
    , procedure_date_16
    , procedure_date_17
    , procedure_date_18
    , procedure_date_19
    , procedure_date_20
    , procedure_date_21
    , procedure_date_22
    , procedure_date_23
    , procedure_date_24
    , procedure_date_25
    , in_network_flag
    , claim_id as x_temp_claim_id
    , payer as x_temp_payer
    , 'medical_claim' as x_tuva_test_extension
    , 'medical_claim' as ext_tuva_test_extension
    , data_source
    , file_date
    , file_name
    , ingest_datetime
from "synthetic_data"."medical_claim"
),  __dbt__cte__input_layer__medical_claim as (

select *
from __dbt__cte__medical_claim
),  __dbt__cte__normalized_input__stg_medical_claim as (


select
      claim_id
    , claim_line_number
    , claim_type
    , person_id
    , member_id
    , payer
    , plan
    , claim_start_date
    , claim_end_date
    , claim_line_start_date
    , claim_line_end_date
    , admission_date
    , discharge_date
    , admit_source_code
    , admit_type_code
    , discharge_disposition_code
    , place_of_service_code
    , bill_type_code
    , drg_code_type
    , drg_code
    , revenue_center_code
    , service_unit_quantity
    , hcpcs_code
    , hcpcs_modifier_1
    , hcpcs_modifier_2
    , hcpcs_modifier_3
    , hcpcs_modifier_4
    , hcpcs_modifier_5
    , rendering_npi
    , rendering_tin
    , billing_npi
    , billing_tin
    , facility_npi
    , paid_date
    , paid_amount
    , allowed_amount
    , charge_amount
    , coinsurance_amount
    , copayment_amount
    , deductible_amount
    , total_cost_amount
    , diagnosis_code_type
    , diagnosis_code_1
    , diagnosis_code_2
    , diagnosis_code_3
    , diagnosis_code_4
    , diagnosis_code_5
    , diagnosis_code_6
    , diagnosis_code_7
    , diagnosis_code_8
    , diagnosis_code_9
    , diagnosis_code_10
    , diagnosis_code_11
    , diagnosis_code_12
    , diagnosis_code_13
    , diagnosis_code_14
    , diagnosis_code_15
    , diagnosis_code_16
    , diagnosis_code_17
    , diagnosis_code_18
    , diagnosis_code_19
    , diagnosis_code_20
    , diagnosis_code_21
    , diagnosis_code_22
    , diagnosis_code_23
    , diagnosis_code_24
    , diagnosis_code_25
    , diagnosis_poa_1
    , diagnosis_poa_2
    , diagnosis_poa_3
    , diagnosis_poa_4
    , diagnosis_poa_5
    , diagnosis_poa_6
    , diagnosis_poa_7
    , diagnosis_poa_8
    , diagnosis_poa_9
    , diagnosis_poa_10
    , diagnosis_poa_11
    , diagnosis_poa_12
    , diagnosis_poa_13
    , diagnosis_poa_14
    , diagnosis_poa_15
    , diagnosis_poa_16
    , diagnosis_poa_17
    , diagnosis_poa_18
    , diagnosis_poa_19
    , diagnosis_poa_20
    , diagnosis_poa_21
    , diagnosis_poa_22
    , diagnosis_poa_23
    , diagnosis_poa_24
    , diagnosis_poa_25
    , procedure_code_type
    , procedure_code_1
    , procedure_code_2
    , procedure_code_3
    , procedure_code_4
    , procedure_code_5
    , procedure_code_6
    , procedure_code_7
    , procedure_code_8
    , procedure_code_9
    , procedure_code_10
    , procedure_code_11
    , procedure_code_12
    , procedure_code_13
    , procedure_code_14
    , procedure_code_15
    , procedure_code_16
    , procedure_code_17
    , procedure_code_18
    , procedure_code_19
    , procedure_code_20
    , procedure_code_21
    , procedure_code_22
    , procedure_code_23
    , procedure_code_24
    , procedure_code_25
    , procedure_date_1
    , procedure_date_2
    , procedure_date_3
    , procedure_date_4
    , procedure_date_5
    , procedure_date_6
    , procedure_date_7
    , procedure_date_8
    , procedure_date_9
    , procedure_date_10
    , procedure_date_11
    , procedure_date_12
    , procedure_date_13
    , procedure_date_14
    , procedure_date_15
    , procedure_date_16
    , procedure_date_17
    , procedure_date_18
    , procedure_date_19
    , procedure_date_20
    , procedure_date_21
    , procedure_date_22
    , procedure_date_23
    , procedure_date_24
    , procedure_date_25
    , in_network_flag
    , data_source
    , file_name
    , file_date
    , ingest_datetime
    
from __dbt__cte__input_layer__medical_claim
),  __dbt__cte___int_normalized_input_admit_source_voting as (




with normalize_cte as (
    select
        med.claim_id
        , med.data_source
        , admit.admit_source_code

        , admit.admit_source_description
    from __dbt__cte__normalized_input__stg_medical_claim as med
    inner join "terminology"."admit_source" as admit
        on med.admit_source_code = admit.admit_source_code
    where claim_type = 'institutional'
)

, distinct_counts as (
    select
        claim_id
        , data_source
        , admit_source_code
        , admit_source_description
        , count(*) as admit_source_occurrence_count
    from normalize_cte
    where admit_source_code is not null
    group by
        claim_id
        , data_source
        , admit_source_code
        , admit_source_description
)

, occurence_comparison as (
    select
        claim_id
        , data_source
        , 'admit_source_code' as column_name
        , admit_source_code as normalized_code
        , admit_source_description as normalized_description
        , admit_source_occurrence_count as occurrence_count
        , coalesce(lead(admit_source_occurrence_count)
            over (partition by claim_id, data_source
order by admit_source_occurrence_count desc), 0) as next_occurrence_count
        , row_number() over (partition by claim_id, data_source
order by admit_source_occurrence_count desc) as occurrence_row_count
    from distinct_counts as dist
)

select
    claim_id
    , data_source
    , column_name
    , normalized_code
    , normalized_description
    , occurrence_count
    , next_occurrence_count
    , occurrence_row_count
    , cast('2026-01-01 00:00:00' as timestamp) as tuva_last_run
from occurence_comparison
),  __dbt__cte___int_normalized_input_admit_source_final as (



select
    claim_id
    , data_source
    , column_name
    , normalized_code
    , normalized_description
    , occurrence_count
    , next_occurrence_count
    , occurrence_row_count
    , cast('2026-01-01 00:00:00' as timestamp) as tuva_last_run
from __dbt__cte___int_normalized_input_admit_source_voting
where (occurrence_row_count = 1
        and occurrence_count > next_occurrence_count)
),  __dbt__cte___int_normalized_input_admit_type_normalize_voting as (



with normalize_cte as (
    select
        med.claim_id
        , med.data_source
        , admit.admit_type_code
        , admit.admit_type_description
    from __dbt__cte__normalized_input__stg_medical_claim as med
    inner join "terminology"."admit_type" as admit
        on med.admit_type_code = admit.admit_type_code
    where claim_type = 'institutional'
)

, distinct_counts as (
    select
        claim_id
        , data_source
        , admit_type_code
        , admit_type_description
        , count(*) as admit_type_occurrence_count
    from normalize_cte

    where admit_type_code is not null
    group by
        claim_id
        , data_source
        , admit_type_code
        , admit_type_description
)

, occurence_comparison as (
    select
        claim_id
        , data_source
        , 'admit_type_code' as column_name
        , admit_type_code as normalized_code
        , admit_type_description as normalized_description
        , admit_type_occurrence_count as occurrence_count
        , coalesce(lead(admit_type_occurrence_count)
            over (partition by claim_id, data_source
order by admit_type_occurrence_count desc), 0) as next_occurrence_count
        , row_number() over (partition by claim_id, data_source
order by admit_type_occurrence_count desc) as occurrence_row_count
    from distinct_counts as dist
)

select
    claim_id
    , data_source
    , column_name
    , normalized_code
    , normalized_description
    , occurrence_count
    , next_occurrence_count
    , occurrence_row_count
    , cast('2026-01-01 00:00:00' as timestamp) as tuva_last_run
from occurence_comparison
),  __dbt__cte___int_normalized_input_admit_type_final as (



select
    claim_id
    , data_source
    , column_name
    , normalized_code
    , normalized_description
    , occurrence_count
    , next_occurrence_count
    , occurrence_row_count
    , cast('2026-01-01 00:00:00' as timestamp) as tuva_last_run
from __dbt__cte___int_normalized_input_admit_type_normalize_voting
where (occurrence_row_count = 1
        and occurrence_count > next_occurrence_count)
),  __dbt__cte___int_normalized_input_bill_type_voting as (



with normalize_cte as (
    select
        med.claim_id
        , med.data_source
        , bill.bill_type_code
        , bill.bill_type_description
    from __dbt__cte__normalized_input__stg_medical_claim as med
    inner join "terminology"."bill_type" as bill
        on ltrim(med.bill_type_code, '0') = bill.bill_type_code
    where claim_type = 'institutional'
)

, distinct_counts as (
    select
        claim_id
        , data_source
        , bill_type_code
        , bill_type_description
        , count(*) as bill_type_occurrence_count
    from normalize_cte
    where bill_type_code is not null
    group by
        claim_id
        , data_source
        , bill_type_code
        , bill_type_description
)

, occurence_comparison as (
    select
        claim_id
        , data_source
        , 'bill_type_code' as column_name
        , bill_type_code as normalized_code
        , bill_type_description as normalized_description
        , bill_type_occurrence_count as occurrence_count
        , row_number() over (partition by claim_id, data_source
order by bill_type_occurrence_count desc, bill_type_code asc) as occurrence_row_count
    from distinct_counts as dist
)

select
    claim_id
    , data_source
    , column_name
    , normalized_code
    , normalized_description
    , occurrence_count
    , occurrence_row_count
    , cast('2026-01-01 00:00:00' as timestamp) as tuva_last_run
from occurence_comparison
),  __dbt__cte___int_normalized_input_bill_type_final as (



select
    claim_id
    , data_source
    , column_name
    , normalized_code
    , normalized_description
    , occurrence_count
    , occurrence_row_count
    , cast('2026-01-01 00:00:00' as timestamp) as tuva_last_run
from __dbt__cte___int_normalized_input_bill_type_voting
where occurrence_row_count = 1
),  __dbt__cte___int_normalized_input_medical_claim_date_normalize as (



select distinct
  med.claim_id
  , med.claim_line_number
  , med.claim_type
  , med.data_source
  , med.claim_start_date as normalized_claim_start_date
  , med.claim_end_date as normalized_claim_end_date
  , med.claim_line_start_date as normalized_claim_line_start_date
  , med.claim_line_end_date as normalized_claim_line_end_date
  , med.admission_date as normalized_admission_date
  , med.discharge_date as normalized_discharge_date
  , cast('2026-01-01 00:00:00' as timestamp) as tuva_last_run
from __dbt__cte__normalized_input__stg_medical_claim as med
),  __dbt__cte___int_normalized_input_medical_date_aggregation as (



select
    claim_id
    , data_source
    , min(normalized_claim_start_date) as minimum_claim_start_date
    , max(normalized_claim_end_date) as maximum_claim_end_date
    , min(normalized_admission_date) as minimum_admission_date
    , max(normalized_discharge_date) as maximum_discharge_date
    , cast('2026-01-01 00:00:00' as timestamp) as tuva_last_run
from __dbt__cte___int_normalized_input_medical_claim_date_normalize
where claim_type = 'institutional'
group by
    claim_id
    , data_source

union all

select
    claim_id
    , data_source
    , min(normalized_claim_start_date) as minimum_claim_start_date
    , max(normalized_claim_end_date) as maximum_claim_end_date
    , null as minimum_admission_date
    , null as maximum_discharge_date
    , cast('2026-01-01 00:00:00' as timestamp) as tuva_last_run
from __dbt__cte___int_normalized_input_medical_claim_date_normalize
where claim_type = 'professional'
group by
    claim_id
    , data_source
),  __dbt__cte___int_normalized_input_medical_npi_normalize as (


with base as (
select distinct
    med.claim_id
  , med.claim_line_number
  , med.claim_type
  , med.data_source
  , rend_prov.npi as normalized_rendering_npi
  , case
      when rend_prov.entity_type_code = '1' then
        cast((rend_prov.provider_last_name || ', ' || rend_prov.provider_first_name) as TEXT)
      else
        cast(rend_prov.provider_organization_name as TEXT)
    end as normalized_rendering_name
  , bill_prov.npi as normalized_billing_npi
  , case
      when bill_prov.entity_type_code = '1' then
        cast((bill_prov.provider_last_name || ', ' || bill_prov.provider_first_name) as TEXT)
      else
        cast(bill_prov.provider_organization_name as TEXT)
    end as normalized_billing_name
  , fac_prov.npi as normalized_facility_npi
  , case
      when fac_prov.entity_type_code = '1' then
        cast((fac_prov.provider_last_name || ', ' || fac_prov.provider_first_name) as TEXT)
      else
        cast(fac_prov.provider_organization_name as TEXT)
    end as normalized_facility_name
  , cast('2026-01-01 00:00:00' as timestamp) as tuva_last_run
from __dbt__cte__normalized_input__stg_medical_claim as med
left outer join "provider_data"."provider" as rend_prov
  on med.rendering_npi = rend_prov.npi
left outer join "provider_data"."provider" as bill_prov
  on med.billing_npi = bill_prov.npi
left outer join "provider_data"."provider" as fac_prov
  on med.facility_npi = fac_prov.npi
  and fac_prov.entity_type_description = 'Organization'
  and med.claim_type = 'institutional'
)

, facility_npi_ranked as (
select
    data_source
  , claim_id
  , normalized_facility_npi
  , normalized_facility_name
  , row_number() over (partition by data_source, claim_id order by claim_line_number) as facility_npi_rank
from base
where normalized_facility_npi is not null
)

select 
    base.claim_id
  , base.claim_line_number
  , base.claim_type
  , base.data_source
  , base.normalized_rendering_npi
  , base.normalized_rendering_name
  , base.normalized_billing_npi
  , base.normalized_billing_name
  , fac.normalized_facility_npi
  , fac.normalized_facility_name
  , base.tuva_last_run
from base
left join facility_npi_ranked as fac
  on base.claim_id = fac.claim_id
  and base.data_source = fac.data_source
  and facility_npi_rank = 1
),  __dbt__cte___int_normalized_input_discharge_disposition_voting as (



with normalize_cte as (
    select
        med.claim_id
        , med.data_source
        , disch.discharge_disposition_code
        , disch.discharge_disposition_description
    from __dbt__cte__normalized_input__stg_medical_claim as med
    inner join "terminology"."discharge_disposition" as disch
        on med.discharge_disposition_code = disch.discharge_disposition_code
    where claim_type = 'institutional'
)

, distinct_counts as (
    select
        claim_id
        , data_source
        , discharge_disposition_code
        , discharge_disposition_description
        , count(*) as discharge_disposition_occurrence_count
    from normalize_cte
    where discharge_disposition_code is not null
    group by
        claim_id
        , data_source
        , discharge_disposition_code
        , discharge_disposition_description
)

, occurence_comparison as (
    select
        claim_id
        , data_source
        , 'discharge_disposition_code' as column_name
        , discharge_disposition_code as normalized_code
        , discharge_disposition_description as normalized_description
        , discharge_disposition_occurrence_count as occurrence_count
        , coalesce(lead(discharge_disposition_occurrence_count)
            over (partition by claim_id, data_source
order by discharge_disposition_occurrence_count desc), 0) as next_occurrence_count
        , row_number() over (partition by claim_id, data_source
order by discharge_disposition_occurrence_count desc) as occurrence_row_count
    from distinct_counts as dist
)

select
    claim_id
    , data_source
    , column_name
    , normalized_code
    , normalized_description
    , occurrence_count
    , next_occurrence_count
    , occurrence_row_count
    , cast('2026-01-01 00:00:00' as timestamp) as tuva_last_run
from occurence_comparison
),  __dbt__cte___int_normalized_input_discharge_disposition_final as (



select
    claim_id
    , data_source
    , column_name
    , normalized_code
    , normalized_description
    , occurrence_count
    , next_occurrence_count
    , occurrence_row_count
    , cast('2026-01-01 00:00:00' as timestamp) as tuva_last_run
from __dbt__cte___int_normalized_input_discharge_disposition_voting
where (occurrence_row_count = 1
        and occurrence_count > next_occurrence_count)
),  __dbt__cte___int_normalized_input_drg_voting as (


with normalize_cte as (
    select
        med.claim_id
        , med.data_source
        , med.drg_code_type
        , coalesce(msdrg.ms_drg_code, aprdrg.apr_drg_code) as drg_code
        , coalesce(msdrg.ms_drg_description, aprdrg.apr_drg_description) as drg_description
    from __dbt__cte__normalized_input__stg_medical_claim as med
    left outer join "terminology"."ms_drg" as msdrg
        on med.drg_code_type = 'ms-drg'
        and med.drg_code = msdrg.ms_drg_code
    left outer join "terminology"."apr_drg" as aprdrg
        on med.drg_code_type = 'apr-drg'
        and med.drg_code = aprdrg.apr_drg_code
    where claim_type = 'institutional'
)

, distinct_counts as (
    select
        claim_id
        , data_source
        , drg_code
        , drg_description
        , count(*) as drg_occurrence_count
    from normalize_cte
    where drg_code is not null
    group by
        claim_id
        , data_source
        , drg_code
        , drg_description
)

, occurence_comparison as (
    select
        claim_id
        , data_source
        , 'drg_code' as column_name
        , drg_code as normalized_code
        , drg_description as normalized_description
        , drg_occurrence_count as occurrence_count
        , coalesce(lead(drg_occurrence_count)
            over (partition by claim_id, data_source
order by drg_occurrence_count desc), 0) as next_occurrence_count
        , row_number() over (partition by claim_id, data_source
order by drg_occurrence_count desc) as occurrence_row_count
    from distinct_counts as dist
)

select
    claim_id
    , data_source
    , column_name
    , normalized_code
    , normalized_description
    , occurrence_count
    , next_occurrence_count
    , occurrence_row_count
    , cast('2026-01-01 00:00:00' as timestamp) as tuva_last_run
from occurence_comparison
),  __dbt__cte___int_normalized_input_drg_final as (



select
    claim_id
    , data_source
    , column_name
    , normalized_code
    , normalized_description
    , occurrence_count
    , next_occurrence_count
    , occurrence_row_count
    , cast('2026-01-01 00:00:00' as timestamp) as tuva_last_run
from __dbt__cte___int_normalized_input_drg_voting
where (occurrence_row_count = 1
        and occurrence_count > next_occurrence_count)
),  __dbt__cte___int_normalized_input_place_of_service_normalize as (



select
    claim_id
    , claim_line_number
    , data_source
    , pos.place_of_service_code as normalized_code
    , pos.place_of_service_description as normalized_description
    , cast('2026-01-01 00:00:00' as timestamp) as tuva_last_run
from __dbt__cte__normalized_input__stg_medical_claim as med
left outer join "terminology"."place_of_service" as pos
    
        on lpad(med.place_of_service_code, 2, '0') = pos.place_of_service_code
    
where claim_type = 'professional'
),  __dbt__cte___int_normalized_input_procedure_code_intermediate as (


-- Define the range of procedure columns to pivot (e.g., procedure_code_1 to procedure_code_25)


-- Pivot procedure columns into a long format
with pivot_procedure as (
    
    select
        claim_id
        , claim_type
        , data_source
        , procedure_code_type
        , 'procedure_code_1' as column_name
        , procedure_code_1 as procedure_code
    from __dbt__cte__normalized_input__stg_medical_claim
    where procedure_code_1 is not null
    union all
    
    select
        claim_id
        , claim_type
        , data_source
        , procedure_code_type
        , 'procedure_code_2' as column_name
        , procedure_code_2 as procedure_code
    from __dbt__cte__normalized_input__stg_medical_claim
    where procedure_code_2 is not null
    union all
    
    select
        claim_id
        , claim_type
        , data_source
        , procedure_code_type
        , 'procedure_code_3' as column_name
        , procedure_code_3 as procedure_code
    from __dbt__cte__normalized_input__stg_medical_claim
    where procedure_code_3 is not null
    union all
    
    select
        claim_id
        , claim_type
        , data_source
        , procedure_code_type
        , 'procedure_code_4' as column_name
        , procedure_code_4 as procedure_code
    from __dbt__cte__normalized_input__stg_medical_claim
    where procedure_code_4 is not null
    union all
    
    select
        claim_id
        , claim_type
        , data_source
        , procedure_code_type
        , 'procedure_code_5' as column_name
        , procedure_code_5 as procedure_code
    from __dbt__cte__normalized_input__stg_medical_claim
    where procedure_code_5 is not null
    union all
    
    select
        claim_id
        , claim_type
        , data_source
        , procedure_code_type
        , 'procedure_code_6' as column_name
        , procedure_code_6 as procedure_code
    from __dbt__cte__normalized_input__stg_medical_claim
    where procedure_code_6 is not null
    union all
    
    select
        claim_id
        , claim_type
        , data_source
        , procedure_code_type
        , 'procedure_code_7' as column_name
        , procedure_code_7 as procedure_code
    from __dbt__cte__normalized_input__stg_medical_claim
    where procedure_code_7 is not null
    union all
    
    select
        claim_id
        , claim_type
        , data_source
        , procedure_code_type
        , 'procedure_code_8' as column_name
        , procedure_code_8 as procedure_code
    from __dbt__cte__normalized_input__stg_medical_claim
    where procedure_code_8 is not null
    union all
    
    select
        claim_id
        , claim_type
        , data_source
        , procedure_code_type
        , 'procedure_code_9' as column_name
        , procedure_code_9 as procedure_code
    from __dbt__cte__normalized_input__stg_medical_claim
    where procedure_code_9 is not null
    union all
    
    select
        claim_id
        , claim_type
        , data_source
        , procedure_code_type
        , 'procedure_code_10' as column_name
        , procedure_code_10 as procedure_code
    from __dbt__cte__normalized_input__stg_medical_claim
    where procedure_code_10 is not null
    union all
    
    select
        claim_id
        , claim_type
        , data_source
        , procedure_code_type
        , 'procedure_code_11' as column_name
        , procedure_code_11 as procedure_code
    from __dbt__cte__normalized_input__stg_medical_claim
    where procedure_code_11 is not null
    union all
    
    select
        claim_id
        , claim_type
        , data_source
        , procedure_code_type
        , 'procedure_code_12' as column_name
        , procedure_code_12 as procedure_code
    from __dbt__cte__normalized_input__stg_medical_claim
    where procedure_code_12 is not null
    union all
    
    select
        claim_id
        , claim_type
        , data_source
        , procedure_code_type
        , 'procedure_code_13' as column_name
        , procedure_code_13 as procedure_code
    from __dbt__cte__normalized_input__stg_medical_claim
    where procedure_code_13 is not null
    union all
    
    select
        claim_id
        , claim_type
        , data_source
        , procedure_code_type
        , 'procedure_code_14' as column_name
        , procedure_code_14 as procedure_code
    from __dbt__cte__normalized_input__stg_medical_claim
    where procedure_code_14 is not null
    union all
    
    select
        claim_id
        , claim_type
        , data_source
        , procedure_code_type
        , 'procedure_code_15' as column_name
        , procedure_code_15 as procedure_code
    from __dbt__cte__normalized_input__stg_medical_claim
    where procedure_code_15 is not null
    union all
    
    select
        claim_id
        , claim_type
        , data_source
        , procedure_code_type
        , 'procedure_code_16' as column_name
        , procedure_code_16 as procedure_code
    from __dbt__cte__normalized_input__stg_medical_claim
    where procedure_code_16 is not null
    union all
    
    select
        claim_id
        , claim_type
        , data_source
        , procedure_code_type
        , 'procedure_code_17' as column_name
        , procedure_code_17 as procedure_code
    from __dbt__cte__normalized_input__stg_medical_claim
    where procedure_code_17 is not null
    union all
    
    select
        claim_id
        , claim_type
        , data_source
        , procedure_code_type
        , 'procedure_code_18' as column_name
        , procedure_code_18 as procedure_code
    from __dbt__cte__normalized_input__stg_medical_claim
    where procedure_code_18 is not null
    union all
    
    select
        claim_id
        , claim_type
        , data_source
        , procedure_code_type
        , 'procedure_code_19' as column_name
        , procedure_code_19 as procedure_code
    from __dbt__cte__normalized_input__stg_medical_claim
    where procedure_code_19 is not null
    union all
    
    select
        claim_id
        , claim_type
        , data_source
        , procedure_code_type
        , 'procedure_code_20' as column_name
        , procedure_code_20 as procedure_code
    from __dbt__cte__normalized_input__stg_medical_claim
    where procedure_code_20 is not null
    union all
    
    select
        claim_id
        , claim_type
        , data_source
        , procedure_code_type
        , 'procedure_code_21' as column_name
        , procedure_code_21 as procedure_code
    from __dbt__cte__normalized_input__stg_medical_claim
    where procedure_code_21 is not null
    union all
    
    select
        claim_id
        , claim_type
        , data_source
        , procedure_code_type
        , 'procedure_code_22' as column_name
        , procedure_code_22 as procedure_code
    from __dbt__cte__normalized_input__stg_medical_claim
    where procedure_code_22 is not null
    union all
    
    select
        claim_id
        , claim_type
        , data_source
        , procedure_code_type
        , 'procedure_code_23' as column_name
        , procedure_code_23 as procedure_code
    from __dbt__cte__normalized_input__stg_medical_claim
    where procedure_code_23 is not null
    union all
    
    select
        claim_id
        , claim_type
        , data_source
        , procedure_code_type
        , 'procedure_code_24' as column_name
        , procedure_code_24 as procedure_code
    from __dbt__cte__normalized_input__stg_medical_claim
    where procedure_code_24 is not null
    union all
    
    select
        claim_id
        , claim_type
        , data_source
        , procedure_code_type
        , 'procedure_code_25' as column_name
        , procedure_code_25 as procedure_code
    from __dbt__cte__normalized_input__stg_medical_claim
    where procedure_code_25 is not null
    
    
)

-- Final output: distinct pivoted procedure records
select distinct
    claim_id
    , data_source
    , procedure_code_type
    , column_name
    , replace(piv.procedure_code, '.', '') as procedure_code
    , cast('2026-01-01 00:00:00' as timestamp) as tuva_last_run
from pivot_procedure as piv
where claim_type = 'institutional'
),  __dbt__cte___int_normalized_input_procedure_code_final as (




select
    claim_id
    , data_source
    
    , max(case when column_name = 'procedure_code_1' then procedure_code else null end) as procedure_code_1
    
    , max(case when column_name = 'procedure_code_2' then procedure_code else null end) as procedure_code_2
    
    , max(case when column_name = 'procedure_code_3' then procedure_code else null end) as procedure_code_3
    
    , max(case when column_name = 'procedure_code_4' then procedure_code else null end) as procedure_code_4
    
    , max(case when column_name = 'procedure_code_5' then procedure_code else null end) as procedure_code_5
    
    , max(case when column_name = 'procedure_code_6' then procedure_code else null end) as procedure_code_6
    
    , max(case when column_name = 'procedure_code_7' then procedure_code else null end) as procedure_code_7
    
    , max(case when column_name = 'procedure_code_8' then procedure_code else null end) as procedure_code_8
    
    , max(case when column_name = 'procedure_code_9' then procedure_code else null end) as procedure_code_9
    
    , max(case when column_name = 'procedure_code_10' then procedure_code else null end) as procedure_code_10
    
    , max(case when column_name = 'procedure_code_11' then procedure_code else null end) as procedure_code_11
    
    , max(case when column_name = 'procedure_code_12' then procedure_code else null end) as procedure_code_12
    
    , max(case when column_name = 'procedure_code_13' then procedure_code else null end) as procedure_code_13
    
    , max(case when column_name = 'procedure_code_14' then procedure_code else null end) as procedure_code_14
    
    , max(case when column_name = 'procedure_code_15' then procedure_code else null end) as procedure_code_15
    
    , max(case when column_name = 'procedure_code_16' then procedure_code else null end) as procedure_code_16
    
    , max(case when column_name = 'procedure_code_17' then procedure_code else null end) as procedure_code_17
    
    , max(case when column_name = 'procedure_code_18' then procedure_code else null end) as procedure_code_18
    
    , max(case when column_name = 'procedure_code_19' then procedure_code else null end) as procedure_code_19
    
    , max(case when column_name = 'procedure_code_20' then procedure_code else null end) as procedure_code_20
    
    , max(case when column_name = 'procedure_code_21' then procedure_code else null end) as procedure_code_21
    
    , max(case when column_name = 'procedure_code_22' then procedure_code else null end) as procedure_code_22
    
    , max(case when column_name = 'procedure_code_23' then procedure_code else null end) as procedure_code_23
    
    , max(case when column_name = 'procedure_code_24' then procedure_code else null end) as procedure_code_24
    
    , max(case when column_name = 'procedure_code_25' then procedure_code else null end) as procedure_code_25
    
    , cast('2026-01-01 00:00:00' as timestamp) as tuva_last_run
from __dbt__cte___int_normalized_input_procedure_code_intermediate
group by
    claim_id
    , data_source
),  __dbt__cte___int_normalized_input_procedure_date_normalize as (



with pivot_procedure as (
    select
        claim_id
        , claim_type
        , data_source
        , 'procedure_date_1' as procedure_column
        , procedure_date_1 as procedure_date
    from __dbt__cte__normalized_input__stg_medical_claim


    union all

    select
        claim_id
        , claim_type
        , data_source
        , 'procedure_date_2' as procedure_column
        , procedure_date_2 as procedure_date
    from __dbt__cte__normalized_input__stg_medical_claim

    union all

    select
        claim_id
        , claim_type
        , data_source
        , 'procedure_date_3' as procedure_column
        , procedure_date_3 as procedure_date
    from __dbt__cte__normalized_input__stg_medical_claim


    union all

    select
        claim_id
        , claim_type
        , data_source
        , 'procedure_date_4' as procedure_column
        , procedure_date_4 as procedure_date
    from __dbt__cte__normalized_input__stg_medical_claim


    union all

    select
        claim_id
        , claim_type
        , data_source
        , 'procedure_date_5' as procedure_column
        , procedure_date_5 as procedure_date
    from __dbt__cte__normalized_input__stg_medical_claim


    union all

    select
        claim_id
        , claim_type
        , data_source
        , 'procedure_date_6' as procedure_column
        , procedure_date_6 as procedure_date
    from __dbt__cte__normalized_input__stg_medical_claim


    union all

    select
        claim_id
        , claim_type
        , data_source
        , 'procedure_date_7' as procedure_column
        , procedure_date_7 as procedure_date
    from __dbt__cte__normalized_input__stg_medical_claim


    union all

    select
        claim_id
        , claim_type
        , data_source
        , 'procedure_date_8' as procedure_column
        , procedure_date_8 as procedure_date
    from __dbt__cte__normalized_input__stg_medical_claim


    union all

    select
        claim_id
        , claim_type
        , data_source
        , 'procedure_date_9' as procedure_column
        , procedure_date_9 as procedure_date
    from __dbt__cte__normalized_input__stg_medical_claim


    union all

    select
        claim_id
        , claim_type
        , data_source
        , 'procedure_date_10' as procedure_column
        , procedure_date_10 as procedure_date
    from __dbt__cte__normalized_input__stg_medical_claim


    union all

    select
        claim_id
        , claim_type
        , data_source
        , 'procedure_date_11' as procedure_column
        , procedure_date_11 as procedure_date
    from __dbt__cte__normalized_input__stg_medical_claim


    union all

    select
        claim_id
        , claim_type
        , data_source
        , 'procedure_date_12' as procedure_column
        , procedure_date_12 as procedure_date
    from __dbt__cte__normalized_input__stg_medical_claim


    union all

    select
        claim_id
        , claim_type
        , data_source
        , 'procedure_date_13' as procedure_column
        , procedure_date_13 as procedure_date
    from __dbt__cte__normalized_input__stg_medical_claim

    union all

    select
        claim_id
        , claim_type
        , data_source
        , 'procedure_date_14' as procedure_column
        , procedure_date_14 as procedure_date
    from __dbt__cte__normalized_input__stg_medical_claim


    union all

    select
        claim_id
        , claim_type
        , data_source
        , 'procedure_date_15' as procedure_column
        , procedure_date_15 as procedure_date
    from __dbt__cte__normalized_input__stg_medical_claim


    union all

    select
        claim_id
        , claim_type
        , data_source
        , 'procedure_date_16' as procedure_column
        , procedure_date_16 as procedure_date
    from __dbt__cte__normalized_input__stg_medical_claim


    union all

    select
        claim_id
        , claim_type
        , data_source
        , 'procedure_date_17' as procedure_column
        , procedure_date_17 as procedure_date
    from __dbt__cte__normalized_input__stg_medical_claim


    union all

    select
        claim_id
        , claim_type
        , data_source
        , 'procedure_date_18' as procedure_column
        , procedure_date_18 as procedure_date
    from __dbt__cte__normalized_input__stg_medical_claim


    union all

    select
        claim_id
        , claim_type
        , data_source
        , 'procedure_date_19' as procedure_column
        , procedure_date_19 as procedure_date
    from __dbt__cte__normalized_input__stg_medical_claim


    union all

    select
        claim_id
        , claim_type
        , data_source
        , 'procedure_date_20' as procedure_column
        , procedure_date_20 as procedure_date
    from __dbt__cte__normalized_input__stg_medical_claim


    union all

    select
        claim_id
        , claim_type
        , data_source
        , 'procedure_date_21' as procedure_column
        , procedure_date_21 as procedure_date
    from __dbt__cte__normalized_input__stg_medical_claim

    union all

    select
        claim_id
        , claim_type
        , data_source
        , 'procedure_date_22' as procedure_column
        , procedure_date_22 as procedure_date
    from __dbt__cte__normalized_input__stg_medical_claim


    union all

    select
        claim_id
        , claim_type
        , data_source
        , 'procedure_date_23' as procedure_column
        , procedure_date_23 as procedure_date
    from __dbt__cte__normalized_input__stg_medical_claim

    union all

    select
        claim_id
        , claim_type
        , data_source
        , 'procedure_date_24' as procedure_column
        , procedure_date_24 as procedure_date
    from __dbt__cte__normalized_input__stg_medical_claim

    union all

    select
        claim_id
        , claim_type
        , data_source
        , 'procedure_date_25' as procedure_column
        , procedure_date_25 as procedure_date
    from __dbt__cte__normalized_input__stg_medical_claim
)

select
    claim_id
    , data_source
    , procedure_column
    , procedure_date
    , count(*) as procedure_date_occurrence_count
    , cast('2026-01-01 00:00:00' as timestamp) as tuva_last_run
from pivot_procedure as piv
where claim_type = 'institutional'
group by
    claim_id
    , data_source
    , procedure_column
    , procedure_date
),  __dbt__cte___int_normalized_input_procedure_date_voting as (


with distinct_count as (
    select
        claim_id
        , data_source
        , procedure_column
        , count(*) as distinct_count
        , cast('2026-01-01 00:00:00' as timestamp) as tuva_last_run
    from __dbt__cte___int_normalized_input_procedure_date_normalize
    group by
        claim_id
        , data_source
        , procedure_column
)

select
    norm.claim_id
    , norm.data_source
    , norm.procedure_column as column_name
    , norm.procedure_date as normalized_code
    , norm.procedure_date_occurrence_count as occurrence_count
    , coalesce(lead(procedure_date_occurrence_count)
        over (partition by norm.claim_id, norm.data_source, norm.procedure_column
order by procedure_date_occurrence_count desc), 0) as next_occurrence_count
    , row_number() over (partition by norm.claim_id, norm.data_source, norm.procedure_column
order by procedure_date_occurrence_count desc) as occurrence_row_count
    , cast('2026-01-01 00:00:00' as timestamp) as tuva_last_run
from __dbt__cte___int_normalized_input_procedure_date_normalize as norm
inner join distinct_count as dist
    on norm.claim_id = dist.claim_id
    and norm.data_source = dist.data_source
    and norm.procedure_column = dist.procedure_column
),  __dbt__cte___int_normalized_input_procedure_date_final as (


select
    claim_id
    , data_source
    , max(case when lower(column_name) = 'procedure_date_1' then normalized_code else null end) as procedure_date_1
    , max(case when lower(column_name) = 'procedure_date_2' then normalized_code else null end) as procedure_date_2
    , max(case when lower(column_name) = 'procedure_date_3' then normalized_code else null end) as procedure_date_3
    , max(case when lower(column_name) = 'procedure_date_4' then normalized_code else null end) as procedure_date_4
    , max(case when lower(column_name) = 'procedure_date_5' then normalized_code else null end) as procedure_date_5
    , max(case when lower(column_name) = 'procedure_date_6' then normalized_code else null end) as procedure_date_6
    , max(case when lower(column_name) = 'procedure_date_7' then normalized_code else null end) as procedure_date_7
    , max(case when lower(column_name) = 'procedure_date_8' then normalized_code else null end) as procedure_date_8
    , max(case when lower(column_name) = 'procedure_date_9' then normalized_code else null end) as procedure_date_9
    , max(case when lower(column_name) = 'procedure_date_10' then normalized_code else null end) as procedure_date_10
    , max(case when lower(column_name) = 'procedure_date_11' then normalized_code else null end) as procedure_date_11
    , max(case when lower(column_name) = 'procedure_date_12' then normalized_code else null end) as procedure_date_12
    , max(case when lower(column_name) = 'procedure_date_13' then normalized_code else null end) as procedure_date_13
    , max(case when lower(column_name) = 'procedure_date_14' then normalized_code else null end) as procedure_date_14
    , max(case when lower(column_name) = 'procedure_date_15' then normalized_code else null end) as procedure_date_15
    , max(case when lower(column_name) = 'procedure_date_16' then normalized_code else null end) as procedure_date_16
    , max(case when lower(column_name) = 'procedure_date_17' then normalized_code else null end) as procedure_date_17
    , max(case when lower(column_name) = 'procedure_date_18' then normalized_code else null end) as procedure_date_18
    , max(case when lower(column_name) = 'procedure_date_19' then normalized_code else null end) as procedure_date_19
    , max(case when lower(column_name) = 'procedure_date_20' then normalized_code else null end) as procedure_date_20
    , max(case when lower(column_name) = 'procedure_date_21' then normalized_code else null end) as procedure_date_21
    , max(case when lower(column_name) = 'procedure_date_22' then normalized_code else null end) as procedure_date_22
    , max(case when lower(column_name) = 'procedure_date_23' then normalized_code else null end) as procedure_date_23
    , max(case when lower(column_name) = 'procedure_date_24' then normalized_code else null end) as procedure_date_24
    , max(case when lower(column_name) = 'procedure_date_25' then normalized_code else null end) as procedure_date_25
    , cast('2026-01-01 00:00:00' as timestamp) as tuva_last_run
from __dbt__cte___int_normalized_input_procedure_date_voting
where (occurrence_row_count = 1
        and occurrence_count > next_occurrence_count)
group by
    claim_id
    , data_source
),  __dbt__cte___int_normalized_input_revenue_center_normalize as (



select
    claim_id
    , claim_line_number
    , data_source
    , rev.revenue_center_code as normalized_code
    , rev.revenue_center_description as normalized_description
    , cast('2026-01-01 00:00:00' as timestamp) as tuva_last_run
from __dbt__cte__normalized_input__stg_medical_claim as med
left outer join "terminology"."revenue_center" as rev
    
        on lpad(med.revenue_center_code, 4, '0') = rev.revenue_center_code
    
where claim_type = 'institutional'
),  __dbt__cte___int_normalized_input_undetermined_claim_type as (



select
    claim_id
    , claim_line_number
    , claim_type
    , person_id
    , member_id
    , payer
    , plan
    , claim_start_date
    , claim_end_date
    , claim_line_start_date
    , claim_line_end_date
    , admission_date
    , discharge_date
    , ad_src.admit_source_code
    , ad_src.admit_source_description
    , ad_type.admit_type_code
    , ad_type.admit_type_description
    , dis.discharge_disposition_code
    , dis.discharge_disposition_description
    , pos.place_of_service_code
    , pos.place_of_service_description
    , tob.bill_type_code
    , tob.bill_type_description
    , med.drg_code_type
    , coalesce(msdrg.ms_drg_code, aprdrg.apr_drg_code) as drg_code
    , coalesce(msdrg.ms_drg_description, aprdrg.apr_drg_description) as drg_description
    , rev.revenue_center_code
    , rev.revenue_center_description
    , service_unit_quantity
    , hcpcs_code
    , hcpcs_modifier_1
    , hcpcs_modifier_2
    , hcpcs_modifier_3
    , hcpcs_modifier_4
    , hcpcs_modifier_5
    , rendering_npi
    , rendnpi.npi as rendering_name
    , billing_npi
    , billnpi.npi as billing_name
    , facility_npi
    , facnpi.npi as facility_name
    , paid_date
    , paid_amount
    , allowed_amount
    , charge_amount
    , coinsurance_amount
    , copayment_amount
    , deductible_amount
    , total_cost_amount
    , diagnosis_code_type
    , diagnosis_code_1
    , diagnosis_code_2
    , diagnosis_code_3
    , diagnosis_code_4
    , diagnosis_code_5
    , diagnosis_code_6
    , diagnosis_code_7
    , diagnosis_code_8
    , diagnosis_code_9
    , diagnosis_code_10
    , diagnosis_code_11
    , diagnosis_code_12
    , diagnosis_code_13
    , diagnosis_code_14
    , diagnosis_code_15
    , diagnosis_code_16
    , diagnosis_code_17
    , diagnosis_code_18
    , diagnosis_code_19
    , diagnosis_code_20
    , diagnosis_code_21
    , diagnosis_code_22
    , diagnosis_code_23
    , diagnosis_code_24
    , diagnosis_code_25
    , diagnosis_poa_1
    , diagnosis_poa_2
    , diagnosis_poa_3
    , diagnosis_poa_4
    , diagnosis_poa_5
    , diagnosis_poa_6
    , diagnosis_poa_7
    , diagnosis_poa_8
    , diagnosis_poa_9
    , diagnosis_poa_10
    , diagnosis_poa_11
    , diagnosis_poa_12
    , diagnosis_poa_13
    , diagnosis_poa_14
    , diagnosis_poa_15
    , diagnosis_poa_16
    , diagnosis_poa_17
    , diagnosis_poa_18
    , diagnosis_poa_19
    , diagnosis_poa_20
    , diagnosis_poa_21
    , diagnosis_poa_22
    , diagnosis_poa_23
    , diagnosis_poa_24
    , diagnosis_poa_25
    , procedure_code_type
    , procedure_code_1
    , procedure_code_2
    , procedure_code_3
    , procedure_code_4
    , procedure_code_5
    , procedure_code_6
    , procedure_code_7
    , procedure_code_8
    , procedure_code_9
    , procedure_code_10
    , procedure_code_11
    , procedure_code_12
    , procedure_code_13
    , procedure_code_14
    , procedure_code_15
    , procedure_code_16
    , procedure_code_17
    , procedure_code_18
    , procedure_code_19
    , procedure_code_20
    , procedure_code_21
    , procedure_code_22
    , procedure_code_23
    , procedure_code_24
    , procedure_code_25
    , procedure_date_1
    , procedure_date_2
    , procedure_date_3
    , procedure_date_4
    , procedure_date_5
    , procedure_date_6
    , procedure_date_7
    , procedure_date_8
    , procedure_date_9
    , procedure_date_10
    , procedure_date_11
    , procedure_date_12
    , procedure_date_13
    , procedure_date_14
    , procedure_date_15
    , procedure_date_16
    , procedure_date_17
    , procedure_date_18
    , procedure_date_19
    , procedure_date_20
    , procedure_date_21
    , procedure_date_22
    , procedure_date_23
    , procedure_date_24
    , procedure_date_25
    , data_source
    , cast('2026-01-01 00:00:00' as timestamp) as tuva_last_run
from __dbt__cte__normalized_input__stg_medical_claim as med
left outer join "terminology"."admit_source" as ad_src
    on med.admit_source_code = ad_src.admit_source_code
left outer join "terminology"."admit_type" as ad_type
    on med.admit_type_code = ad_type.admit_type_code
left outer join "terminology"."discharge_disposition" as dis
    on med.discharge_disposition_code = dis.discharge_disposition_code
left outer join "terminology"."place_of_service" as pos
    on med.place_of_service_code = pos.place_of_service_code
left outer join "terminology"."bill_type" as tob
    on med.bill_type_code = tob.bill_type_code
left outer join "terminology"."ms_drg" as msdrg
    on med.drg_code_type = 'ms-drg'
    and med.drg_code = msdrg.ms_drg_code
left outer join "terminology"."apr_drg" as aprdrg
    on med.drg_code_type = 'apr-drg'
    and med.drg_code = aprdrg.apr_drg_code
left outer join "terminology"."revenue_center" as rev
    on med.revenue_center_code = rev.revenue_center_code
left outer join "provider_data"."provider" as rendnpi
    on med.rendering_npi = rendnpi.npi
left outer join "provider_data"."provider" as billnpi
    on med.billing_npi = billnpi.npi
left outer join "provider_data"."provider" as facnpi
    on med.facility_npi = facnpi.npi
where claim_type in ('undetermined')
),  __dbt__cte__normalized__medical_claim as (




select
    (med.claim_id || '-' || med.claim_line_number || '-' || med.data_source) as medical_claim_id
    , cast(med.claim_id as TEXT) as claim_id
    , cast(med.claim_line_number as int) as claim_line_number
    , cast(med.claim_type as TEXT) as claim_type
    , cast(med.person_id as TEXT) as person_id
    , cast(med.member_id as TEXT) as member_id
    , cast(med.payer as TEXT) as payer
    , cast(med.plan as TEXT) as plan
    , cast(coalesce(dates.minimum_claim_start_date, undetermined.claim_start_date) as date) as claim_start_date
    , cast(coalesce(dates.maximum_claim_end_date, undetermined.claim_start_date) as date) as claim_end_date
    , cast(coalesce(claim_line_dates.normalized_claim_line_start_date, undetermined.claim_line_start_date) as date) as claim_line_start_date
    , cast(coalesce(claim_line_dates.normalized_claim_line_end_date, undetermined.claim_line_end_date) as date) as claim_line_end_date
    , cast(coalesce(dates.minimum_admission_date, undetermined.admission_date) as date) as admission_date
    , cast(coalesce(dates.maximum_discharge_date, undetermined.discharge_date) as date) as discharge_date
    , cast(coalesce(ad_source.normalized_code, undetermined.admit_source_code) as TEXT) as admit_source_code
    , cast(coalesce(ad_source.normalized_description, undetermined.admit_source_description) as TEXT) as admit_source_description
    , cast(coalesce(ad_type.normalized_code, undetermined.admit_type_code) as TEXT) as admit_type_code
    , cast(coalesce(ad_type.normalized_description, undetermined.admit_type_description) as TEXT) as admit_type_description
    , cast(coalesce(disch_disp.normalized_code, undetermined.discharge_disposition_code) as TEXT) as discharge_disposition_code
    , cast(coalesce(disch_disp.normalized_description, undetermined.discharge_disposition_description) as TEXT) as discharge_disposition_description
    , cast(coalesce(pos.normalized_code, undetermined.place_of_service_code) as TEXT) as place_of_service_code
    , cast(coalesce(pos.normalized_description, undetermined.place_of_service_description) as TEXT) as place_of_service_description
    , cast(coalesce(bill.normalized_code, undetermined.bill_type_code) as TEXT) as bill_type_code
    , cast(coalesce(bill.normalized_description, undetermined.bill_type_description) as TEXT) as bill_type_description
    , cast(med.drg_code_type as TEXT) as drg_code_type
    , cast(coalesce(drg.normalized_code, undetermined.drg_code) as TEXT) as drg_code
    , cast(coalesce(drg.normalized_description, undetermined.drg_description) as TEXT) as drg_description
    , cast(coalesce(rev.normalized_code, undetermined.revenue_center_code) as TEXT) as revenue_center_code
    , cast(coalesce(rev.normalized_description, undetermined.revenue_center_description) as TEXT) as revenue_center_description
    , cast(med.service_unit_quantity as numeric(28,6)) as service_unit_quantity
    , cast(med.hcpcs_code as TEXT) as hcpcs_code
    , cast(med.hcpcs_modifier_1 as TEXT) as hcpcs_modifier_1
    , cast(med.hcpcs_modifier_2 as TEXT) as hcpcs_modifier_2
    , cast(med.hcpcs_modifier_3 as TEXT) as hcpcs_modifier_3
    , cast(med.hcpcs_modifier_4 as TEXT) as hcpcs_modifier_4
    , cast(med.hcpcs_modifier_5 as TEXT) as hcpcs_modifier_5
    , cast(coalesce(med_npi.normalized_rendering_npi, undetermined.rendering_npi) as TEXT) as rendering_npi
    , cast(med.rendering_tin as TEXT) as rendering_tin
    , cast(coalesce(med_npi.normalized_rendering_name, undetermined.rendering_name) as TEXT) as rendering_name
    , cast(coalesce(med_npi.normalized_billing_npi, undetermined.billing_npi) as TEXT) as billing_npi
    , cast(med.billing_tin as TEXT) as billing_tin
    , cast(coalesce(med_npi.normalized_billing_name, undetermined.billing_name) as TEXT) as billing_name
    , cast(coalesce(med_npi.normalized_facility_npi, undetermined.facility_npi) as TEXT) as facility_npi
    , cast(coalesce(med_npi.normalized_facility_name, undetermined.facility_name) as TEXT) as facility_name
    , cast(med.paid_date as date) as paid_date
    , cast(med.paid_amount as numeric(28,6)) as paid_amount
    , cast(med.allowed_amount as numeric(28,6)) as allowed_amount
    , cast(med.charge_amount as numeric(28,6)) as charge_amount
    , cast(med.coinsurance_amount as numeric(28,6)) as coinsurance_amount
    , cast(med.copayment_amount as numeric(28,6)) as copayment_amount
    , cast(med.deductible_amount as numeric(28,6)) as deductible_amount
    , cast(med.total_cost_amount as numeric(28,6)) as total_cost_amount
    , cast(med.diagnosis_code_type as TEXT) as diagnosis_code_type
    
    , cast(replace(med.diagnosis_code_1, '.', '') as TEXT) as diagnosis_code_1
    
    , cast(replace(med.diagnosis_code_2, '.', '') as TEXT) as diagnosis_code_2
    
    , cast(replace(med.diagnosis_code_3, '.', '') as TEXT) as diagnosis_code_3
    
    , cast(replace(med.diagnosis_code_4, '.', '') as TEXT) as diagnosis_code_4
    
    , cast(replace(med.diagnosis_code_5, '.', '') as TEXT) as diagnosis_code_5
    
    , cast(replace(med.diagnosis_code_6, '.', '') as TEXT) as diagnosis_code_6
    
    , cast(replace(med.diagnosis_code_7, '.', '') as TEXT) as diagnosis_code_7
    
    , cast(replace(med.diagnosis_code_8, '.', '') as TEXT) as diagnosis_code_8
    
    , cast(replace(med.diagnosis_code_9, '.', '') as TEXT) as diagnosis_code_9
    
    , cast(replace(med.diagnosis_code_10, '.', '') as TEXT) as diagnosis_code_10
    
    , cast(replace(med.diagnosis_code_11, '.', '') as TEXT) as diagnosis_code_11
    
    , cast(replace(med.diagnosis_code_12, '.', '') as TEXT) as diagnosis_code_12
    
    , cast(replace(med.diagnosis_code_13, '.', '') as TEXT) as diagnosis_code_13
    
    , cast(replace(med.diagnosis_code_14, '.', '') as TEXT) as diagnosis_code_14
    
    , cast(replace(med.diagnosis_code_15, '.', '') as TEXT) as diagnosis_code_15
    
    , cast(replace(med.diagnosis_code_16, '.', '') as TEXT) as diagnosis_code_16
    
    , cast(replace(med.diagnosis_code_17, '.', '') as TEXT) as diagnosis_code_17
    
    , cast(replace(med.diagnosis_code_18, '.', '') as TEXT) as diagnosis_code_18
    
    , cast(replace(med.diagnosis_code_19, '.', '') as TEXT) as diagnosis_code_19
    
    , cast(replace(med.diagnosis_code_20, '.', '') as TEXT) as diagnosis_code_20
    
    , cast(replace(med.diagnosis_code_21, '.', '') as TEXT) as diagnosis_code_21
    
    , cast(replace(med.diagnosis_code_22, '.', '') as TEXT) as diagnosis_code_22
    
    , cast(replace(med.diagnosis_code_23, '.', '') as TEXT) as diagnosis_code_23
    
    , cast(replace(med.diagnosis_code_24, '.', '') as TEXT) as diagnosis_code_24
    
    , cast(replace(med.diagnosis_code_25, '.', '') as TEXT) as diagnosis_code_25
    
    
    , cast(med.diagnosis_poa_1 as TEXT) as diagnosis_poa_1
    
    , cast(med.diagnosis_poa_2 as TEXT) as diagnosis_poa_2
    
    , cast(med.diagnosis_poa_3 as TEXT) as diagnosis_poa_3
    
    , cast(med.diagnosis_poa_4 as TEXT) as diagnosis_poa_4
    
    , cast(med.diagnosis_poa_5 as TEXT) as diagnosis_poa_5
    
    , cast(med.diagnosis_poa_6 as TEXT) as diagnosis_poa_6
    
    , cast(med.diagnosis_poa_7 as TEXT) as diagnosis_poa_7
    
    , cast(med.diagnosis_poa_8 as TEXT) as diagnosis_poa_8
    
    , cast(med.diagnosis_poa_9 as TEXT) as diagnosis_poa_9
    
    , cast(med.diagnosis_poa_10 as TEXT) as diagnosis_poa_10
    
    , cast(med.diagnosis_poa_11 as TEXT) as diagnosis_poa_11
    
    , cast(med.diagnosis_poa_12 as TEXT) as diagnosis_poa_12
    
    , cast(med.diagnosis_poa_13 as TEXT) as diagnosis_poa_13
    
    , cast(med.diagnosis_poa_14 as TEXT) as diagnosis_poa_14
    
    , cast(med.diagnosis_poa_15 as TEXT) as diagnosis_poa_15
    
    , cast(med.diagnosis_poa_16 as TEXT) as diagnosis_poa_16
    
    , cast(med.diagnosis_poa_17 as TEXT) as diagnosis_poa_17
    
    , cast(med.diagnosis_poa_18 as TEXT) as diagnosis_poa_18
    
    , cast(med.diagnosis_poa_19 as TEXT) as diagnosis_poa_19
    
    , cast(med.diagnosis_poa_20 as TEXT) as diagnosis_poa_20
    
    , cast(med.diagnosis_poa_21 as TEXT) as diagnosis_poa_21
    
    , cast(med.diagnosis_poa_22 as TEXT) as diagnosis_poa_22
    
    , cast(med.diagnosis_poa_23 as TEXT) as diagnosis_poa_23
    
    , cast(med.diagnosis_poa_24 as TEXT) as diagnosis_poa_24
    
    , cast(med.diagnosis_poa_25 as TEXT) as diagnosis_poa_25
    
    , cast(med.procedure_code_type as TEXT) as procedure_code_type
    
    , cast(coalesce(px_code.procedure_code_1, undetermined.procedure_code_1) as TEXT) as procedure_code_1
    
    , cast(coalesce(px_code.procedure_code_2, undetermined.procedure_code_2) as TEXT) as procedure_code_2
    
    , cast(coalesce(px_code.procedure_code_3, undetermined.procedure_code_3) as TEXT) as procedure_code_3
    
    , cast(coalesce(px_code.procedure_code_4, undetermined.procedure_code_4) as TEXT) as procedure_code_4
    
    , cast(coalesce(px_code.procedure_code_5, undetermined.procedure_code_5) as TEXT) as procedure_code_5
    
    , cast(coalesce(px_code.procedure_code_6, undetermined.procedure_code_6) as TEXT) as procedure_code_6
    
    , cast(coalesce(px_code.procedure_code_7, undetermined.procedure_code_7) as TEXT) as procedure_code_7
    
    , cast(coalesce(px_code.procedure_code_8, undetermined.procedure_code_8) as TEXT) as procedure_code_8
    
    , cast(coalesce(px_code.procedure_code_9, undetermined.procedure_code_9) as TEXT) as procedure_code_9
    
    , cast(coalesce(px_code.procedure_code_10, undetermined.procedure_code_10) as TEXT) as procedure_code_10
    
    , cast(coalesce(px_code.procedure_code_11, undetermined.procedure_code_11) as TEXT) as procedure_code_11
    
    , cast(coalesce(px_code.procedure_code_12, undetermined.procedure_code_12) as TEXT) as procedure_code_12
    
    , cast(coalesce(px_code.procedure_code_13, undetermined.procedure_code_13) as TEXT) as procedure_code_13
    
    , cast(coalesce(px_code.procedure_code_14, undetermined.procedure_code_14) as TEXT) as procedure_code_14
    
    , cast(coalesce(px_code.procedure_code_15, undetermined.procedure_code_15) as TEXT) as procedure_code_15
    
    , cast(coalesce(px_code.procedure_code_16, undetermined.procedure_code_16) as TEXT) as procedure_code_16
    
    , cast(coalesce(px_code.procedure_code_17, undetermined.procedure_code_17) as TEXT) as procedure_code_17
    
    , cast(coalesce(px_code.procedure_code_18, undetermined.procedure_code_18) as TEXT) as procedure_code_18
    
    , cast(coalesce(px_code.procedure_code_19, undetermined.procedure_code_19) as TEXT) as procedure_code_19
    
    , cast(coalesce(px_code.procedure_code_20, undetermined.procedure_code_20) as TEXT) as procedure_code_20
    
    , cast(coalesce(px_code.procedure_code_21, undetermined.procedure_code_21) as TEXT) as procedure_code_21
    
    , cast(coalesce(px_code.procedure_code_22, undetermined.procedure_code_22) as TEXT) as procedure_code_22
    
    , cast(coalesce(px_code.procedure_code_23, undetermined.procedure_code_23) as TEXT) as procedure_code_23
    
    , cast(coalesce(px_code.procedure_code_24, undetermined.procedure_code_24) as TEXT) as procedure_code_24
    
    , cast(coalesce(px_code.procedure_code_25, undetermined.procedure_code_25) as TEXT) as procedure_code_25
    
    
    , cast(coalesce(px_date.procedure_date_1, undetermined.procedure_date_1) as date) as procedure_date_1
    
    , cast(coalesce(px_date.procedure_date_2, undetermined.procedure_date_2) as date) as procedure_date_2
    
    , cast(coalesce(px_date.procedure_date_3, undetermined.procedure_date_3) as date) as procedure_date_3
    
    , cast(coalesce(px_date.procedure_date_4, undetermined.procedure_date_4) as date) as procedure_date_4
    
    , cast(coalesce(px_date.procedure_date_5, undetermined.procedure_date_5) as date) as procedure_date_5
    
    , cast(coalesce(px_date.procedure_date_6, undetermined.procedure_date_6) as date) as procedure_date_6
    
    , cast(coalesce(px_date.procedure_date_7, undetermined.procedure_date_7) as date) as procedure_date_7
    
    , cast(coalesce(px_date.procedure_date_8, undetermined.procedure_date_8) as date) as procedure_date_8
    
    , cast(coalesce(px_date.procedure_date_9, undetermined.procedure_date_9) as date) as procedure_date_9
    
    , cast(coalesce(px_date.procedure_date_10, undetermined.procedure_date_10) as date) as procedure_date_10
    
    , cast(coalesce(px_date.procedure_date_11, undetermined.procedure_date_11) as date) as procedure_date_11
    
    , cast(coalesce(px_date.procedure_date_12, undetermined.procedure_date_12) as date) as procedure_date_12
    
    , cast(coalesce(px_date.procedure_date_13, undetermined.procedure_date_13) as date) as procedure_date_13
    
    , cast(coalesce(px_date.procedure_date_14, undetermined.procedure_date_14) as date) as procedure_date_14
    
    , cast(coalesce(px_date.procedure_date_15, undetermined.procedure_date_15) as date) as procedure_date_15
    
    , cast(coalesce(px_date.procedure_date_16, undetermined.procedure_date_16) as date) as procedure_date_16
    
    , cast(coalesce(px_date.procedure_date_17, undetermined.procedure_date_17) as date) as procedure_date_17
    
    , cast(coalesce(px_date.procedure_date_18, undetermined.procedure_date_18) as date) as procedure_date_18
    
    , cast(coalesce(px_date.procedure_date_19, undetermined.procedure_date_19) as date) as procedure_date_19
    
    , cast(coalesce(px_date.procedure_date_20, undetermined.procedure_date_20) as date) as procedure_date_20
    
    , cast(coalesce(px_date.procedure_date_21, undetermined.procedure_date_21) as date) as procedure_date_21
    
    , cast(coalesce(px_date.procedure_date_22, undetermined.procedure_date_22) as date) as procedure_date_22
    
    , cast(coalesce(px_date.procedure_date_23, undetermined.procedure_date_23) as date) as procedure_date_23
    
    , cast(coalesce(px_date.procedure_date_24, undetermined.procedure_date_24) as date) as procedure_date_24
    
    , cast(coalesce(px_date.procedure_date_25, undetermined.procedure_date_25) as date) as procedure_date_25
    
    , cast(med.data_source as TEXT) as data_source
    , cast(med.in_network_flag as integer) as in_network_flag
    , cast(med.file_date as date) as file_date
    , cast(med.ingest_datetime as timestamp) as ingest_datetime
    , cast(med.file_name as TEXT) as file_name
    , cast('2026-01-01 00:00:00' as timestamp) as tuva_last_run
    
    
from __dbt__cte__normalized_input__stg_medical_claim as med
left outer join __dbt__cte___int_normalized_input_admit_source_final as ad_source
    on med.claim_id = ad_source.claim_id
    and med.data_source = ad_source.data_source
left outer join __dbt__cte___int_normalized_input_admit_type_final as ad_type
    on med.claim_id = ad_type.claim_id
    and med.data_source = ad_type.data_source
left outer join __dbt__cte___int_normalized_input_bill_type_final as bill
    on med.claim_id = bill.claim_id
    and med.data_source = bill.data_source
left outer join __dbt__cte___int_normalized_input_medical_claim_date_normalize as claim_line_dates
    on med.claim_id = claim_line_dates.claim_id
    and med.claim_line_number = claim_line_dates.claim_line_number
    and med.data_source = claim_line_dates.data_source
left outer join __dbt__cte___int_normalized_input_medical_date_aggregation as dates
    on med.claim_id = dates.claim_id
    and med.data_source = dates.data_source
left outer join __dbt__cte___int_normalized_input_medical_npi_normalize as med_npi
    on med.claim_id = med_npi.claim_id
    and med.claim_line_number = med_npi.claim_line_number
    and med.data_source = med_npi.data_source
left outer join __dbt__cte___int_normalized_input_discharge_disposition_final as disch_disp
    on med.claim_id = disch_disp.claim_id
    and med.data_source = disch_disp.data_source
left outer join __dbt__cte___int_normalized_input_drg_final as drg
    on med.claim_id = drg.claim_id
    and med.data_source = drg.data_source
left outer join __dbt__cte___int_normalized_input_place_of_service_normalize as pos
    on med.claim_id = pos.claim_id
    and med.claim_line_number = pos.claim_line_number
    and med.data_source = pos.data_source
left outer join __dbt__cte___int_normalized_input_procedure_code_final as px_code
    on med.claim_id = px_code.claim_id
    and med.data_source = px_code.data_source
left outer join __dbt__cte___int_normalized_input_procedure_date_final as px_date
    on med.claim_id = px_date.claim_id
    and med.data_source = px_date.data_source
left outer join __dbt__cte___int_normalized_input_revenue_center_normalize as rev
    on med.claim_id = rev.claim_id
    and med.claim_line_number = rev.claim_line_number
    and med.data_source = rev.data_source
left outer join __dbt__cte___int_normalized_input_undetermined_claim_type as undetermined
    on med.claim_id = undetermined.claim_id
    and med.claim_line_number = undetermined.claim_line_number
    and med.data_source = undetermined.data_source
),  __dbt__cte__encounters__patient_data_source_id as (


with multiple_sources as (
select distinct person_id
, data_source
from __dbt__cte__normalized__medical_claim

union distinct

select distinct person_id
, data_source
from __dbt__cte__normalized__eligibility
)

select
person_id
, data_source
, lower(md5(((case when 'patient data source' is null then 'N' else ('V' || replace(replace(cast('patient data source' as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when person_id is null then 'N' else ('V' || replace(replace(cast(person_id as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when data_source is null then 'N' else ('V' || replace(replace(cast(data_source as TEXT), '%', '%25'), '|', '%7C')) end)))) as patient_data_source_id
from multiple_sources
),  __dbt__cte__encounters__stg_medical_claim as (


with ccs_release_year as (
  select
    max(release_year) as max_release_year
  from "value_sets"."ccs_services_procedures"
)

, final as (
  select
      m.person_id
    , d.patient_data_source_id
    , m.claim_id
    , m.claim_line_number
    , (m.claim_id || '|' || cast(m.claim_line_number as TEXT)) as claim_line_id
    , m.claim_type
    , coalesce(m.admission_date, m.claim_line_start_date, m.claim_start_date) as start_date
    , coalesce(m.discharge_date, m.claim_line_end_date, m.claim_end_date) as end_date
    , m.admission_date
    , m.discharge_date
    , m.claim_start_date
    , m.claim_end_date
    , m.claim_line_start_date
    , m.claim_line_end_date
    , g.service_category_1
    , g.service_category_2
    , g.service_category_3
    , m.bill_type_code
    , bt.bill_type_description
    , m.hcpcs_code
    , m.hcpcs_modifier_1
    , m.hcpcs_modifier_2
    , m.hcpcs_modifier_3
    , m.hcpcs_modifier_4
    , m.hcpcs_modifier_5
    , m.drg_code_type
    , m.drg_code
    , coalesce(msdrg.ms_drg_description, aprdrg.apr_drg_description) as drg_description
    , m.admit_source_code
    , m.admit_type_code
    , m.place_of_service_code
    , pos.place_of_service_description
    , m.revenue_center_code
    , r.revenue_center_description
    , m.diagnosis_code_type
    , coalesce(claim_dx.diagnosis_code_1, m.diagnosis_code_1) as diagnosis_code_1
    , p.primary_taxonomy_code
    , p.primary_specialty_description
    , n.modality
    , m.billing_npi
    , m.rendering_npi
    , rend.primary_specialty_description as rend_primary_specialty_description
    , m.facility_npi
    , m.discharge_disposition_code
    , m.paid_amount
    , m.charge_amount
    , m.allowed_amount
    , m.data_source
  from __dbt__cte__normalized__medical_claim as m
  left outer join "intermediate"."int_claim_level_diagnosis" as claim_dx
    on m.claim_id = claim_dx.claim_id
    and m.data_source = claim_dx.data_source
  inner join "claims_preprocessing"."service_category_grouper" as g on m.claim_id = g.claim_id
    and m.claim_line_number = g.claim_line_number
    and m.data_source = g.data_source
    and g.duplicate_row_number = 1
  inner join __dbt__cte__encounters__patient_data_source_id as d on m.person_id = d.person_id
    and m.data_source = d.data_source
  left outer join "provider_data"."provider" as p on m.facility_npi = p.npi
  left outer join "terminology"."nitos" as n on m.hcpcs_code = n.hcpcs_code
  left outer join "terminology"."ms_drg" as msdrg on m.drg_code_type = 'ms-drg' and m.drg_code = msdrg.ms_drg_code
  left outer join "terminology"."apr_drg" as aprdrg on m.drg_code_type = 'apr-drg' and m.drg_code = aprdrg.apr_drg_code
  left outer join "terminology"."revenue_center" as r on m.revenue_center_code = r.revenue_center_code
  left outer join "terminology"."place_of_service" as pos on m.place_of_service_code = pos.place_of_service_code
  left outer join "terminology"."bill_type" as bt on m.bill_type_code = bt.bill_type_code
  left outer join "provider_data"."provider" as rend on m.rendering_npi = rend.npi
)

select
    f.person_id
  , f.patient_data_source_id
  , f.claim_id
  , f.claim_line_number
  , f.claim_line_id
  , f.claim_type
  , f.start_date
  , f.end_date
  , f.admission_date
  , f.discharge_date
  , f.claim_start_date
  , f.claim_end_date
  , f.claim_line_start_date
  , f.claim_line_end_date
  , f.service_category_1
  , f.service_category_2
  , f.service_category_3
  , f.bill_type_code
  , f.bill_type_description
  , f.hcpcs_code
  , f.hcpcs_modifier_1
  , f.hcpcs_modifier_2
  , f.hcpcs_modifier_3
  , f.hcpcs_modifier_4
  , f.hcpcs_modifier_5
  , c.ccs_category
  , c.ccs_category_description
  , f.drg_code_type
  , f.drg_code
  , f.drg_description
  , f.admit_source_code
  , f.admit_type_code
  , f.place_of_service_code
  , f.place_of_service_description
  , f.revenue_center_code
  , f.revenue_center_description
  , f.diagnosis_code_type
  , f.diagnosis_code_1
  , f.primary_taxonomy_code
  , f.primary_specialty_description
  , f.modality
  , f.billing_npi
  , f.rendering_npi
  , f.rend_primary_specialty_description
  , f.facility_npi
  , f.discharge_disposition_code
  , f.paid_amount
  , f.charge_amount
  , f.allowed_amount
  , f.data_source
  , cast('2026-01-01 00:00:00' as timestamp) as tuva_last_run
from final as f
cross join ccs_release_year as cry
left outer join "value_sets"."ccs_services_procedures" as c
  on f.hcpcs_code = c.hcpcs_code
  and (
    (
      f.start_date >= c.start_valid_date
      and f.start_date <= c.end_valid_date
    )
    or (
      date_part('year', f.start_date) > cry.max_release_year
      and c.release_year = cry.max_release_year
    )
  )
),  __dbt__cte__acute_inpatient__generate_encounter_id as (


with claim_start_end as (
select claim_id
, patient_data_source_id
, min(start_date) as start_date
, max(end_date) as end_date
  from __dbt__cte__encounters__stg_medical_claim
  group by claim_id
  , patient_data_source_id
)

, base as (
  select distinct
      enc.claim_id
    , enc.patient_data_source_id
    , c.start_date
    , c.end_date
    , enc.facility_npi
    , enc.discharge_disposition_code
  from __dbt__cte__encounters__stg_medical_claim as enc
    inner join claim_start_end as c on enc.claim_id = c.claim_id
  and
  c.patient_data_source_id = enc.patient_data_source_id
  where
    service_category_2 in ('acute inpatient')
    and claim_type = 'institutional'
)

, add_row_num as (
  select
      patient_data_source_id
    , claim_id
    , start_date
    , end_date
    , discharge_disposition_code
    , facility_npi
    , rank() over (partition by patient_data_source_id
order by end_date, start_date, claim_id) as row_num
  from base
)

, check_for_merges_with_larger_row_num as (
  select
      aa.patient_data_source_id
    , aa.claim_id as claim_id_a
    , bb.claim_id as claim_id_b
    , aa.row_num as row_num_a
    , bb.row_num as row_num_b
    , case
      -- Condition 1: Exact End Date Match (Catches duplicates/corrections)
      when aa.end_date = bb.end_date
        and aa.facility_npi = bb.facility_npi then 1

      -- Condition 2: Consecutive Stay with Transfer (Catches month-end billing)
      when 

    (aa.end_date + cast(1 as bigint) * interval 1 day) = bb.start_date
        and aa.facility_npi = bb.facility_npi
        and aa.discharge_disposition_code = '30' then 1

      -- Condition 3: Same-Day Start / Superseded Claim
      when aa.start_date = bb.start_date
        and aa.facility_npi = bb.facility_npi
        and aa.discharge_disposition_code = '30' then 1

      -- Condition 4: General Overlapping Stay
      when aa.end_date <> bb.end_date
        and aa.end_date > bb.start_date
        and aa.facility_npi = bb.facility_npi then 1
      else 0
    end as merge_flag
  from add_row_num as aa
  inner join add_row_num as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
    and aa.row_num < bb.row_num
  where aa.claim_id <> bb.claim_id
)

, merges_with_larger_row_num as (
  select
      patient_data_source_id
    , claim_id_a
    , claim_id_b
    , row_num_a
    , row_num_b
    , merge_flag
  from check_for_merges_with_larger_row_num
  where merge_flag = 1
)

, claim_ids_that_merge_with_larger_row_num as (
  select distinct
      patient_data_source_id
    , claim_id_a as claim_id
  from merges_with_larger_row_num
)

, claim_ids_having_a_smaller_row_num_merging_with_a_larger_row_num as (
  select distinct
      aa.patient_data_source_id
    , aa.claim_id as claim_id
  from add_row_num as aa
  inner join merges_with_larger_row_num as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
    and bb.row_num_a < aa.row_num
    and bb.row_num_b > aa.row_num
)

, close_flags as (
  select
      aa.patient_data_source_id
    , aa.claim_id
    , aa.start_date
    , aa.end_date
    , aa.discharge_disposition_code
    , aa.facility_npi
    , aa.row_num
    , case
        when bb.claim_id is null
          and cc.claim_id is null then 1
        else 0
      end as close_flag
  from add_row_num as aa
  left outer join claim_ids_that_merge_with_larger_row_num as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
    and aa.claim_id = bb.claim_id
  left outer join claim_ids_having_a_smaller_row_num_merging_with_a_larger_row_num as cc
    on aa.patient_data_source_id = cc.patient_data_source_id
    and aa.claim_id = cc.claim_id
)

, join_every_row_to_later_closes as (
  select
      aa.patient_data_source_id
    , aa.claim_id
    , aa.row_num
    , bb.row_num as row_num_b
  from close_flags as aa
  inner join close_flags as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
    and aa.row_num <= bb.row_num
  where bb.close_flag = 1
)

, find_min_closing_row_num_for_every_claim as (
  select
      patient_data_source_id
    , claim_id
    , min(row_num_b) as min_closing_row
  from join_every_row_to_later_closes
  group by
      patient_data_source_id
    , claim_id
)

, add_min_closing_row_to_every_claim as (
  select
      aa.patient_data_source_id
    , aa.claim_id
    , aa.start_date
    , aa.end_date
    , aa.discharge_disposition_code
    , aa.facility_npi
    , aa.row_num
    , aa.close_flag
    , bb.min_closing_row
  from close_flags as aa
  left outer join find_min_closing_row_num_for_every_claim as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
    and aa.claim_id = bb.claim_id
)

, add_encounter_id as (
  select
      aa.patient_data_source_id
    , aa.claim_id
    , aa.start_date
    , aa.end_date
    , aa.discharge_disposition_code
    , aa.facility_npi
    , aa.row_num
    , aa.close_flag
    , aa.min_closing_row
    , bb.claim_id as encounter_id
  from add_min_closing_row_to_every_claim as aa
  left outer join add_min_closing_row_to_every_claim as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
    and aa.min_closing_row = bb.row_num
)

select
    patient_data_source_id
  , claim_id
  , start_date
  , end_date
  , discharge_disposition_code
  , facility_npi
  , row_number() over (partition by patient_data_source_id, encounter_id
order by start_date, end_date, claim_id) as encounter_claim_number
  , row_number() over (partition by patient_data_source_id, encounter_id
order by start_date desc, end_date desc, claim_id desc) as encounter_claim_number_desc
  , close_flag
  , min_closing_row
  , encounter_id as anchor_claim_id
  , lower(md5(((case when 'acute inpatient' is null then 'N' else ('V' || replace(replace(cast('acute inpatient' as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when patient_data_source_id is null then 'N' else ('V' || replace(replace(cast(patient_data_source_id as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when encounter_id is null then 'N' else ('V' || replace(replace(cast(encounter_id as TEXT), '%', '%25'), '|', '%7C')) end)))) as encounter_id
from add_encounter_id
),  __dbt__cte__acute_inpatient__start_end_dates as (


select encounter_id
, min(anchor_claim_id) as anchor_claim_id
, min(start_date) as encounter_start_date
, max(end_date) as encounter_end_date
from __dbt__cte__acute_inpatient__generate_encounter_id
group by encounter_id
),  __dbt__cte__encounters__stg_professional as (


select
  claim_id
, claim_line_number
, claim_line_id
, service_type
, data_source
, cast('2026-01-01 00:00:00' as timestamp) as tuva_last_run
from "claims_preprocessing"."service_category__stg_professional" as a
),  __dbt__cte__encounters__prof_and_lower_priority as (
/*This model unions professional with institutional claims that are "lower priority" (dme/lab/ambulance)
and should be part of a higher priority encounter where one exists. We are unioning professional and these institutional claims
here to access downstream from one place */



with prof_and_low_priority_inst_claims as (
select
  claim_id
, claim_line_number
, data_source
from __dbt__cte__encounters__stg_professional as a

union distinct

select
  scg.claim_id
, scg.claim_line_number
, scg.data_source
from "claims_preprocessing"."service_category_grouper" as scg
where duplicate_row_number = 1
and service_category_2 in ('lab', 'durable medical equipment', 'ambulance')
)

select distinct
  claim_id
, claim_line_number
, data_source
, cast('2026-01-01 00:00:00' as timestamp) as tuva_last_run
from prof_and_low_priority_inst_claims
),  __dbt__cte__acute_inpatient__prof_claims as (


with first_claim as (

    select
          encounter_id
        , patient_data_source_id
    from __dbt__cte__acute_inpatient__generate_encounter_id
    where encounter_claim_number = 1

)

, join_first_claim_dates as (

    select
          f.encounter_id
        , f.patient_data_source_id
        , dat.anchor_claim_id
        , dat.encounter_end_date
        , dat.encounter_start_date
    from first_claim as f
    inner join __dbt__cte__acute_inpatient__start_end_dates as dat
        on f.encounter_id = dat.encounter_id

)

-- ensuring each prof claim is only attributed to one institutional claim with claim_attribution_number
select
      dat.encounter_id
    , dat.encounter_start_date
    , dat.encounter_end_date
    , med.claim_id
    , med.claim_line_number
    , med.data_source
    , row_number() over (
        partition by med.claim_id, med.claim_line_number, med.data_source
        order by dat.anchor_claim_id
      ) as claim_attribution_number
from __dbt__cte__encounters__stg_medical_claim as med
inner join __dbt__cte__encounters__prof_and_lower_priority as plp
    on med.claim_id = plp.claim_id
    and med.claim_line_number = plp.claim_line_number
    and med.data_source = plp.data_source
inner join join_first_claim_dates as dat
    on med.patient_data_source_id = dat.patient_data_source_id
    and med.start_date between dat.encounter_start_date and dat.encounter_end_date
),  __dbt__cte__emergency_department__generate_encounter_id_pre_sort as (


with claim_start_end as (
  select
      claim_id
    , patient_data_source_id
    , min(start_date) as start_date
    , max(end_date) as end_date
  from __dbt__cte__encounters__stg_medical_claim
  group by claim_id, patient_data_source_id
)

, base as (
  select distinct
      enc.claim_id
    , enc.patient_data_source_id
    , c.start_date
    , c.end_date
    , enc.facility_npi
    , enc.discharge_disposition_code
    , enc.claim_type  -- 'institutional' | 'professional'
  from __dbt__cte__encounters__stg_medical_claim as enc
  inner join claim_start_end as c
    on enc.claim_id = c.claim_id
   and c.patient_data_source_id = enc.patient_data_source_id
  where enc.service_category_2 = 'emergency department' -- both inst and prof
)

, add_row_num as (
  select
      patient_data_source_id
    , claim_id
    , start_date
    , end_date
    , discharge_disposition_code
    , facility_npi
    , claim_type
    , case when claim_type = 'professional' then 1 else 0 end as is_professional
    , rank() over (
        partition by patient_data_source_id
        order by end_date, start_date, claim_id
      ) as row_num
  from base
)

, check_for_merges_with_larger_row_num as (
  select
      aa.patient_data_source_id
    , aa.claim_id as claim_id_a
    , bb.claim_id as claim_id_b
    , aa.row_num as row_num_a
    , bb.row_num as row_num_b
    , case
        -- 1) exact end-date match at the same facility (dups/corrections)
        when aa.end_date = bb.end_date
         and aa.facility_npi = bb.facility_npi then 1

        -- 2) consecutive stay with transfer (end_date + 1 day == next start_date and discharge '30')
        when 

    (aa.end_date + cast(1 as bigint) * interval 1 day) = bb.start_date
         and aa.facility_npi = bb.facility_npi
         and aa.discharge_disposition_code = '30' then 1

        -- 3) general overlap at the same facility
        when aa.end_date <> bb.end_date
         and aa.end_date >= bb.start_date
         and aa.facility_npi = bb.facility_npi then 1

        -- 4) liberal rule when at least one is PROFESSIONAL:
        -- overlap OR 1-day gap; ignore facility/discharge as professional doesn't have these fields.
        when (aa.is_professional = 1 or bb.is_professional = 1)
         and 

    (aa.end_date + cast(1 as bigint) * interval 1 day) >= bb.start_date then 1

        else 0
      end as merge_flag
  from add_row_num as aa
  inner join add_row_num as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
   and aa.row_num < bb.row_num
   and aa.claim_id <> bb.claim_id
)



, merges_with_larger_row_num as (
  select
      patient_data_source_id
    , claim_id_a
    , claim_id_b
    , row_num_a
    , row_num_b
    , merge_flag
  from check_for_merges_with_larger_row_num
  where merge_flag = 1
)

, claim_ids_that_merge_with_larger_row_num as (
  select distinct
      patient_data_source_id
    , claim_id_a as claim_id
  from merges_with_larger_row_num
)

, claim_ids_having_a_smaller_row_num_merging_with_a_larger_row_num as (
  select distinct
      aa.patient_data_source_id
    , aa.claim_id as claim_id
  from add_row_num as aa
  inner join merges_with_larger_row_num as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
    and bb.row_num_a < aa.row_num
    and bb.row_num_b > aa.row_num
)

, close_flags as (
  select
      aa.patient_data_source_id
    , aa.claim_id
    , aa.start_date
    , aa.end_date
    , aa.discharge_disposition_code
    , aa.facility_npi
    , aa.row_num
    , case
        when bb.claim_id is null
          and cc.claim_id is null then 1
        else 0
      end as close_flag
  from add_row_num as aa
  left outer join claim_ids_that_merge_with_larger_row_num as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
    and aa.claim_id = bb.claim_id
  left outer join claim_ids_having_a_smaller_row_num_merging_with_a_larger_row_num as cc
    on aa.patient_data_source_id = cc.patient_data_source_id
    and aa.claim_id = cc.claim_id
)

, join_every_row_to_later_closes as (
  select
      aa.patient_data_source_id
    , aa.claim_id
    , aa.row_num
    , bb.row_num as row_num_b
  from close_flags as aa
  inner join close_flags as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
    and aa.row_num <= bb.row_num
  where bb.close_flag = 1
)

, find_min_closing_row_num_for_every_claim as (
  select
      patient_data_source_id
    , claim_id
    , min(row_num_b) as min_closing_row
  from join_every_row_to_later_closes
  group by
      patient_data_source_id
    , claim_id
)

, add_min_closing_row_to_every_claim as (
  select
      aa.patient_data_source_id
    , aa.claim_id
    , aa.start_date
    , aa.end_date
    , aa.discharge_disposition_code
    , aa.facility_npi
    , aa.row_num
    , aa.close_flag
    , bb.min_closing_row
  from close_flags as aa
  left outer join find_min_closing_row_num_for_every_claim as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
    and aa.claim_id = bb.claim_id
)


  select
      aa.patient_data_source_id
    , aa.claim_id
    , aa.start_date
    , aa.end_date
    , aa.discharge_disposition_code
    , aa.facility_npi
    , aa.row_num
    , aa.close_flag
    , aa.min_closing_row
    , bb.claim_id as encounter_id
  from add_min_closing_row_to_every_claim as aa
  left outer join add_min_closing_row_to_every_claim as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
    and aa.min_closing_row = bb.row_num
),  __dbt__cte__emergency_department__generate_encounter_id as (



select
    patient_data_source_id
  , claim_id
  , start_date
  , end_date
  , discharge_disposition_code
  , facility_npi
  , row_number() over (partition by patient_data_source_id, encounter_id
order by start_date, end_date, claim_id) as encounter_claim_number
  , row_number() over (partition by patient_data_source_id, encounter_id
order by start_date desc, end_date desc, claim_id desc) as encounter_claim_number_desc
  , close_flag
  , min_closing_row
  , lower(md5(((case when 'emergency department' is null then 'N' else ('V' || replace(replace(cast('emergency department' as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when patient_data_source_id is null then 'N' else ('V' || replace(replace(cast(patient_data_source_id as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when encounter_id is null then 'N' else ('V' || replace(replace(cast(encounter_id as TEXT), '%', '%25'), '|', '%7C')) end)))) as encounter_id
  , encounter_id as original_anchor_claim
from __dbt__cte__emergency_department__generate_encounter_id_pre_sort
),  __dbt__cte__emergency_department__start_end_dates as (


select encounter_id
, min(start_date) as encounter_start_date
, max(end_date) as encounter_end_date
from __dbt__cte__emergency_department__generate_encounter_id
group by encounter_id
),  __dbt__cte__encounters__stg_outpatient_institutional as (



select
  claim_id
, data_source
, service_type
, cast('2026-01-01 00:00:00' as timestamp) as tuva_last_run
from "claims_preprocessing"."service_category__stg_outpatient_institutional" as a
),  __dbt__cte__emergency_department__prof_claims as (


with first_claim as (
    select *
    from __dbt__cte__emergency_department__generate_encounter_id
    where encounter_claim_number = 1
)

, join_first_claim_dates as (
    select f.*
    , dat.encounter_end_date
    , dat.encounter_start_date
    from first_claim as f
    inner join __dbt__cte__emergency_department__start_end_dates as dat on f.encounter_id = dat.encounter_id
)


-- ensuring each claim is only attributed to one encounter with claim_attribution_number
, inst_and_prof as (
select dat.encounter_id
, dat.encounter_start_date
, dat.encounter_end_date
, dat.original_anchor_claim
, prof.claim_id
, prof.claim_line_number
, prof.data_source
from __dbt__cte__encounters__stg_medical_claim as med
inner join __dbt__cte__encounters__stg_professional as prof on med.claim_line_id = prof.claim_line_id
and med.data_source = prof.data_source
inner join join_first_claim_dates as dat on med.patient_data_source_id = dat.patient_data_source_id
and med.start_date between dat.encounter_start_date and dat.encounter_end_date

union all

select dat.encounter_id
, dat.encounter_start_date
, dat.encounter_end_date
, dat.original_anchor_claim
, med.claim_id
, med.claim_line_number
, med.data_source
from __dbt__cte__encounters__stg_medical_claim as med
inner join __dbt__cte__encounters__stg_outpatient_institutional as inst on med.claim_id = inst.claim_id
and med.data_source = inst.data_source
inner join join_first_claim_dates as dat on med.patient_data_source_id = dat.patient_data_source_id
and med.start_date between dat.encounter_start_date and dat.encounter_end_date
where dat.claim_id <> med.claim_id
)

select distinct encounter_id
, encounter_start_date
, encounter_end_date
, claim_id
, claim_line_number
, data_source
, row_number() over (partition by claim_id, claim_line_number, data_source
order by original_anchor_claim, encounter_id) as claim_attribution_number
from inst_and_prof
),  __dbt__cte__inpatient_psych__generate_encounter_id as (


with claim_start_end as (
  select
    claim_id
    , patient_data_source_id
    , min(start_date) as start_date
    , max(end_date) as end_date
  from __dbt__cte__encounters__stg_medical_claim
  group by claim_id, patient_data_source_id
)

, base as (
  select distinct
    enc.claim_id
    , enc.patient_data_source_id
    , c.start_date
    , c.end_date
    , enc.facility_npi
    , enc.discharge_disposition_code
  from __dbt__cte__encounters__stg_medical_claim as enc
  inner join claim_start_end as c
    on enc.claim_id = c.claim_id
    and c.patient_data_source_id = enc.patient_data_source_id
  where
    enc.service_category_2 = 'inpatient psychiatric'
    and enc.claim_type = 'institutional'
)

, add_row_num as (
  select
    patient_data_source_id
    , claim_id
    , start_date
    , end_date
    , discharge_disposition_code
    , facility_npi
    , rank() over (partition by patient_data_source_id
order by end_date, start_date, claim_id) as row_num
  from base
)

, check_for_merges_with_larger_row_num as (
  select
    aa.patient_data_source_id
    , aa.claim_id as claim_id_a
    , bb.claim_id as claim_id_b
    , aa.row_num as row_num_a
    , bb.row_num as row_num_b
    , case
      -- Condition 1: Exact End Date Match (Catches duplicates/corrections)
      when aa.end_date = bb.end_date
        and aa.facility_npi = bb.facility_npi then 1

      -- Condition 2: Consecutive Stay with Transfer (Catches month-end billing)
      when 

    (aa.end_date + cast(1 as bigint) * interval 1 day) = bb.start_date
        and aa.facility_npi = bb.facility_npi
        and aa.discharge_disposition_code = '30' then 1

      -- Condition 3: Same-Day Start / Superseded Claim
      when aa.start_date = bb.start_date
        and aa.facility_npi = bb.facility_npi
        and aa.discharge_disposition_code = '30' then 1

      -- Condition 4: General Overlapping Stay
      when aa.end_date <> bb.end_date
        and aa.end_date > bb.start_date
        and aa.facility_npi = bb.facility_npi then 1
      else 0
    end as merge_flag
  from add_row_num as aa
  inner join add_row_num as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
    and aa.row_num < bb.row_num
    and aa.claim_id <> bb.claim_id
)


, merges_with_larger_row_num as (
  select
      patient_data_source_id
    , claim_id_a
    , claim_id_b
    , row_num_a
    , row_num_b
    , merge_flag
  from check_for_merges_with_larger_row_num
  where merge_flag = 1
)

, claim_ids_that_merge_with_larger_row_num as (
  select distinct
      patient_data_source_id
    , claim_id_a as claim_id
  from merges_with_larger_row_num
)

, claim_ids_having_a_smaller_row_num_merging_with_a_larger_row_num as (
  select distinct
      aa.patient_data_source_id
    , aa.claim_id as claim_id
  from add_row_num as aa
  inner join merges_with_larger_row_num as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
    and bb.row_num_a < aa.row_num
    and bb.row_num_b > aa.row_num
)

, close_flags as (
  select
      aa.patient_data_source_id
    , aa.claim_id
    , aa.start_date
    , aa.end_date
    , aa.discharge_disposition_code
    , aa.facility_npi
    , aa.row_num
    , case
        when bb.claim_id is null
          and cc.claim_id is null then 1
        else 0
      end as close_flag
  from add_row_num as aa
  left outer join claim_ids_that_merge_with_larger_row_num as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
    and aa.claim_id = bb.claim_id
  left outer join claim_ids_having_a_smaller_row_num_merging_with_a_larger_row_num as cc
    on aa.patient_data_source_id = cc.patient_data_source_id
    and aa.claim_id = cc.claim_id
)

, join_every_row_to_later_closes as (
  select
      aa.patient_data_source_id
    , aa.claim_id
    , aa.row_num
    , bb.row_num as row_num_b
  from close_flags as aa
  inner join close_flags as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
    and aa.row_num <= bb.row_num
  where bb.close_flag = 1
)

, find_min_closing_row_num_for_every_claim as (
  select
      patient_data_source_id
    , claim_id
    , min(row_num_b) as min_closing_row
  from join_every_row_to_later_closes
  group by
      patient_data_source_id
    , claim_id
)

, add_min_closing_row_to_every_claim as (
  select
      aa.patient_data_source_id
    , aa.claim_id
    , aa.start_date
    , aa.end_date
    , aa.discharge_disposition_code
    , aa.facility_npi
    , aa.row_num
    , aa.close_flag
    , bb.min_closing_row
  from close_flags as aa
  left outer join find_min_closing_row_num_for_every_claim as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
    and aa.claim_id = bb.claim_id
)

, add_encounter_id as (
  select
      aa.patient_data_source_id
    , aa.claim_id
    , aa.start_date
    , aa.end_date
    , aa.discharge_disposition_code
    , aa.facility_npi
    , aa.row_num
    , aa.close_flag
    , aa.min_closing_row
    , bb.claim_id as encounter_id
  from add_min_closing_row_to_every_claim as aa
  left outer join add_min_closing_row_to_every_claim as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
    and aa.min_closing_row = bb.row_num
)

select
    patient_data_source_id
  , claim_id
  , start_date
  , end_date
  , discharge_disposition_code
  , facility_npi
  , row_number() over (partition by patient_data_source_id, encounter_id
order by start_date, end_date, claim_id) as encounter_claim_number
  , row_number() over (partition by patient_data_source_id, encounter_id
order by start_date desc, end_date desc, claim_id desc) as encounter_claim_number_desc
  , close_flag
  , min_closing_row
  , encounter_id as anchor_claim_id
  , lower(md5(((case when 'inpatient psych' is null then 'N' else ('V' || replace(replace(cast('inpatient psych' as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when patient_data_source_id is null then 'N' else ('V' || replace(replace(cast(patient_data_source_id as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when encounter_id is null then 'N' else ('V' || replace(replace(cast(encounter_id as TEXT), '%', '%25'), '|', '%7C')) end)))) as encounter_id
from add_encounter_id
),  __dbt__cte__inpatient_psych__start_end_dates as (


select encounter_id
, min(anchor_claim_id) as anchor_claim_id
, min(start_date) as encounter_start_date
, max(end_date) as encounter_end_date
from __dbt__cte__inpatient_psych__generate_encounter_id
group by encounter_id
),  __dbt__cte__inpatient_psych__prof_claims as (


with first_claim as (

    select
          encounter_id
        , patient_data_source_id
    from __dbt__cte__inpatient_psych__generate_encounter_id
    where encounter_claim_number = 1

)

, join_first_claim_dates as (

    select
          f.encounter_id
        , f.patient_data_source_id
        , dat.anchor_claim_id
        , dat.encounter_end_date
        , dat.encounter_start_date
    from first_claim as f
    inner join __dbt__cte__inpatient_psych__start_end_dates as dat
        on f.encounter_id = dat.encounter_id

)

-- ensuring each prof claim is only attributed to one institutional claim with claim_attribution_number
select
      dat.encounter_id
    , dat.encounter_start_date
    , dat.encounter_end_date
    , med.claim_id
    , med.claim_line_number
    , med.data_source
    , row_number() over (
        partition by med.claim_id, med.claim_line_number, med.data_source
        order by dat.anchor_claim_id
      ) as claim_attribution_number
from __dbt__cte__encounters__stg_medical_claim as med
inner join __dbt__cte__encounters__prof_and_lower_priority as plp
    on med.claim_id = plp.claim_id
    and med.claim_line_number = plp.claim_line_number
    and med.data_source = plp.data_source
inner join join_first_claim_dates as dat
    on med.patient_data_source_id = dat.patient_data_source_id
    and med.start_date between dat.encounter_start_date and dat.encounter_end_date
),  __dbt__cte__inpatient_rehab__generate_encounter_id as (


with claim_start_end as (
  select
    claim_id
    , patient_data_source_id
    , min(start_date) as start_date
    , max(end_date) as end_date
  from __dbt__cte__encounters__stg_medical_claim
  group by claim_id, patient_data_source_id
)

, base as (
  select distinct
    enc.claim_id
    , enc.patient_data_source_id
    , c.start_date
    , c.end_date
    , enc.facility_npi
    , enc.discharge_disposition_code
  from __dbt__cte__encounters__stg_medical_claim as enc
  inner join claim_start_end as c
    on enc.claim_id = c.claim_id
    and c.patient_data_source_id = enc.patient_data_source_id
  where
    enc.service_category_2 = 'inpatient rehabilitation'
    and enc.claim_type = 'institutional'
)

, add_row_num as (
  select
    patient_data_source_id
    , claim_id
    , start_date
    , end_date
    , discharge_disposition_code
    , facility_npi
    , rank() over (partition by patient_data_source_id
order by end_date, start_date, claim_id) as row_num
  from base
)

, check_for_merges_with_larger_row_num as (
  select
    aa.patient_data_source_id
    , aa.claim_id as claim_id_a
    , bb.claim_id as claim_id_b
    , aa.row_num as row_num_a
    , bb.row_num as row_num_b
    , case
      -- Condition 1: Exact End Date Match (Catches duplicates/corrections)
      when aa.end_date = bb.end_date
        and aa.facility_npi = bb.facility_npi then 1

      -- Condition 2: Consecutive Stay with Transfer (Catches month-end billing)
      when 

    (aa.end_date + cast(1 as bigint) * interval 1 day) = bb.start_date
        and aa.facility_npi = bb.facility_npi
        and aa.discharge_disposition_code = '30' then 1

      -- Condition 3: Same-Day Start / Superseded Claim
      when aa.start_date = bb.start_date
        and aa.facility_npi = bb.facility_npi
        and aa.discharge_disposition_code = '30' then 1

      -- Condition 4: General Overlapping Stay
      when aa.end_date <> bb.end_date
        and aa.end_date > bb.start_date
        and aa.facility_npi = bb.facility_npi then 1
      else 0
    end as merge_flag
  from add_row_num as aa
  inner join add_row_num as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
    and aa.row_num < bb.row_num
    and aa.claim_id <> bb.claim_id
)


, merges_with_larger_row_num as (
  select
      patient_data_source_id
    , claim_id_a
    , claim_id_b
    , row_num_a
    , row_num_b
    , merge_flag
  from check_for_merges_with_larger_row_num
  where merge_flag = 1
)

, claim_ids_that_merge_with_larger_row_num as (
  select distinct
      patient_data_source_id
    , claim_id_a as claim_id
  from merges_with_larger_row_num
)

, claim_ids_having_a_smaller_row_num_merging_with_a_larger_row_num as (
  select distinct
      aa.patient_data_source_id
    , aa.claim_id as claim_id
  from add_row_num as aa
  inner join merges_with_larger_row_num as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
    and bb.row_num_a < aa.row_num
    and bb.row_num_b > aa.row_num
)

, close_flags as (
  select
      aa.patient_data_source_id
    , aa.claim_id
    , aa.start_date
    , aa.end_date
    , aa.discharge_disposition_code
    , aa.facility_npi
    , aa.row_num
    , case
        when bb.claim_id is null
          and cc.claim_id is null then 1
        else 0
      end as close_flag
  from add_row_num as aa
  left outer join claim_ids_that_merge_with_larger_row_num as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
    and aa.claim_id = bb.claim_id
  left outer join claim_ids_having_a_smaller_row_num_merging_with_a_larger_row_num as cc
    on aa.patient_data_source_id = cc.patient_data_source_id
    and aa.claim_id = cc.claim_id
)

, join_every_row_to_later_closes as (
  select
      aa.patient_data_source_id
    , aa.claim_id
    , aa.row_num
    , bb.row_num as row_num_b
  from close_flags as aa
  inner join close_flags as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
    and aa.row_num <= bb.row_num
  where bb.close_flag = 1
)

, find_min_closing_row_num_for_every_claim as (
  select
      patient_data_source_id
    , claim_id
    , min(row_num_b) as min_closing_row
  from join_every_row_to_later_closes
  group by
      patient_data_source_id
    , claim_id
)

, add_min_closing_row_to_every_claim as (
  select
      aa.patient_data_source_id
    , aa.claim_id
    , aa.start_date
    , aa.end_date
    , aa.discharge_disposition_code
    , aa.facility_npi
    , aa.row_num
    , aa.close_flag
    , bb.min_closing_row
  from close_flags as aa
  left outer join find_min_closing_row_num_for_every_claim as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
    and aa.claim_id = bb.claim_id
)

, add_encounter_id as (
  select
      aa.patient_data_source_id
    , aa.claim_id
    , aa.start_date
    , aa.end_date
    , aa.discharge_disposition_code
    , aa.facility_npi
    , aa.row_num
    , aa.close_flag
    , aa.min_closing_row
    , bb.claim_id as encounter_id
  from add_min_closing_row_to_every_claim as aa
  left outer join add_min_closing_row_to_every_claim as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
    and aa.min_closing_row = bb.row_num
)

select
    patient_data_source_id
  , claim_id
  , start_date
  , end_date
  , discharge_disposition_code
  , facility_npi
  , row_number() over (partition by patient_data_source_id, encounter_id
order by start_date, end_date, claim_id) as encounter_claim_number
  , row_number() over (partition by patient_data_source_id, encounter_id
order by start_date desc, end_date desc, claim_id desc) as encounter_claim_number_desc
  , close_flag
  , min_closing_row
  , encounter_id as anchor_claim_id
  , lower(md5(((case when 'inpatient rehabilitation' is null then 'N' else ('V' || replace(replace(cast('inpatient rehabilitation' as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when patient_data_source_id is null then 'N' else ('V' || replace(replace(cast(patient_data_source_id as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when encounter_id is null then 'N' else ('V' || replace(replace(cast(encounter_id as TEXT), '%', '%25'), '|', '%7C')) end)))) as encounter_id
from add_encounter_id
),  __dbt__cte__inpatient_rehab__start_end_dates as (


select encounter_id
, min(anchor_claim_id) as anchor_claim_id
, min(start_date) as encounter_start_date
, max(end_date) as encounter_end_date
from __dbt__cte__inpatient_rehab__generate_encounter_id
group by encounter_id
),  __dbt__cte__inpatient_rehab__prof_claims as (


with first_claim as (

    select
          encounter_id
        , patient_data_source_id
    from __dbt__cte__inpatient_rehab__generate_encounter_id
    where encounter_claim_number = 1

)

, join_first_claim_dates as (

    select
          f.encounter_id
        , f.patient_data_source_id
        , dat.anchor_claim_id
        , dat.encounter_end_date
        , dat.encounter_start_date
    from first_claim as f
    inner join __dbt__cte__inpatient_rehab__start_end_dates as dat
        on f.encounter_id = dat.encounter_id

)

-- ensuring each prof claim is only attributed to one institutional claim with claim_attribution_number
select
      dat.encounter_id
    , dat.encounter_start_date
    , dat.encounter_end_date
    , med.claim_id
    , med.claim_line_number
    , med.data_source
    , row_number() over (
        partition by med.claim_id, med.claim_line_number, med.data_source
        order by dat.anchor_claim_id
      ) as claim_attribution_number
from __dbt__cte__encounters__stg_medical_claim as med
inner join __dbt__cte__encounters__prof_and_lower_priority as plp
    on med.claim_id = plp.claim_id
    and med.claim_line_number = plp.claim_line_number
    and med.data_source = plp.data_source
inner join join_first_claim_dates as dat
    on med.patient_data_source_id = dat.patient_data_source_id
    and med.start_date between dat.encounter_start_date and dat.encounter_end_date
),  __dbt__cte__inpatient_long_term__generate_encounter_id as (


with claim_start_end as (
  select
    claim_id
    , patient_data_source_id
    , min(start_date) as start_date
    , max(end_date) as end_date
  from __dbt__cte__encounters__stg_medical_claim
  group by claim_id, patient_data_source_id
)

, base as (
  select distinct
    enc.claim_id
    , enc.patient_data_source_id
    , c.start_date
    , c.end_date
    , enc.facility_npi
    , enc.discharge_disposition_code
  from __dbt__cte__encounters__stg_medical_claim as enc
  inner join claim_start_end as c
    on enc.claim_id = c.claim_id
    and c.patient_data_source_id = enc.patient_data_source_id
  where
    enc.service_category_2 = 'inpatient long term acute care'
    and enc.claim_type = 'institutional'
)

, add_row_num as (
  select
    patient_data_source_id
    , claim_id
    , start_date
    , end_date
    , discharge_disposition_code
    , facility_npi
    , rank() over (partition by patient_data_source_id
order by end_date, start_date, claim_id) as row_num
  from base
)

, check_for_merges_with_larger_row_num as (
  select
    aa.patient_data_source_id
    , aa.claim_id as claim_id_a
    , bb.claim_id as claim_id_b
    , aa.row_num as row_num_a
    , bb.row_num as row_num_b
    , case
      -- Condition 1: Exact End Date Match (Catches duplicates/corrections)
      when aa.end_date = bb.end_date
        and aa.facility_npi = bb.facility_npi then 1

      -- Condition 2: Consecutive Stay with Transfer (Catches month-end billing)
      when 

    (aa.end_date + cast(1 as bigint) * interval 1 day) = bb.start_date
        and aa.facility_npi = bb.facility_npi
        and aa.discharge_disposition_code = '30' then 1

      -- Condition 3: Same-Day Start / Superseded Claim
      when aa.start_date = bb.start_date
        and aa.facility_npi = bb.facility_npi
        and aa.discharge_disposition_code = '30' then 1

      -- Condition 4: General Overlapping Stay
      when aa.end_date <> bb.end_date
        and aa.end_date > bb.start_date
        and aa.facility_npi = bb.facility_npi then 1
      else 0
    end as merge_flag
  from add_row_num as aa
  inner join add_row_num as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
    and aa.row_num < bb.row_num
    and aa.claim_id <> bb.claim_id
)


, merges_with_larger_row_num as (
  select
      patient_data_source_id
    , claim_id_a
    , claim_id_b
    , row_num_a
    , row_num_b
    , merge_flag
  from check_for_merges_with_larger_row_num
  where merge_flag = 1
)

, claim_ids_that_merge_with_larger_row_num as (
  select distinct
      patient_data_source_id
    , claim_id_a as claim_id
  from merges_with_larger_row_num
)

, claim_ids_having_a_smaller_row_num_merging_with_a_larger_row_num as (
  select distinct
      aa.patient_data_source_id
    , aa.claim_id as claim_id
  from add_row_num as aa
  inner join merges_with_larger_row_num as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
    and bb.row_num_a < aa.row_num
    and bb.row_num_b > aa.row_num
)

, close_flags as (
  select
      aa.patient_data_source_id
    , aa.claim_id
    , aa.start_date
    , aa.end_date
    , aa.discharge_disposition_code
    , aa.facility_npi
    , aa.row_num
    , case
        when bb.claim_id is null
          and cc.claim_id is null then 1
        else 0
      end as close_flag
  from add_row_num as aa
  left outer join claim_ids_that_merge_with_larger_row_num as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
    and aa.claim_id = bb.claim_id
  left outer join claim_ids_having_a_smaller_row_num_merging_with_a_larger_row_num as cc
    on aa.patient_data_source_id = cc.patient_data_source_id
    and aa.claim_id = cc.claim_id
)

, join_every_row_to_later_closes as (
  select
      aa.patient_data_source_id
    , aa.claim_id
    , aa.row_num
    , bb.row_num as row_num_b
  from close_flags as aa
  inner join close_flags as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
    and aa.row_num <= bb.row_num
  where bb.close_flag = 1
)

, find_min_closing_row_num_for_every_claim as (
  select
      patient_data_source_id
    , claim_id
    , min(row_num_b) as min_closing_row
  from join_every_row_to_later_closes
  group by
      patient_data_source_id
    , claim_id
)

, add_min_closing_row_to_every_claim as (
  select
      aa.patient_data_source_id
    , aa.claim_id
    , aa.start_date
    , aa.end_date
    , aa.discharge_disposition_code
    , aa.facility_npi
    , aa.row_num
    , aa.close_flag
    , bb.min_closing_row
  from close_flags as aa
  left outer join find_min_closing_row_num_for_every_claim as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
    and aa.claim_id = bb.claim_id
)

, add_encounter_id as (
  select
      aa.patient_data_source_id
    , aa.claim_id
    , aa.start_date
    , aa.end_date
    , aa.discharge_disposition_code
    , aa.facility_npi
    , aa.row_num
    , aa.close_flag
    , aa.min_closing_row
    , bb.claim_id as encounter_id
  from add_min_closing_row_to_every_claim as aa
  left outer join add_min_closing_row_to_every_claim as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
    and aa.min_closing_row = bb.row_num
)

select
    patient_data_source_id
  , claim_id
  , start_date
  , end_date
  , discharge_disposition_code
  , facility_npi
  , row_number() over (partition by patient_data_source_id, encounter_id
order by start_date, end_date, claim_id) as encounter_claim_number
  , row_number() over (partition by patient_data_source_id, encounter_id
order by start_date desc, end_date desc, claim_id desc) as encounter_claim_number_desc
  , close_flag
  , min_closing_row
  , encounter_id as anchor_claim_id
  , lower(md5(((case when 'inpatient long term acute care' is null then 'N' else ('V' || replace(replace(cast('inpatient long term acute care' as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when patient_data_source_id is null then 'N' else ('V' || replace(replace(cast(patient_data_source_id as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when encounter_id is null then 'N' else ('V' || replace(replace(cast(encounter_id as TEXT), '%', '%25'), '|', '%7C')) end)))) as encounter_id
from add_encounter_id
),  __dbt__cte__inpatient_long_term__start_end_dates as (


select encounter_id
, min(anchor_claim_id) as anchor_claim_id
, min(start_date) as encounter_start_date
, max(end_date) as encounter_end_date
from __dbt__cte__inpatient_long_term__generate_encounter_id
group by encounter_id
),  __dbt__cte__inpatient_long_term__prof_claims as (


with first_claim as (

    select
          encounter_id
        , patient_data_source_id
    from __dbt__cte__inpatient_long_term__generate_encounter_id
    where encounter_claim_number = 1

)

, join_first_claim_dates as (

    select
          f.encounter_id
        , f.patient_data_source_id
        , dat.anchor_claim_id
        , dat.encounter_end_date
        , dat.encounter_start_date
    from first_claim as f
    inner join __dbt__cte__inpatient_long_term__start_end_dates as dat
         on f.encounter_id = dat.encounter_id

)

-- ensuring each prof claim is only attributed to one institutional claim with claim_attribution_number
select
      dat.encounter_id
    , dat.encounter_start_date
    , dat.encounter_end_date
    , med.claim_id
    , med.claim_line_number
    , med.data_source
    , row_number() over (
        partition by med.claim_id, med.claim_line_number, med.data_source
        order by dat.anchor_claim_id
      ) as claim_attribution_number
from __dbt__cte__encounters__stg_medical_claim as med
inner join __dbt__cte__encounters__prof_and_lower_priority as plp
    on med.claim_id = plp.claim_id
    and med.claim_line_number = plp.claim_line_number
    and med.data_source = plp.data_source
inner join join_first_claim_dates as dat
    on med.patient_data_source_id = dat.patient_data_source_id
    and med.start_date between dat.encounter_start_date and dat.encounter_end_date
),  __dbt__cte__inpatient_snf__generate_encounter_id as (


with claim_start_end as (
  select
    claim_id
    , patient_data_source_id
    , min(start_date) as start_date
    , max(end_date) as end_date
  from __dbt__cte__encounters__stg_medical_claim
  group by claim_id, patient_data_source_id
)

, base as (
  select distinct
    enc.claim_id
    , enc.patient_data_source_id
    , c.start_date
    , c.end_date
    , enc.facility_npi
    , enc.discharge_disposition_code
  from __dbt__cte__encounters__stg_medical_claim as enc
  inner join claim_start_end as c
    on enc.claim_id = c.claim_id
    and c.patient_data_source_id = enc.patient_data_source_id
  where
    enc.service_category_2 = 'skilled nursing'
    and enc.claim_type = 'institutional'
)

, add_row_num as (
  select
    patient_data_source_id
    , claim_id
    , start_date
    , end_date
    , discharge_disposition_code
    , facility_npi
    , rank() over (partition by patient_data_source_id
order by end_date, start_date, claim_id) as row_num
  from base
)

, check_for_merges_with_larger_row_num as (
  select
    aa.patient_data_source_id
    , aa.claim_id as claim_id_a
    , bb.claim_id as claim_id_b
    , aa.row_num as row_num_a
    , bb.row_num as row_num_b
    , case
      -- Condition 1: Exact End Date Match (Catches duplicates/corrections)
      when aa.end_date = bb.end_date
        and aa.facility_npi = bb.facility_npi then 1

      -- Condition 2: Consecutive Stay with Transfer (Catches month-end billing)
      when 

    (aa.end_date + cast(1 as bigint) * interval 1 day) = bb.start_date
        and aa.facility_npi = bb.facility_npi
        and aa.discharge_disposition_code = '30' then 1

      -- Condition 3: Same-Day Start / Superseded Claim
      when aa.start_date = bb.start_date
        and aa.facility_npi = bb.facility_npi
        and aa.discharge_disposition_code = '30' then 1

      -- Condition 4: General Overlapping Stay
      when aa.end_date <> bb.end_date
        and aa.end_date > bb.start_date
        and aa.facility_npi = bb.facility_npi then 1
      else 0
    end as merge_flag
  from add_row_num as aa
  inner join add_row_num as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
    and aa.row_num < bb.row_num
    and aa.claim_id <> bb.claim_id
)

, merges_with_larger_row_num as (
  select
      patient_data_source_id
    , claim_id_a
    , claim_id_b
    , row_num_a
    , row_num_b
    , merge_flag
  from check_for_merges_with_larger_row_num
  where merge_flag = 1
)

, claim_ids_that_merge_with_larger_row_num as (
  select distinct
      patient_data_source_id
    , claim_id_a as claim_id
  from merges_with_larger_row_num
)

, claim_ids_having_a_smaller_row_num_merging_with_a_larger_row_num as (
  select distinct
      aa.patient_data_source_id
    , aa.claim_id as claim_id
  from add_row_num as aa
  inner join merges_with_larger_row_num as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
    and bb.row_num_a < aa.row_num
    and bb.row_num_b > aa.row_num
)

, close_flags as (
  select
      aa.patient_data_source_id
    , aa.claim_id
    , aa.start_date
    , aa.end_date
    , aa.discharge_disposition_code
    , aa.facility_npi
    , aa.row_num
    , case
        when bb.claim_id is null
          and cc.claim_id is null then 1
        else 0
      end as close_flag
  from add_row_num as aa
  left outer join claim_ids_that_merge_with_larger_row_num as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
    and aa.claim_id = bb.claim_id
  left outer join claim_ids_having_a_smaller_row_num_merging_with_a_larger_row_num as cc
    on aa.patient_data_source_id = cc.patient_data_source_id
    and aa.claim_id = cc.claim_id
)

, join_every_row_to_later_closes as (
  select
      aa.patient_data_source_id
    , aa.claim_id
    , aa.row_num
    , bb.row_num as row_num_b
  from close_flags as aa
  inner join close_flags as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
    and aa.row_num <= bb.row_num
  where bb.close_flag = 1
)

, find_min_closing_row_num_for_every_claim as (
  select
      patient_data_source_id
    , claim_id
    , min(row_num_b) as min_closing_row
  from join_every_row_to_later_closes
  group by
      patient_data_source_id
    , claim_id
)

, add_min_closing_row_to_every_claim as (
  select
      aa.patient_data_source_id
    , aa.claim_id
    , aa.start_date
    , aa.end_date
    , aa.discharge_disposition_code
    , aa.facility_npi
    , aa.row_num
    , aa.close_flag
    , bb.min_closing_row
  from close_flags as aa
  left outer join find_min_closing_row_num_for_every_claim as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
    and aa.claim_id = bb.claim_id
)

, add_encounter_id as (
  select
      aa.patient_data_source_id
    , aa.claim_id
    , aa.start_date
    , aa.end_date
    , aa.discharge_disposition_code
    , aa.facility_npi
    , aa.row_num
    , aa.close_flag
    , aa.min_closing_row
    , bb.claim_id as encounter_id
  from add_min_closing_row_to_every_claim as aa
  left outer join add_min_closing_row_to_every_claim as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
    and aa.min_closing_row = bb.row_num
)

select
    patient_data_source_id
  , claim_id
  , start_date
  , end_date
  , discharge_disposition_code
  , facility_npi
  , row_number() over (partition by patient_data_source_id, encounter_id
order by start_date, end_date, claim_id) as encounter_claim_number
  , row_number() over (partition by patient_data_source_id, encounter_id
order by start_date desc, end_date desc, claim_id desc) as encounter_claim_number_desc
  , close_flag
  , min_closing_row
  , encounter_id as anchor_claim_id
  , lower(md5(((case when 'inpatient skilled nursing' is null then 'N' else ('V' || replace(replace(cast('inpatient skilled nursing' as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when patient_data_source_id is null then 'N' else ('V' || replace(replace(cast(patient_data_source_id as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when encounter_id is null then 'N' else ('V' || replace(replace(cast(encounter_id as TEXT), '%', '%25'), '|', '%7C')) end)))) as encounter_id
from add_encounter_id
),  __dbt__cte__inpatient_snf__start_end_dates as (


select encounter_id
, min(anchor_claim_id) as anchor_claim_id
, min(start_date) as encounter_start_date
, max(end_date) as encounter_end_date
from __dbt__cte__inpatient_snf__generate_encounter_id
group by encounter_id
),  __dbt__cte__inpatient_snf__prof_claims as (


with first_claim as (

    select
          encounter_id
        , patient_data_source_id
    from __dbt__cte__inpatient_snf__generate_encounter_id
    where encounter_claim_number = 1

)

, join_first_claim_dates as (

    select
          f.encounter_id
        , f.patient_data_source_id
        , dat.anchor_claim_id
        , dat.encounter_end_date
        , dat.encounter_start_date
    from first_claim as f
    inner join __dbt__cte__inpatient_snf__start_end_dates as dat
        on f.encounter_id = dat.encounter_id

)

-- ensuring each prof claim is only attributed to one institutional claim with claim_attribution_number
select
      dat.encounter_id
    , dat.encounter_start_date
    , dat.encounter_end_date
    , med.claim_id
    , med.claim_line_number
    , med.data_source
    , row_number() over (
        partition by med.claim_id, med.claim_line_number, med.data_source
        order by dat.anchor_claim_id
      ) as claim_attribution_number
from __dbt__cte__encounters__stg_medical_claim as med
inner join __dbt__cte__encounters__prof_and_lower_priority as plp
    on med.claim_id = plp.claim_id
    and med.claim_line_number = plp.claim_line_number
    and med.data_source = plp.data_source
inner join join_first_claim_dates as dat
    on med.patient_data_source_id = dat.patient_data_source_id
    and med.start_date between dat.encounter_start_date and dat.encounter_end_date
),  __dbt__cte__inpatient_hospice__generate_encounter_id as (


with claim_start_end as (
  select
    claim_id
    , patient_data_source_id
    , min(start_date) as start_date
    , max(end_date) as end_date
  from __dbt__cte__encounters__stg_medical_claim
  group by claim_id, patient_data_source_id
)

, base as (
  select distinct
    enc.claim_id
    , enc.patient_data_source_id
    , c.start_date
    , c.end_date
    , enc.facility_npi
    , enc.discharge_disposition_code
  from __dbt__cte__encounters__stg_medical_claim as enc
  inner join claim_start_end as c
    on enc.claim_id = c.claim_id
    and c.patient_data_source_id = enc.patient_data_source_id
  where
    enc.service_category_2 in ('inpatient hospice')
    and enc.claim_type = 'institutional'
)

, add_row_num as (
  select
    patient_data_source_id
    , claim_id
    , start_date
    , end_date
    , discharge_disposition_code
    , facility_npi
    , rank() over (partition by patient_data_source_id
order by end_date, start_date, claim_id) as row_num
  from base
)

, check_for_merges_with_larger_row_num as (
  select
    aa.patient_data_source_id
    , aa.claim_id as claim_id_a
    , bb.claim_id as claim_id_b
    , aa.row_num as row_num_a
    , bb.row_num as row_num_b
    , case
      -- Condition 1: Exact End Date Match (Catches duplicates/corrections)
      when aa.end_date = bb.end_date
        and aa.facility_npi = bb.facility_npi then 1

      -- Condition 2: Consecutive Stay with Transfer (Catches month-end billing)
      when 

    (aa.end_date + cast(1 as bigint) * interval 1 day) = bb.start_date
        and aa.facility_npi = bb.facility_npi
        and aa.discharge_disposition_code = '30' then 1

      -- Condition 3: Same-Day Start / Superseded Claim
      when aa.start_date = bb.start_date
        and aa.facility_npi = bb.facility_npi
        and aa.discharge_disposition_code = '30' then 1

      -- Condition 4: General Overlapping Stay
      when aa.end_date <> bb.end_date
        and aa.end_date > bb.start_date
        and aa.facility_npi = bb.facility_npi then 1
      else 0
    end as merge_flag
  from add_row_num as aa
  inner join add_row_num as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
    and aa.row_num < bb.row_num
    and aa.claim_id <> bb.claim_id
)


, merges_with_larger_row_num as (
  select
      patient_data_source_id
    , claim_id_a
    , claim_id_b
    , row_num_a
    , row_num_b
    , merge_flag
  from check_for_merges_with_larger_row_num
  where merge_flag = 1
)

, claim_ids_that_merge_with_larger_row_num as (
  select distinct
      patient_data_source_id
    , claim_id_a as claim_id
  from merges_with_larger_row_num
)

, claim_ids_having_a_smaller_row_num_merging_with_a_larger_row_num as (
  select distinct
      aa.patient_data_source_id
    , aa.claim_id as claim_id
  from add_row_num as aa
  inner join merges_with_larger_row_num as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
    and bb.row_num_a < aa.row_num
    and bb.row_num_b > aa.row_num
)

, close_flags as (
  select
      aa.patient_data_source_id
    , aa.claim_id
    , aa.start_date
    , aa.end_date
    , aa.discharge_disposition_code
    , aa.facility_npi
    , aa.row_num
    , case
        when bb.claim_id is null
          and cc.claim_id is null then 1
        else 0
      end as close_flag
  from add_row_num as aa
  left outer join claim_ids_that_merge_with_larger_row_num as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
    and aa.claim_id = bb.claim_id
  left outer join claim_ids_having_a_smaller_row_num_merging_with_a_larger_row_num as cc
    on aa.patient_data_source_id = cc.patient_data_source_id
    and aa.claim_id = cc.claim_id
)

, join_every_row_to_later_closes as (
  select
      aa.patient_data_source_id
    , aa.claim_id
    , aa.row_num
    , bb.row_num as row_num_b
  from close_flags as aa
  inner join close_flags as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
    and aa.row_num <= bb.row_num
  where bb.close_flag = 1
)

, find_min_closing_row_num_for_every_claim as (
  select
      patient_data_source_id
    , claim_id
    , min(row_num_b) as min_closing_row
  from join_every_row_to_later_closes
  group by
      patient_data_source_id
    , claim_id
)

, add_min_closing_row_to_every_claim as (
  select
      aa.patient_data_source_id
    , aa.claim_id
    , aa.start_date
    , aa.end_date
    , aa.discharge_disposition_code
    , aa.facility_npi
    , aa.row_num
    , aa.close_flag
    , bb.min_closing_row
  from close_flags as aa
  left outer join find_min_closing_row_num_for_every_claim as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
    and aa.claim_id = bb.claim_id
)

, add_encounter_id as (
  select
      aa.patient_data_source_id
    , aa.claim_id
    , aa.start_date
    , aa.end_date
    , aa.discharge_disposition_code
    , aa.facility_npi
    , aa.row_num
    , aa.close_flag
    , aa.min_closing_row
    , bb.claim_id as encounter_id
  from add_min_closing_row_to_every_claim as aa
  left outer join add_min_closing_row_to_every_claim as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
    and aa.min_closing_row = bb.row_num
)

select
    patient_data_source_id
  , claim_id
  , start_date
  , end_date
  , discharge_disposition_code
  , facility_npi
  , row_number() over (partition by patient_data_source_id, encounter_id
order by start_date, end_date, claim_id) as encounter_claim_number
  , row_number() over (partition by patient_data_source_id, encounter_id
order by start_date desc, end_date desc, claim_id desc) as encounter_claim_number_desc
  , close_flag
  , min_closing_row
  , encounter_id as anchor_claim_id
  , lower(md5(((case when 'inpatient hospice' is null then 'N' else ('V' || replace(replace(cast('inpatient hospice' as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when patient_data_source_id is null then 'N' else ('V' || replace(replace(cast(patient_data_source_id as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when encounter_id is null then 'N' else ('V' || replace(replace(cast(encounter_id as TEXT), '%', '%25'), '|', '%7C')) end)))) as encounter_id
from add_encounter_id
),  __dbt__cte__inpatient_hospice__start_end_dates as (


select encounter_id
, min(anchor_claim_id) as anchor_claim_id
, min(start_date) as encounter_start_date
, max(end_date) as encounter_end_date
from __dbt__cte__inpatient_hospice__generate_encounter_id
group by encounter_id
),  __dbt__cte__inpatient_hospice__prof_claims as (


with first_claim as (

    select
          encounter_id
        , patient_data_source_id
    from __dbt__cte__inpatient_hospice__generate_encounter_id
    where encounter_claim_number = 1

)

, join_first_claim_dates as (

    select
          f.encounter_id
        , f.patient_data_source_id
        , dat.anchor_claim_id
        , dat.encounter_end_date
        , dat.encounter_start_date
    from first_claim as f
    inner join __dbt__cte__inpatient_hospice__start_end_dates as dat
         on f.encounter_id = dat.encounter_id

)

-- ensuring each prof claim is only attributed to one institutional claim with claim_attribution_number
select
      dat.encounter_id
    , dat.encounter_start_date
    , dat.encounter_end_date
    , med.claim_id
    , med.claim_line_number
    , med.data_source
    , row_number() over (
        partition by med.claim_id, med.claim_line_number, med.data_source
        order by dat.anchor_claim_id
      ) as claim_attribution_number
from __dbt__cte__encounters__stg_medical_claim as med
inner join __dbt__cte__encounters__prof_and_lower_priority as plp
    on med.claim_id = plp.claim_id
    and med.claim_line_number = plp.claim_line_number
    and med.data_source = plp.data_source
inner join join_first_claim_dates as dat
    on med.patient_data_source_id = dat.patient_data_source_id
    and med.start_date between dat.encounter_start_date and dat.encounter_end_date
),  __dbt__cte__inpatient_substance_use__generate_encounter_id as (


with claim_start_end as (
  select
    claim_id
    , patient_data_source_id
    , min(start_date) as start_date
    , max(end_date) as end_date
  from __dbt__cte__encounters__stg_medical_claim
  group by claim_id, patient_data_source_id
)

, base as (
  select distinct
    enc.claim_id
    , enc.patient_data_source_id
    , c.start_date
    , c.end_date
    , enc.facility_npi
    , enc.discharge_disposition_code
  from __dbt__cte__encounters__stg_medical_claim as enc
  inner join claim_start_end as c
    on enc.claim_id = c.claim_id
    and c.patient_data_source_id = enc.patient_data_source_id
  where
    enc.service_category_2 = 'inpatient substance use'
    and enc.claim_type = 'institutional'
)

, add_row_num as (
  select
    patient_data_source_id
    , claim_id
    , start_date
    , end_date
    , discharge_disposition_code
    , facility_npi
    , rank() over (partition by patient_data_source_id
order by end_date, start_date, claim_id) as row_num
  from base
)

, check_for_merges_with_larger_row_num as (
  select
    aa.patient_data_source_id
    , aa.claim_id as claim_id_a
    , bb.claim_id as claim_id_b
    , aa.row_num as row_num_a
    , bb.row_num as row_num_b
    , case
      -- Condition 1: Exact End Date Match (Catches duplicates/corrections)
      when aa.end_date = bb.end_date
        and aa.facility_npi = bb.facility_npi then 1

      -- Condition 2: Consecutive Stay with Transfer (Catches month-end billing)
      when 

    (aa.end_date + cast(1 as bigint) * interval 1 day) = bb.start_date
        and aa.facility_npi = bb.facility_npi
        and aa.discharge_disposition_code = '30' then 1

      -- Condition 3: Same-Day Start / Superseded Claim
      when aa.start_date = bb.start_date
        and aa.facility_npi = bb.facility_npi
        and aa.discharge_disposition_code = '30' then 1

      -- Condition 4: General Overlapping Stay
      when aa.end_date <> bb.end_date
        and aa.end_date > bb.start_date
        and aa.facility_npi = bb.facility_npi then 1
      else 0
    end as merge_flag
  from add_row_num as aa
  inner join add_row_num as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
    and aa.row_num < bb.row_num
    and aa.claim_id <> bb.claim_id
)


, merges_with_larger_row_num as (
  select
      patient_data_source_id
    , claim_id_a
    , claim_id_b
    , row_num_a
    , row_num_b
    , merge_flag
  from check_for_merges_with_larger_row_num
  where merge_flag = 1
)

, claim_ids_that_merge_with_larger_row_num as (
  select distinct
      patient_data_source_id
    , claim_id_a as claim_id
  from merges_with_larger_row_num
)

, claim_ids_having_a_smaller_row_num_merging_with_a_larger_row_num as (
  select distinct
      aa.patient_data_source_id
    , aa.claim_id as claim_id
  from add_row_num as aa
  inner join merges_with_larger_row_num as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
    and bb.row_num_a < aa.row_num
    and bb.row_num_b > aa.row_num
)

, close_flags as (
  select
      aa.patient_data_source_id
    , aa.claim_id
    , aa.start_date
    , aa.end_date
    , aa.discharge_disposition_code
    , aa.facility_npi
    , aa.row_num
    , case
        when bb.claim_id is null
          and cc.claim_id is null then 1
        else 0
      end as close_flag
  from add_row_num as aa
  left outer join claim_ids_that_merge_with_larger_row_num as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
    and aa.claim_id = bb.claim_id
  left outer join claim_ids_having_a_smaller_row_num_merging_with_a_larger_row_num as cc
    on aa.patient_data_source_id = cc.patient_data_source_id
    and aa.claim_id = cc.claim_id
)

, join_every_row_to_later_closes as (
  select
      aa.patient_data_source_id
    , aa.claim_id
    , aa.row_num
    , bb.row_num as row_num_b
  from close_flags as aa
  inner join close_flags as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
    and aa.row_num <= bb.row_num
  where bb.close_flag = 1
)

, find_min_closing_row_num_for_every_claim as (
  select
      patient_data_source_id
    , claim_id
    , min(row_num_b) as min_closing_row
  from join_every_row_to_later_closes
  group by
      patient_data_source_id
    , claim_id
)

, add_min_closing_row_to_every_claim as (
  select
      aa.patient_data_source_id
    , aa.claim_id
    , aa.start_date
    , aa.end_date
    , aa.discharge_disposition_code
    , aa.facility_npi
    , aa.row_num
    , aa.close_flag
    , bb.min_closing_row
  from close_flags as aa
  left outer join find_min_closing_row_num_for_every_claim as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
    and aa.claim_id = bb.claim_id
)

, add_encounter_id as (
  select
      aa.patient_data_source_id
    , aa.claim_id
    , aa.start_date
    , aa.end_date
    , aa.discharge_disposition_code
    , aa.facility_npi
    , aa.row_num
    , aa.close_flag
    , aa.min_closing_row
    , bb.claim_id as encounter_id
  from add_min_closing_row_to_every_claim as aa
  left outer join add_min_closing_row_to_every_claim as bb
    on aa.patient_data_source_id = bb.patient_data_source_id
    and aa.min_closing_row = bb.row_num
)

select
    patient_data_source_id
  , claim_id
  , start_date
  , end_date
  , discharge_disposition_code
  , facility_npi
  , row_number() over (partition by patient_data_source_id, encounter_id
order by start_date, end_date, claim_id) as encounter_claim_number
  , row_number() over (partition by patient_data_source_id, encounter_id
order by start_date desc, end_date desc, claim_id desc) as encounter_claim_number_desc
  , close_flag
  , min_closing_row
  , encounter_id as anchor_claim_id
  , lower(md5(((case when 'inpatient substance use' is null then 'N' else ('V' || replace(replace(cast('inpatient substance use' as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when patient_data_source_id is null then 'N' else ('V' || replace(replace(cast(patient_data_source_id as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when encounter_id is null then 'N' else ('V' || replace(replace(cast(encounter_id as TEXT), '%', '%25'), '|', '%7C')) end)))) as encounter_id
from add_encounter_id
),  __dbt__cte__inpatient_substance_use__start_end_dates as (


select encounter_id
, min(anchor_claim_id) as anchor_claim_id
, min(start_date) as encounter_start_date
, max(end_date) as encounter_end_date
from __dbt__cte__inpatient_substance_use__generate_encounter_id
group by encounter_id
),  __dbt__cte__inpatient_substance_use__prof_claims as (


with first_claim as (

    select
          encounter_id
        , patient_data_source_id
    from __dbt__cte__inpatient_substance_use__generate_encounter_id
    where encounter_claim_number = 1

)

, join_first_claim_dates as (

    select
          f.encounter_id
        , f.patient_data_source_id
        , dat.anchor_claim_id
        , dat.encounter_end_date
        , dat.encounter_start_date
    from first_claim as f
    inner join __dbt__cte__inpatient_substance_use__start_end_dates as dat
        on f.encounter_id = dat.encounter_id

)

-- ensuring each prof claim is only attributed to one institutional claim with claim_attribution_number
select
      dat.encounter_id
    , dat.encounter_start_date
    , dat.encounter_end_date
    , med.claim_id
    , med.claim_line_number
    , med.data_source
    , row_number() over (
        partition by med.claim_id, med.claim_line_number, med.data_source
        order by dat.anchor_claim_id
      ) as claim_attribution_number
from __dbt__cte__encounters__stg_medical_claim as med
inner join __dbt__cte__encounters__prof_and_lower_priority as plp
    on med.claim_id = plp.claim_id
    and med.claim_line_number = plp.claim_line_number
    and med.data_source = plp.data_source
inner join join_first_claim_dates as dat
    on med.patient_data_source_id = dat.patient_data_source_id
    and med.start_date between dat.encounter_start_date and dat.encounter_end_date
),  __dbt__cte__office_visits__int_office_visits as (


with anchor as (
    select distinct
        mc.patient_data_source_id
      , mc.data_source
      , mc.start_date
      , mc.claim_id
      , mc.claim_line_number
      , mc.service_category_1
      , mc.service_category_2
      , mc.service_category_3
    from __dbt__cte__encounters__stg_medical_claim as mc
    inner join "claims_preprocessing"."_int_combined_professional" as p -- joining in all sc regardless of final priority
      on mc.claim_id = p.claim_id
      and mc.claim_line_number = p.claim_line_number
      and mc.data_source = p.data_source
    where p.service_category_1 = 'office-based'
)

select
    patient_data_source_id
  , data_source
  , start_date
  , claim_id
  , claim_line_number
  , service_category_1
  , service_category_2
  , service_category_3
  , lower(md5(((case when 'office based' is null then 'N' else ('V' || replace(replace(cast('office based' as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when patient_data_source_id is null then 'N' else ('V' || replace(replace(cast(patient_data_source_id as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when start_date is null then 'N' else ('V' || replace(replace(cast(start_date as TEXT), '%', '%25'), '|', '%7C')) end)))) as old_encounter_id
from anchor
),  __dbt__cte__office_visits__int_office_visits_radiology as (


select distinct
    ov.patient_data_source_id
    , ov.data_source
    , ov.start_date
    , ov.claim_id
    , ov.claim_line_number
    , mc.hcpcs_code
    , lower(md5(((case when 'office visit radiology' is null then 'N' else ('V' || replace(replace(cast('office visit radiology' as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when ov.patient_data_source_id is null then 'N' else ('V' || replace(replace(cast(ov.patient_data_source_id as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when ov.start_date is null then 'N' else ('V' || replace(replace(cast(ov.start_date as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when mc.hcpcs_code is null then 'N' else ('V' || replace(replace(cast(mc.hcpcs_code as TEXT), '%', '%25'), '|', '%7C')) end)))) as old_encounter_id
from __dbt__cte__office_visits__int_office_visits as ov
inner join __dbt__cte__encounters__stg_medical_claim as mc on mc.claim_id = ov.claim_id
    and mc.claim_line_number = ov.claim_line_number
    and mc.data_source = ov.data_source
inner join "claims_preprocessing"."service_category__office_based_radiology" as scrad on mc.claim_id = scrad.claim_id
    and mc.claim_line_number = scrad.claim_line_number
    and mc.data_source = scrad.data_source
),  __dbt__cte__office_visits__int_office_visits_surgery as (


select distinct
    ov.patient_data_source_id
    , ov.data_source
    , ov.start_date
    , ov.claim_id
    , ov.claim_line_number
    , ov.old_encounter_id
from __dbt__cte__office_visits__int_office_visits as ov
where service_category_2 = 'office-based surgery'
),  __dbt__cte__office_visits__int_office_visits_injections as (


select distinct
    ov.patient_data_source_id
    , ov.data_source
    , ov.start_date
    , ov.claim_id
    , ov.claim_line_number
    , ov.old_encounter_id
from __dbt__cte__office_visits__int_office_visits as ov
inner join __dbt__cte__encounters__stg_medical_claim as mc on mc.claim_id = ov.claim_id
    and mc.claim_line_number = ov.claim_line_number
    and mc.data_source = ov.data_source
where substring(hcpcs_code, 1, 1) = 'J'
),  __dbt__cte__office_visits__int_office_visits_ptotst as (


select distinct
    ov.patient_data_source_id
    , ov.data_source
    , ov.start_date
    , ov.claim_id
    , ov.claim_line_number
    , ov.old_encounter_id
from __dbt__cte__office_visits__int_office_visits as ov
where service_category_2 = 'office-based pt/ot/st'
),  __dbt__cte__office_visits__int_office_visits_em as (


select distinct
    ov.patient_data_source_id
    , ov.data_source
    , ov.start_date
    , ov.claim_id
    , ov.claim_line_number
    , ov.old_encounter_id
from __dbt__cte__office_visits__int_office_visits as ov
where service_category_2 = 'office-based visit'
),  __dbt__cte__office_visits__int_office_visits_telehealth as (


select distinct
    ov.patient_data_source_id
    , ov.data_source
    , ov.start_date
    , ov.claim_id
    , ov.claim_line_number
    , ov.old_encounter_id
from __dbt__cte__office_visits__int_office_visits as ov
where service_category_2 = 'telehealth visit'
),  __dbt__cte__office_visits__int_office_visits_union as (



select claim_id
, claim_line_number
, data_source
, old_encounter_id
, 'office visit radiology' as encounter_type
, 0 as priority_number
from __dbt__cte__office_visits__int_office_visits_radiology

union distinct

select claim_id
, claim_line_number
, data_source
, old_encounter_id
, 'office visit surgery' as encounter_type
, 1 as priority_number
from __dbt__cte__office_visits__int_office_visits_surgery

union distinct

select claim_id
, claim_line_number
, data_source
, old_encounter_id
, 'office visit injections' as encounter_type
, 2 as priority_number
from __dbt__cte__office_visits__int_office_visits_injections

union distinct

select claim_id
, claim_line_number
, data_source
, old_encounter_id
, 'office visit pt/ot/st' as encounter_type
, 3 as priority_number
from __dbt__cte__office_visits__int_office_visits_ptotst

union distinct

select claim_id
, claim_line_number
, data_source
, old_encounter_id
, 'office visit' as encounter_type
, 4 as priority_number
from __dbt__cte__office_visits__int_office_visits_em

union distinct

select claim_id
, claim_line_number
, data_source
, old_encounter_id
, 'telehealth' as encounter_type
, 5 as priority_number
from __dbt__cte__office_visits__int_office_visits_telehealth

union distinct

select claim_id
, claim_line_number
, data_source
, old_encounter_id
, 'office visit - other' as encounter_type
, 9999 as priority_number
from __dbt__cte__office_visits__int_office_visits
),  __dbt__cte__office_visits__int_office_visits_encounter_ranking as (


with rank_cte as (
select *
from __dbt__cte__office_visits__int_office_visits_union
)

, dist_encounter as (
select distinct old_encounter_id
, encounter_type
, priority_number
from rank_cte
)

select
old_encounter_id
, encounter_type
, priority_number
, row_number() over (partition by old_encounter_id
order by priority_number) as relative_rank
from dist_encounter
),  __dbt__cte__office_visits__int_office_visits_claim_line as (


with rank_cte as (
select *
from __dbt__cte__office_visits__int_office_visits_union
)

, crosswalk_cte as (
select old_encounter_id
, encounter_type
from __dbt__cte__office_visits__int_office_visits_encounter_ranking
where relative_rank = 1
)

select r.claim_id
, r.claim_line_number
, r.data_source
, r.old_encounter_id
, x.encounter_type
from rank_cte as r
inner join crosswalk_cte as x on r.old_encounter_id = x.old_encounter_id
),  __dbt__cte__urgent_care__anchor_events as (


  select distinct
      claim_id
    , data_source
  from __dbt__cte__encounters__stg_medical_claim
  where
    service_category_2 in ('urgent care') --both inst and prof anchor
),  __dbt__cte__urgent_care__generate_encounter_id as (


with anchor as (
select distinct m.patient_data_source_id
 , m.data_source
 , m.start_date
 , m.claim_id
from __dbt__cte__encounters__stg_medical_claim as m
inner join __dbt__cte__urgent_care__anchor_events as u on m.claim_id = u.claim_id
and
m.data_source = u.data_source
)

select patient_data_source_id
, data_source
, start_date
, claim_id
, lower(md5(((case when 'urgent care' is null then 'N' else ('V' || replace(replace(cast('urgent care' as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when patient_data_source_id is null then 'N' else ('V' || replace(replace(cast(patient_data_source_id as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when start_date is null then 'N' else ('V' || replace(replace(cast(start_date as TEXT), '%', '%25'), '|', '%7C')) end)))) as old_encounter_id
from anchor
),  __dbt__cte__urgent_care__match_claims_to_anchor as (


select distinct m.patient_data_source_id
 , m.data_source
 , m.start_date
 , m.claim_id
 , m.claim_line_number
 , u.old_encounter_id
from __dbt__cte__encounters__stg_medical_claim as m
inner join __dbt__cte__urgent_care__generate_encounter_id as u on m.patient_data_source_id = u.patient_data_source_id
and
m.start_date = u.start_date
),  __dbt__cte__outpatient_psych__anchor_events as (


with service_category as (
  select distinct
      claim_id
    , data_source
    , patient_data_source_id
    , start_date
  from __dbt__cte__encounters__stg_medical_claim
  where
    service_category_2 = 'outpatient psychiatric' --both inst and prof as anchor
)

select distinct
claim_id
, data_source
, cast('2026-01-01 00:00:00' as timestamp) as tuva_last_run
from service_category
),  __dbt__cte__outpatient_psych__generate_encounter_id as (


with anchor as (
select distinct m.patient_data_source_id
 , m.data_source
 , m.start_date
 , m.claim_id
from __dbt__cte__encounters__stg_medical_claim as m
inner join __dbt__cte__outpatient_psych__anchor_events as u on m.claim_id = u.claim_id
and
m.data_source = u.data_source
)

select patient_data_source_id
, data_source
, start_date
, claim_id
, lower(md5(((case when 'outpatient psych' is null then 'N' else ('V' || replace(replace(cast('outpatient psych' as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when patient_data_source_id is null then 'N' else ('V' || replace(replace(cast(patient_data_source_id as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when start_date is null then 'N' else ('V' || replace(replace(cast(start_date as TEXT), '%', '%25'), '|', '%7C')) end)))) as old_encounter_id
from anchor
),  __dbt__cte__outpatient_psych__match_claims_to_anchor as (


select distinct m.patient_data_source_id
 , m.data_source
 , m.start_date
 , m.claim_id
 , m.claim_line_number
 , u.old_encounter_id
from __dbt__cte__encounters__stg_medical_claim as m
inner join __dbt__cte__outpatient_psych__generate_encounter_id as u on m.patient_data_source_id = u.patient_data_source_id
and
m.start_date = u.start_date
),  __dbt__cte__outpatient_rehab__anchor_events as (


with service_category as (
  select distinct
      claim_id
    , data_source
    , patient_data_source_id
    , start_date
  from __dbt__cte__encounters__stg_medical_claim
  where
    service_category_2 = 'outpatient rehabilitation'
)

select distinct
claim_id
, data_source
, cast('2026-01-01 00:00:00' as timestamp) as tuva_last_run
from service_category
),  __dbt__cte__outpatient_rehab__generate_encounter_id as (


with anchor as (
select distinct m.patient_data_source_id
 , m.data_source
 , m.start_date
 , m.claim_id
from __dbt__cte__encounters__stg_medical_claim as m
inner join __dbt__cte__outpatient_rehab__anchor_events as u on m.claim_id = u.claim_id
and
m.data_source = u.data_source
)

select patient_data_source_id
, data_source
, start_date
, claim_id
, lower(md5(((case when 'outpatient rehabilitation' is null then 'N' else ('V' || replace(replace(cast('outpatient rehabilitation' as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when patient_data_source_id is null then 'N' else ('V' || replace(replace(cast(patient_data_source_id as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when start_date is null then 'N' else ('V' || replace(replace(cast(start_date as TEXT), '%', '%25'), '|', '%7C')) end)))) as old_encounter_id
from anchor
),  __dbt__cte__outpatient_rehab__match_claims_to_anchor as (


select distinct m.patient_data_source_id
 , m.data_source
 , m.start_date
 , m.claim_id
 , m.claim_line_number
 , u.old_encounter_id
from __dbt__cte__encounters__stg_medical_claim as m
inner join __dbt__cte__outpatient_rehab__generate_encounter_id as u on m.patient_data_source_id = u.patient_data_source_id
and
m.start_date = u.start_date
),  __dbt__cte__asc__anchor_events as (


with service_category as (
  select distinct
      claim_id
    , data_source
    , patient_data_source_id
    , start_date
    , end_date
  from __dbt__cte__encounters__stg_medical_claim
  where
    service_category_2 = 'ambulatory surgery center' -- include both professional and institutional claims as anchor events

)

select distinct
claim_id
, data_source
, cast('2026-01-01 00:00:00' as timestamp) as tuva_last_run
from service_category
),  __dbt__cte__asc__generate_encounter_id as (


with base_data as (
    select distinct
        m.patient_data_source_id
      , m.data_source
      , m.start_date
      , m.end_date
      , m.claim_id
    from __dbt__cte__encounters__stg_medical_claim as m
    inner join __dbt__cte__asc__anchor_events as u
      on m.claim_id = u.claim_id
      and m.data_source = u.data_source
)

-- Determine Previous Maximum End Date
, grouped_data as (
    select
        bd.*
      , max(end_date) over (
            partition by patient_data_source_id
            order by start_date, claim_id
            rows between unbounded preceding and 1 preceding
        ) as previous_max_end_date
    from base_data as bd
)

-- Flag New Encounter Groups
, flagged_data as (
    select
        gd.*
      , case
            when start_date > coalesce(previous_max_end_date, 
    cast('1900-01-01' as date)
 ) then 1
            else 0
        end as new_group_flag
    from grouped_data as gd
)

-- Assign Encounter Groups per Patient
, numbered_data as (
    select
        fd.*
      , sum(new_group_flag) over (
            partition by patient_data_source_id
            order by start_date, claim_id
            rows unbounded preceding
        ) as encounter_group
    from flagged_data as fd
)

-- Identify Unique Encounters
, unique_encounters as (
    select
        patient_data_source_id
      , encounter_group
      , min(start_date) as encounter_start_date
    from numbered_data
    group by
        patient_data_source_id
      , encounter_group
)

-- Assign asc encounter_id
, numbered_encounters as (
    select
        patient_data_source_id
      , encounter_group
      , lower(md5(((case when 'ambulatory surgery center' is null then 'N' else ('V' || replace(replace(cast('ambulatory surgery center' as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when patient_data_source_id is null then 'N' else ('V' || replace(replace(cast(patient_data_source_id as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when encounter_start_date is null then 'N' else ('V' || replace(replace(cast(encounter_start_date as TEXT), '%', '%25'), '|', '%7C')) end)))) as encounter_id
    from unique_encounters
)

-- Merge Encounters with Claims
select
    nd.patient_data_source_id
  , nd.data_source
  , nd.start_date
  , nd.end_date
  , nd.claim_id
  , ne.encounter_id as old_encounter_id
from numbered_data as nd
inner join numbered_encounters as ne
  on nd.patient_data_source_id = ne.patient_data_source_id
  and nd.encounter_group = ne.encounter_group
),  __dbt__cte__asc__start_end_dates as (


select
    patient_data_source_id
  , old_encounter_id
  , min(start_date) as encounter_start_date
  , max(end_date) as encounter_end_date
from __dbt__cte__asc__generate_encounter_id
group by
    patient_data_source_id
  , old_encounter_id
),  __dbt__cte__asc__match_claims_to_anchor as (


select
    dat.old_encounter_id
  , dat.encounter_start_date
  , dat.encounter_end_date
  , med.claim_id
  , med.claim_line_number
  , med.data_source
  , row_number() over (
        partition by med.claim_id, med.claim_line_number, med.data_source
        order by dat.encounter_start_date, dat.old_encounter_id
    ) as claim_attribution_number
from __dbt__cte__encounters__stg_medical_claim as med
inner join __dbt__cte__asc__start_end_dates as dat
  on med.patient_data_source_id = dat.patient_data_source_id
  and med.start_date between dat.encounter_start_date and dat.encounter_end_date
),  __dbt__cte__dialysis__anchor_events as (


with service_category as (
  select distinct
      claim_id
    , data_source
    , patient_data_source_id
    , start_date
  from __dbt__cte__encounters__stg_medical_claim
  where
    service_category_2 = 'dialysis' --both inst and professional as anchor

)

select distinct
claim_id
, data_source
, cast('2026-01-01 00:00:00' as timestamp) as tuva_last_run
from service_category
),  __dbt__cte__dialysis__generate_encounter_id as (


with anchor as (
select distinct m.patient_data_source_id
 , m.data_source
 , m.start_date
 , m.claim_id
from __dbt__cte__encounters__stg_medical_claim as m
inner join __dbt__cte__dialysis__anchor_events as u on m.claim_id = u.claim_id
and
m.data_source = u.data_source
)

select patient_data_source_id
, data_source
, start_date
, claim_id
, lower(md5(((case when 'dialysis' is null then 'N' else ('V' || replace(replace(cast('dialysis' as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when patient_data_source_id is null then 'N' else ('V' || replace(replace(cast(patient_data_source_id as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when start_date is null then 'N' else ('V' || replace(replace(cast(start_date as TEXT), '%', '%25'), '|', '%7C')) end)))) as old_encounter_id
from anchor
),  __dbt__cte__dialysis__match_claims_to_anchor as (


select distinct m.patient_data_source_id
 , m.data_source
 , m.start_date
 , m.claim_id
 , m.claim_line_number
 , u.old_encounter_id
from __dbt__cte__encounters__stg_medical_claim as m
inner join __dbt__cte__dialysis__generate_encounter_id as u on m.patient_data_source_id = u.patient_data_source_id
and
m.start_date = u.start_date
),  __dbt__cte__outpatient_hospice__anchor_events as (


with service_category as (
  select distinct
      claim_id
    , data_source
    , patient_data_source_id
    , start_date
  from __dbt__cte__encounters__stg_medical_claim
  where
    service_category_2 = 'outpatient hospice' --both inst and prof

)

select distinct
claim_id
, data_source
, cast('2026-01-01 00:00:00' as timestamp) as tuva_last_run
from service_category
),  __dbt__cte__outpatient_hospice__generate_encounter_id as (


with anchor as (
select distinct m.patient_data_source_id
 , m.data_source
 , m.start_date
 , m.claim_id
from __dbt__cte__encounters__stg_medical_claim as m
inner join __dbt__cte__outpatient_hospice__anchor_events as u on m.claim_id = u.claim_id
and
m.data_source = u.data_source
)

select patient_data_source_id
, data_source
, start_date
, claim_id
, lower(md5(((case when 'outpatient hospice' is null then 'N' else ('V' || replace(replace(cast('outpatient hospice' as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when patient_data_source_id is null then 'N' else ('V' || replace(replace(cast(patient_data_source_id as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when start_date is null then 'N' else ('V' || replace(replace(cast(start_date as TEXT), '%', '%25'), '|', '%7C')) end)))) as old_encounter_id
from anchor
),  __dbt__cte__outpatient_hospice__match_claims_to_anchor as (


select distinct m.patient_data_source_id
 , m.data_source
 , m.start_date
 , m.claim_id
 , m.claim_line_number
 , u.old_encounter_id
from __dbt__cte__encounters__stg_medical_claim as m
inner join __dbt__cte__outpatient_hospice__generate_encounter_id as u on m.patient_data_source_id = u.patient_data_source_id
and
m.start_date = u.start_date
),  __dbt__cte__home_health__anchor_events as (


with service_category as (
  select distinct
      claim_id
    , data_source
    , patient_data_source_id
    , start_date
  from __dbt__cte__encounters__stg_medical_claim
  where
    service_category_2 = 'home health' -- both prof and inst as anchors

)

select distinct
claim_id
, data_source
, cast('2026-01-01 00:00:00' as timestamp) as tuva_last_run
from service_category
),  __dbt__cte__home_health__generate_encounter_id as (


with anchor as (
select distinct m.patient_data_source_id
 , m.data_source
 , m.start_date
 , m.claim_id
from __dbt__cte__encounters__stg_medical_claim as m
inner join __dbt__cte__home_health__anchor_events as u on m.claim_id = u.claim_id
and
m.data_source = u.data_source
)

select patient_data_source_id
, data_source
, start_date
, claim_id
, lower(md5(((case when 'home health' is null then 'N' else ('V' || replace(replace(cast('home health' as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when patient_data_source_id is null then 'N' else ('V' || replace(replace(cast(patient_data_source_id as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when start_date is null then 'N' else ('V' || replace(replace(cast(start_date as TEXT), '%', '%25'), '|', '%7C')) end)))) as old_encounter_id
from anchor
),  __dbt__cte__home_health__match_claims_to_anchor as (


select distinct m.patient_data_source_id
 , m.data_source
 , m.start_date
 , m.claim_id
 , m.claim_line_number
 , u.old_encounter_id
from __dbt__cte__encounters__stg_medical_claim as m
inner join __dbt__cte__home_health__generate_encounter_id as u on m.patient_data_source_id = u.patient_data_source_id
and
m.start_date = u.start_date
),  __dbt__cte__outpatient_surgery__anchor_events as (


with service_category as (
  select distinct
      claim_id
    , data_source
    , patient_data_source_id
    , start_date
  from __dbt__cte__encounters__stg_medical_claim
  where
    service_category_2 in ('outpatient surgery') --both inst and prof anchor
)

select distinct
claim_id
, data_source
, cast('2026-01-01 00:00:00' as timestamp) as tuva_last_run
from service_category
),  __dbt__cte__outpatient_surgery__generate_encounter_id as (

with anchor as (
select distinct m.patient_data_source_id
 , m.data_source
 , m.start_date
 , m.claim_id
from __dbt__cte__encounters__stg_medical_claim as m
inner join __dbt__cte__outpatient_surgery__anchor_events as u on m.claim_id = u.claim_id
and
m.data_source = u.data_source
)

select patient_data_source_id
, data_source
, start_date
, claim_id
, lower(md5(((case when 'outpatient surgery' is null then 'N' else ('V' || replace(replace(cast('outpatient surgery' as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when patient_data_source_id is null then 'N' else ('V' || replace(replace(cast(patient_data_source_id as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when start_date is null then 'N' else ('V' || replace(replace(cast(start_date as TEXT), '%', '%25'), '|', '%7C')) end)))) as old_encounter_id
from anchor
),  __dbt__cte__outpatient_surgery__match_claims_to_anchor as (


select distinct m.patient_data_source_id
 , m.data_source
 , m.start_date
 , m.claim_id
 , m.claim_line_number
 , u.old_encounter_id
from __dbt__cte__encounters__stg_medical_claim as m
inner join __dbt__cte__outpatient_surgery__generate_encounter_id as u on m.patient_data_source_id = u.patient_data_source_id
and
m.start_date = u.start_date
),  __dbt__cte__outpatient_injections__anchor_events as (


with multiple_sources as (
select distinct
    med.patient_data_source_id
    , med.data_source
    , med.start_date
from __dbt__cte__encounters__stg_medical_claim as med
inner join __dbt__cte__encounters__stg_outpatient_institutional as outpatient
    on med.claim_id = outpatient.claim_id
    and med.data_source = outpatient.data_source
where substring(med.hcpcs_code, 1, 1) = 'J'
)


select distinct
    patient_data_source_id
    , data_source
    , start_date
, cast('2026-01-01 00:00:00' as timestamp) as tuva_last_run
from multiple_sources
),  __dbt__cte__outpatient_injections__generate_encounter_id as (


select patient_data_source_id
, data_source
, start_date
, lower(md5(((case when 'outpatient injections' is null then 'N' else ('V' || replace(replace(cast('outpatient injections' as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when patient_data_source_id is null then 'N' else ('V' || replace(replace(cast(patient_data_source_id as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when start_date is null then 'N' else ('V' || replace(replace(cast(start_date as TEXT), '%', '%25'), '|', '%7C')) end)))) as old_encounter_id
from __dbt__cte__outpatient_injections__anchor_events
),  __dbt__cte__outpatient_injections__match_claims_to_anchor as (


select distinct m.patient_data_source_id
 , m.data_source
 , m.start_date
 , m.claim_id
 , m.claim_line_number
 , u.old_encounter_id
from __dbt__cte__encounters__stg_medical_claim as m
inner join __dbt__cte__outpatient_injections__generate_encounter_id as u on m.patient_data_source_id = u.patient_data_source_id
and
m.start_date = u.start_date
),  __dbt__cte__outpatient_ptotst__anchor_events as (


with service_category as (
  select distinct
      claim_id
    , data_source
    , patient_data_source_id
    , start_date
  from __dbt__cte__encounters__stg_medical_claim
  where
    service_category_2 = 'outpatient pt/ot/st' --both inst and prof as anchor
)

select distinct
claim_id
, data_source
, cast('2026-01-01 00:00:00' as timestamp) as tuva_last_run
from service_category
),  __dbt__cte__outpatient_ptotst__generate_encounter_id as (


with anchor as (
select distinct m.patient_data_source_id
 , m.data_source
 , m.start_date
 , m.claim_id
from __dbt__cte__encounters__stg_medical_claim as m
inner join __dbt__cte__outpatient_ptotst__anchor_events as u on m.claim_id = u.claim_id
and
m.data_source = u.data_source
)

select patient_data_source_id
, data_source
, start_date
, claim_id
, lower(md5(((case when 'outpatient pt/ot/st' is null then 'N' else ('V' || replace(replace(cast('outpatient pt/ot/st' as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when patient_data_source_id is null then 'N' else ('V' || replace(replace(cast(patient_data_source_id as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when start_date is null then 'N' else ('V' || replace(replace(cast(start_date as TEXT), '%', '%25'), '|', '%7C')) end)))) as old_encounter_id
from anchor
),  __dbt__cte__outpatient_ptotst__match_claims_to_anchor as (


select distinct m.patient_data_source_id
 , m.data_source
 , m.start_date
 , m.claim_id
 , m.claim_line_number
 , u.old_encounter_id
from __dbt__cte__encounters__stg_medical_claim as m
inner join __dbt__cte__outpatient_ptotst__generate_encounter_id as u on m.patient_data_source_id = u.patient_data_source_id
and
m.start_date = u.start_date
),  __dbt__cte__outpatient_substance_use__anchor_events as (


with service_category as (
  select distinct
      claim_id
    , data_source
    , patient_data_source_id
    , start_date
  from __dbt__cte__encounters__stg_medical_claim
  where
    service_category_2 = 'outpatient substance use'
    and claim_type = 'institutional'
)

select distinct
claim_id
, data_source
, cast('2026-01-01 00:00:00' as timestamp) as tuva_last_run
from service_category
),  __dbt__cte__outpatient_substance_use__generate_encounter_id as (

with anchor as (
select distinct m.patient_data_source_id
 , m.data_source
 , m.start_date
 , m.claim_id
from __dbt__cte__encounters__stg_medical_claim as m
inner join __dbt__cte__outpatient_substance_use__anchor_events as u on m.claim_id = u.claim_id
and
m.data_source = u.data_source
)

select patient_data_source_id
, data_source
, start_date
, claim_id
, lower(md5(((case when 'outpatient substance use' is null then 'N' else ('V' || replace(replace(cast('outpatient substance use' as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when patient_data_source_id is null then 'N' else ('V' || replace(replace(cast(patient_data_source_id as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when start_date is null then 'N' else ('V' || replace(replace(cast(start_date as TEXT), '%', '%25'), '|', '%7C')) end)))) as old_encounter_id
from anchor
),  __dbt__cte__outpatient_substance_use__match_claims_to_anchor as (


select distinct m.patient_data_source_id
 , m.data_source
 , m.start_date
 , m.claim_id
 , m.claim_line_number
 , u.old_encounter_id
from __dbt__cte__encounters__stg_medical_claim as m
inner join __dbt__cte__outpatient_substance_use__generate_encounter_id as u on m.patient_data_source_id = u.patient_data_source_id
and
m.start_date = u.start_date
),  __dbt__cte__outpatient_radiology__anchor_events as (


with service_category as (
  select distinct
    patient_data_source_id
    , data_source
    , start_date
    , hcpcs_code
  from __dbt__cte__encounters__stg_medical_claim
  where
    service_category_2 = 'outpatient radiology' --both professional and inst
)

select distinct
    patient_data_source_id
    , data_source
    , start_date
    , hcpcs_code
, cast('2026-01-01 00:00:00' as timestamp) as tuva_last_run
from service_category
),  __dbt__cte__outpatient_radiology__generate_encounter_id as (


select patient_data_source_id
, data_source
, start_date
, hcpcs_code
, lower(md5(((case when 'outpatient radiology' is null then 'N' else ('V' || replace(replace(cast('outpatient radiology' as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when patient_data_source_id is null then 'N' else ('V' || replace(replace(cast(patient_data_source_id as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when start_date is null then 'N' else ('V' || replace(replace(cast(start_date as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when hcpcs_code is null then 'N' else ('V' || replace(replace(cast(hcpcs_code as TEXT), '%', '%25'), '|', '%7C')) end)))) as old_encounter_id
from __dbt__cte__outpatient_radiology__anchor_events
),  __dbt__cte__outpatient_radiology__match_claims_to_anchor as (


select distinct m.patient_data_source_id
 , m.data_source
 , m.start_date
 , m.claim_id
 , m.claim_line_number
 , m.hcpcs_code
 , u.old_encounter_id
from __dbt__cte__encounters__stg_medical_claim as m
inner join __dbt__cte__outpatient_radiology__generate_encounter_id as u on m.patient_data_source_id = u.patient_data_source_id
and
m.start_date = u.start_date
and
m.hcpcs_code = u.hcpcs_code
),  __dbt__cte__outpatient_hospital_or_clinic__anchor_events as (


with service_category as (
    select distinct
        claim_id
      , data_source
      , patient_data_source_id
      , start_date
    from __dbt__cte__encounters__stg_medical_claim
    where
      service_category_2 in (
          'outpatient hospital or clinic' -- both prof and inst
        , 'observation' -- orphaned obs that didn't roll up to other encounter
      )
)

select distinct
    claim_id
  , data_source
  , cast('2026-01-01 00:00:00' as timestamp) as tuva_last_run
from service_category
),  __dbt__cte__outpatient_hospital_or_clinic__generate_encounter_id as (


with anchor as (
select distinct m.patient_data_source_id
 , m.data_source
 , m.start_date
 , m.claim_id
from __dbt__cte__encounters__stg_medical_claim as m
inner join __dbt__cte__outpatient_hospital_or_clinic__anchor_events as u on m.claim_id = u.claim_id
and
m.data_source = u.data_source
)

select patient_data_source_id
, data_source
, start_date
, claim_id
, lower(md5(((case when 'outpatient hospital or clinic' is null then 'N' else ('V' || replace(replace(cast('outpatient hospital or clinic' as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when patient_data_source_id is null then 'N' else ('V' || replace(replace(cast(patient_data_source_id as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when start_date is null then 'N' else ('V' || replace(replace(cast(start_date as TEXT), '%', '%25'), '|', '%7C')) end)))) as old_encounter_id
from anchor
),  __dbt__cte__outpatient_hospital_or_clinic__match_claims_to_anchor as (


select distinct m.patient_data_source_id
 , m.data_source
 , m.start_date
 , m.claim_id
 , m.claim_line_number
 , u.old_encounter_id
from __dbt__cte__encounters__stg_medical_claim as m
inner join __dbt__cte__outpatient_hospital_or_clinic__generate_encounter_id as u on m.patient_data_source_id = u.patient_data_source_id
and
m.start_date = u.start_date
),  __dbt__cte__encounters__int_institutional_claim_lines as (


with unioned as (
    select enc.claim_id
, enc.patient_data_source_id
, enc.encounter_id
, 'acute inpatient' as encounter_type
, 'inpatient' as encounter_group
, 0 as priority_number
, null as anchor_claim_id
from __dbt__cte__acute_inpatient__generate_encounter_id as enc


union all

select enc.claim_id
, enc.patient_data_source_id
, enc.encounter_id
, 'emergency department' as encounter_type
, 'outpatient' as encounter_group
, 1 as priority_number
, original_anchor_claim as anchor_claim_id
from __dbt__cte__emergency_department__generate_encounter_id as enc


union all

select enc.claim_id
, enc.patient_data_source_id
, enc.encounter_id
, 'inpatient hospice' as encounter_type
, 'inpatient' as encounter_group
, 1 as priority_number
, null as anchor_claim_id
from __dbt__cte__inpatient_hospice__generate_encounter_id as enc


union all

select enc.claim_id
, enc.patient_data_source_id
, enc.encounter_id
, 'inpatient psych' as encounter_type
, 'inpatient' as encounter_group
, 2 as priority_number
, null as anchor_claim_id
from __dbt__cte__inpatient_psych__generate_encounter_id as enc


union all

select enc.claim_id
, enc.patient_data_source_id
, enc.encounter_id
, 'inpatient rehabilitation' as encounter_type
, 'inpatient' as encounter_group
, 3 as priority_number
, null as anchor_claim_id
from __dbt__cte__inpatient_rehab__generate_encounter_id as enc


union all

select enc.claim_id
, enc.patient_data_source_id
, enc.encounter_id
, 'inpatient long term acute care' as encounter_type
, 'inpatient' as encounter_group
, 4 as priority_number
, null as anchor_claim_id
from __dbt__cte__inpatient_long_term__generate_encounter_id as enc



union all

select enc.claim_id
, enc.patient_data_source_id
, enc.encounter_id
, 'inpatient skilled nursing' as encounter_type
, 'inpatient' as encounter_group
, 5 as priority_number
, null as anchor_claim_id
from __dbt__cte__inpatient_snf__generate_encounter_id as enc


union all

select enc.claim_id
, enc.patient_data_source_id
, enc.encounter_id
, 'inpatient substance use' as encounter_type
, 'inpatient' as encounter_group
, 6 as priority_number
, null as anchor_claim_id
from __dbt__cte__inpatient_substance_use__generate_encounter_id as enc

)

, final as (
    select
        enc.claim_id
        , med.claim_line_number
        , med.data_source
        , enc.encounter_id
        , encounter_type
        , encounter_group
        , priority_number
        , anchor_claim_id
    from unioned as enc
    inner join __dbt__cte__encounters__stg_medical_claim as med on enc.claim_id = med.claim_id
        and enc.patient_data_source_id = med.patient_data_source_id
)

select * from final
),  __dbt__cte__lab__anchor_events as (


with service_category as (
  select distinct
      claim_id
    , data_source
    , patient_data_source_id
    , start_date
  from __dbt__cte__encounters__stg_medical_claim
  where
    service_category_2 = 'lab' --both inst and prof

)

select distinct
claim_id
, data_source
, cast('2026-01-01 00:00:00' as timestamp) as tuva_last_run
from service_category
),  __dbt__cte__lab__generate_encounter_id as (


with anchor as (
select distinct m.patient_data_source_id
 , m.data_source
 , m.start_date
 , m.claim_id
from __dbt__cte__encounters__stg_medical_claim as m
inner join __dbt__cte__lab__anchor_events as u on m.claim_id = u.claim_id
and
m.data_source = u.data_source
)

select patient_data_source_id
, data_source
, start_date
, claim_id
, lower(md5(((case when 'lab' is null then 'N' else ('V' || replace(replace(cast('lab' as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when patient_data_source_id is null then 'N' else ('V' || replace(replace(cast(patient_data_source_id as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when start_date is null then 'N' else ('V' || replace(replace(cast(start_date as TEXT), '%', '%25'), '|', '%7C')) end)))) as old_encounter_id
from anchor
),  __dbt__cte__lab__match_claims_to_anchor as (


select distinct m.patient_data_source_id
 , m.data_source
 , m.start_date
 , m.claim_id
 , m.claim_line_number
 , u.old_encounter_id
from __dbt__cte__encounters__stg_medical_claim as m
inner join __dbt__cte__lab__generate_encounter_id as u on m.patient_data_source_id = u.patient_data_source_id
and
m.start_date = u.start_date
where m.service_category_2 = 'lab'
),  __dbt__cte__dme__anchor_events as (


with service_category as (
  select distinct
      claim_id
    , data_source
    , patient_data_source_id
    , start_date
  from __dbt__cte__encounters__stg_medical_claim
  where
    service_category_2 = 'durable medical equipment' --both inst and prof

)

select distinct
claim_id
, data_source
, cast('2026-01-01 00:00:00' as timestamp) as tuva_last_run
from service_category
),  __dbt__cte__dme__generate_encounter_id as (


with anchor as (
select distinct m.patient_data_source_id
 , m.data_source
 , m.start_date
 , m.claim_id
from __dbt__cte__encounters__stg_medical_claim as m
inner join __dbt__cte__dme__anchor_events as u on m.claim_id = u.claim_id
and
m.data_source = u.data_source
)

select patient_data_source_id
, data_source
, start_date
, claim_id
, lower(md5(((case when 'dme' is null then 'N' else ('V' || replace(replace(cast('dme' as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when patient_data_source_id is null then 'N' else ('V' || replace(replace(cast(patient_data_source_id as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when start_date is null then 'N' else ('V' || replace(replace(cast(start_date as TEXT), '%', '%25'), '|', '%7C')) end)))) as old_encounter_id
from anchor
),  __dbt__cte__dme__match_claims_to_anchor as (


select distinct m.patient_data_source_id
 , m.data_source
 , m.start_date
 , m.claim_id
 , m.claim_line_number
 , u.old_encounter_id
from __dbt__cte__encounters__stg_medical_claim as m
inner join __dbt__cte__dme__generate_encounter_id as u on m.patient_data_source_id = u.patient_data_source_id
and
m.start_date = u.start_date
where m.service_category_2 = 'durable medical equipment'
),  __dbt__cte__ambulance__anchor_events as (


with service_category as (
  select distinct
      claim_id
    , data_source
    , patient_data_source_id
    , start_date
  from __dbt__cte__encounters__stg_medical_claim
  where
    service_category_2 = 'ambulance' --both inst and prof

)

select distinct
claim_id
, data_source
, cast('2026-01-01 00:00:00' as timestamp) as tuva_last_run
from service_category
),  __dbt__cte__ambulance__generate_encounter_id as (


with anchor as (
select distinct m.patient_data_source_id
 , m.data_source
 , m.start_date
 , m.claim_id
from __dbt__cte__encounters__stg_medical_claim as m
inner join __dbt__cte__ambulance__anchor_events as u on m.claim_id = u.claim_id
and
m.data_source = u.data_source
)

select patient_data_source_id
, data_source
, start_date
, claim_id
, lower(md5(((case when 'ambulance' is null then 'N' else ('V' || replace(replace(cast('ambulance' as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when patient_data_source_id is null then 'N' else ('V' || replace(replace(cast(patient_data_source_id as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when start_date is null then 'N' else ('V' || replace(replace(cast(start_date as TEXT), '%', '%25'), '|', '%7C')) end)))) as old_encounter_id
from anchor
),  __dbt__cte__ambulance__match_claims_to_anchor as (


select distinct m.patient_data_source_id
 , m.data_source
 , m.start_date
 , m.claim_id
 , m.claim_line_number
 , u.old_encounter_id
from __dbt__cte__encounters__stg_medical_claim as m
inner join __dbt__cte__ambulance__generate_encounter_id as u on m.patient_data_source_id = u.patient_data_source_id
and
m.start_date = u.start_date
where m.service_category_2 = 'ambulance'
),  __dbt__cte__encounters__combined_claim_line_crosswalk as (
/* This model unions together claim lines to encounter crosswalk, and assigns them a unqiue encounter type if claims were assigned to multiple encounters
(This can happen a few ways - professional claims assigned to anchor event overlap and assigned to multiple  )
*/


with undetermined_claim_lines as (

select distinct
      claim_id
    , claim_line_number
    , data_source
    , 1 as is_undetermined
from __dbt__cte__encounters__stg_medical_claim
where claim_type = 'undetermined'

)

, cte as (
select claim_id
 , claim_line_number
 , data_source
 , encounter_id
 , 'acute inpatient' as encounter_type
 , 'inpatient' as encounter_group
 , 0 as priority_number
, null as anchor_claim_id
from __dbt__cte__acute_inpatient__prof_claims
where claim_attribution_number = 1

union all

/* Intentionally bringing in professional claims assigned to inpatient stays in case admit is assigned to ED  */
select claim_id
 , claim_line_number
 , data_source
 , encounter_id
 , 'emergency department' as encounter_type
 , 'outpatient' as encounter_group
 , 1 as priority_number
, null as anchor_claim_id
from __dbt__cte__acute_inpatient__prof_claims
where claim_attribution_number = 1

union all

select claim_id
 , claim_line_number
 , data_source
 , encounter_id
 , 'emergency department' as encounter_type
 , 'outpatient' as encounter_group
 , 1 as priority_number
, null as anchor_claim_id
from __dbt__cte__emergency_department__prof_claims
where claim_attribution_number = 1

union all

select claim_id
, claim_line_number
, data_source
, encounter_id
, 'inpatient psych' as encounter_type
, 'inpatient' as encounter_group
, 2 as priority_number
, null as anchor_claim_id
from __dbt__cte__inpatient_psych__prof_claims
where claim_attribution_number = 1

union all

select claim_id
, claim_line_number
, data_source
, encounter_id
, 'inpatient rehabilitation' as encounter_type
, 'inpatient' as encounter_group
, 3 as priority_number
, null as anchor_claim_id
from __dbt__cte__inpatient_rehab__prof_claims
where claim_attribution_number = 1

union all

select claim_id
, claim_line_number
, data_source
, encounter_id
, 'inpatient long term acute care' as encounter_type
, 'inpatient' as encounter_group
, 4 as priority_number
, null as anchor_claim_id
from __dbt__cte__inpatient_long_term__prof_claims
where claim_attribution_number = 1

union all

select claim_id
, claim_line_number
, data_source
, encounter_id
, 'inpatient skilled nursing' as encounter_type
, 'inpatient' as encounter_group
, 5 as priority_number
, null as anchor_claim_id
from __dbt__cte__inpatient_snf__prof_claims
where claim_attribution_number = 1

union all

/* Keep hospice below the existing inpatient priorities but above every
   office-based and outpatient candidate. Priority 7 is intentionally shared
   with office visit radiology so existing published priority values do not
   change; the final row_number tie-break puts inpatient hospice first. */
select claim_id
, claim_line_number
, data_source
, encounter_id
, 'inpatient hospice' as encounter_type
, 'inpatient' as encounter_group
, 7 as priority_number
, null as anchor_claim_id
from __dbt__cte__inpatient_hospice__prof_claims
where claim_attribution_number = 1

union all

select claim_id
, claim_line_number
, data_source
, encounter_id
, 'inpatient substance use' as encounter_type
, 'inpatient' as encounter_group
, 6 as priority_number
, null as anchor_claim_id
from __dbt__cte__inpatient_substance_use__prof_claims
where claim_attribution_number = 1

union all

/* Priority of sub office based types from office based group are set within office_visits__int_office_visits_union model */
select claim_id
, claim_line_number
, data_source
, old_encounter_id
, encounter_type
, 'office based' as encounter_group
, 7 as priority_number
, null as anchor_claim_id
from __dbt__cte__office_visits__int_office_visits_claim_line
where encounter_type = 'office visit radiology'

union all

select claim_id
, claim_line_number
, data_source
, old_encounter_id
, encounter_type
, 'office based' as encounter_group
, 8 as priority_number
, null as anchor_claim_id
from __dbt__cte__office_visits__int_office_visits_claim_line
where encounter_type <> 'office visit radiology'

union all

/* urgent care set at lower priority than ed and inpatient to avoid over flagging urgent care due to variations in billing practices */
select claim_id
, claim_line_number
, data_source
, old_encounter_id
, 'urgent care' as encounter_type
, 'outpatient' as encounter_group
, 9 as priority_number
, null as anchor_claim_id
from __dbt__cte__urgent_care__match_claims_to_anchor

union all

select claim_id
, claim_line_number
, data_source
, old_encounter_id
, 'outpatient psych' as encounter_type
, 'outpatient' as encounter_group
, 10 as priority_number
, null as anchor_claim_id
from __dbt__cte__outpatient_psych__match_claims_to_anchor

union all

select claim_id
, claim_line_number
, data_source
, old_encounter_id
, 'outpatient rehabilitation' as encounter_type
, 'outpatient' as encounter_group
, 11 as priority_number
, null as anchor_claim_id
from __dbt__cte__outpatient_rehab__match_claims_to_anchor

union all

select claim_id
, claim_line_number
, data_source
, old_encounter_id
, 'ambulatory surgery center' as encounter_type
, 'outpatient' as encounter_group
, 12 as priority_number
, null as anchor_claim_id
from __dbt__cte__asc__match_claims_to_anchor

union all

select claim_id
, claim_line_number
, data_source
, old_encounter_id
, 'dialysis' as encounter_type
, 'outpatient' as encounter_group
, 13 as priority_number
, null as anchor_claim_id
from __dbt__cte__dialysis__match_claims_to_anchor

union all

select claim_id
, claim_line_number
, data_source
, old_encounter_id
, 'outpatient hospice' as encounter_type
, 'outpatient' as encounter_group
, 14 as priority_number
, null as anchor_claim_id
from __dbt__cte__outpatient_hospice__match_claims_to_anchor

union all

select claim_id
, claim_line_number
, data_source
, old_encounter_id
, 'home health' as encounter_type
, 'outpatient' as encounter_group
, 15 as priority_number
, null as anchor_claim_id
from __dbt__cte__home_health__match_claims_to_anchor

union all

select claim_id
, claim_line_number
, data_source
, old_encounter_id
, 'outpatient surgery' as encounter_type
, 'outpatient' as encounter_group
, 16 as priority_number
, null as anchor_claim_id
from __dbt__cte__outpatient_surgery__match_claims_to_anchor

union all

select claim_id
, claim_line_number
, data_source
, old_encounter_id
, 'outpatient injections' as encounter_type
, 'outpatient' as encounter_group
, 17 as priority_number
, null as anchor_claim_id
from __dbt__cte__outpatient_injections__match_claims_to_anchor

union all

select claim_id
, claim_line_number
, data_source
, old_encounter_id
, 'outpatient pt/ot/st' as encounter_type
, 'outpatient' as encounter_group
, 18 as priority_number
, null as anchor_claim_id
from __dbt__cte__outpatient_ptotst__match_claims_to_anchor

union all

select claim_id
, claim_line_number
, data_source
, old_encounter_id
, 'outpatient substance use' as encounter_type
, 'outpatient' as encounter_group
, 19 as priority_number
, null as anchor_claim_id
from __dbt__cte__outpatient_substance_use__match_claims_to_anchor

union all

select claim_id
, claim_line_number
, data_source
, old_encounter_id
, 'outpatient radiology' as encounter_type
, 'outpatient' as encounter_group
, 20 as priority_number
, null as anchor_claim_id
from __dbt__cte__outpatient_radiology__match_claims_to_anchor

union all

/* Set as lowest outpatient priority "catch all", roll up to more specific encounter type when available */
select claim_id
, claim_line_number
, data_source
, old_encounter_id
, 'outpatient hospital or clinic' as encounter_type
, 'outpatient' as encounter_group
, 999 as priority_number
, null as anchor_claim_id
from __dbt__cte__outpatient_hospital_or_clinic__match_claims_to_anchor

union all

select claim_id
, claim_line_number
, data_source
, encounter_id
, encounter_type
, encounter_group
, priority_number
, anchor_claim_id
from __dbt__cte__encounters__int_institutional_claim_lines

union all

/* orphaned encounters are "last resort". Labs/DME/ambulance should roll up to inpatient/home health/etc. If unable to match, then they get their own encounter*/

select claim_id
, claim_line_number
, data_source
, old_encounter_id
, 'lab - orphaned' as encounter_type
, 'other' as encounter_group
, 1000000 as priority_number
, null as anchor_claim_id
from __dbt__cte__lab__match_claims_to_anchor

union all

select claim_id
, claim_line_number
, data_source
, old_encounter_id
, 'dme - orphaned' as encounter_type
, 'other' as encounter_group
, 1000001 as priority_number
, null as anchor_claim_id
from __dbt__cte__dme__match_claims_to_anchor

union all

select claim_id
, claim_line_number
, data_source
, old_encounter_id
, 'ambulance - orphaned' as encounter_type
, 'other' as encounter_group
, 1000002 as priority_number
, null as anchor_claim_id
from __dbt__cte__ambulance__match_claims_to_anchor

)


select
  candidate.claim_id
, candidate.claim_line_number
, candidate.data_source
, candidate.encounter_id as old_encounter_id
, lower(md5(((case when candidate.encounter_type is null then 'N' else ('V' || replace(replace(cast(candidate.encounter_type as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when candidate.encounter_id is null then 'N' else ('V' || replace(replace(cast(candidate.encounter_id as TEXT), '%', '%25'), '|', '%7C')) end)))) as encounter_id
, candidate.encounter_type
, candidate.encounter_group
, candidate.priority_number
, candidate.anchor_claim_id
, row_number() over (partition by candidate.claim_id, candidate.claim_line_number, candidate.data_source
order by candidate.priority_number
       , case when candidate.encounter_type = 'inpatient hospice' then 0 else 1 end
       , case when candidate.claim_id = candidate.anchor_claim_id then 1 else 99 end
       , candidate.encounter_type, candidate.encounter_id) as claim_line_attribution_number
from cte as candidate
left outer join undetermined_claim_lines
    on candidate.claim_id = undetermined_claim_lines.claim_id
    and candidate.claim_line_number = undetermined_claim_lines.claim_line_number
    and candidate.data_source = undetermined_claim_lines.data_source
where undetermined_claim_lines.is_undetermined is null
),  __dbt__cte__encounters__orphaned_claims as (


with cte as (
  select stg.claim_id
  , stg.claim_line_number
  , stg.service_category_1
  , stg.service_category_2
  , stg.service_category_3
  , stg.claim_type
  , stg.claim_start_date
  , stg.claim_end_date
  , stg.start_date
  , stg.end_date
  , stg.patient_data_source_id
  , stg.data_source
  from __dbt__cte__encounters__stg_medical_claim as stg
  left outer join __dbt__cte__encounters__combined_claim_line_crosswalk as enc on stg.claim_id = enc.claim_id
  and
  stg.claim_line_number = enc.claim_line_number
  and
  stg.data_source = enc.data_source
  where enc.claim_id is null -- missing from encounter mapping table
)

select
  claim_id
, claim_line_number
, data_source
, lower(md5(((case when 'orphaned claim' is null then 'N' else ('V' || replace(replace(cast('orphaned claim' as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when patient_data_source_id is null then 'N' else ('V' || replace(replace(cast(patient_data_source_id as TEXT), '%', '%25'), '|', '%7C')) end) || '|' || (case when claim_id is null then 'N' else ('V' || replace(replace(cast(claim_id as TEXT), '%', '%25'), '|', '%7C')) end)))) as encounter_id
, 'orphaned claim' as encounter_type
, 'other' as encounter_group
from cte
),  __dbt__cte___stg_medical_claim as (


with all_encounters as (
    select
        claim_id
      , claim_line_number
      , data_source
      , encounter_id
    from __dbt__cte__encounters__combined_claim_line_crosswalk
    where claim_line_attribution_number = 1

    union all

    select
        claim_id
      , claim_line_number
      , data_source
      , encounter_id
    from __dbt__cte__encounters__orphaned_claims
)

select
    cast(med.claim_id as TEXT) as claim_id
  , cast(med.claim_line_number as integer) as claim_line_number
  , cast(med.person_id as TEXT) as person_id
  , try_cast( med.claim_start_date as date ) as claim_start_date
  , try_cast( med.claim_end_date as date ) as claim_end_date
  , cast(med.allowed_amount as numeric(28,6)) as allowed_amount
  , cast(med.paid_amount as numeric(28,6)) as paid_amount
  , cast(med.rendering_npi as TEXT) as rendering_npi
  , cast(med.hcpcs_code as TEXT) as hcpcs_code
  , cast(med.data_source as TEXT) as data_source
  , cast(enc.encounter_id as TEXT) as encounter_id
from __dbt__cte__normalized__medical_claim as med
inner join "claims_preprocessing"."service_category_grouper" as srv_group
  on med.claim_id = srv_group.claim_id
  and med.claim_line_number = srv_group.claim_line_number
  and med.data_source = srv_group.data_source
  and srv_group.duplicate_row_number = 1
inner join all_encounters as enc
  on med.claim_id = enc.claim_id
  and med.claim_line_number = enc.claim_line_number
  and med.data_source = enc.data_source
),  __dbt__cte___int_provider_classification as (


-- Map NPI to specialty description and CMS assignment bucket (pcp/specialist/npp)
with base as (
  select
      cast(p.npi as TEXT) as provider_id
    , p.primary_taxonomy_code
    , p.primary_specialty_description as prov_specialty
    , lower(trim(p.entity_type_description)) as entity_type
  from "provider_data"."provider" as p
)

, mapped as (
  select
      b.provider_id
    , b.prov_specialty
    , case
        when lower(a.primary_care_physician_step1) = 'yes' and a.physician = 1 then 'pcp'
        when lower(a.specialist_physician_step_2) = 'yes' and a.physician = 1 then 'specialist'
        when a.physician = 0 then 'npp'
        else 'unknown'
      end as provider_bucket
  from base as b
  inner join "provider_data"."medicare_provider_and_supplier_taxonomy_crosswalk" as x
    on trim(cast(b.primary_taxonomy_code as TEXT)) = trim(cast(x.provider_taxonomy_code as TEXT))
  inner join "value_sets"."_provider_specialty_assignment_codes" as a
  
    on right(concat('00', trim(x.medicare_specialty_code)), 2)
     = right(concat('00', trim(a.specialty_code)), 2)
  
  where b.entity_type = 'individual'
)

, rnk as (
  select
      provider_id
    , prov_specialty
    , provider_bucket
    , row_number() over (partition by provider_id
order by bucket_priority) as bucket_rank
  from (
    select
        mapped.*
      , case provider_bucket
          when 'pcp' then 1
          when 'npp' then 2
          when 'specialist' then 3
          else 4
        end as bucket_priority
    from mapped
  ) as prioritized
)

select
    provider_id
  , prov_specialty
  , provider_bucket
from rnk
where bucket_rank = 1
),  __dbt__cte___int_primary_care_claims as (


-- Primary care services from input_layer medical_claim, joined to member months and provider bucket

with member_months as (
  select
      person_id
    , year_month
    , data_source
  from __dbt__cte__member_month
  group by
      person_id
    , year_month
    , data_source
)

, claim_month as (
  select
      mc.person_id
    , mc.claim_id
    , mc.claim_line_number
    , cast(mc.encounter_id as TEXT) as encounter_id
    , mc.claim_start_date
    , mc.claim_end_date
    , cast(date_part('year', mc.claim_start_date) as integer) as claim_year
    , cast(date_part('month', mc.claim_start_date) as integer) as claim_month
    , cast(strftime('%Y%m', cast(mc.claim_start_date as date)) as integer) as claim_year_month_int
    , strftime('%Y%m', cast(mc.claim_start_date as date)) as claim_year_month
    -- Fallback to paid_amount when allowed_amount is absent; many payers omit allowed values.
    , coalesce(nullif(mc.allowed_amount, 0), mc.paid_amount, 0) as allowed_amount
    , cast(mc.rendering_npi as TEXT) as provider_id
    , mc.hcpcs_code
    , mc.data_source
  from __dbt__cte___stg_medical_claim as mc
)

, eligible_claims as (
  select c.*
  from claim_month as c
  inner join member_months as mm
    on c.person_id = mm.person_id
   and c.data_source = mm.data_source
   and c.claim_year_month = mm.year_month
)

, primary_care_claims as (
  select
      e.person_id
    , e.claim_id
    , e.claim_line_number
    , e.encounter_id
    , e.claim_start_date
    , e.claim_end_date
    , e.claim_year
    , e.claim_month
    , e.claim_year_month
    , e.claim_year_month_int
    , e.allowed_amount
    , e.provider_id
    , e.hcpcs_code
    , e.data_source
  from eligible_claims as e
  inner join "value_sets"."_primary_care_hcpcs_codes" as pc
    on e.hcpcs_code = pc.hcpcs_code
)


, with_bucket as (
  select
      pcc.*
    , coalesce(pc.provider_bucket, 'other_individual') as provider_bucket
    , coalesce(pc.prov_specialty, sp.primary_specialty_description) as prov_specialty
  from primary_care_claims as pcc
  left outer join __dbt__cte___int_provider_classification as pc
    on pcc.provider_id = pc.provider_id
  inner join "provider_data"."provider" as sp
    on cast(pcc.provider_id as TEXT) = cast(sp.npi as TEXT)
   and lower(trim(sp.entity_type_description)) = 'individual'
)

select * from with_bucket
),  __dbt__cte___int_person_years as (


with mm as (
  select
      person_id
    , data_source
        
        , left(year_month, 4) as performance_year
        
  from __dbt__cte__member_month
  group by person_id
        , data_source
        
        , left(year_month, 4)
        
)
select
    cast(person_id as TEXT) as person_id
  , cast(data_source as TEXT) as data_source
  , cast(performance_year as integer) as performance_year
from mm
), member_months as (
  select
      person_id
    , year_month
    , data_source
  from __dbt__cte__member_month
  group by
      person_id
    , year_month
    , data_source
)

, calendar_months as (
  select distinct
      year_month_int
    , first_day_of_month
    , last_day_of_month
  from __dbt__cte__member_month__month_spine
)

, claim_bounds as (
  select
      data_source
    , max(claim_end_date) as max_claim_end_date
  from __dbt__cte___int_primary_care_claims
  group by data_source
)



, params as (
  select
    data_source,
    
      cast('2018-12-31' as date) as as_of_date
    
  from claim_bounds
)

-- Rolling “current” window helpers
, months_12 as (
  select distinct
      p.data_source
    , c.year_month_int
    , c.first_day_of_month
    , c.last_day_of_month
  from calendar_months as c
  cross join params as p
  where c.last_day_of_month >= cast(

    (p.as_of_date + cast(-11 as bigint) * interval 1 month) as date)
    and c.first_day_of_month <= p.as_of_date
)

, months_24 as (
  select distinct
      p.data_source
    , c.year_month_int
    , c.first_day_of_month
    , c.last_day_of_month
  from calendar_months as c
  cross join params as p
  where c.last_day_of_month >= cast(

    (p.as_of_date + cast(-23 as bigint) * interval 1 month) as date)
    and c.first_day_of_month <= p.as_of_date
)

, lookback_12 as (
  select
      data_source
    , min(first_day_of_month) as lookback_start_date_12
  from months_12
  group by data_source
)

, lookback_24 as (
  select
      data_source
    , min(first_day_of_month) as lookback_start_date_24
  from months_24
  group by data_source
)

, lookback_bounds as (
  select
      l12.data_source
    , l12.lookback_start_date_12
    , l24.lookback_start_date_24
  from lookback_12 as l12
  inner join lookback_24 as l24
    on l12.data_source = l24.data_source
)

, eligible_current as (
  -- Current-scope eligibility: beneficiaries with at least one member month
  -- within the 12-month window ending at as_of_date.
  select distinct
      mm.person_id
    , mm.data_source
  from member_months as mm
  inner join months_12 as m
    on mm.data_source = m.data_source
   and mm.year_month = cast(m.year_month_int as TEXT)
)

, claims_12 as (
  select
      c.person_id
    , c.data_source
    , c.provider_id
    , c.provider_bucket
    , c.prov_specialty
    , c.encounter_id
    , c.claim_id
    , c.claim_year_month
    , c.claim_year_month_int
    , c.claim_end_date
    , c.allowed_amount
  from __dbt__cte___int_primary_care_claims as c
  inner join months_12 as m
    on c.data_source = m.data_source
   and c.claim_year_month_int = m.year_month_int
  inner join params as p
    on c.data_source = p.data_source
  where c.claim_end_date <= p.as_of_date
)

, claims_24 as (
  select
      c.person_id
    , c.data_source
    , c.provider_id
    , c.provider_bucket
    , c.prov_specialty
    , c.encounter_id
    , c.claim_id
    , c.claim_year_month
    , c.claim_year_month_int
    , c.claim_end_date
    , c.allowed_amount
  from __dbt__cte___int_primary_care_claims as c
  inner join months_24 as m
    on c.data_source = m.data_source
   and c.claim_year_month_int = m.year_month_int
  inner join params as p
    on c.data_source = p.data_source
  where c.claim_end_date <= p.as_of_date
)

, all_claim_month as (
  select
      mc.person_id
    , mc.data_source
    , mc.claim_id
    , mc.claim_line_number
    , cast(mc.encounter_id as TEXT) as encounter_id
    , mc.claim_start_date
    , mc.claim_end_date
    , cast(strftime('%Y%m', cast(mc.claim_start_date as date)) as integer) as claim_year_month_int
    , strftime('%Y%m', cast(mc.claim_start_date as date)) as claim_year_month
    , coalesce(nullif(mc.allowed_amount, 0), mc.paid_amount, 0) as allowed_amount
    , cast(mc.rendering_npi as TEXT) as provider_id
  from __dbt__cte___stg_medical_claim as mc
)

, eligible_all_claims as (
  select ac.*
  from all_claim_month as ac
  inner join member_months as mm
    on ac.person_id = mm.person_id
   and ac.data_source = mm.data_source
   and ac.claim_year_month = mm.year_month
)

, all_rendering_claims as (
  select
      e.person_id
    , e.data_source
    , e.provider_id
    , e.encounter_id
    , e.claim_id
    , e.claim_year_month
    , e.claim_year_month_int
    , e.claim_end_date
    , e.allowed_amount
    , coalesce(pc.provider_bucket, 'other_individual') as provider_bucket
    , coalesce(pc.prov_specialty, sp.primary_specialty_description) as prov_specialty
  from eligible_all_claims as e
  inner join "provider_data"."provider" as sp
    on cast(e.provider_id as TEXT) = cast(sp.npi as TEXT)
   and lower(trim(sp.entity_type_description)) = 'individual'
  left outer join __dbt__cte___int_provider_classification as pc
    on e.provider_id = pc.provider_id
)

-- Build all potential providers (no bene-level gating across steps), then
-- collapse to the earliest qualifying step per person/provider.
, current_all_steps as (
  select person_id, data_source, provider_id, provider_bucket, prov_specialty, 1 as step
         , sum(allowed_amount) as allowed_amount
         , count(distinct encounter_id) as visits
  from claims_12
  where provider_id is not null and provider_bucket in ('pcp', 'npp')
  group by person_id, data_source, provider_id, provider_bucket, prov_specialty

  union all
  select person_id, data_source, provider_id, provider_bucket, prov_specialty, 2 as step
         , sum(allowed_amount) as allowed_amount
         , count(distinct encounter_id) as visits
  from claims_12
  where provider_id is not null and provider_bucket = 'specialist'
  group by person_id, data_source, provider_id, provider_bucket, prov_specialty

  union all
  select person_id, data_source, provider_id, provider_bucket, prov_specialty, 3 as step
         , sum(allowed_amount) as allowed_amount
         , count(distinct encounter_id) as visits
  from claims_24
  where provider_id is not null and provider_bucket in ('pcp', 'npp')
  group by person_id, data_source, provider_id, provider_bucket, prov_specialty

  union all
  select person_id, data_source, provider_id, provider_bucket, prov_specialty, 4 as step
         , sum(allowed_amount) as allowed_amount
         , count(distinct encounter_id) as visits
  from claims_24
  where provider_id is not null
  group by person_id, data_source, provider_id, provider_bucket, prov_specialty

  union all
  select arc.person_id, arc.data_source, arc.provider_id, coalesce(arc.provider_bucket, 'unknown') as provider_bucket
         , arc.prov_specialty, 5 as step
         , sum(arc.allowed_amount) as allowed_amount
         , count(distinct arc.encounter_id) as visits
  from all_rendering_claims as arc
  inner join months_24 as m
    on arc.data_source = m.data_source
   and arc.claim_year_month_int = m.year_month_int
  inner join params as p
    on arc.data_source = p.data_source
  where arc.provider_id is not null and arc.claim_end_date <= p.as_of_date
  group by arc.person_id, arc.data_source, arc.provider_id, coalesce(arc.provider_bucket, 'unknown'), arc.prov_specialty
)

, current_unique as (
  select *
  from (
    select
        s.*
      , row_number() over (partition by s.person_id, s.provider_id
                                        , s.data_source
order by s.step) as step_choice_rank
    from current_all_steps as s
  ) as d
  where step_choice_rank = 1
)

, yearly_all_steps as (
  -- Yearly windows are driven by performance_year (Jan..Dec for 12-month and Jan(Y-1)..Dec(Y) for expanded)
  select
      py.person_id
    , py.data_source
    , py.performance_year
    , c.provider_id
    , coalesce(c.provider_bucket, 'unknown') as provider_bucket
    , c.prov_specialty
    , 1 as step
    , sum(c.allowed_amount) as allowed_amount
    , count(distinct c.encounter_id) as visits
  from __dbt__cte___int_person_years as py
  inner join __dbt__cte___int_primary_care_claims as c
    on py.person_id = c.person_id
   and py.data_source = c.data_source
   and c.claim_year = py.performance_year
   and c.provider_id is not null
   and c.provider_bucket in ('pcp', 'npp')
  group by py.person_id, py.data_source, py.performance_year, c.provider_id, coalesce(c.provider_bucket, 'unknown'), c.prov_specialty

  union all
  select
      py.person_id
    , py.data_source
    , py.performance_year
    , c.provider_id
    , coalesce(c.provider_bucket, 'unknown') as provider_bucket
    , c.prov_specialty
    , 2 as step
    , sum(c.allowed_amount) as allowed_amount
    , count(distinct c.encounter_id) as visits
  from __dbt__cte___int_person_years as py
  inner join __dbt__cte___int_primary_care_claims as c
    on py.person_id = c.person_id
   and py.data_source = c.data_source
   and c.claim_year = py.performance_year
   and c.provider_id is not null
   and c.provider_bucket = 'specialist'
  group by py.person_id, py.data_source, py.performance_year, c.provider_id, coalesce(c.provider_bucket, 'unknown'), c.prov_specialty

  union all
  select
      py.person_id
    , py.data_source
    , py.performance_year
    , c.provider_id
    , coalesce(c.provider_bucket, 'unknown') as provider_bucket
    , c.prov_specialty
    , 3 as step
    , sum(c.allowed_amount) as allowed_amount
    , count(distinct c.encounter_id) as visits
  from __dbt__cte___int_person_years as py
  inner join __dbt__cte___int_primary_care_claims as c
    on py.person_id = c.person_id
   and py.data_source = c.data_source
   and c.claim_year_month_int between ((py.performance_year - 1) * 100 + 1)
                                  and (py.performance_year * 100 + 12)
   and c.provider_id is not null
   and c.provider_bucket in ('pcp', 'npp')
  group by py.person_id, py.data_source, py.performance_year, c.provider_id, coalesce(c.provider_bucket, 'unknown'), c.prov_specialty

  union all
  select
      py.person_id
    , py.data_source
    , py.performance_year
    , c.provider_id
    , coalesce(c.provider_bucket, 'unknown') as provider_bucket
    , c.prov_specialty
    , 4 as step
    , sum(c.allowed_amount) as allowed_amount
    , count(distinct c.encounter_id) as visits
  from __dbt__cte___int_person_years as py
  inner join __dbt__cte___int_primary_care_claims as c
    on py.person_id = c.person_id
   and py.data_source = c.data_source
   and c.claim_year_month_int between ((py.performance_year - 1) * 100 + 1)
                                  and (py.performance_year * 100 + 12)
   and c.provider_id is not null
  group by py.person_id, py.data_source, py.performance_year, c.provider_id, coalesce(c.provider_bucket, 'unknown'), c.prov_specialty

  union all
  select
      py.person_id
    , py.data_source
    , py.performance_year
    , arc.provider_id
    , coalesce(arc.provider_bucket, 'unknown') as provider_bucket
    , arc.prov_specialty
    , 5 as step
    , sum(arc.allowed_amount) as allowed_amount
    , count(distinct arc.encounter_id) as visits
  from __dbt__cte___int_person_years as py
  inner join all_rendering_claims as arc
    on py.person_id = arc.person_id
   and py.data_source = arc.data_source
   and arc.claim_year_month_int between ((py.performance_year - 1) * 100 + 1)
                                   and (py.performance_year * 100 + 12)
  where arc.provider_id is not null
  group by py.person_id, py.data_source, py.performance_year, arc.provider_id, coalesce(arc.provider_bucket, 'unknown'), arc.prov_specialty
)

, yearly_unique as (
  select * from (
    select
        s.*
      , row_number() over (
          partition by s.person_id, s.performance_year, s.provider_id
          , s.data_source
          order by s.step
        ) as step_choice_rank
    from yearly_all_steps as s
  ) as d
  where step_choice_rank = 1
)

, yearly as (
  select
      y.person_id
    , y.data_source
    , cast(y.performance_year as integer) as performance_year
    , cast(null as date) as as_of_date
    , y.provider_id
    , y.provider_bucket
    , y.prov_specialty
    , y.step
    , case y.step
        when 1 then '12-month PCP/NPP primary-care HCPCS'
        when 2 then '12-month specialist primary-care HCPCS'
        when 3 then '24-month PCP/NPP primary-care HCPCS'
        when 4 then '24-month primary-care HCPCS (any classification)'
        when 5 then '24-month any rendering NPI'
        else 'Unknown'
      end as step_description
    , y.allowed_amount
    , y.visits
    , 'yearly' as scope
    , case
        when y.step in (3, 4, 5)
          then coalesce(start_prev.first_day_of_month, start_curr.first_day_of_month)
        else start_curr.first_day_of_month
      end as lookback_start_date
    , end_curr.last_day_of_month as lookback_end_date
    , ('yearly|' || y.data_source || '|' || cast(y.performance_year as TEXT) || '|' || y.person_id) as attribution_key
    , rank() over (partition by y.person_id, y.data_source, y.performance_year
                   order by y.step asc, y.allowed_amount desc, y.visits desc, y.provider_id) as ranking
  from yearly_unique as y
  left outer join calendar_months as start_curr
    on start_curr.year_month_int = (y.performance_year * 100) + 1
  left outer join calendar_months as start_prev
    on start_prev.year_month_int = ((y.performance_year - 1) * 100) + 1
  left outer join calendar_months as end_curr
    on end_curr.year_month_int = (y.performance_year * 100) + 12
)

, current_scope as (
  select
      s.person_id
    , s.data_source
    , null as performance_year
    , p.as_of_date
    , s.provider_id
    , s.provider_bucket
    , s.prov_specialty
    , s.step
    , case s.step
        when 1 then '12-month PCP/NPP primary-care HCPCS'
        when 2 then '12-month specialist primary-care HCPCS'
        when 3 then '24-month PCP/NPP primary-care HCPCS'
        when 4 then '24-month primary-care HCPCS (any classification)'
        when 5 then '24-month any rendering NPI'
        else 'Unknown'
      end as step_description
    , s.allowed_amount
    , s.visits
    , 'current' as scope
    , case when s.step in (1, 2) then lb.lookback_start_date_12 else lb.lookback_start_date_24 end as lookback_start_date
    , p.as_of_date as lookback_end_date
    , ('current|' || s.data_source || '|' || replace(cast(p.as_of_date as TEXT),'-','') || '|' || s.person_id) as attribution_key
    , rank() over (partition by s.person_id, s.data_source
                   order by s.step asc, s.allowed_amount desc, s.visits desc, s.provider_id) as ranking
  from current_unique as s
  inner join eligible_current as ec
    on s.person_id = ec.person_id
   and s.data_source = ec.data_source
  inner join params as p
    on s.data_source = p.data_source
  inner join lookback_bounds as lb
    on s.data_source = lb.data_source
)

select
    person_id
  , data_source
  , performance_year
  , as_of_date
  , provider_id
  , provider_bucket
  , prov_specialty
  , step
  , step_description
  , allowed_amount
  , visits
  , scope
  , lookback_start_date
  , lookback_end_date
  , ranking
  , attribution_key
  , cast('2026-01-01 00:00:00' as timestamp) as tuva_last_run
from yearly

union all

select
    person_id
  , data_source
  , performance_year
  , as_of_date
  , provider_id
  , provider_bucket
  , prov_specialty
  , step
  , step_description
  , allowed_amount
  , visits
  , scope
  , lookback_start_date
  , lookback_end_date
  , ranking
  , attribution_key
  , cast('2026-01-01 00:00:00' as timestamp) as tuva_last_run
from current_scope