-- OHDSI PhenotypeLibrary cohort 29, rendered by SqlRender for duckdb, temp tables folded into CTEs
WITH Codesets AS (
SELECT CAST(codeset_id AS int) AS codeset_id, CAST(concept_id AS bigint) AS concept_id FROM (
SELECT * FROM (
SELECT 0 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (201254,435216,40484648))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (201254,435216,40484648))
) I
) C UNION ALL 
SELECT 2 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (4297650,4334806,46273442,2107572,2107561,4058299,4107913,2108721,4102493,80809,2107560,2107558,2107559,4083556,4035611))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4297650,4334806,46273442,2107572,2107561,4058299,4107913,2108721,4102493,80809,2107560,2107558,2107559,4083556,4035611))
) I
) C UNION ALL 
SELECT 3 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (75614,140168))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (75614,140168))
) I
) C UNION ALL 
SELECT 4 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (81931))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (81931))
) I
) C UNION ALL 
SELECT 5 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (374919))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (374919))
) I
) C UNION ALL 
SELECT 6 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (46273369,4145240,257628))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (46273369,4145240,257628))
) I
) C UNION ALL 
SELECT 7 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (443394))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (443394))
) I
) C UNION ALL 
SELECT 8 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (4232076))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4232076))
) I
) C UNION ALL 
SELECT 9 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (254443))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (254443))
) I
) C UNION ALL 
SELECT 10 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (135215))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (135215))
) I
) C UNION ALL 
SELECT 11 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (76685,432893))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (76685,432893))
) I
) C UNION ALL 
SELECT 12 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (4137275))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4137275))
) I
) C UNION ALL 
SELECT 13 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (432295))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (432295))
) I
) C UNION ALL 
SELECT 14 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (194992))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (194992))
) I
) C UNION ALL 
SELECT 15 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (46272236,4331739,441928,255304,4105026,46270482,134442,4063582))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (46272236,4331739,441928,255304,4105026,46270482,134442,4063582))
) I
) C UNION ALL 
SELECT 16 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (438688,4262578,45772123,4105005))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (438688,4262578,45772123,4105005))
) I
) C UNION ALL 
SELECT 17 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (46273477,81893,46273478,46274082,46269952,4116142))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (46273477,81893,46273478,46274082,46269952,4116142))
) I
) C UNION ALL 
SELECT 18 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (201606,46269889,46269999))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (201606,46269889,46269999))
) I
) C
)
) AS _t(codeset_id, concept_id)
),
qualified_events AS (
SELECT
event_id, person_id, start_date, end_date, op_start_date, op_end_date, visit_occurrence_id
FROM
(
  select pe.event_id, pe.person_id, pe.start_date, pe.end_date, pe.op_start_date, pe.op_end_date, row_number() over (partition by pe.person_id order by pe.start_date ASC) as ordinal, cast(pe.visit_occurrence_id as bigint) as visit_occurrence_id
  FROM (-- Begin Primary Events
select P.ordinal as event_id, P.person_id, P.start_date, P.end_date, op_start_date, op_end_date, cast(P.visit_occurrence_id as bigint) as visit_occurrence_id
FROM
(
  select E.person_id, E.start_date, E.end_date,
         row_number() OVER (PARTITION BY E.person_id ORDER BY E.sort_date ASC, E.event_id) ordinal,
         OP.observation_period_start_date as op_start_date, OP.observation_period_end_date as op_end_date, cast(E.visit_occurrence_id as bigint) as visit_occurrence_id
  FROM 
  (
SELECT C.person_id, C.condition_occurrence_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id, C.start_date as sort_date
FROM 
(
  SELECT co.person_id,co.condition_occurrence_id,co.condition_concept_id,co.visit_occurrence_id,co.condition_start_date as start_date, COALESCE(co.condition_end_date, (co.condition_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.CONDITION_OCCURRENCE co
  JOIN Codesets cs on (co.condition_concept_id = cs.concept_id and cs.codeset_id = 0)
) C
UNION ALL
SELECT C.person_id, C.condition_occurrence_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id, C.start_date as sort_date
FROM 
(
  SELECT co.person_id,co.condition_occurrence_id,co.condition_concept_id,co.visit_occurrence_id,co.condition_start_date as start_date, COALESCE(co.condition_end_date, (co.condition_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.CONDITION_OCCURRENCE co
  JOIN Codesets cs on (co.condition_concept_id = cs.concept_id and cs.codeset_id = 2)
) C
UNION ALL
select C.person_id, C.observation_id as event_id, C.start_date, C.END_DATE,
       C.visit_occurrence_id, C.start_date as sort_date
from 
(
  select o.person_id,o.observation_id,o.observation_concept_id,o.visit_occurrence_id,o.value_as_number,o.observation_date as start_date, (o.observation_date + TO_DAYS(CAST(1 AS INTEGER))) as end_date 
  FROM main.OBSERVATION o
JOIN Codesets cs on (o.observation_concept_id = cs.concept_id and cs.codeset_id = 2)
) C
UNION ALL
SELECT C.person_id, C.condition_occurrence_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id, C.start_date as sort_date
FROM 
(
  SELECT co.person_id,co.condition_occurrence_id,co.condition_concept_id,co.visit_occurrence_id,co.condition_start_date as start_date, COALESCE(co.condition_end_date, (co.condition_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.CONDITION_OCCURRENCE co
  JOIN Codesets cs on (co.condition_concept_id = cs.concept_id and cs.codeset_id = 3)
) C
UNION ALL
SELECT C.person_id, C.condition_occurrence_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id, C.start_date as sort_date
FROM 
(
  SELECT co.person_id,co.condition_occurrence_id,co.condition_concept_id,co.visit_occurrence_id,co.condition_start_date as start_date, COALESCE(co.condition_end_date, (co.condition_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.CONDITION_OCCURRENCE co
  JOIN Codesets cs on (co.condition_concept_id = cs.concept_id and cs.codeset_id = 4)
) C
UNION ALL
SELECT C.person_id, C.condition_occurrence_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id, C.start_date as sort_date
FROM 
(
  SELECT co.person_id,co.condition_occurrence_id,co.condition_concept_id,co.visit_occurrence_id,co.condition_start_date as start_date, COALESCE(co.condition_end_date, (co.condition_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.CONDITION_OCCURRENCE co
  JOIN Codesets cs on (co.condition_concept_id = cs.concept_id and cs.codeset_id = 5)
) C
UNION ALL
SELECT C.person_id, C.condition_occurrence_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id, C.start_date as sort_date
FROM 
(
  SELECT co.person_id,co.condition_occurrence_id,co.condition_concept_id,co.visit_occurrence_id,co.condition_start_date as start_date, COALESCE(co.condition_end_date, (co.condition_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.CONDITION_OCCURRENCE co
  JOIN Codesets cs on (co.condition_concept_id = cs.concept_id and cs.codeset_id = 6)
) C
UNION ALL
SELECT C.person_id, C.condition_occurrence_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id, C.start_date as sort_date
FROM 
(
  SELECT co.person_id,co.condition_occurrence_id,co.condition_concept_id,co.visit_occurrence_id,co.condition_start_date as start_date, COALESCE(co.condition_end_date, (co.condition_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.CONDITION_OCCURRENCE co
  JOIN Codesets cs on (co.condition_concept_id = cs.concept_id and cs.codeset_id = 7)
) C
UNION ALL
SELECT C.person_id, C.condition_occurrence_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id, C.start_date as sort_date
FROM 
(
  SELECT co.person_id,co.condition_occurrence_id,co.condition_concept_id,co.visit_occurrence_id,co.condition_start_date as start_date, COALESCE(co.condition_end_date, (co.condition_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.CONDITION_OCCURRENCE co
  JOIN Codesets cs on (co.condition_concept_id = cs.concept_id and cs.codeset_id = 8)
) C
UNION ALL
SELECT C.person_id, C.condition_occurrence_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id, C.start_date as sort_date
FROM 
(
  SELECT co.person_id,co.condition_occurrence_id,co.condition_concept_id,co.visit_occurrence_id,co.condition_start_date as start_date, COALESCE(co.condition_end_date, (co.condition_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.CONDITION_OCCURRENCE co
  JOIN Codesets cs on (co.condition_concept_id = cs.concept_id and cs.codeset_id = 9)
) C
UNION ALL
SELECT C.person_id, C.condition_occurrence_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id, C.start_date as sort_date
FROM 
(
  SELECT co.person_id,co.condition_occurrence_id,co.condition_concept_id,co.visit_occurrence_id,co.condition_start_date as start_date, COALESCE(co.condition_end_date, (co.condition_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.CONDITION_OCCURRENCE co
  JOIN Codesets cs on (co.condition_concept_id = cs.concept_id and cs.codeset_id = 10)
) C
UNION ALL
SELECT C.person_id, C.condition_occurrence_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id, C.start_date as sort_date
FROM 
(
  SELECT co.person_id,co.condition_occurrence_id,co.condition_concept_id,co.visit_occurrence_id,co.condition_start_date as start_date, COALESCE(co.condition_end_date, (co.condition_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.CONDITION_OCCURRENCE co
  JOIN Codesets cs on (co.condition_concept_id = cs.concept_id and cs.codeset_id = 11)
) C
UNION ALL
SELECT C.person_id, C.condition_occurrence_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id, C.start_date as sort_date
FROM 
(
  SELECT co.person_id,co.condition_occurrence_id,co.condition_concept_id,co.visit_occurrence_id,co.condition_start_date as start_date, COALESCE(co.condition_end_date, (co.condition_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.CONDITION_OCCURRENCE co
  JOIN Codesets cs on (co.condition_concept_id = cs.concept_id and cs.codeset_id = 12)
) C
UNION ALL
SELECT C.person_id, C.condition_occurrence_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id, C.start_date as sort_date
FROM 
(
  SELECT co.person_id,co.condition_occurrence_id,co.condition_concept_id,co.visit_occurrence_id,co.condition_start_date as start_date, COALESCE(co.condition_end_date, (co.condition_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.CONDITION_OCCURRENCE co
  JOIN Codesets cs on (co.condition_concept_id = cs.concept_id and cs.codeset_id = 13)
) C
UNION ALL
SELECT C.person_id, C.condition_occurrence_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id, C.start_date as sort_date
FROM 
(
  SELECT co.person_id,co.condition_occurrence_id,co.condition_concept_id,co.visit_occurrence_id,co.condition_start_date as start_date, COALESCE(co.condition_end_date, (co.condition_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.CONDITION_OCCURRENCE co
  JOIN Codesets cs on (co.condition_concept_id = cs.concept_id and cs.codeset_id = 14)
) C
UNION ALL
SELECT C.person_id, C.condition_occurrence_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id, C.start_date as sort_date
FROM 
(
  SELECT co.person_id,co.condition_occurrence_id,co.condition_concept_id,co.visit_occurrence_id,co.condition_start_date as start_date, COALESCE(co.condition_end_date, (co.condition_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.CONDITION_OCCURRENCE co
  JOIN Codesets cs on (co.condition_concept_id = cs.concept_id and cs.codeset_id = 15)
) C
UNION ALL
SELECT C.person_id, C.condition_occurrence_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id, C.start_date as sort_date
FROM 
(
  SELECT co.person_id,co.condition_occurrence_id,co.condition_concept_id,co.visit_occurrence_id,co.condition_start_date as start_date, COALESCE(co.condition_end_date, (co.condition_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.CONDITION_OCCURRENCE co
  JOIN Codesets cs on (co.condition_concept_id = cs.concept_id and cs.codeset_id = 16)
) C
UNION ALL
SELECT C.person_id, C.condition_occurrence_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id, C.start_date as sort_date
FROM 
(
  SELECT co.person_id,co.condition_occurrence_id,co.condition_concept_id,co.visit_occurrence_id,co.condition_start_date as start_date, COALESCE(co.condition_end_date, (co.condition_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.CONDITION_OCCURRENCE co
  JOIN Codesets cs on (co.condition_concept_id = cs.concept_id and cs.codeset_id = 17)
) C
UNION ALL
select C.person_id, C.observation_id as event_id, C.start_date, C.END_DATE,
       C.visit_occurrence_id, C.start_date as sort_date
from 
(
  select o.person_id,o.observation_id,o.observation_concept_id,o.visit_occurrence_id,o.value_as_number,o.observation_date as start_date, (o.observation_date + TO_DAYS(CAST(1 AS INTEGER))) as end_date 
  FROM main.OBSERVATION o
JOIN Codesets cs on (o.observation_concept_id = cs.concept_id and cs.codeset_id = 17)
) C
UNION ALL
SELECT C.person_id, C.condition_occurrence_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id, C.start_date as sort_date
FROM 
(
  SELECT co.person_id,co.condition_occurrence_id,co.condition_concept_id,co.visit_occurrence_id,co.condition_start_date as start_date, COALESCE(co.condition_end_date, (co.condition_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.CONDITION_OCCURRENCE co
  JOIN Codesets cs on (co.condition_concept_id = cs.concept_id and cs.codeset_id = 18)
) C
UNION ALL
select C.person_id, C.observation_id as event_id, C.start_date, C.END_DATE,
       C.visit_occurrence_id, C.start_date as sort_date
from 
(
  select o.person_id,o.observation_id,o.observation_concept_id,o.visit_occurrence_id,o.value_as_number,o.observation_date as start_date, (o.observation_date + TO_DAYS(CAST(1 AS INTEGER))) as end_date 
  FROM main.OBSERVATION o
JOIN Codesets cs on (o.observation_concept_id = cs.concept_id and cs.codeset_id = 18)
) C
  ) E
	JOIN main.observation_period OP on E.person_id = OP.person_id and E.start_date >=  OP.observation_period_start_date and E.start_date <= op.observation_period_end_date
  WHERE (OP.OBSERVATION_PERIOD_START_DATE + TO_DAYS(CAST(0 AS INTEGER))) <= E.START_DATE AND (E.START_DATE + TO_DAYS(CAST(0 AS INTEGER))) <= OP.OBSERVATION_PERIOD_END_DATE
) P
WHERE P.ordinal = 1
) pe
) QE
),
inclusion_events AS (
SELECT CAST(NULL AS bigint) AS inclusion_rule_id, CAST(NULL AS bigint) AS person_id, CAST(NULL AS bigint) AS event_id WHERE FALSE
),
included_events AS (
SELECT
event_id, person_id, start_date, end_date, op_start_date, op_end_date
FROM
(
  SELECT event_id, person_id, start_date, end_date, op_start_date, op_end_date, row_number() over (partition by person_id order by start_date ASC) as ordinal
  from
  (
    select Q.event_id, Q.person_id, Q.start_date, Q.end_date, Q.op_start_date, Q.op_end_date, SUM(coalesce(POWER(cast(2 as bigint), I.inclusion_rule_id), 0)) as inclusion_rule_mask
    from qualified_events Q
    LEFT JOIN inclusion_events I on I.person_id = Q.person_id and I.event_id = Q.event_id
    GROUP BY Q.event_id, Q.person_id, Q.start_date, Q.end_date, Q.op_start_date, Q.op_end_date
  ) MG -- matching groups
) Results
WHERE Results.ordinal = 1
),
cohort_rows AS (
SELECT
person_id, start_date, end_date
FROM
( -- first_ends
	select F.person_id, F.start_date, F.end_date
	FROM (
	  select I.event_id, I.person_id, I.start_date, CE.end_date, row_number() over (partition by I.person_id, I.event_id order by CE.end_date) as ordinal
	  from included_events I
	  join ( -- cohort_ends
select event_id, person_id, op_end_date as end_date from included_events
    ) CE on I.event_id = CE.event_id and I.person_id = CE.person_id and CE.end_date >= I.start_date
	) F
	WHERE F.ordinal = 1
) FE
),
final_cohort AS (
SELECT
person_id, min(start_date) as start_date, (max(end_date) + TO_DAYS(CAST(-1 * 0 AS INTEGER))) as end_date
FROM
(
  select person_id, start_date, end_date, sum(is_start) over (partition by person_id order by start_date, is_start desc rows unbounded preceding) group_idx
  from (
    select person_id, start_date, end_date, 
      case when max(end_date) over (partition by person_id order by start_date rows between unbounded preceding and 1 preceding) >= start_date then 0 else 1 end is_start
    from (
      select person_id, start_date, (end_date + TO_DAYS(CAST(0 AS INTEGER))) as end_date
      from cohort_rows
    ) CR
  ) ST
) GR
group by person_id, group_idx
)
SELECT CAST(cohort_definition_id AS BIGINT) AS cohort_definition_id, CAST(subject_id AS BIGINT) AS subject_id, CAST(cohort_start_date AS DATE) AS cohort_start_date, CAST(cohort_end_date AS DATE) AS cohort_end_date FROM (
select 1 as cohort_definition_id, person_id, start_date, end_date 
FROM final_cohort CO
) AS _f(cohort_definition_id, subject_id, cohort_start_date, cohort_end_date)
