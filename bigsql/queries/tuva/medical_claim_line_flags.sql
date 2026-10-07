-- Tuva medical_claim_line_flags, compiled by dbt with every upstream model ephemeral (service_category and core__stg_claims_encounter stay tables)










    

    

    

    

    

    

    

    

    

    

    

    

    

    

    

    

    

    

    

    

    

    

    

    

    




    

    

    

    

    

    

    

    

    

    

    

    

    

    

    

    

    

    

    

    

    

    

    

    




    

    

    

    

    

    

    

    

    

    

    

    

    

    

    

    

    

    

    

    

    

    

    

    

    






















    
    

    
    

    
    

    
    

    
    

    
    

    
    

    
    

    
    

    
    

    
    

    
    

    
    

    
    

    
    

    
    

    
    

    
    

    
    

    
    

    
    

    
    

    
    

    
    

    
    




    
    

    
    

    
    

    
    

    
    

    
    

    
    

    
    

    
    

    
    

    
    

    
    

    
    

    
    

    
    

    
    

    
    

    
    

    
    

    
    

    
    

    
    

    
    

    
    

    
    


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
), eligibility_rows_with_effective_end_date as (
    select
          eligibility_rows.*
        , case
            when eligibility_rows.enrollment_end_date is null
              or eligibility_rows.enrollment_end_date = 
    cast('9999-12-31' as date)

              or eligibility_rows.enrollment_end_date > cast(cast('2026-01-01 00:00:00' as timestamp) as date)
                then cast(cast('2026-01-01 00:00:00' as timestamp) as date)
            else eligibility_rows.enrollment_end_date
          end as _dq_effective_enrollment_end_date
    from __dbt__cte__input_layer__eligibility as eligibility_rows
),

source_rows as (
    select
          medical_claim_rows.*
        , medical_claim_rows.claim_id as _dq_claim_id_key
        , medical_claim_rows.claim_line_number as _dq_claim_line_number_key
        , medical_claim_rows.data_source as _dq_data_source_key
    from __dbt__cte__input_layer__medical_claim as medical_claim_rows
),

source_eligibility_member_months as (
    select distinct
          source_rows.person_id
        , source_rows.member_id
        , source_rows.payer
        , source_rows.plan
        , source_rows.data_source
        , cast(date_part('year', coalesce(source_rows.claim_line_start_date, source_rows.claim_start_date, source_rows.admission_date)) as integer) as _dq_claim_year
        , cast(date_part('month', coalesce(source_rows.claim_line_start_date, source_rows.claim_start_date, source_rows.admission_date)) as integer) as _dq_claim_month
    from source_rows
    where source_rows.person_id is not null and source_rows.member_id is not null and source_rows.payer is not null and source_rows.plan is not null and source_rows.data_source is not null and coalesce(source_rows.claim_line_start_date, source_rows.claim_start_date, source_rows.admission_date) is not null and (coalesce(source_rows.claim_line_start_date, source_rows.claim_start_date, source_rows.admission_date) >= 
    cast('2000-01-01' as date)
 and coalesce(source_rows.claim_line_start_date, source_rows.claim_start_date, source_rows.admission_date) <= cast(now() as date)) and (coalesce(source_rows.claim_line_start_date, source_rows.claim_start_date, source_rows.admission_date) >= 
    cast('1900-01-01' as date)
 and coalesce(source_rows.claim_line_start_date, source_rows.claim_start_date, source_rows.admission_date) <= 
    cast('2100-12-31' as date)
) and coalesce(source_rows.claim_line_start_date, source_rows.claim_start_date, source_rows.admission_date) <= cast(cast('2026-01-01 00:00:00' as timestamp) as date)
),

matching_eligibility_member_months as (
    select distinct
          source_member_months.person_id
        , source_member_months.member_id
        , source_member_months.payer
        , source_member_months.plan
        , source_member_months.data_source
        , source_member_months._dq_claim_year
        , source_member_months._dq_claim_month
        , 1 as _dq_has_matching_eligibility
    from source_eligibility_member_months as source_member_months
    inner join eligibility_rows_with_effective_end_date as eligibility_rows
        on eligibility_rows.person_id = source_member_months.person_id
       and eligibility_rows.member_id = source_member_months.member_id
       and eligibility_rows.payer = source_member_months.payer
       and eligibility_rows.plan = source_member_months.plan
       and eligibility_rows.data_source = source_member_months.data_source
       and (
            source_member_months._dq_claim_year > cast(date_part('year', eligibility_rows.enrollment_start_date) as integer)
            or (
                source_member_months._dq_claim_year = cast(date_part('year', eligibility_rows.enrollment_start_date) as integer)
                and source_member_months._dq_claim_month >= cast(date_part('month', eligibility_rows.enrollment_start_date) as integer)
            )
       )
       and (
            source_member_months._dq_claim_year < cast(date_part('year', eligibility_rows._dq_effective_enrollment_end_date) as integer)
            or (
                source_member_months._dq_claim_year = cast(date_part('year', eligibility_rows._dq_effective_enrollment_end_date) as integer)
                and source_member_months._dq_claim_month <= cast(date_part('month', eligibility_rows._dq_effective_enrollment_end_date) as integer)
            )
       )
),

diagnosis_codes as (
    select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 1 as diagnosis_position
            , cast(source_rows.diagnosis_code_type as TEXT) as diagnosis_code_type
            , replace(cast(source_rows.diagnosis_code_1 as TEXT), '.', '') as diagnosis_code
        from source_rows
        where source_rows.diagnosis_code_1 is not null
          and cast(source_rows.diagnosis_code_type as TEXT) in ('icd-10-cm', 'icd-9-cm')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 2 as diagnosis_position
            , cast(source_rows.diagnosis_code_type as TEXT) as diagnosis_code_type
            , replace(cast(source_rows.diagnosis_code_2 as TEXT), '.', '') as diagnosis_code
        from source_rows
        where source_rows.diagnosis_code_2 is not null
          and cast(source_rows.diagnosis_code_type as TEXT) in ('icd-10-cm', 'icd-9-cm')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 3 as diagnosis_position
            , cast(source_rows.diagnosis_code_type as TEXT) as diagnosis_code_type
            , replace(cast(source_rows.diagnosis_code_3 as TEXT), '.', '') as diagnosis_code
        from source_rows
        where source_rows.diagnosis_code_3 is not null
          and cast(source_rows.diagnosis_code_type as TEXT) in ('icd-10-cm', 'icd-9-cm')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 4 as diagnosis_position
            , cast(source_rows.diagnosis_code_type as TEXT) as diagnosis_code_type
            , replace(cast(source_rows.diagnosis_code_4 as TEXT), '.', '') as diagnosis_code
        from source_rows
        where source_rows.diagnosis_code_4 is not null
          and cast(source_rows.diagnosis_code_type as TEXT) in ('icd-10-cm', 'icd-9-cm')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 5 as diagnosis_position
            , cast(source_rows.diagnosis_code_type as TEXT) as diagnosis_code_type
            , replace(cast(source_rows.diagnosis_code_5 as TEXT), '.', '') as diagnosis_code
        from source_rows
        where source_rows.diagnosis_code_5 is not null
          and cast(source_rows.diagnosis_code_type as TEXT) in ('icd-10-cm', 'icd-9-cm')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 6 as diagnosis_position
            , cast(source_rows.diagnosis_code_type as TEXT) as diagnosis_code_type
            , replace(cast(source_rows.diagnosis_code_6 as TEXT), '.', '') as diagnosis_code
        from source_rows
        where source_rows.diagnosis_code_6 is not null
          and cast(source_rows.diagnosis_code_type as TEXT) in ('icd-10-cm', 'icd-9-cm')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 7 as diagnosis_position
            , cast(source_rows.diagnosis_code_type as TEXT) as diagnosis_code_type
            , replace(cast(source_rows.diagnosis_code_7 as TEXT), '.', '') as diagnosis_code
        from source_rows
        where source_rows.diagnosis_code_7 is not null
          and cast(source_rows.diagnosis_code_type as TEXT) in ('icd-10-cm', 'icd-9-cm')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 8 as diagnosis_position
            , cast(source_rows.diagnosis_code_type as TEXT) as diagnosis_code_type
            , replace(cast(source_rows.diagnosis_code_8 as TEXT), '.', '') as diagnosis_code
        from source_rows
        where source_rows.diagnosis_code_8 is not null
          and cast(source_rows.diagnosis_code_type as TEXT) in ('icd-10-cm', 'icd-9-cm')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 9 as diagnosis_position
            , cast(source_rows.diagnosis_code_type as TEXT) as diagnosis_code_type
            , replace(cast(source_rows.diagnosis_code_9 as TEXT), '.', '') as diagnosis_code
        from source_rows
        where source_rows.diagnosis_code_9 is not null
          and cast(source_rows.diagnosis_code_type as TEXT) in ('icd-10-cm', 'icd-9-cm')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 10 as diagnosis_position
            , cast(source_rows.diagnosis_code_type as TEXT) as diagnosis_code_type
            , replace(cast(source_rows.diagnosis_code_10 as TEXT), '.', '') as diagnosis_code
        from source_rows
        where source_rows.diagnosis_code_10 is not null
          and cast(source_rows.diagnosis_code_type as TEXT) in ('icd-10-cm', 'icd-9-cm')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 11 as diagnosis_position
            , cast(source_rows.diagnosis_code_type as TEXT) as diagnosis_code_type
            , replace(cast(source_rows.diagnosis_code_11 as TEXT), '.', '') as diagnosis_code
        from source_rows
        where source_rows.diagnosis_code_11 is not null
          and cast(source_rows.diagnosis_code_type as TEXT) in ('icd-10-cm', 'icd-9-cm')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 12 as diagnosis_position
            , cast(source_rows.diagnosis_code_type as TEXT) as diagnosis_code_type
            , replace(cast(source_rows.diagnosis_code_12 as TEXT), '.', '') as diagnosis_code
        from source_rows
        where source_rows.diagnosis_code_12 is not null
          and cast(source_rows.diagnosis_code_type as TEXT) in ('icd-10-cm', 'icd-9-cm')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 13 as diagnosis_position
            , cast(source_rows.diagnosis_code_type as TEXT) as diagnosis_code_type
            , replace(cast(source_rows.diagnosis_code_13 as TEXT), '.', '') as diagnosis_code
        from source_rows
        where source_rows.diagnosis_code_13 is not null
          and cast(source_rows.diagnosis_code_type as TEXT) in ('icd-10-cm', 'icd-9-cm')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 14 as diagnosis_position
            , cast(source_rows.diagnosis_code_type as TEXT) as diagnosis_code_type
            , replace(cast(source_rows.diagnosis_code_14 as TEXT), '.', '') as diagnosis_code
        from source_rows
        where source_rows.diagnosis_code_14 is not null
          and cast(source_rows.diagnosis_code_type as TEXT) in ('icd-10-cm', 'icd-9-cm')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 15 as diagnosis_position
            , cast(source_rows.diagnosis_code_type as TEXT) as diagnosis_code_type
            , replace(cast(source_rows.diagnosis_code_15 as TEXT), '.', '') as diagnosis_code
        from source_rows
        where source_rows.diagnosis_code_15 is not null
          and cast(source_rows.diagnosis_code_type as TEXT) in ('icd-10-cm', 'icd-9-cm')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 16 as diagnosis_position
            , cast(source_rows.diagnosis_code_type as TEXT) as diagnosis_code_type
            , replace(cast(source_rows.diagnosis_code_16 as TEXT), '.', '') as diagnosis_code
        from source_rows
        where source_rows.diagnosis_code_16 is not null
          and cast(source_rows.diagnosis_code_type as TEXT) in ('icd-10-cm', 'icd-9-cm')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 17 as diagnosis_position
            , cast(source_rows.diagnosis_code_type as TEXT) as diagnosis_code_type
            , replace(cast(source_rows.diagnosis_code_17 as TEXT), '.', '') as diagnosis_code
        from source_rows
        where source_rows.diagnosis_code_17 is not null
          and cast(source_rows.diagnosis_code_type as TEXT) in ('icd-10-cm', 'icd-9-cm')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 18 as diagnosis_position
            , cast(source_rows.diagnosis_code_type as TEXT) as diagnosis_code_type
            , replace(cast(source_rows.diagnosis_code_18 as TEXT), '.', '') as diagnosis_code
        from source_rows
        where source_rows.diagnosis_code_18 is not null
          and cast(source_rows.diagnosis_code_type as TEXT) in ('icd-10-cm', 'icd-9-cm')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 19 as diagnosis_position
            , cast(source_rows.diagnosis_code_type as TEXT) as diagnosis_code_type
            , replace(cast(source_rows.diagnosis_code_19 as TEXT), '.', '') as diagnosis_code
        from source_rows
        where source_rows.diagnosis_code_19 is not null
          and cast(source_rows.diagnosis_code_type as TEXT) in ('icd-10-cm', 'icd-9-cm')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 20 as diagnosis_position
            , cast(source_rows.diagnosis_code_type as TEXT) as diagnosis_code_type
            , replace(cast(source_rows.diagnosis_code_20 as TEXT), '.', '') as diagnosis_code
        from source_rows
        where source_rows.diagnosis_code_20 is not null
          and cast(source_rows.diagnosis_code_type as TEXT) in ('icd-10-cm', 'icd-9-cm')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 21 as diagnosis_position
            , cast(source_rows.diagnosis_code_type as TEXT) as diagnosis_code_type
            , replace(cast(source_rows.diagnosis_code_21 as TEXT), '.', '') as diagnosis_code
        from source_rows
        where source_rows.diagnosis_code_21 is not null
          and cast(source_rows.diagnosis_code_type as TEXT) in ('icd-10-cm', 'icd-9-cm')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 22 as diagnosis_position
            , cast(source_rows.diagnosis_code_type as TEXT) as diagnosis_code_type
            , replace(cast(source_rows.diagnosis_code_22 as TEXT), '.', '') as diagnosis_code
        from source_rows
        where source_rows.diagnosis_code_22 is not null
          and cast(source_rows.diagnosis_code_type as TEXT) in ('icd-10-cm', 'icd-9-cm')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 23 as diagnosis_position
            , cast(source_rows.diagnosis_code_type as TEXT) as diagnosis_code_type
            , replace(cast(source_rows.diagnosis_code_23 as TEXT), '.', '') as diagnosis_code
        from source_rows
        where source_rows.diagnosis_code_23 is not null
          and cast(source_rows.diagnosis_code_type as TEXT) in ('icd-10-cm', 'icd-9-cm')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 24 as diagnosis_position
            , cast(source_rows.diagnosis_code_type as TEXT) as diagnosis_code_type
            , replace(cast(source_rows.diagnosis_code_24 as TEXT), '.', '') as diagnosis_code
        from source_rows
        where source_rows.diagnosis_code_24 is not null
          and cast(source_rows.diagnosis_code_type as TEXT) in ('icd-10-cm', 'icd-9-cm')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 25 as diagnosis_position
            , cast(source_rows.diagnosis_code_type as TEXT) as diagnosis_code_type
            , replace(cast(source_rows.diagnosis_code_25 as TEXT), '.', '') as diagnosis_code
        from source_rows
        where source_rows.diagnosis_code_25 is not null
          and cast(source_rows.diagnosis_code_type as TEXT) in ('icd-10-cm', 'icd-9-cm')
),

diagnosis_code_flags as (
    select
          diagnosis_codes._dq_claim_id_key
        , diagnosis_codes._dq_claim_line_number_key
        , diagnosis_codes._dq_data_source_key
        , cast(max(
            case
                when diagnosis_codes.diagnosis_position = 1
                 and diagnosis_codes.diagnosis_code_type = 'icd-10-cm'
                 and icd_10_diagnosis_lookup.icd_10_cm is null
                    then 1
                when diagnosis_codes.diagnosis_position = 1
                 and diagnosis_codes.diagnosis_code_type = 'icd-9-cm'
                 and icd_9_diagnosis_lookup.icd_9_cm is null
                    then 1
                else 0
            end
          ) as integer) as diagnosis_code_1_invalid
        , cast(max(
            case
                when diagnosis_codes.diagnosis_position > 1
                 and diagnosis_codes.diagnosis_code_type = 'icd-10-cm'
                 and icd_10_diagnosis_lookup.icd_10_cm is null
                    then 1
                when diagnosis_codes.diagnosis_position > 1
                 and diagnosis_codes.diagnosis_code_type = 'icd-9-cm'
                 and icd_9_diagnosis_lookup.icd_9_cm is null
                    then 1
                else 0
            end
          ) as integer) as diagnosis_code_2_to_25_invalid
    from diagnosis_codes
    left join "terminology"."icd_10_cm" as icd_10_diagnosis_lookup
        on diagnosis_codes.diagnosis_code_type = 'icd-10-cm'
       and diagnosis_codes.diagnosis_code = replace(cast(icd_10_diagnosis_lookup.icd_10_cm as TEXT), '.', '')
    left join "terminology"."icd_9_cm" as icd_9_diagnosis_lookup
        on diagnosis_codes.diagnosis_code_type = 'icd-9-cm'
       and diagnosis_codes.diagnosis_code = replace(cast(icd_9_diagnosis_lookup.icd_9_cm as TEXT), '.', '')
    group by
          diagnosis_codes._dq_claim_id_key
        , diagnosis_codes._dq_claim_line_number_key
        , diagnosis_codes._dq_data_source_key
),

procedure_codes as (
    select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 1 as procedure_position
            , cast(source_rows.procedure_code_type as TEXT) as procedure_code_type
            , replace(cast(source_rows.procedure_code_1 as TEXT), '.', '') as procedure_code
        from source_rows
        where source_rows.procedure_code_1 is not null
          and cast(source_rows.procedure_code_type as TEXT) in ('icd-10-pcs', 'icd-9-pcs')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 2 as procedure_position
            , cast(source_rows.procedure_code_type as TEXT) as procedure_code_type
            , replace(cast(source_rows.procedure_code_2 as TEXT), '.', '') as procedure_code
        from source_rows
        where source_rows.procedure_code_2 is not null
          and cast(source_rows.procedure_code_type as TEXT) in ('icd-10-pcs', 'icd-9-pcs')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 3 as procedure_position
            , cast(source_rows.procedure_code_type as TEXT) as procedure_code_type
            , replace(cast(source_rows.procedure_code_3 as TEXT), '.', '') as procedure_code
        from source_rows
        where source_rows.procedure_code_3 is not null
          and cast(source_rows.procedure_code_type as TEXT) in ('icd-10-pcs', 'icd-9-pcs')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 4 as procedure_position
            , cast(source_rows.procedure_code_type as TEXT) as procedure_code_type
            , replace(cast(source_rows.procedure_code_4 as TEXT), '.', '') as procedure_code
        from source_rows
        where source_rows.procedure_code_4 is not null
          and cast(source_rows.procedure_code_type as TEXT) in ('icd-10-pcs', 'icd-9-pcs')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 5 as procedure_position
            , cast(source_rows.procedure_code_type as TEXT) as procedure_code_type
            , replace(cast(source_rows.procedure_code_5 as TEXT), '.', '') as procedure_code
        from source_rows
        where source_rows.procedure_code_5 is not null
          and cast(source_rows.procedure_code_type as TEXT) in ('icd-10-pcs', 'icd-9-pcs')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 6 as procedure_position
            , cast(source_rows.procedure_code_type as TEXT) as procedure_code_type
            , replace(cast(source_rows.procedure_code_6 as TEXT), '.', '') as procedure_code
        from source_rows
        where source_rows.procedure_code_6 is not null
          and cast(source_rows.procedure_code_type as TEXT) in ('icd-10-pcs', 'icd-9-pcs')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 7 as procedure_position
            , cast(source_rows.procedure_code_type as TEXT) as procedure_code_type
            , replace(cast(source_rows.procedure_code_7 as TEXT), '.', '') as procedure_code
        from source_rows
        where source_rows.procedure_code_7 is not null
          and cast(source_rows.procedure_code_type as TEXT) in ('icd-10-pcs', 'icd-9-pcs')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 8 as procedure_position
            , cast(source_rows.procedure_code_type as TEXT) as procedure_code_type
            , replace(cast(source_rows.procedure_code_8 as TEXT), '.', '') as procedure_code
        from source_rows
        where source_rows.procedure_code_8 is not null
          and cast(source_rows.procedure_code_type as TEXT) in ('icd-10-pcs', 'icd-9-pcs')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 9 as procedure_position
            , cast(source_rows.procedure_code_type as TEXT) as procedure_code_type
            , replace(cast(source_rows.procedure_code_9 as TEXT), '.', '') as procedure_code
        from source_rows
        where source_rows.procedure_code_9 is not null
          and cast(source_rows.procedure_code_type as TEXT) in ('icd-10-pcs', 'icd-9-pcs')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 10 as procedure_position
            , cast(source_rows.procedure_code_type as TEXT) as procedure_code_type
            , replace(cast(source_rows.procedure_code_10 as TEXT), '.', '') as procedure_code
        from source_rows
        where source_rows.procedure_code_10 is not null
          and cast(source_rows.procedure_code_type as TEXT) in ('icd-10-pcs', 'icd-9-pcs')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 11 as procedure_position
            , cast(source_rows.procedure_code_type as TEXT) as procedure_code_type
            , replace(cast(source_rows.procedure_code_11 as TEXT), '.', '') as procedure_code
        from source_rows
        where source_rows.procedure_code_11 is not null
          and cast(source_rows.procedure_code_type as TEXT) in ('icd-10-pcs', 'icd-9-pcs')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 12 as procedure_position
            , cast(source_rows.procedure_code_type as TEXT) as procedure_code_type
            , replace(cast(source_rows.procedure_code_12 as TEXT), '.', '') as procedure_code
        from source_rows
        where source_rows.procedure_code_12 is not null
          and cast(source_rows.procedure_code_type as TEXT) in ('icd-10-pcs', 'icd-9-pcs')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 13 as procedure_position
            , cast(source_rows.procedure_code_type as TEXT) as procedure_code_type
            , replace(cast(source_rows.procedure_code_13 as TEXT), '.', '') as procedure_code
        from source_rows
        where source_rows.procedure_code_13 is not null
          and cast(source_rows.procedure_code_type as TEXT) in ('icd-10-pcs', 'icd-9-pcs')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 14 as procedure_position
            , cast(source_rows.procedure_code_type as TEXT) as procedure_code_type
            , replace(cast(source_rows.procedure_code_14 as TEXT), '.', '') as procedure_code
        from source_rows
        where source_rows.procedure_code_14 is not null
          and cast(source_rows.procedure_code_type as TEXT) in ('icd-10-pcs', 'icd-9-pcs')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 15 as procedure_position
            , cast(source_rows.procedure_code_type as TEXT) as procedure_code_type
            , replace(cast(source_rows.procedure_code_15 as TEXT), '.', '') as procedure_code
        from source_rows
        where source_rows.procedure_code_15 is not null
          and cast(source_rows.procedure_code_type as TEXT) in ('icd-10-pcs', 'icd-9-pcs')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 16 as procedure_position
            , cast(source_rows.procedure_code_type as TEXT) as procedure_code_type
            , replace(cast(source_rows.procedure_code_16 as TEXT), '.', '') as procedure_code
        from source_rows
        where source_rows.procedure_code_16 is not null
          and cast(source_rows.procedure_code_type as TEXT) in ('icd-10-pcs', 'icd-9-pcs')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 17 as procedure_position
            , cast(source_rows.procedure_code_type as TEXT) as procedure_code_type
            , replace(cast(source_rows.procedure_code_17 as TEXT), '.', '') as procedure_code
        from source_rows
        where source_rows.procedure_code_17 is not null
          and cast(source_rows.procedure_code_type as TEXT) in ('icd-10-pcs', 'icd-9-pcs')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 18 as procedure_position
            , cast(source_rows.procedure_code_type as TEXT) as procedure_code_type
            , replace(cast(source_rows.procedure_code_18 as TEXT), '.', '') as procedure_code
        from source_rows
        where source_rows.procedure_code_18 is not null
          and cast(source_rows.procedure_code_type as TEXT) in ('icd-10-pcs', 'icd-9-pcs')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 19 as procedure_position
            , cast(source_rows.procedure_code_type as TEXT) as procedure_code_type
            , replace(cast(source_rows.procedure_code_19 as TEXT), '.', '') as procedure_code
        from source_rows
        where source_rows.procedure_code_19 is not null
          and cast(source_rows.procedure_code_type as TEXT) in ('icd-10-pcs', 'icd-9-pcs')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 20 as procedure_position
            , cast(source_rows.procedure_code_type as TEXT) as procedure_code_type
            , replace(cast(source_rows.procedure_code_20 as TEXT), '.', '') as procedure_code
        from source_rows
        where source_rows.procedure_code_20 is not null
          and cast(source_rows.procedure_code_type as TEXT) in ('icd-10-pcs', 'icd-9-pcs')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 21 as procedure_position
            , cast(source_rows.procedure_code_type as TEXT) as procedure_code_type
            , replace(cast(source_rows.procedure_code_21 as TEXT), '.', '') as procedure_code
        from source_rows
        where source_rows.procedure_code_21 is not null
          and cast(source_rows.procedure_code_type as TEXT) in ('icd-10-pcs', 'icd-9-pcs')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 22 as procedure_position
            , cast(source_rows.procedure_code_type as TEXT) as procedure_code_type
            , replace(cast(source_rows.procedure_code_22 as TEXT), '.', '') as procedure_code
        from source_rows
        where source_rows.procedure_code_22 is not null
          and cast(source_rows.procedure_code_type as TEXT) in ('icd-10-pcs', 'icd-9-pcs')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 23 as procedure_position
            , cast(source_rows.procedure_code_type as TEXT) as procedure_code_type
            , replace(cast(source_rows.procedure_code_23 as TEXT), '.', '') as procedure_code
        from source_rows
        where source_rows.procedure_code_23 is not null
          and cast(source_rows.procedure_code_type as TEXT) in ('icd-10-pcs', 'icd-9-pcs')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 24 as procedure_position
            , cast(source_rows.procedure_code_type as TEXT) as procedure_code_type
            , replace(cast(source_rows.procedure_code_24 as TEXT), '.', '') as procedure_code
        from source_rows
        where source_rows.procedure_code_24 is not null
          and cast(source_rows.procedure_code_type as TEXT) in ('icd-10-pcs', 'icd-9-pcs')
union all
select
              source_rows._dq_claim_id_key
            , source_rows._dq_claim_line_number_key
            , source_rows._dq_data_source_key
            , 25 as procedure_position
            , cast(source_rows.procedure_code_type as TEXT) as procedure_code_type
            , replace(cast(source_rows.procedure_code_25 as TEXT), '.', '') as procedure_code
        from source_rows
        where source_rows.procedure_code_25 is not null
          and cast(source_rows.procedure_code_type as TEXT) in ('icd-10-pcs', 'icd-9-pcs')
),

procedure_code_flags as (
    select
          procedure_codes._dq_claim_id_key
        , procedure_codes._dq_claim_line_number_key
        , procedure_codes._dq_data_source_key
        , cast(max(
            case
                when procedure_codes.procedure_code_type = 'icd-10-pcs'
                 and icd_10_procedure_lookup.icd_10_pcs is null
                    then 1
                when procedure_codes.procedure_code_type = 'icd-9-pcs'
                 and icd_9_procedure_lookup.icd_9_pcs is null
                    then 1
                else 0
            end
          ) as integer) as procedure_code_1_to_25_invalid
    from procedure_codes
    left join "terminology"."icd_10_pcs" as icd_10_procedure_lookup
        on procedure_codes.procedure_code_type = 'icd-10-pcs'
       and procedure_codes.procedure_code = replace(cast(icd_10_procedure_lookup.icd_10_pcs as TEXT), '.', '')
    left join "terminology"."icd_9_pcs" as icd_9_procedure_lookup
        on procedure_codes.procedure_code_type = 'icd-9-pcs'
       and procedure_codes.procedure_code = replace(cast(icd_9_procedure_lookup.icd_9_pcs as TEXT), '.', '')
    group by
          procedure_codes._dq_claim_id_key
        , procedure_codes._dq_claim_line_number_key
        , procedure_codes._dq_data_source_key
),

final as (
    select
          source_rows.claim_id
        , source_rows.claim_line_number
        , source_rows.data_source
        , cast(case when source_rows.claim_line_number is not null then case when source_rows.claim_line_number <= 0 then 1 else 0 end else null end as integer) as claim_line_number_not_positive
        , cast(case when 1 = 1 then case when source_rows.claim_type is null then 1 else 0 end else null end as integer) as claim_type_null
        , cast(case when source_rows.claim_type is not null then case when cast(source_rows.claim_type as TEXT) not in ('professional', 'institutional', 'undetermined') then 1 else 0 end else null end as integer) as claim_type_invalid
        , cast(case when cast(source_rows.claim_type as TEXT) = 'professional' then case when (source_rows.bill_type_code is not null or source_rows.drg_code is not null or source_rows.admit_type_code is not null or source_rows.admit_source_code is not null or source_rows.discharge_disposition_code is not null or source_rows.revenue_center_code is not null) then 1 else 0 end else null end as integer) as institutional_indicators_present_for_professional_claim
        , cast(case when 1 = 1 then case when source_rows.person_id is null then 1 else 0 end else null end as integer) as person_id_null
        , cast(case when source_rows.in_network_flag is not null then case when trim(cast(source_rows.in_network_flag as TEXT)) not in ('0', '1') then 1 else 0 end else null end as integer) as in_network_flag_invalid
        , cast(case when 1 = 1 then case when source_rows.claim_start_date is null then 1 else 0 end else null end as integer) as claim_start_date_null
        , cast(case when 1 = 1 then case when source_rows.claim_end_date is null then 1 else 0 end else null end as integer) as claim_end_date_null
        , cast(case when 1 = 1 then case when source_rows.claim_line_start_date is null then 1 else 0 end else null end as integer) as claim_line_start_date_null
        , cast(case when 1 = 1 then case when source_rows.claim_line_end_date is null then 1 else 0 end else null end as integer) as claim_line_end_date_null
        , cast(case when source_rows.claim_start_date is not null then case when source_rows.claim_start_date < 
    cast('2000-01-01' as date)
 or source_rows.claim_start_date > cast(now() as date) then 1 else 0 end else null end as integer) as claim_start_date_out_of_reasonable_range
        , cast(case when source_rows.claim_end_date is not null then case when source_rows.claim_end_date < 
    cast('2000-01-01' as date)
 or source_rows.claim_end_date > cast(now() as date) then 1 else 0 end else null end as integer) as claim_end_date_out_of_reasonable_range
        , cast(case when source_rows.claim_line_start_date is not null then case when source_rows.claim_line_start_date < 
    cast('2000-01-01' as date)
 or source_rows.claim_line_start_date > cast(now() as date) then 1 else 0 end else null end as integer) as claim_line_start_date_out_of_reasonable_range
        , cast(case when source_rows.claim_line_end_date is not null then case when source_rows.claim_line_end_date < 
    cast('2000-01-01' as date)
 or source_rows.claim_line_end_date > cast(now() as date) then 1 else 0 end else null end as integer) as claim_line_end_date_out_of_reasonable_range
        , cast(case when source_rows.claim_start_date is not null and source_rows.claim_end_date is not null then case when source_rows.claim_start_date > source_rows.claim_end_date then 1 else 0 end else null end as integer) as claim_start_after_claim_end
        , cast(case when source_rows.claim_line_start_date is not null and source_rows.claim_line_end_date is not null then case when source_rows.claim_line_start_date > source_rows.claim_line_end_date then 1 else 0 end else null end as integer) as claim_line_start_after_claim_line_end
        , cast(case when source_rows.admission_date is not null and source_rows.discharge_date is not null then case when source_rows.admission_date > source_rows.discharge_date then 1 else 0 end else null end as integer) as admission_date_after_discharge_date
        , cast(case when source_rows.admission_date is not null then case when source_rows.admission_date < 
    cast('2000-01-01' as date)
 or source_rows.admission_date > cast(now() as date) then 1 else 0 end else null end as integer) as admission_date_out_of_reasonable_range
        , cast(case when cast(source_rows.claim_type as TEXT) = 'institutional' and source_rows.bill_type_code is not null and 
  substring(cast(source_rows.bill_type_code as TEXT), 1, 2)
 in ('11', '12', '15', '16', '17', '18', '21', '22', '25', '26', '27', '28', '31', '41', '42', '45', '46', '47', '48', '61', '62', '65', '66', '67', '68', '82') then case when source_rows.admission_date is null then 1 else 0 end else null end as integer) as admission_date_null_for_inpatient_claim
        , cast(case when cast(source_rows.claim_type as TEXT) = 'institutional' and source_rows.bill_type_code is not null and 
  substring(cast(source_rows.bill_type_code as TEXT), 1, 2)
 in ('11', '12', '15', '16', '17', '18', '21', '22', '25', '26', '27', '28', '31', '41', '42', '45', '46', '47', '48', '61', '62', '65', '66', '67', '68', '82') then case when source_rows.discharge_date is null then 1 else 0 end else null end as integer) as discharge_date_null_for_inpatient_claim
        , cast(case when source_rows.discharge_date is not null then case when source_rows.discharge_date < 
    cast('2000-01-01' as date)
 or source_rows.discharge_date > cast(now() as date) then 1 else 0 end else null end as integer) as discharge_date_out_of_reasonable_range
        , cast(case when 1 = 1 then case when source_rows.paid_date is null then 1 else 0 end else null end as integer) as paid_date_null
        , cast(case when source_rows.paid_date is not null then case when source_rows.paid_date < 
    cast('2000-01-01' as date)
 or source_rows.paid_date > cast(now() as date) then 1 else 0 end else null end as integer) as paid_date_out_of_reasonable_range
        , cast(case when source_rows.paid_date is not null and source_rows.claim_end_date is not null then case when source_rows.paid_date < source_rows.claim_end_date then 1 else 0 end else null end as integer) as paid_date_before_claim_end_date
        
        , cast(case when source_rows.procedure_date_1 is not null then case when not (source_rows.procedure_date_1 >= 
    cast('2000-01-01' as date)
 and source_rows.procedure_date_1 <= cast(now() as date)) then 1 else 0 end else null end as integer) as procedure_date_1_outside_supported_date_range
        
        , cast(case when source_rows.procedure_date_2 is not null then case when not (source_rows.procedure_date_2 >= 
    cast('2000-01-01' as date)
 and source_rows.procedure_date_2 <= cast(now() as date)) then 1 else 0 end else null end as integer) as procedure_date_2_outside_supported_date_range
        
        , cast(case when source_rows.procedure_date_3 is not null then case when not (source_rows.procedure_date_3 >= 
    cast('2000-01-01' as date)
 and source_rows.procedure_date_3 <= cast(now() as date)) then 1 else 0 end else null end as integer) as procedure_date_3_outside_supported_date_range
        
        , cast(case when source_rows.procedure_date_4 is not null then case when not (source_rows.procedure_date_4 >= 
    cast('2000-01-01' as date)
 and source_rows.procedure_date_4 <= cast(now() as date)) then 1 else 0 end else null end as integer) as procedure_date_4_outside_supported_date_range
        
        , cast(case when source_rows.procedure_date_5 is not null then case when not (source_rows.procedure_date_5 >= 
    cast('2000-01-01' as date)
 and source_rows.procedure_date_5 <= cast(now() as date)) then 1 else 0 end else null end as integer) as procedure_date_5_outside_supported_date_range
        
        , cast(case when source_rows.procedure_date_6 is not null then case when not (source_rows.procedure_date_6 >= 
    cast('2000-01-01' as date)
 and source_rows.procedure_date_6 <= cast(now() as date)) then 1 else 0 end else null end as integer) as procedure_date_6_outside_supported_date_range
        
        , cast(case when source_rows.procedure_date_7 is not null then case when not (source_rows.procedure_date_7 >= 
    cast('2000-01-01' as date)
 and source_rows.procedure_date_7 <= cast(now() as date)) then 1 else 0 end else null end as integer) as procedure_date_7_outside_supported_date_range
        
        , cast(case when source_rows.procedure_date_8 is not null then case when not (source_rows.procedure_date_8 >= 
    cast('2000-01-01' as date)
 and source_rows.procedure_date_8 <= cast(now() as date)) then 1 else 0 end else null end as integer) as procedure_date_8_outside_supported_date_range
        
        , cast(case when source_rows.procedure_date_9 is not null then case when not (source_rows.procedure_date_9 >= 
    cast('2000-01-01' as date)
 and source_rows.procedure_date_9 <= cast(now() as date)) then 1 else 0 end else null end as integer) as procedure_date_9_outside_supported_date_range
        
        , cast(case when source_rows.procedure_date_10 is not null then case when not (source_rows.procedure_date_10 >= 
    cast('2000-01-01' as date)
 and source_rows.procedure_date_10 <= cast(now() as date)) then 1 else 0 end else null end as integer) as procedure_date_10_outside_supported_date_range
        
        , cast(case when source_rows.procedure_date_11 is not null then case when not (source_rows.procedure_date_11 >= 
    cast('2000-01-01' as date)
 and source_rows.procedure_date_11 <= cast(now() as date)) then 1 else 0 end else null end as integer) as procedure_date_11_outside_supported_date_range
        
        , cast(case when source_rows.procedure_date_12 is not null then case when not (source_rows.procedure_date_12 >= 
    cast('2000-01-01' as date)
 and source_rows.procedure_date_12 <= cast(now() as date)) then 1 else 0 end else null end as integer) as procedure_date_12_outside_supported_date_range
        
        , cast(case when source_rows.procedure_date_13 is not null then case when not (source_rows.procedure_date_13 >= 
    cast('2000-01-01' as date)
 and source_rows.procedure_date_13 <= cast(now() as date)) then 1 else 0 end else null end as integer) as procedure_date_13_outside_supported_date_range
        
        , cast(case when source_rows.procedure_date_14 is not null then case when not (source_rows.procedure_date_14 >= 
    cast('2000-01-01' as date)
 and source_rows.procedure_date_14 <= cast(now() as date)) then 1 else 0 end else null end as integer) as procedure_date_14_outside_supported_date_range
        
        , cast(case when source_rows.procedure_date_15 is not null then case when not (source_rows.procedure_date_15 >= 
    cast('2000-01-01' as date)
 and source_rows.procedure_date_15 <= cast(now() as date)) then 1 else 0 end else null end as integer) as procedure_date_15_outside_supported_date_range
        
        , cast(case when source_rows.procedure_date_16 is not null then case when not (source_rows.procedure_date_16 >= 
    cast('2000-01-01' as date)
 and source_rows.procedure_date_16 <= cast(now() as date)) then 1 else 0 end else null end as integer) as procedure_date_16_outside_supported_date_range
        
        , cast(case when source_rows.procedure_date_17 is not null then case when not (source_rows.procedure_date_17 >= 
    cast('2000-01-01' as date)
 and source_rows.procedure_date_17 <= cast(now() as date)) then 1 else 0 end else null end as integer) as procedure_date_17_outside_supported_date_range
        
        , cast(case when source_rows.procedure_date_18 is not null then case when not (source_rows.procedure_date_18 >= 
    cast('2000-01-01' as date)
 and source_rows.procedure_date_18 <= cast(now() as date)) then 1 else 0 end else null end as integer) as procedure_date_18_outside_supported_date_range
        
        , cast(case when source_rows.procedure_date_19 is not null then case when not (source_rows.procedure_date_19 >= 
    cast('2000-01-01' as date)
 and source_rows.procedure_date_19 <= cast(now() as date)) then 1 else 0 end else null end as integer) as procedure_date_19_outside_supported_date_range
        
        , cast(case when source_rows.procedure_date_20 is not null then case when not (source_rows.procedure_date_20 >= 
    cast('2000-01-01' as date)
 and source_rows.procedure_date_20 <= cast(now() as date)) then 1 else 0 end else null end as integer) as procedure_date_20_outside_supported_date_range
        
        , cast(case when source_rows.procedure_date_21 is not null then case when not (source_rows.procedure_date_21 >= 
    cast('2000-01-01' as date)
 and source_rows.procedure_date_21 <= cast(now() as date)) then 1 else 0 end else null end as integer) as procedure_date_21_outside_supported_date_range
        
        , cast(case when source_rows.procedure_date_22 is not null then case when not (source_rows.procedure_date_22 >= 
    cast('2000-01-01' as date)
 and source_rows.procedure_date_22 <= cast(now() as date)) then 1 else 0 end else null end as integer) as procedure_date_22_outside_supported_date_range
        
        , cast(case when source_rows.procedure_date_23 is not null then case when not (source_rows.procedure_date_23 >= 
    cast('2000-01-01' as date)
 and source_rows.procedure_date_23 <= cast(now() as date)) then 1 else 0 end else null end as integer) as procedure_date_23_outside_supported_date_range
        
        , cast(case when source_rows.procedure_date_24 is not null then case when not (source_rows.procedure_date_24 >= 
    cast('2000-01-01' as date)
 and source_rows.procedure_date_24 <= cast(now() as date)) then 1 else 0 end else null end as integer) as procedure_date_24_outside_supported_date_range
        
        , cast(case when source_rows.procedure_date_25 is not null then case when not (source_rows.procedure_date_25 >= 
    cast('2000-01-01' as date)
 and source_rows.procedure_date_25 <= cast(now() as date)) then 1 else 0 end else null end as integer) as procedure_date_25_outside_supported_date_range
        
        , cast(case when source_rows.file_date is not null then case when not (source_rows.file_date >= 
    cast('2000-01-01' as date)
 and source_rows.file_date <= cast(now() as date)) then 1 else 0 end else null end as integer) as file_date_outside_supported_date_range
        , cast(case when source_rows.ingest_datetime is not null then case when not (cast(source_rows.ingest_datetime as date) >= 
    cast('2000-01-01' as date)
 and cast(source_rows.ingest_datetime as date) <= cast(now() as date)) then 1 else 0 end else null end as integer) as ingest_datetime_out_of_reasonable_range
        , cast(case when 1 = 1 then case when source_rows.paid_amount is null then 1 else 0 end else null end as integer) as paid_amount_null
        , cast(case when source_rows.paid_amount is not null then case when source_rows.paid_amount < 0 then 1 else 0 end else null end as integer) as paid_amount_lt_zero
        , cast(case when 1 = 1 then case when source_rows.allowed_amount is null then 1 else 0 end else null end as integer) as allowed_amount_null
        , cast(case when source_rows.allowed_amount is not null then case when source_rows.allowed_amount < 0 then 1 else 0 end else null end as integer) as allowed_amount_lt_zero
        , cast(case when source_rows.paid_amount is not null and source_rows.allowed_amount is not null then case when source_rows.paid_amount > source_rows.allowed_amount then 1 else 0 end else null end as integer) as paid_amount_gt_allowed_amount
        , cast(case when cast(source_rows.claim_type as TEXT) = 'institutional' and source_rows.admit_source_code is not null then case when admit_source_lookup.admit_source_code is null then 1 else 0 end else null end as integer) as admit_source_code_invalid
        , cast(case when cast(source_rows.claim_type as TEXT) = 'institutional' and source_rows.admit_type_code is not null then case when admit_type_lookup.admit_type_code is null then 1 else 0 end else null end as integer) as admit_type_code_invalid
        , cast(case when cast(source_rows.claim_type as TEXT) = 'institutional' and source_rows.discharge_disposition_code is not null then case when discharge_disposition_lookup.discharge_disposition_code is null then 1 else 0 end else null end as integer) as discharge_disposition_code_invalid
        , cast(case when cast(source_rows.claim_type as TEXT) = 'professional' and source_rows.place_of_service_code is not null then case when place_of_service_lookup.place_of_service_code is null then 1 else 0 end else null end as integer) as place_of_service_code_invalid
        , cast(case when cast(source_rows.claim_type as TEXT) = 'institutional' and source_rows.bill_type_code is not null then case when bill_type_lookup.bill_type_code is null then 1 else 0 end else null end as integer) as bill_type_code_invalid
        , cast(case when cast(source_rows.claim_type as TEXT) = 'institutional' and source_rows.revenue_center_code is not null then case when revenue_center_lookup.revenue_center_code is null then 1 else 0 end else null end as integer) as revenue_center_code_invalid
        , cast(case when cast(source_rows.claim_type as TEXT) = 'professional' then case when source_rows.place_of_service_code is null then 1 else 0 end else null end as integer) as place_of_service_code_null_for_professional_claim
        , cast(case when cast(source_rows.claim_type as TEXT) = 'institutional' then case when source_rows.place_of_service_code is not null then 1 else 0 end else null end as integer) as place_of_service_code_present_for_institutional_claim
        , cast(case when cast(source_rows.claim_type as TEXT) = 'institutional' then case when source_rows.bill_type_code is null then 1 else 0 end else null end as integer) as bill_type_code_null_for_institutional_claim
        , cast(case when cast(source_rows.claim_type as TEXT) = 'institutional' then case when source_rows.revenue_center_code is null then 1 else 0 end else null end as integer) as revenue_center_code_null_for_institutional_claim
        , cast(case when cast(source_rows.claim_type as TEXT) = 'professional' then case when source_rows.hcpcs_code is null then 1 else 0 end else null end as integer) as hcpcs_code_null_for_professional_claim
        , cast(case when source_rows.rendering_npi is not null then case when rendering_provider_lookup.npi is null then 1 else 0 end else null end as integer) as rendering_npi_invalid
        , cast(case when source_rows.billing_npi is not null then case when billing_provider_lookup.npi is null then 1 else 0 end else null end as integer) as billing_npi_invalid
        , cast(case when source_rows.facility_npi is not null then case when facility_provider_lookup.npi is null then 1 else 0 end else null end as integer) as facility_npi_invalid
        , cast(case when 1 = 1 then case when source_rows.rendering_npi is null then 1 else 0 end else null end as integer) as rendering_npi_null
        , cast(case when 1 = 1 then case when source_rows.billing_npi is null then 1 else 0 end else null end as integer) as billing_npi_null
        , cast(case when cast(source_rows.claim_type as TEXT) = 'institutional' and source_rows.bill_type_code is not null and 
  substring(cast(source_rows.bill_type_code as TEXT), 1, 2)
 in ('11', '12', '15', '16', '17', '18', '21', '22', '25', '26', '27', '28', '31', '41', '42', '45', '46', '47', '48', '61', '62', '65', '66', '67', '68', '82') then case when source_rows.facility_npi is null then 1 else 0 end else null end as integer) as facility_npi_null_for_inpatient_claim
        , cast(case when source_rows.drg_code is not null then case when source_rows.drg_code_type is null then 1 else 0 end else null end as integer) as drg_code_type_null_when_drg_code_present
        , cast(case when source_rows.drg_code_type is not null then case when source_rows.drg_code_type is not null and cast(source_rows.drg_code_type as TEXT) not in ('ms-drg', 'apr-drg') then 1 else 0 end else null end as integer) as drg_code_type_invalid
        , cast(case when cast(source_rows.claim_type as TEXT) = 'institutional' and source_rows.drg_code is not null and cast(source_rows.drg_code_type as TEXT) in ('ms-drg', 'apr-drg') then case when ms_drg_lookup.ms_drg_code is null and apr_drg_lookup.apr_drg_code is null then 1 else 0 end else null end as integer) as drg_code_invalid
        , cast(case when cast(source_rows.claim_type as TEXT) = 'institutional' and source_rows.bill_type_code is not null and 
  substring(cast(source_rows.bill_type_code as TEXT), 1, 2)
 in ('11', '12') then case when source_rows.drg_code is null then 1 else 0 end else null end as integer) as drg_code_null_for_acute_inpatient_claim
        , cast(case when 1 = 1 then case when source_rows.diagnosis_code_1 is null then 1 else 0 end else null end as integer) as diagnosis_code_1_null
        , cast(case when (source_rows.diagnosis_code_1 is not null or source_rows.diagnosis_code_2 is not null or source_rows.diagnosis_code_3 is not null or source_rows.diagnosis_code_4 is not null or source_rows.diagnosis_code_5 is not null or source_rows.diagnosis_code_6 is not null or source_rows.diagnosis_code_7 is not null or source_rows.diagnosis_code_8 is not null or source_rows.diagnosis_code_9 is not null or source_rows.diagnosis_code_10 is not null or source_rows.diagnosis_code_11 is not null or source_rows.diagnosis_code_12 is not null or source_rows.diagnosis_code_13 is not null or source_rows.diagnosis_code_14 is not null or source_rows.diagnosis_code_15 is not null or source_rows.diagnosis_code_16 is not null or source_rows.diagnosis_code_17 is not null or source_rows.diagnosis_code_18 is not null or source_rows.diagnosis_code_19 is not null or source_rows.diagnosis_code_20 is not null or source_rows.diagnosis_code_21 is not null or source_rows.diagnosis_code_22 is not null or source_rows.diagnosis_code_23 is not null or source_rows.diagnosis_code_24 is not null or source_rows.diagnosis_code_25 is not null) then case when source_rows.diagnosis_code_type is null then 1 else 0 end else null end as integer) as diagnosis_code_type_null_when_diagnosis_code_present
        , cast(case when source_rows.diagnosis_code_type is not null then case when source_rows.diagnosis_code_type is not null and cast(source_rows.diagnosis_code_type as TEXT) not in ('icd-10-cm', 'icd-9-cm') then 1 else 0 end else null end as integer) as diagnosis_code_type_invalid
        , cast(case when source_rows.diagnosis_code_1 is not null and cast(source_rows.diagnosis_code_type as TEXT) in ('icd-10-cm', 'icd-9-cm') then case when coalesce(diagnosis_code_flags.diagnosis_code_1_invalid, 0) = 1 then 1 else 0 end else null end as integer) as diagnosis_code_1_invalid
        , cast(case when (source_rows.diagnosis_code_2 is not null or source_rows.diagnosis_code_3 is not null or source_rows.diagnosis_code_4 is not null or source_rows.diagnosis_code_5 is not null or source_rows.diagnosis_code_6 is not null or source_rows.diagnosis_code_7 is not null or source_rows.diagnosis_code_8 is not null or source_rows.diagnosis_code_9 is not null or source_rows.diagnosis_code_10 is not null or source_rows.diagnosis_code_11 is not null or source_rows.diagnosis_code_12 is not null or source_rows.diagnosis_code_13 is not null or source_rows.diagnosis_code_14 is not null or source_rows.diagnosis_code_15 is not null or source_rows.diagnosis_code_16 is not null or source_rows.diagnosis_code_17 is not null or source_rows.diagnosis_code_18 is not null or source_rows.diagnosis_code_19 is not null or source_rows.diagnosis_code_20 is not null or source_rows.diagnosis_code_21 is not null or source_rows.diagnosis_code_22 is not null or source_rows.diagnosis_code_23 is not null or source_rows.diagnosis_code_24 is not null or source_rows.diagnosis_code_25 is not null) and cast(source_rows.diagnosis_code_type as TEXT) in ('icd-10-cm', 'icd-9-cm') then case when coalesce(diagnosis_code_flags.diagnosis_code_2_to_25_invalid, 0) = 1 then 1 else 0 end else null end as integer) as diagnosis_code_2_to_25_invalid
        , cast(case when (source_rows.procedure_code_1 is not null or source_rows.procedure_code_2 is not null or source_rows.procedure_code_3 is not null or source_rows.procedure_code_4 is not null or source_rows.procedure_code_5 is not null or source_rows.procedure_code_6 is not null or source_rows.procedure_code_7 is not null or source_rows.procedure_code_8 is not null or source_rows.procedure_code_9 is not null or source_rows.procedure_code_10 is not null or source_rows.procedure_code_11 is not null or source_rows.procedure_code_12 is not null or source_rows.procedure_code_13 is not null or source_rows.procedure_code_14 is not null or source_rows.procedure_code_15 is not null or source_rows.procedure_code_16 is not null or source_rows.procedure_code_17 is not null or source_rows.procedure_code_18 is not null or source_rows.procedure_code_19 is not null or source_rows.procedure_code_20 is not null or source_rows.procedure_code_21 is not null or source_rows.procedure_code_22 is not null or source_rows.procedure_code_23 is not null or source_rows.procedure_code_24 is not null or source_rows.procedure_code_25 is not null) then case when source_rows.procedure_code_type is null then 1 else 0 end else null end as integer) as procedure_code_type_null_when_procedure_code_present
        , cast(case when source_rows.procedure_code_type is not null then case when source_rows.procedure_code_type is not null and cast(source_rows.procedure_code_type as TEXT) not in ('icd-10-pcs', 'icd-9-pcs') then 1 else 0 end else null end as integer) as procedure_code_type_invalid
        , cast(case when (source_rows.procedure_code_1 is not null or source_rows.procedure_code_2 is not null or source_rows.procedure_code_3 is not null or source_rows.procedure_code_4 is not null or source_rows.procedure_code_5 is not null or source_rows.procedure_code_6 is not null or source_rows.procedure_code_7 is not null or source_rows.procedure_code_8 is not null or source_rows.procedure_code_9 is not null or source_rows.procedure_code_10 is not null or source_rows.procedure_code_11 is not null or source_rows.procedure_code_12 is not null or source_rows.procedure_code_13 is not null or source_rows.procedure_code_14 is not null or source_rows.procedure_code_15 is not null or source_rows.procedure_code_16 is not null or source_rows.procedure_code_17 is not null or source_rows.procedure_code_18 is not null or source_rows.procedure_code_19 is not null or source_rows.procedure_code_20 is not null or source_rows.procedure_code_21 is not null or source_rows.procedure_code_22 is not null or source_rows.procedure_code_23 is not null or source_rows.procedure_code_24 is not null or source_rows.procedure_code_25 is not null) and cast(source_rows.procedure_code_type as TEXT) in ('icd-10-pcs', 'icd-9-pcs') then case when coalesce(procedure_code_flags.procedure_code_1_to_25_invalid, 0) = 1 then 1 else 0 end else null end as integer) as procedure_code_1_to_25_invalid
        , cast(case when source_rows.person_id is not null and source_rows.member_id is not null and source_rows.payer is not null and source_rows.plan is not null and source_rows.data_source is not null and coalesce(source_rows.claim_line_start_date, source_rows.claim_start_date, source_rows.admission_date) is not null and (coalesce(source_rows.claim_line_start_date, source_rows.claim_start_date, source_rows.admission_date) >= 
    cast('2000-01-01' as date)
 and coalesce(source_rows.claim_line_start_date, source_rows.claim_start_date, source_rows.admission_date) <= cast(now() as date)) and (coalesce(source_rows.claim_line_start_date, source_rows.claim_start_date, source_rows.admission_date) >= 
    cast('1900-01-01' as date)
 and coalesce(source_rows.claim_line_start_date, source_rows.claim_start_date, source_rows.admission_date) <= 
    cast('2100-12-31' as date)
) and coalesce(source_rows.claim_line_start_date, source_rows.claim_start_date, source_rows.admission_date) <= cast(cast('2026-01-01 00:00:00' as timestamp) as date) then case when matching_eligibility_member_months._dq_has_matching_eligibility is null then 1 else 0 end else null end as integer) as no_matching_eligibility_span
    from source_rows
    left join matching_eligibility_member_months
        on matching_eligibility_member_months.person_id = source_rows.person_id
       and matching_eligibility_member_months.member_id = source_rows.member_id
       and matching_eligibility_member_months.payer = source_rows.payer
       and matching_eligibility_member_months.plan = source_rows.plan
       and matching_eligibility_member_months.data_source = source_rows.data_source
       and matching_eligibility_member_months._dq_claim_year = cast(date_part('year', coalesce(source_rows.claim_line_start_date, source_rows.claim_start_date, source_rows.admission_date)) as integer)
       and matching_eligibility_member_months._dq_claim_month = cast(date_part('month', coalesce(source_rows.claim_line_start_date, source_rows.claim_start_date, source_rows.admission_date)) as integer)
    left join diagnosis_code_flags
        on (
            source_rows._dq_claim_id_key = diagnosis_code_flags._dq_claim_id_key
            or (source_rows._dq_claim_id_key is null and diagnosis_code_flags._dq_claim_id_key is null)
        )
       and (
            source_rows._dq_claim_line_number_key = diagnosis_code_flags._dq_claim_line_number_key
            or (source_rows._dq_claim_line_number_key is null and diagnosis_code_flags._dq_claim_line_number_key is null)
       )
       and (
            source_rows._dq_data_source_key = diagnosis_code_flags._dq_data_source_key
            or (source_rows._dq_data_source_key is null and diagnosis_code_flags._dq_data_source_key is null)
       )
    left join procedure_code_flags
        on (
            source_rows._dq_claim_id_key = procedure_code_flags._dq_claim_id_key
            or (source_rows._dq_claim_id_key is null and procedure_code_flags._dq_claim_id_key is null)
        )
       and (
            source_rows._dq_claim_line_number_key = procedure_code_flags._dq_claim_line_number_key
            or (source_rows._dq_claim_line_number_key is null and procedure_code_flags._dq_claim_line_number_key is null)
       )
       and (
            source_rows._dq_data_source_key = procedure_code_flags._dq_data_source_key
            or (source_rows._dq_data_source_key is null and procedure_code_flags._dq_data_source_key is null)
       )
    left join "terminology"."admit_source" as admit_source_lookup
        on cast(source_rows.admit_source_code as TEXT) = cast(admit_source_lookup.admit_source_code as TEXT)
    left join "terminology"."admit_type" as admit_type_lookup
        on cast(source_rows.admit_type_code as TEXT) = cast(admit_type_lookup.admit_type_code as TEXT)
    left join "terminology"."discharge_disposition" as discharge_disposition_lookup
        on cast(source_rows.discharge_disposition_code as TEXT) = cast(discharge_disposition_lookup.discharge_disposition_code as TEXT)
    left join "terminology"."place_of_service" as place_of_service_lookup
        on cast(source_rows.place_of_service_code as TEXT) = cast(place_of_service_lookup.place_of_service_code as TEXT)
    left join "terminology"."bill_type" as bill_type_lookup
        on cast(source_rows.bill_type_code as TEXT) = cast(bill_type_lookup.bill_type_code as TEXT)
    left join "terminology"."revenue_center" as revenue_center_lookup
        on cast(source_rows.revenue_center_code as TEXT) = cast(revenue_center_lookup.revenue_center_code as TEXT)
    left join "provider_data"."provider" as rendering_provider_lookup
        on cast(source_rows.rendering_npi as TEXT) = cast(rendering_provider_lookup.npi as TEXT)
    left join "provider_data"."provider" as billing_provider_lookup
        on cast(source_rows.billing_npi as TEXT) = cast(billing_provider_lookup.npi as TEXT)
    left join "provider_data"."provider" as facility_provider_lookup
        on cast(source_rows.facility_npi as TEXT) = cast(facility_provider_lookup.npi as TEXT)
    left join "terminology"."ms_drg" as ms_drg_lookup
        on cast(source_rows.drg_code_type as TEXT) = 'ms-drg'
       and cast(source_rows.drg_code as TEXT) = cast(ms_drg_lookup.ms_drg_code as TEXT)
    left join "terminology"."apr_drg" as apr_drg_lookup
        on cast(source_rows.drg_code_type as TEXT) = 'apr-drg'
       and cast(source_rows.drg_code as TEXT) = cast(apr_drg_lookup.apr_drg_code as TEXT)
)

select *
from final