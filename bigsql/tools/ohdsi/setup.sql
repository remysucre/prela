CREATE TABLE main.cohort_out (cohort_definition_id BIGINT, subject_id BIGINT, cohort_start_date DATE, cohort_end_date DATE);
CREATE TABLE main.cohort_censor_stats (cohort_definition_id BIGINT, lost_count BIGINT);
CREATE TABLE main.cohort_inclusion (cohort_definition_id BIGINT, rule_sequence INT, name VARCHAR, description VARCHAR);
CREATE TABLE main.cohort_inclusion_result (cohort_definition_id BIGINT, inclusion_rule_mask BIGINT, person_count BIGINT, mode_id INT);
CREATE TABLE main.cohort_inclusion_stats (cohort_definition_id BIGINT, rule_sequence INT, person_count BIGINT, gain_count BIGINT, person_total BIGINT, mode_id INT);
CREATE TABLE main.cohort_summary_stats (cohort_definition_id BIGINT, base_count BIGINT, final_count BIGINT, mode_id INT);
