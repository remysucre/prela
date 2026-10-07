-- OHDSI PhenotypeLibrary cohort 862, rendered by SqlRender for duckdb, temp tables folded into CTEs
WITH Codesets AS (
SELECT CAST(codeset_id AS int) AS codeset_id, CAST(concept_id AS bigint) AS concept_id FROM (
SELECT * FROM (
SELECT 0 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (378419))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (378419))
) I
) C UNION ALL 
SELECT 2 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (4182210))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4182210))
) I
LEFT JOIN
(
  select concept_id from main.CONCEPT where (concept_id in (378419,4182210,43530666))
) E ON I.concept_id = E.concept_id
WHERE E.concept_id is null
) C UNION ALL 
SELECT 3 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (715997,757627,701322,733523,836654))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (715997,757627,701322,733523,836654))
) I
) C UNION ALL 
SELECT 6 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (443605,44782710,380701,4043378))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (443605,44782710,380701,4043378))
) I
) C UNION ALL 
SELECT 8 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (372924,375557,376713,443454,441874,439847,43530727,42538062,4148906,432923))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (376713,443454,439847))
) I
LEFT JOIN
(
  select concept_id from main.CONCEPT where (concept_id in (379778))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (379778))
) E ON I.concept_id = E.concept_id
WHERE E.concept_id is null
) C UNION ALL 
SELECT 10 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (4140090))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4140090))
) I
) C UNION ALL 
SELECT 11 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (789578,720810,713823,715710,786426,837027,782211))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (789578,720810,713823,715710,786426,837027,782211))
) I
) C UNION ALL 
SELECT 12 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (43530666,4182210))
) I
) C UNION ALL 
SELECT 13 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (4182210))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4182210))
) I
LEFT JOIN
(
  select concept_id from main.CONCEPT where (concept_id in (378419,4182210))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (378419))
) E ON I.concept_id = E.concept_id
WHERE E.concept_id is null
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
select PE.person_id, PE.event_id, PE.start_date, PE.end_date, PE.visit_occurrence_id, PE.sort_date FROM (
select C.person_id, C.drug_exposure_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id,C.start_date as sort_date
from 
(
  select de.person_id,de.drug_exposure_id,de.drug_concept_id,de.visit_occurrence_id,days_supply,quantity,refills,de.drug_exposure_start_date as start_date, COALESCE(de.drug_exposure_end_date, (de.drug_exposure_start_date + TO_DAYS(CAST(de.days_supply AS INTEGER))), (de.drug_exposure_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.DRUG_EXPOSURE de
JOIN Codesets cs on (de.drug_concept_id = cs.concept_id and cs.codeset_id = 3)
) C
) PE
JOIN (
select 0 as index_id, person_id, event_id
FROM
(
  select E.person_id, E.event_id 
  FROM (SELECT Q.person_id, Q.event_id, Q.start_date, Q.end_date, Q.visit_occurrence_id, OP.observation_period_start_date as op_start_date, OP.observation_period_end_date as op_end_date
FROM (-- Begin Drug Exposure Criteria
select C.person_id, C.drug_exposure_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id,C.start_date as sort_date
from 
(
  select de.person_id,de.drug_exposure_id,de.drug_concept_id,de.visit_occurrence_id,days_supply,quantity,refills,de.drug_exposure_start_date as start_date, COALESCE(de.drug_exposure_end_date, (de.drug_exposure_start_date + TO_DAYS(CAST(de.days_supply AS INTEGER))), (de.drug_exposure_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.DRUG_EXPOSURE de
JOIN Codesets cs on (de.drug_concept_id = cs.concept_id and cs.codeset_id = 3)
) C
) Q
JOIN main.OBSERVATION_PERIOD OP on Q.person_id = OP.person_id 
  and OP.observation_period_start_date <= Q.start_date and OP.observation_period_end_date >= Q.start_date
) E
  INNER JOIN
  (
select 0 as index_id, cc.person_id, cc.event_id
from (SELECT p.person_id, p.event_id 
FROM (SELECT Q.person_id, Q.event_id, Q.start_date, Q.end_date, Q.visit_occurrence_id, OP.observation_period_start_date as op_start_date, OP.observation_period_end_date as op_end_date
FROM (-- Begin Drug Exposure Criteria
select C.person_id, C.drug_exposure_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id,C.start_date as sort_date
from 
(
  select de.person_id,de.drug_exposure_id,de.drug_concept_id,de.visit_occurrence_id,days_supply,quantity,refills,de.drug_exposure_start_date as start_date, COALESCE(de.drug_exposure_end_date, (de.drug_exposure_start_date + TO_DAYS(CAST(de.days_supply AS INTEGER))), (de.drug_exposure_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.DRUG_EXPOSURE de
JOIN Codesets cs on (de.drug_concept_id = cs.concept_id and cs.codeset_id = 3)
) C
) Q
JOIN main.OBSERVATION_PERIOD OP on Q.person_id = OP.person_id 
  and OP.observation_period_start_date <= Q.start_date and OP.observation_period_end_date >= Q.start_date
) P
JOIN (
SELECT C.person_id, C.condition_occurrence_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id, C.start_date as sort_date
FROM 
(
  SELECT co.person_id,co.condition_occurrence_id,co.condition_concept_id,co.visit_occurrence_id,co.condition_start_date as start_date, COALESCE(co.condition_end_date, (co.condition_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.CONDITION_OCCURRENCE co
  JOIN Codesets cs on (co.condition_concept_id = cs.concept_id and cs.codeset_id = 0)
) C
) A on A.person_id = P.person_id  AND A.START_DATE >= P.OP_START_DATE AND A.START_DATE <= P.OP_END_DATE AND A.START_DATE >= (P.START_DATE + TO_DAYS(CAST(1 AS INTEGER))) AND A.START_DATE <= (P.START_DATE + TO_DAYS(CAST(365 AS INTEGER))) ) cc 
GROUP BY cc.person_id, cc.event_id
HAVING COUNT(cc.event_id) >= 1
UNION ALL
select 1 as index_id, person_id, event_id
FROM
(
  select E.person_id, E.event_id 
  FROM (SELECT Q.person_id, Q.event_id, Q.start_date, Q.end_date, Q.visit_occurrence_id, OP.observation_period_start_date as op_start_date, OP.observation_period_end_date as op_end_date
FROM (-- Begin Drug Exposure Criteria
select C.person_id, C.drug_exposure_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id,C.start_date as sort_date
from 
(
  select de.person_id,de.drug_exposure_id,de.drug_concept_id,de.visit_occurrence_id,days_supply,quantity,refills,de.drug_exposure_start_date as start_date, COALESCE(de.drug_exposure_end_date, (de.drug_exposure_start_date + TO_DAYS(CAST(de.days_supply AS INTEGER))), (de.drug_exposure_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.DRUG_EXPOSURE de
JOIN Codesets cs on (de.drug_concept_id = cs.concept_id and cs.codeset_id = 3)
) C
) Q
JOIN main.OBSERVATION_PERIOD OP on Q.person_id = OP.person_id 
  and OP.observation_period_start_date <= Q.start_date and OP.observation_period_end_date >= Q.start_date
) E
  INNER JOIN
  (
select 0 as index_id, p.person_id, p.event_id
from (SELECT Q.person_id, Q.event_id, Q.start_date, Q.end_date, Q.visit_occurrence_id, OP.observation_period_start_date as op_start_date, OP.observation_period_end_date as op_end_date
FROM (-- Begin Drug Exposure Criteria
select C.person_id, C.drug_exposure_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id,C.start_date as sort_date
from 
(
  select de.person_id,de.drug_exposure_id,de.drug_concept_id,de.visit_occurrence_id,days_supply,quantity,refills,de.drug_exposure_start_date as start_date, COALESCE(de.drug_exposure_end_date, (de.drug_exposure_start_date + TO_DAYS(CAST(de.days_supply AS INTEGER))), (de.drug_exposure_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.DRUG_EXPOSURE de
JOIN Codesets cs on (de.drug_concept_id = cs.concept_id and cs.codeset_id = 3)
) C
) Q
JOIN main.OBSERVATION_PERIOD OP on Q.person_id = OP.person_id 
  and OP.observation_period_start_date <= Q.start_date and OP.observation_period_end_date >= Q.start_date
) p
LEFT JOIN (
SELECT p.person_id, p.event_id 
FROM (SELECT Q.person_id, Q.event_id, Q.start_date, Q.end_date, Q.visit_occurrence_id, OP.observation_period_start_date as op_start_date, OP.observation_period_end_date as op_end_date
FROM (-- Begin Drug Exposure Criteria
select C.person_id, C.drug_exposure_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id,C.start_date as sort_date
from 
(
  select de.person_id,de.drug_exposure_id,de.drug_concept_id,de.visit_occurrence_id,days_supply,quantity,refills,de.drug_exposure_start_date as start_date, COALESCE(de.drug_exposure_end_date, (de.drug_exposure_start_date + TO_DAYS(CAST(de.days_supply AS INTEGER))), (de.drug_exposure_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.DRUG_EXPOSURE de
JOIN Codesets cs on (de.drug_concept_id = cs.concept_id and cs.codeset_id = 3)
) C
) Q
JOIN main.OBSERVATION_PERIOD OP on Q.person_id = OP.person_id 
  and OP.observation_period_start_date <= Q.start_date and OP.observation_period_end_date >= Q.start_date
) P
JOIN (
SELECT C.person_id, C.condition_occurrence_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id, C.start_date as sort_date
FROM 
(
  SELECT co.person_id,co.condition_occurrence_id,co.condition_concept_id,co.visit_occurrence_id,co.condition_start_date as start_date, COALESCE(co.condition_end_date, (co.condition_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.CONDITION_OCCURRENCE co
  JOIN Codesets cs on (co.condition_concept_id = cs.concept_id and cs.codeset_id = 13)
) C
) A on A.person_id = P.person_id  AND A.START_DATE >= P.OP_START_DATE AND A.START_DATE <= P.OP_END_DATE AND A.START_DATE >= P.OP_START_DATE AND A.START_DATE <= (P.START_DATE + TO_DAYS(CAST(7 AS INTEGER))) ) cc on p.person_id = cc.person_id and p.event_id = cc.event_id
GROUP BY p.person_id, p.event_id
HAVING COUNT(cc.event_id) = 0
UNION ALL
select 1 as index_id, p.person_id, p.event_id
from (SELECT Q.person_id, Q.event_id, Q.start_date, Q.end_date, Q.visit_occurrence_id, OP.observation_period_start_date as op_start_date, OP.observation_period_end_date as op_end_date
FROM (-- Begin Drug Exposure Criteria
select C.person_id, C.drug_exposure_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id,C.start_date as sort_date
from 
(
  select de.person_id,de.drug_exposure_id,de.drug_concept_id,de.visit_occurrence_id,days_supply,quantity,refills,de.drug_exposure_start_date as start_date, COALESCE(de.drug_exposure_end_date, (de.drug_exposure_start_date + TO_DAYS(CAST(de.days_supply AS INTEGER))), (de.drug_exposure_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.DRUG_EXPOSURE de
JOIN Codesets cs on (de.drug_concept_id = cs.concept_id and cs.codeset_id = 3)
) C
) Q
JOIN main.OBSERVATION_PERIOD OP on Q.person_id = OP.person_id 
  and OP.observation_period_start_date <= Q.start_date and OP.observation_period_end_date >= Q.start_date
) p
LEFT JOIN (
SELECT p.person_id, p.event_id 
FROM (SELECT Q.person_id, Q.event_id, Q.start_date, Q.end_date, Q.visit_occurrence_id, OP.observation_period_start_date as op_start_date, OP.observation_period_end_date as op_end_date
FROM (-- Begin Drug Exposure Criteria
select C.person_id, C.drug_exposure_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id,C.start_date as sort_date
from 
(
  select de.person_id,de.drug_exposure_id,de.drug_concept_id,de.visit_occurrence_id,days_supply,quantity,refills,de.drug_exposure_start_date as start_date, COALESCE(de.drug_exposure_end_date, (de.drug_exposure_start_date + TO_DAYS(CAST(de.days_supply AS INTEGER))), (de.drug_exposure_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.DRUG_EXPOSURE de
JOIN Codesets cs on (de.drug_concept_id = cs.concept_id and cs.codeset_id = 3)
) C
) Q
JOIN main.OBSERVATION_PERIOD OP on Q.person_id = OP.person_id 
  and OP.observation_period_start_date <= Q.start_date and OP.observation_period_end_date >= Q.start_date
) P
JOIN (
SELECT C.person_id, C.condition_occurrence_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id, C.start_date as sort_date
FROM 
(
  SELECT co.person_id,co.condition_occurrence_id,co.condition_concept_id,co.visit_occurrence_id,co.condition_start_date as start_date, COALESCE(co.condition_end_date, (co.condition_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.CONDITION_OCCURRENCE co
  JOIN Codesets cs on (co.condition_concept_id = cs.concept_id and cs.codeset_id = 6)
) C
) A on A.person_id = P.person_id  AND A.START_DATE >= P.OP_START_DATE AND A.START_DATE <= P.OP_END_DATE AND A.START_DATE >= P.OP_START_DATE AND A.START_DATE <= (P.START_DATE + TO_DAYS(CAST(7 AS INTEGER))) ) cc on p.person_id = cc.person_id and p.event_id = cc.event_id
GROUP BY p.person_id, p.event_id
HAVING COUNT(cc.event_id) = 0
UNION ALL
select 2 as index_id, p.person_id, p.event_id
from (SELECT Q.person_id, Q.event_id, Q.start_date, Q.end_date, Q.visit_occurrence_id, OP.observation_period_start_date as op_start_date, OP.observation_period_end_date as op_end_date
FROM (-- Begin Drug Exposure Criteria
select C.person_id, C.drug_exposure_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id,C.start_date as sort_date
from 
(
  select de.person_id,de.drug_exposure_id,de.drug_concept_id,de.visit_occurrence_id,days_supply,quantity,refills,de.drug_exposure_start_date as start_date, COALESCE(de.drug_exposure_end_date, (de.drug_exposure_start_date + TO_DAYS(CAST(de.days_supply AS INTEGER))), (de.drug_exposure_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.DRUG_EXPOSURE de
JOIN Codesets cs on (de.drug_concept_id = cs.concept_id and cs.codeset_id = 3)
) C
) Q
JOIN main.OBSERVATION_PERIOD OP on Q.person_id = OP.person_id 
  and OP.observation_period_start_date <= Q.start_date and OP.observation_period_end_date >= Q.start_date
) p
LEFT JOIN (
SELECT p.person_id, p.event_id 
FROM (SELECT Q.person_id, Q.event_id, Q.start_date, Q.end_date, Q.visit_occurrence_id, OP.observation_period_start_date as op_start_date, OP.observation_period_end_date as op_end_date
FROM (-- Begin Drug Exposure Criteria
select C.person_id, C.drug_exposure_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id,C.start_date as sort_date
from 
(
  select de.person_id,de.drug_exposure_id,de.drug_concept_id,de.visit_occurrence_id,days_supply,quantity,refills,de.drug_exposure_start_date as start_date, COALESCE(de.drug_exposure_end_date, (de.drug_exposure_start_date + TO_DAYS(CAST(de.days_supply AS INTEGER))), (de.drug_exposure_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.DRUG_EXPOSURE de
JOIN Codesets cs on (de.drug_concept_id = cs.concept_id and cs.codeset_id = 3)
) C
) Q
JOIN main.OBSERVATION_PERIOD OP on Q.person_id = OP.person_id 
  and OP.observation_period_start_date <= Q.start_date and OP.observation_period_end_date >= Q.start_date
) P
JOIN (
SELECT C.person_id, C.condition_occurrence_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id, C.start_date as sort_date
FROM 
(
  SELECT co.person_id,co.condition_occurrence_id,co.condition_concept_id,co.visit_occurrence_id,co.condition_start_date as start_date, COALESCE(co.condition_end_date, (co.condition_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.CONDITION_OCCURRENCE co
  JOIN Codesets cs on (co.condition_concept_id = cs.concept_id and cs.codeset_id = 10)
) C
) A on A.person_id = P.person_id  AND A.START_DATE >= P.OP_START_DATE AND A.START_DATE <= P.OP_END_DATE AND A.START_DATE >= (P.START_DATE + TO_DAYS(CAST(-365 AS INTEGER))) AND A.START_DATE <= (P.START_DATE + TO_DAYS(CAST(7 AS INTEGER))) ) cc on p.person_id = cc.person_id and p.event_id = cc.event_id
GROUP BY p.person_id, p.event_id
HAVING COUNT(cc.event_id) = 0
UNION ALL
select 3 as index_id, p.person_id, p.event_id
from (SELECT Q.person_id, Q.event_id, Q.start_date, Q.end_date, Q.visit_occurrence_id, OP.observation_period_start_date as op_start_date, OP.observation_period_end_date as op_end_date
FROM (-- Begin Drug Exposure Criteria
select C.person_id, C.drug_exposure_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id,C.start_date as sort_date
from 
(
  select de.person_id,de.drug_exposure_id,de.drug_concept_id,de.visit_occurrence_id,days_supply,quantity,refills,de.drug_exposure_start_date as start_date, COALESCE(de.drug_exposure_end_date, (de.drug_exposure_start_date + TO_DAYS(CAST(de.days_supply AS INTEGER))), (de.drug_exposure_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.DRUG_EXPOSURE de
JOIN Codesets cs on (de.drug_concept_id = cs.concept_id and cs.codeset_id = 3)
) C
) Q
JOIN main.OBSERVATION_PERIOD OP on Q.person_id = OP.person_id 
  and OP.observation_period_start_date <= Q.start_date and OP.observation_period_end_date >= Q.start_date
) p
LEFT JOIN (
SELECT p.person_id, p.event_id 
FROM (SELECT Q.person_id, Q.event_id, Q.start_date, Q.end_date, Q.visit_occurrence_id, OP.observation_period_start_date as op_start_date, OP.observation_period_end_date as op_end_date
FROM (-- Begin Drug Exposure Criteria
select C.person_id, C.drug_exposure_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id,C.start_date as sort_date
from 
(
  select de.person_id,de.drug_exposure_id,de.drug_concept_id,de.visit_occurrence_id,days_supply,quantity,refills,de.drug_exposure_start_date as start_date, COALESCE(de.drug_exposure_end_date, (de.drug_exposure_start_date + TO_DAYS(CAST(de.days_supply AS INTEGER))), (de.drug_exposure_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.DRUG_EXPOSURE de
JOIN Codesets cs on (de.drug_concept_id = cs.concept_id and cs.codeset_id = 3)
) C
) Q
JOIN main.OBSERVATION_PERIOD OP on Q.person_id = OP.person_id 
  and OP.observation_period_start_date <= Q.start_date and OP.observation_period_end_date >= Q.start_date
) P
JOIN (
SELECT C.person_id, C.condition_occurrence_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id, C.start_date as sort_date
FROM 
(
  SELECT co.person_id,co.condition_occurrence_id,co.condition_concept_id,co.visit_occurrence_id,co.condition_start_date as start_date, COALESCE(co.condition_end_date, (co.condition_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.CONDITION_OCCURRENCE co
  JOIN Codesets cs on (co.condition_concept_id = cs.concept_id and cs.codeset_id = 8)
) C
) A on A.person_id = P.person_id  AND A.START_DATE >= P.OP_START_DATE AND A.START_DATE <= P.OP_END_DATE AND A.START_DATE >= (P.START_DATE + TO_DAYS(CAST(-365 AS INTEGER))) AND A.START_DATE <= (P.START_DATE + TO_DAYS(CAST(7 AS INTEGER))) ) cc on p.person_id = cc.person_id and p.event_id = cc.event_id
GROUP BY p.person_id, p.event_id
HAVING COUNT(cc.event_id) = 0
UNION ALL
select 4 as index_id, p.person_id, p.event_id
from (SELECT Q.person_id, Q.event_id, Q.start_date, Q.end_date, Q.visit_occurrence_id, OP.observation_period_start_date as op_start_date, OP.observation_period_end_date as op_end_date
FROM (-- Begin Drug Exposure Criteria
select C.person_id, C.drug_exposure_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id,C.start_date as sort_date
from 
(
  select de.person_id,de.drug_exposure_id,de.drug_concept_id,de.visit_occurrence_id,days_supply,quantity,refills,de.drug_exposure_start_date as start_date, COALESCE(de.drug_exposure_end_date, (de.drug_exposure_start_date + TO_DAYS(CAST(de.days_supply AS INTEGER))), (de.drug_exposure_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.DRUG_EXPOSURE de
JOIN Codesets cs on (de.drug_concept_id = cs.concept_id and cs.codeset_id = 3)
) C
) Q
JOIN main.OBSERVATION_PERIOD OP on Q.person_id = OP.person_id 
  and OP.observation_period_start_date <= Q.start_date and OP.observation_period_end_date >= Q.start_date
) p
LEFT JOIN (
SELECT p.person_id, p.event_id 
FROM (SELECT Q.person_id, Q.event_id, Q.start_date, Q.end_date, Q.visit_occurrence_id, OP.observation_period_start_date as op_start_date, OP.observation_period_end_date as op_end_date
FROM (-- Begin Drug Exposure Criteria
select C.person_id, C.drug_exposure_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id,C.start_date as sort_date
from 
(
  select de.person_id,de.drug_exposure_id,de.drug_concept_id,de.visit_occurrence_id,days_supply,quantity,refills,de.drug_exposure_start_date as start_date, COALESCE(de.drug_exposure_end_date, (de.drug_exposure_start_date + TO_DAYS(CAST(de.days_supply AS INTEGER))), (de.drug_exposure_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.DRUG_EXPOSURE de
JOIN Codesets cs on (de.drug_concept_id = cs.concept_id and cs.codeset_id = 3)
) C
) Q
JOIN main.OBSERVATION_PERIOD OP on Q.person_id = OP.person_id 
  and OP.observation_period_start_date <= Q.start_date and OP.observation_period_end_date >= Q.start_date
) P
JOIN (
select C.person_id, C.drug_exposure_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id,C.start_date as sort_date
from 
(
  select de.person_id,de.drug_exposure_id,de.drug_concept_id,de.visit_occurrence_id,days_supply,quantity,refills,de.drug_exposure_start_date as start_date, COALESCE(de.drug_exposure_end_date, (de.drug_exposure_start_date + TO_DAYS(CAST(de.days_supply AS INTEGER))), (de.drug_exposure_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.DRUG_EXPOSURE de
JOIN Codesets cs on (de.drug_concept_id = cs.concept_id and cs.codeset_id = 11)
) C
) A on A.person_id = P.person_id  AND A.START_DATE >= P.OP_START_DATE AND A.START_DATE <= P.OP_END_DATE AND A.START_DATE >= (P.START_DATE + TO_DAYS(CAST(-365 AS INTEGER))) AND A.START_DATE <= (P.START_DATE + TO_DAYS(CAST(7 AS INTEGER))) ) cc on p.person_id = cc.person_id and p.event_id = cc.event_id
GROUP BY p.person_id, p.event_id
HAVING COUNT(cc.event_id) = 0
  ) CQ on E.person_id = CQ.person_id and E.event_id = CQ.event_id
  GROUP BY E.person_id, E.event_id
  HAVING COUNT(index_id) = 5
) G
UNION ALL
select 2 as index_id, person_id, event_id
FROM
(
  select E.person_id, E.event_id 
  FROM (SELECT Q.person_id, Q.event_id, Q.start_date, Q.end_date, Q.visit_occurrence_id, OP.observation_period_start_date as op_start_date, OP.observation_period_end_date as op_end_date
FROM (-- Begin Drug Exposure Criteria
select C.person_id, C.drug_exposure_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id,C.start_date as sort_date
from 
(
  select de.person_id,de.drug_exposure_id,de.drug_concept_id,de.visit_occurrence_id,days_supply,quantity,refills,de.drug_exposure_start_date as start_date, COALESCE(de.drug_exposure_end_date, (de.drug_exposure_start_date + TO_DAYS(CAST(de.days_supply AS INTEGER))), (de.drug_exposure_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.DRUG_EXPOSURE de
JOIN Codesets cs on (de.drug_concept_id = cs.concept_id and cs.codeset_id = 3)
) C
) Q
JOIN main.OBSERVATION_PERIOD OP on Q.person_id = OP.person_id 
  and OP.observation_period_start_date <= Q.start_date and OP.observation_period_end_date >= Q.start_date
) E
  INNER JOIN
  (
select 0 as index_id, cc.person_id, cc.event_id
from (SELECT p.person_id, p.event_id 
FROM (SELECT Q.person_id, Q.event_id, Q.start_date, Q.end_date, Q.visit_occurrence_id, OP.observation_period_start_date as op_start_date, OP.observation_period_end_date as op_end_date
FROM (-- Begin Drug Exposure Criteria
select C.person_id, C.drug_exposure_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id,C.start_date as sort_date
from 
(
  select de.person_id,de.drug_exposure_id,de.drug_concept_id,de.visit_occurrence_id,days_supply,quantity,refills,de.drug_exposure_start_date as start_date, COALESCE(de.drug_exposure_end_date, (de.drug_exposure_start_date + TO_DAYS(CAST(de.days_supply AS INTEGER))), (de.drug_exposure_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.DRUG_EXPOSURE de
JOIN Codesets cs on (de.drug_concept_id = cs.concept_id and cs.codeset_id = 3)
) C
) Q
JOIN main.OBSERVATION_PERIOD OP on Q.person_id = OP.person_id 
  and OP.observation_period_start_date <= Q.start_date and OP.observation_period_end_date >= Q.start_date
) P
JOIN (
SELECT C.person_id, C.condition_occurrence_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id, C.start_date as sort_date
FROM 
(
  SELECT co.person_id,co.condition_occurrence_id,co.condition_concept_id,co.visit_occurrence_id,co.condition_start_date as start_date, COALESCE(co.condition_end_date, (co.condition_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.CONDITION_OCCURRENCE co
  JOIN Codesets cs on (co.condition_concept_id = cs.concept_id and cs.codeset_id = 12)
) C
) A on A.person_id = P.person_id  AND A.START_DATE >= P.OP_START_DATE AND A.START_DATE <= P.OP_END_DATE AND A.START_DATE >= (P.START_DATE + TO_DAYS(CAST(-365 AS INTEGER))) AND A.START_DATE <= (P.START_DATE + TO_DAYS(CAST(7 AS INTEGER))) ) cc 
GROUP BY cc.person_id, cc.event_id
HAVING COUNT(cc.event_id) >= 1
UNION ALL
select 1 as index_id, p.person_id, p.event_id
from (SELECT Q.person_id, Q.event_id, Q.start_date, Q.end_date, Q.visit_occurrence_id, OP.observation_period_start_date as op_start_date, OP.observation_period_end_date as op_end_date
FROM (-- Begin Drug Exposure Criteria
select C.person_id, C.drug_exposure_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id,C.start_date as sort_date
from 
(
  select de.person_id,de.drug_exposure_id,de.drug_concept_id,de.visit_occurrence_id,days_supply,quantity,refills,de.drug_exposure_start_date as start_date, COALESCE(de.drug_exposure_end_date, (de.drug_exposure_start_date + TO_DAYS(CAST(de.days_supply AS INTEGER))), (de.drug_exposure_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.DRUG_EXPOSURE de
JOIN Codesets cs on (de.drug_concept_id = cs.concept_id and cs.codeset_id = 3)
) C
) Q
JOIN main.OBSERVATION_PERIOD OP on Q.person_id = OP.person_id 
  and OP.observation_period_start_date <= Q.start_date and OP.observation_period_end_date >= Q.start_date
) p
LEFT JOIN (
SELECT p.person_id, p.event_id 
FROM (SELECT Q.person_id, Q.event_id, Q.start_date, Q.end_date, Q.visit_occurrence_id, OP.observation_period_start_date as op_start_date, OP.observation_period_end_date as op_end_date
FROM (-- Begin Drug Exposure Criteria
select C.person_id, C.drug_exposure_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id,C.start_date as sort_date
from 
(
  select de.person_id,de.drug_exposure_id,de.drug_concept_id,de.visit_occurrence_id,days_supply,quantity,refills,de.drug_exposure_start_date as start_date, COALESCE(de.drug_exposure_end_date, (de.drug_exposure_start_date + TO_DAYS(CAST(de.days_supply AS INTEGER))), (de.drug_exposure_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.DRUG_EXPOSURE de
JOIN Codesets cs on (de.drug_concept_id = cs.concept_id and cs.codeset_id = 3)
) C
) Q
JOIN main.OBSERVATION_PERIOD OP on Q.person_id = OP.person_id 
  and OP.observation_period_start_date <= Q.start_date and OP.observation_period_end_date >= Q.start_date
) P
JOIN (
SELECT C.person_id, C.condition_occurrence_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id, C.start_date as sort_date
FROM 
(
  SELECT co.person_id,co.condition_occurrence_id,co.condition_concept_id,co.visit_occurrence_id,co.condition_start_date as start_date, COALESCE(co.condition_end_date, (co.condition_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.CONDITION_OCCURRENCE co
  JOIN Codesets cs on (co.condition_concept_id = cs.concept_id and cs.codeset_id = 2)
) C
) A on A.person_id = P.person_id  AND A.START_DATE >= P.OP_START_DATE AND A.START_DATE <= P.OP_END_DATE AND A.START_DATE >= P.OP_START_DATE AND A.START_DATE <= (P.START_DATE + TO_DAYS(CAST(7 AS INTEGER))) ) cc on p.person_id = cc.person_id and p.event_id = cc.event_id
GROUP BY p.person_id, p.event_id
HAVING COUNT(cc.event_id) = 0
UNION ALL
select 2 as index_id, p.person_id, p.event_id
from (SELECT Q.person_id, Q.event_id, Q.start_date, Q.end_date, Q.visit_occurrence_id, OP.observation_period_start_date as op_start_date, OP.observation_period_end_date as op_end_date
FROM (-- Begin Drug Exposure Criteria
select C.person_id, C.drug_exposure_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id,C.start_date as sort_date
from 
(
  select de.person_id,de.drug_exposure_id,de.drug_concept_id,de.visit_occurrence_id,days_supply,quantity,refills,de.drug_exposure_start_date as start_date, COALESCE(de.drug_exposure_end_date, (de.drug_exposure_start_date + TO_DAYS(CAST(de.days_supply AS INTEGER))), (de.drug_exposure_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.DRUG_EXPOSURE de
JOIN Codesets cs on (de.drug_concept_id = cs.concept_id and cs.codeset_id = 3)
) C
) Q
JOIN main.OBSERVATION_PERIOD OP on Q.person_id = OP.person_id 
  and OP.observation_period_start_date <= Q.start_date and OP.observation_period_end_date >= Q.start_date
) p
LEFT JOIN (
SELECT p.person_id, p.event_id 
FROM (SELECT Q.person_id, Q.event_id, Q.start_date, Q.end_date, Q.visit_occurrence_id, OP.observation_period_start_date as op_start_date, OP.observation_period_end_date as op_end_date
FROM (-- Begin Drug Exposure Criteria
select C.person_id, C.drug_exposure_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id,C.start_date as sort_date
from 
(
  select de.person_id,de.drug_exposure_id,de.drug_concept_id,de.visit_occurrence_id,days_supply,quantity,refills,de.drug_exposure_start_date as start_date, COALESCE(de.drug_exposure_end_date, (de.drug_exposure_start_date + TO_DAYS(CAST(de.days_supply AS INTEGER))), (de.drug_exposure_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.DRUG_EXPOSURE de
JOIN Codesets cs on (de.drug_concept_id = cs.concept_id and cs.codeset_id = 3)
) C
) Q
JOIN main.OBSERVATION_PERIOD OP on Q.person_id = OP.person_id 
  and OP.observation_period_start_date <= Q.start_date and OP.observation_period_end_date >= Q.start_date
) P
JOIN (
SELECT C.person_id, C.condition_occurrence_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id, C.start_date as sort_date
FROM 
(
  SELECT co.person_id,co.condition_occurrence_id,co.condition_concept_id,co.visit_occurrence_id,co.condition_start_date as start_date, COALESCE(co.condition_end_date, (co.condition_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.CONDITION_OCCURRENCE co
  JOIN Codesets cs on (co.condition_concept_id = cs.concept_id and cs.codeset_id = 6)
) C
) A on A.person_id = P.person_id  AND A.START_DATE >= P.OP_START_DATE AND A.START_DATE <= P.OP_END_DATE AND A.START_DATE >= P.OP_START_DATE AND A.START_DATE <= (P.START_DATE + TO_DAYS(CAST(7 AS INTEGER))) ) cc on p.person_id = cc.person_id and p.event_id = cc.event_id
GROUP BY p.person_id, p.event_id
HAVING COUNT(cc.event_id) = 0
UNION ALL
select 3 as index_id, p.person_id, p.event_id
from (SELECT Q.person_id, Q.event_id, Q.start_date, Q.end_date, Q.visit_occurrence_id, OP.observation_period_start_date as op_start_date, OP.observation_period_end_date as op_end_date
FROM (-- Begin Drug Exposure Criteria
select C.person_id, C.drug_exposure_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id,C.start_date as sort_date
from 
(
  select de.person_id,de.drug_exposure_id,de.drug_concept_id,de.visit_occurrence_id,days_supply,quantity,refills,de.drug_exposure_start_date as start_date, COALESCE(de.drug_exposure_end_date, (de.drug_exposure_start_date + TO_DAYS(CAST(de.days_supply AS INTEGER))), (de.drug_exposure_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.DRUG_EXPOSURE de
JOIN Codesets cs on (de.drug_concept_id = cs.concept_id and cs.codeset_id = 3)
) C
) Q
JOIN main.OBSERVATION_PERIOD OP on Q.person_id = OP.person_id 
  and OP.observation_period_start_date <= Q.start_date and OP.observation_period_end_date >= Q.start_date
) p
LEFT JOIN (
SELECT p.person_id, p.event_id 
FROM (SELECT Q.person_id, Q.event_id, Q.start_date, Q.end_date, Q.visit_occurrence_id, OP.observation_period_start_date as op_start_date, OP.observation_period_end_date as op_end_date
FROM (-- Begin Drug Exposure Criteria
select C.person_id, C.drug_exposure_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id,C.start_date as sort_date
from 
(
  select de.person_id,de.drug_exposure_id,de.drug_concept_id,de.visit_occurrence_id,days_supply,quantity,refills,de.drug_exposure_start_date as start_date, COALESCE(de.drug_exposure_end_date, (de.drug_exposure_start_date + TO_DAYS(CAST(de.days_supply AS INTEGER))), (de.drug_exposure_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.DRUG_EXPOSURE de
JOIN Codesets cs on (de.drug_concept_id = cs.concept_id and cs.codeset_id = 3)
) C
) Q
JOIN main.OBSERVATION_PERIOD OP on Q.person_id = OP.person_id 
  and OP.observation_period_start_date <= Q.start_date and OP.observation_period_end_date >= Q.start_date
) P
JOIN (
SELECT C.person_id, C.condition_occurrence_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id, C.start_date as sort_date
FROM 
(
  SELECT co.person_id,co.condition_occurrence_id,co.condition_concept_id,co.visit_occurrence_id,co.condition_start_date as start_date, COALESCE(co.condition_end_date, (co.condition_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.CONDITION_OCCURRENCE co
  JOIN Codesets cs on (co.condition_concept_id = cs.concept_id and cs.codeset_id = 8)
) C
) A on A.person_id = P.person_id  AND A.START_DATE >= P.OP_START_DATE AND A.START_DATE <= P.OP_END_DATE AND A.START_DATE >= P.OP_START_DATE AND A.START_DATE <= (P.START_DATE + TO_DAYS(CAST(0 AS INTEGER))) ) cc on p.person_id = cc.person_id and p.event_id = cc.event_id
GROUP BY p.person_id, p.event_id
HAVING COUNT(cc.event_id) = 0
UNION ALL
select 4 as index_id, p.person_id, p.event_id
from (SELECT Q.person_id, Q.event_id, Q.start_date, Q.end_date, Q.visit_occurrence_id, OP.observation_period_start_date as op_start_date, OP.observation_period_end_date as op_end_date
FROM (-- Begin Drug Exposure Criteria
select C.person_id, C.drug_exposure_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id,C.start_date as sort_date
from 
(
  select de.person_id,de.drug_exposure_id,de.drug_concept_id,de.visit_occurrence_id,days_supply,quantity,refills,de.drug_exposure_start_date as start_date, COALESCE(de.drug_exposure_end_date, (de.drug_exposure_start_date + TO_DAYS(CAST(de.days_supply AS INTEGER))), (de.drug_exposure_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.DRUG_EXPOSURE de
JOIN Codesets cs on (de.drug_concept_id = cs.concept_id and cs.codeset_id = 3)
) C
) Q
JOIN main.OBSERVATION_PERIOD OP on Q.person_id = OP.person_id 
  and OP.observation_period_start_date <= Q.start_date and OP.observation_period_end_date >= Q.start_date
) p
LEFT JOIN (
SELECT p.person_id, p.event_id 
FROM (SELECT Q.person_id, Q.event_id, Q.start_date, Q.end_date, Q.visit_occurrence_id, OP.observation_period_start_date as op_start_date, OP.observation_period_end_date as op_end_date
FROM (-- Begin Drug Exposure Criteria
select C.person_id, C.drug_exposure_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id,C.start_date as sort_date
from 
(
  select de.person_id,de.drug_exposure_id,de.drug_concept_id,de.visit_occurrence_id,days_supply,quantity,refills,de.drug_exposure_start_date as start_date, COALESCE(de.drug_exposure_end_date, (de.drug_exposure_start_date + TO_DAYS(CAST(de.days_supply AS INTEGER))), (de.drug_exposure_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.DRUG_EXPOSURE de
JOIN Codesets cs on (de.drug_concept_id = cs.concept_id and cs.codeset_id = 3)
) C
) Q
JOIN main.OBSERVATION_PERIOD OP on Q.person_id = OP.person_id 
  and OP.observation_period_start_date <= Q.start_date and OP.observation_period_end_date >= Q.start_date
) P
JOIN (
SELECT C.person_id, C.condition_occurrence_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id, C.start_date as sort_date
FROM 
(
  SELECT co.person_id,co.condition_occurrence_id,co.condition_concept_id,co.visit_occurrence_id,co.condition_start_date as start_date, COALESCE(co.condition_end_date, (co.condition_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.CONDITION_OCCURRENCE co
  JOIN Codesets cs on (co.condition_concept_id = cs.concept_id and cs.codeset_id = 10)
) C
) A on A.person_id = P.person_id  AND A.START_DATE >= P.OP_START_DATE AND A.START_DATE <= P.OP_END_DATE AND A.START_DATE >= P.OP_START_DATE AND A.START_DATE <= (P.START_DATE + TO_DAYS(CAST(7 AS INTEGER))) ) cc on p.person_id = cc.person_id and p.event_id = cc.event_id
GROUP BY p.person_id, p.event_id
HAVING COUNT(cc.event_id) = 0
UNION ALL
select 5 as index_id, p.person_id, p.event_id
from (SELECT Q.person_id, Q.event_id, Q.start_date, Q.end_date, Q.visit_occurrence_id, OP.observation_period_start_date as op_start_date, OP.observation_period_end_date as op_end_date
FROM (-- Begin Drug Exposure Criteria
select C.person_id, C.drug_exposure_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id,C.start_date as sort_date
from 
(
  select de.person_id,de.drug_exposure_id,de.drug_concept_id,de.visit_occurrence_id,days_supply,quantity,refills,de.drug_exposure_start_date as start_date, COALESCE(de.drug_exposure_end_date, (de.drug_exposure_start_date + TO_DAYS(CAST(de.days_supply AS INTEGER))), (de.drug_exposure_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.DRUG_EXPOSURE de
JOIN Codesets cs on (de.drug_concept_id = cs.concept_id and cs.codeset_id = 3)
) C
) Q
JOIN main.OBSERVATION_PERIOD OP on Q.person_id = OP.person_id 
  and OP.observation_period_start_date <= Q.start_date and OP.observation_period_end_date >= Q.start_date
) p
LEFT JOIN (
SELECT p.person_id, p.event_id 
FROM (SELECT Q.person_id, Q.event_id, Q.start_date, Q.end_date, Q.visit_occurrence_id, OP.observation_period_start_date as op_start_date, OP.observation_period_end_date as op_end_date
FROM (-- Begin Drug Exposure Criteria
select C.person_id, C.drug_exposure_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id,C.start_date as sort_date
from 
(
  select de.person_id,de.drug_exposure_id,de.drug_concept_id,de.visit_occurrence_id,days_supply,quantity,refills,de.drug_exposure_start_date as start_date, COALESCE(de.drug_exposure_end_date, (de.drug_exposure_start_date + TO_DAYS(CAST(de.days_supply AS INTEGER))), (de.drug_exposure_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.DRUG_EXPOSURE de
JOIN Codesets cs on (de.drug_concept_id = cs.concept_id and cs.codeset_id = 3)
) C
) Q
JOIN main.OBSERVATION_PERIOD OP on Q.person_id = OP.person_id 
  and OP.observation_period_start_date <= Q.start_date and OP.observation_period_end_date >= Q.start_date
) P
JOIN (
select C.person_id, C.drug_exposure_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id,C.start_date as sort_date
from 
(
  select de.person_id,de.drug_exposure_id,de.drug_concept_id,de.visit_occurrence_id,days_supply,quantity,refills,de.drug_exposure_start_date as start_date, COALESCE(de.drug_exposure_end_date, (de.drug_exposure_start_date + TO_DAYS(CAST(de.days_supply AS INTEGER))), (de.drug_exposure_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.DRUG_EXPOSURE de
JOIN Codesets cs on (de.drug_concept_id = cs.concept_id and cs.codeset_id = 11)
) C
) A on A.person_id = P.person_id  AND A.START_DATE >= P.OP_START_DATE AND A.START_DATE <= P.OP_END_DATE AND A.START_DATE >= (P.START_DATE + TO_DAYS(CAST(-365 AS INTEGER))) AND A.START_DATE <= (P.START_DATE + TO_DAYS(CAST(7 AS INTEGER))) ) cc on p.person_id = cc.person_id and p.event_id = cc.event_id
GROUP BY p.person_id, p.event_id
HAVING COUNT(cc.event_id) = 0
  ) CQ on E.person_id = CQ.person_id and E.event_id = CQ.event_id
  GROUP BY E.person_id, E.event_id
  HAVING COUNT(index_id) = 6
) G
  ) CQ on E.person_id = CQ.person_id and E.event_id = CQ.event_id
  GROUP BY E.person_id, E.event_id
  HAVING COUNT(index_id) > 0
) G
) AC on AC.person_id = pe.person_id and AC.event_id = pe.event_id
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
