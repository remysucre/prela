-- OHDSI PhenotypeLibrary cohort 27, rendered by SqlRender for duckdb, temp tables folded into CTEs
WITH Codesets AS (
SELECT CAST(codeset_id AS int) AS codeset_id, CAST(concept_id AS bigint) AS concept_id FROM (
SELECT * FROM (
SELECT 0 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (317009,4235703,4279553))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (317009,4235703,4279553))
) I
) C UNION ALL 
SELECT 2 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (255573,258780))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (255573,258780))
) I
) C UNION ALL 
SELECT 3 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (21603292))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (21603292))
) I
) C UNION ALL 
SELECT 4 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (42483138,36812530,40727741,1356123,40142665,46234463,1356111,1356108,40142703,21603292,41143395,36421291,40745353,36787954,21158944,36894458,21090035,36883710,35130061,40142784,42479684,35133500,783228,40142910,36787269,1356143,35135829,1356140,44120754,41205832,40924271,40986621,40830666,41205663,40861866,43274335,43263324,43621601,1356154,1356147,40143105,41080205,41174344,41111516,1356173,1356180,40144087,43134418,40727834,36811735,40861768,40727839,35150375,44081619,1356217,1356215,1356101,42800913,40143326,36894464,44817882,40143337,42941603,35146684,35160199,44029688,42481922,36813480,36812414,1356244,40143708,21150787,44082127,41206083,1356138,1356136,40152662,40156382,41048760,40754973,21089505,40144020,36882733,42482744,40144024,37592046,43532281,43291091,41267401,40144035,35147990,35149212,40144037,40223712,42629522,42480849,1356191,1356187,44107471,43145868,43259954,1356196,35145836,42479568,1356211,44055847,44081467,1356120,40182262,40746100,21174574,45775117,41205378,40152687,40167702,35158799,783089,1356189,35152712))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (42483138,36812530,40727741,1356123,40142665,46234463,1356111,1356108,40142703,21603292,41143395,36421291,40745353,36787954,21158944,36894458,21090035,36883710,35130061,40142784,42479684,35133500,783228,40142910,36787269,1356143,35135829,1356140,44120754,41205832,40924271,40986621,40830666,41205663,40861866,43274335,43263324,43621601,1356154,1356147,40143105,41080205,41174344,41111516,1356173,1356180,40144087,43134418,40727834,36811735,40861768,40727839,35150375,44081619,1356217,1356215,1356101,42800913,40143326,36894464,44817882,40143337,42941603,35146684,35160199,44029688,42481922,36813480,36812414,1356244,40143708,21150787,44082127,41206083,1356138,1356136,40152662,40156382,41048760,40754973,21089505,40144020,36882733,42482744,40144024,37592046,43532281,43291091,41267401,40144035,35147990,35149212,40144037,40223712,42629522,42480849,1356191,1356187,44107471,43145868,43259954,1356196,35145836,42479568,1356211,44055847,44081467,1356120,40182262,40746100,21174574,45775117,41205378,40152687,40167702,35158799,783089,1356189,35152712))
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
  select PE.person_id, PE.event_id, PE.start_date, PE.end_date, PE.visit_occurrence_id, PE.sort_date FROM (
select C.person_id, C.drug_exposure_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id,C.start_date as sort_date
from 
(
  select de.person_id,de.drug_exposure_id,de.drug_concept_id,de.visit_occurrence_id,days_supply,quantity,refills,de.drug_exposure_start_date as start_date, COALESCE(de.drug_exposure_end_date, (de.drug_exposure_start_date + TO_DAYS(CAST(de.days_supply AS INTEGER))), (de.drug_exposure_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.DRUG_EXPOSURE de
JOIN Codesets cs on (de.drug_concept_id = cs.concept_id and cs.codeset_id = 4)
) C
JOIN main.PERSON P on C.person_id = P.person_id
WHERE YEAR(CAST(C.start_date AS DATE)) - P.year_of_birth < 55
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
JOIN Codesets cs on (de.drug_concept_id = cs.concept_id and cs.codeset_id = 4)
) C
JOIN main.PERSON P on C.person_id = P.person_id
WHERE YEAR(CAST(C.start_date AS DATE)) - P.year_of_birth < 55
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
JOIN Codesets cs on (de.drug_concept_id = cs.concept_id and cs.codeset_id = 4)
) C
JOIN main.PERSON P on C.person_id = P.person_id
WHERE YEAR(CAST(C.start_date AS DATE)) - P.year_of_birth < 55
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
JOIN Codesets cs on (de.drug_concept_id = cs.concept_id and cs.codeset_id = 4)
) C
) A on A.person_id = P.person_id  AND A.START_DATE >= (P.START_DATE + TO_DAYS(CAST(-365 AS INTEGER))) AND A.START_DATE <= (P.START_DATE + TO_DAYS(CAST(-180 AS INTEGER))) ) cc 
GROUP BY cc.person_id, cc.event_id
HAVING COUNT(cc.event_id) >= 1
  ) CQ on E.person_id = CQ.person_id and E.event_id = CQ.event_id
  GROUP BY E.person_id, E.event_id
  HAVING COUNT(index_id) = 1
) G
) AC on AC.person_id = pe.person_id and AC.event_id = pe.event_id
UNION ALL
SELECT C.person_id, C.condition_occurrence_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id, C.start_date as sort_date
FROM 
(
  SELECT co.person_id,co.condition_occurrence_id,co.condition_concept_id,co.visit_occurrence_id,co.condition_start_date as start_date, COALESCE(co.condition_end_date, (co.condition_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.CONDITION_OCCURRENCE co
  JOIN Codesets cs on (co.condition_concept_id = cs.concept_id and cs.codeset_id = 0)
) C
UNION ALL
select C.person_id, C.observation_id as event_id, C.start_date, C.END_DATE,
       C.visit_occurrence_id, C.start_date as sort_date
from 
(
  select o.person_id,o.observation_id,o.observation_concept_id,o.visit_occurrence_id,o.value_as_number,o.observation_date as start_date, (o.observation_date + TO_DAYS(CAST(1 AS INTEGER))) as end_date 
  FROM main.OBSERVATION o
JOIN Codesets cs on (o.observation_concept_id = cs.concept_id and cs.codeset_id = 0)
) C
  ) E
	JOIN main.observation_period OP on E.person_id = OP.person_id and E.start_date >=  OP.observation_period_start_date and E.start_date <= op.observation_period_end_date
  WHERE (OP.OBSERVATION_PERIOD_START_DATE + TO_DAYS(CAST(0 AS INTEGER))) <= E.START_DATE AND (E.START_DATE + TO_DAYS(CAST(0 AS INTEGER))) <= OP.OBSERVATION_PERIOD_END_DATE
) P
WHERE P.ordinal = 1
) pe
) QE
),
Inclusion_0 AS (
SELECT
0 as inclusion_rule_id, person_id, event_id
FROM
(
  select pe.person_id, pe.event_id
  FROM qualified_events pe
JOIN (
select 0 as index_id, person_id, event_id
FROM
(
  select E.person_id, E.event_id 
  FROM qualified_events E
  INNER JOIN
  (
select 0 as index_id, p.person_id, p.event_id
from qualified_events p
LEFT JOIN (
SELECT p.person_id, p.event_id 
FROM qualified_events P
JOIN (
SELECT C.person_id, C.condition_occurrence_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id, C.start_date as sort_date
FROM 
(
  SELECT co.person_id,co.condition_occurrence_id,co.condition_concept_id,co.visit_occurrence_id,co.condition_start_date as start_date, COALESCE(co.condition_end_date, (co.condition_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.CONDITION_OCCURRENCE co
  JOIN Codesets cs on (co.condition_concept_id = cs.concept_id and cs.codeset_id = 2)
) C
) A on A.person_id = P.person_id  AND A.START_DATE >= P.OP_START_DATE AND A.START_DATE <= P.OP_END_DATE AND A.START_DATE >= P.OP_START_DATE AND A.START_DATE <= (P.START_DATE + TO_DAYS(CAST(0 AS INTEGER))) ) cc on p.person_id = cc.person_id and p.event_id = cc.event_id
GROUP BY p.person_id, p.event_id
HAVING COUNT(cc.event_id) = 0
UNION ALL
select 1 as index_id, p.person_id, p.event_id
from qualified_events p
LEFT JOIN (
SELECT p.person_id, p.event_id 
FROM qualified_events P
JOIN (
select C.person_id, C.drug_exposure_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id,C.start_date as sort_date
from 
(
  select de.person_id,de.drug_exposure_id,de.drug_concept_id,de.visit_occurrence_id,days_supply,quantity,refills,de.drug_exposure_start_date as start_date, COALESCE(de.drug_exposure_end_date, (de.drug_exposure_start_date + TO_DAYS(CAST(de.days_supply AS INTEGER))), (de.drug_exposure_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.DRUG_EXPOSURE de
JOIN Codesets cs on (de.drug_concept_id = cs.concept_id and cs.codeset_id = 3)
) C
) A on A.person_id = P.person_id  AND A.START_DATE >= P.OP_START_DATE AND A.START_DATE <= P.OP_END_DATE AND A.START_DATE >= P.OP_START_DATE AND A.START_DATE <= (P.START_DATE + TO_DAYS(CAST(0 AS INTEGER))) ) cc on p.person_id = cc.person_id and p.event_id = cc.event_id
GROUP BY p.person_id, p.event_id
HAVING COUNT(cc.event_id) = 0
  ) CQ on E.person_id = CQ.person_id and E.event_id = CQ.event_id
  GROUP BY E.person_id, E.event_id
  HAVING COUNT(index_id) = 2
) G
) AC on AC.person_id = pe.person_id AND AC.event_id = pe.event_id
) Results
),
inclusion_events AS (
SELECT
inclusion_rule_id, person_id, event_id
FROM
(select inclusion_rule_id, person_id, event_id from Inclusion_0) I
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
  WHERE (MG.inclusion_rule_mask = POWER(cast(2 as bigint),1)-1)
) Results
WHERE Results.ordinal = 1
),
strategy_ends AS (
SELECT
event_id, person_id, 
  case when (end_date + TO_DAYS(CAST(0 AS INTEGER))) > op_end_date then op_end_date else (end_date + TO_DAYS(CAST(0 AS INTEGER))) end as end_date
FROM
included_events
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
SELECT event_id, person_id, end_date from strategy_ends
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
