-- OHDSI PhenotypeLibrary cohort 1106, rendered by SqlRender for duckdb, temp tables folded into CTEs
WITH Codesets AS (
SELECT CAST(codeset_id AS int) AS codeset_id, CAST(concept_id AS bigint) AS concept_id FROM (
SELECT * FROM (
SELECT 0 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (9201))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (9201))
) I
) C UNION ALL 
SELECT 1 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (4135199,3363351,2107924,2107949,2107837,4283095,4240486,4142641,4234989,4035152,2107740))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4135199,3363351,2107924,2107949,2107837,4240486,4142641,4234989))
) I
LEFT JOIN
(
  select concept_id from main.CONCEPT where (concept_id in (4311405,4249893,4304358,4250892,4300528,4138127,4042673,2853855,4272324,2108020,46273729,4230535))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4311405,4249893,4304358,4250892,4300528,4138127,4042673,2853855))
) E ON I.concept_id = E.concept_id
WHERE E.concept_id is null
) C UNION ALL 
SELECT 2 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (4236706))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4236706))
) I
LEFT JOIN
(
  select concept_id from main.CONCEPT where (concept_id in (4311405,4249893,4304358,4250892,4300528,4138127,4042673,4272324,2108020,46273729,4230535))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4311405,4249893,4304358,4250892,4300528,4138127,4042673))
) E ON I.concept_id = E.concept_id
WHERE E.concept_id is null
) C UNION ALL 
SELECT 30 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (4195136))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4195136))
) I
LEFT JOIN
(
  select concept_id from main.CONCEPT where (concept_id in (4311405,4249893,4304358,4250892,4300528,4138127,4042673,4225223,4272324,2108020,46273729,4230535))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4311405,4249893,4304358,4250892,4300528,4138127,4042673,4225223))
) E ON I.concept_id = E.concept_id
WHERE E.concept_id is null
) C UNION ALL 
SELECT 31 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (4021530))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4021530))
) I
LEFT JOIN
(
  select concept_id from main.CONCEPT where (concept_id in (4311405,4249893,4304358,4250892,4300528,4138127,4042673,4272324,2108020,46273729,4230535))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4311405,4249893,4304358,4250892,4300528,4138127,4042673))
) E ON I.concept_id = E.concept_id
WHERE E.concept_id is null
) C UNION ALL 
SELECT 32 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (4127886,4127887,4021529,2110177,4023403,42538159,42538160,4032622,2110209,2110211,43018211,4138738,4306298,4306070,4072416,4070209,45769914,45769913,45769911))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (2110177,4023403,42538159,42538160,43018211,4306298))
) I
LEFT JOIN
(
  select concept_id from main.CONCEPT where (concept_id in (4311405,4249893,4304358,4250892,4300528,4138127,4042673,4272324,2108020,46273729,4230535))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4311405,4249893,4304358,4250892,4300528,4138127,4042673))
) E ON I.concept_id = E.concept_id
WHERE E.concept_id is null
) C UNION ALL 
SELECT 33 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (4096783))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4096783))
) I
LEFT JOIN
(
  select concept_id from main.CONCEPT where (concept_id in (4311405,4249893,4304358,4250892,4300528,4138127,4042673,4272324,2108020,46273729,4230535))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4311405,4249893,4304358,4250892,4300528,4138127,4042673))
) E ON I.concept_id = E.concept_id
WHERE E.concept_id is null
) C UNION ALL 
SELECT 34 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (4234536))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4234536))
) I
LEFT JOIN
(
  select concept_id from main.CONCEPT where (concept_id in (4311405,4249893,4304358,4250892,4300528,4138127,4042673,4272324,2108020,46273729,4230535))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4311405,4249893,4304358,4250892,4300528,4138127,4042673))
) E ON I.concept_id = E.concept_id
WHERE E.concept_id is null
) C UNION ALL 
SELECT 38 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (4312749))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4312749))
) I
LEFT JOIN
(
  select concept_id from main.CONCEPT where (concept_id in (4323208,4311405,4249893,4304358,4250892,4300528,4138127,4042673,4272324,2108020,46273729,4230535))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4323208,4311405,4249893,4304358,4250892,4300528,4138127,4042673))
) E ON I.concept_id = E.concept_id
WHERE E.concept_id is null
) C UNION ALL 
SELECT 43 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (262,9203))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (262,9203))
) I
) C UNION ALL 
SELECT 45 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (4013040,4196081,38001162,40482705,4128868,4314436,4125173))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4013040,4196081,38001162,40482705,4128868,4314436,4125173))
) I
LEFT JOIN
(
  select concept_id from main.CONCEPT where (concept_id in (4144721,4035031,4195115,4311405,4170609,4249893,4304358,4250892,4300528,4138127,4179713,4126235,4194372,4062763,4127886,44809616,4179797,4030387,2002949,4144723,4042673,4166761,4199870,4200964,4272324,2108020,46273729,4230535))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4144721,4035031,4195115,4311405,4170609,4249893,4304358,4250892,4300528,4138127,4179713,4126235,4194372,4062763,4127886,44809616,4179797,4030387,2002949,4144723,4042673,4166761,4199870,4200964))
) E ON I.concept_id = E.concept_id
WHERE E.concept_id is null
) C UNION ALL 
SELECT 46 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (4101626))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4101626))
) I
LEFT JOIN
(
  select concept_id from main.CONCEPT where (concept_id in (4311405,4249893,4304358,4232401,4250892,4300528,4138127,40490839,4343795,4243985,4080349,4296539,4069383,4182228,4284392,4121136,4042673,45887543,2103134,2103133,4265725,4136234,4272324,2108020,46273729,4230535))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4311405,4249893,4304358,4232401,4250892,4300528,4138127,40490839,4343795,4243985,4080349,4296539,4069383,4182228,4284392,4121136,4042673,45887543,2103134,2103133,4265725,4136234))
) E ON I.concept_id = E.concept_id
WHERE E.concept_id is null
) C UNION ALL 
SELECT 47 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (2108643,2108645,2768119,2830031,4077122,2103867,2740455,2756407,2760818,2740654,2756413,2756411))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (2108643,2108645,2768119,2830031,4077122,2103867,2740455,2756407,2760818,2740654,2756413,2756411))
) I
LEFT JOIN
(
  select concept_id from main.CONCEPT where (concept_id in (4311405,4249893,4304358,4250892,4300528,4138127,42872673,4003042,4030715,4042673,4124209,2768121,2768142,2768127,2768130,4272324,2108020,46273729,4230535))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4311405,4249893,4304358,4250892,4300528,4138127,42872673,4003042,4030715,4042673,4124209,2768121,2768142,2768127,2768130))
) E ON I.concept_id = E.concept_id
WHERE E.concept_id is null
) C UNION ALL 
SELECT 48 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (4078547,43531648))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (43531648))
) I
LEFT JOIN
(
  select concept_id from main.CONCEPT where (concept_id in (4311405,4249893,4304358,4250892,4300528,4138127,4239294,2824731,2005871,4042673,4180403,4106397,2000080,2000081,2000079,2105129,2000082,4272324,2108020,46273729,4230535))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4311405,4249893,4304358,4250892,4300528,4138127,4239294,2824731,2005871,4042673,4180403,4106397,2000080,2000081,2000079,2105129,2000082))
) E ON I.concept_id = E.concept_id
WHERE E.concept_id is null
) C UNION ALL 
SELECT 51 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (2811735,2896830,2839189,4345349,4322818,4067461,4069129,4312613,40487106,46270641,4142436,2109316,4251035))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (2811735,2896830,2839189,4345349))
) I
LEFT JOIN
(
  select concept_id from main.CONCEPT where (concept_id in (4030150,4314001,4147687,4076862,4311405,4272324,4249893,4304358,4250892,4300528,4138127,46273729,4230535))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4030150,4314001,4147687,4076862,46273729))
) E ON I.concept_id = E.concept_id
WHERE E.concept_id is null
) C UNION ALL 
SELECT 55 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (4180074))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4180074))
) I
LEFT JOIN
(
  select concept_id from main.CONCEPT where (concept_id in (4311405,4249893,4304358,4250892,4300528,4138127,4042673,4272324,2108020,46273729,4230535))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4311405,4249893,4304358,4250892,4300528,4138127,4042673))
) E ON I.concept_id = E.concept_id
WHERE E.concept_id is null
) C UNION ALL 
SELECT 56 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (4049675,4244434,4236715,42872711,2211642,4052252,2107574,4263586,4294254,4022030,40757028,2733576,4305183,2733582,2733567,4243023,4051031,40493483,40489434,40756876,2107882,2107931,2733577,2733579))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4049675))
) I
LEFT JOIN
(
  select concept_id from main.CONCEPT where (concept_id in (4267446,4311405,42736434,4249893,4304358,4250892,4300528,4138127,2211645,2211646,2107573,4042673,4181152,4272324,2108020,46273729,4230535))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4267446,4311405,42736434,4249893,4304358,4250892,4300528,4138127,2211645,2211646,2107573,4042673,4181152,46273729))
) E ON I.concept_id = E.concept_id
WHERE E.concept_id is null
) C UNION ALL 
SELECT 57 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (42872694))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (42872694))
) I
LEFT JOIN
(
  select concept_id from main.CONCEPT where (concept_id in (4050715,4311405,4249893,4304358,42896616,42896602,42896609,42896661,42896610,2731751,2731749,42896667,2731750,4250892,4300528,4138127,4042673,4272324,2108020,46273729,4230535))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4050715,4311405,4249893,4304358,42896616,42896602,42896609,42896661,42896610,2731751,2731749,42896667,2731750,4250892,4300528,4138127,4042673))
) E ON I.concept_id = E.concept_id
WHERE E.concept_id is null
) C UNION ALL 
SELECT 58 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (42872696))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (42872696))
) I
LEFT JOIN
(
  select concept_id from main.CONCEPT where (concept_id in (4050131,4311405,2730933,4249893,4304358,4116625,4250892,4300528,4138127,4195806,4042673,4272324,2108020,46273729,4230535))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4050131,4311405,2730933,4249893,4304358,4116625,4250892,4300528,4138127,4195806,4042673))
) E ON I.concept_id = E.concept_id
WHERE E.concept_id is null
) C UNION ALL 
SELECT 59 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (4022030))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4022030))
) I
LEFT JOIN
(
  select concept_id from main.CONCEPT where (concept_id in (4311405,4249893,4304358,4250892,4300528,4138127,4168349,4042673,4229447,4272324,2108020,46273729,4230535))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4311405,4249893,4304358,4250892,4300528,4138127,4168349,4042673,4229447))
) E ON I.concept_id = E.concept_id
WHERE E.concept_id is null
) C UNION ALL 
SELECT 61 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (4049675,3188936,3186130,4259553,46271672,4000088,44793144,4181152,4199966,45887730,2107101,2107112))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (3188936,3186130,4259553,46271672,4000088,44793144,4181152,4199966,45887730,2107101,2107112))
) I
LEFT JOIN
(
  select concept_id from main.CONCEPT where (concept_id in (4311405,2107453,4272324,4249893,4304358,4250892,4300528,4138127,2211645,2211646,2107431,2107430,2211648,2107581,2107429,2107428,2211647,4042673,2107425,2107310,2108020,46273729,4230535))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4311405,2107453,4272324,4249893,4304358,4250892,4300528,4138127,2211645,2211646,2107431,2107430,2211648,2107581,2107429,2107428,2211647,4042673,2107425,2107310))
) E ON I.concept_id = E.concept_id
WHERE E.concept_id is null
) C UNION ALL 
SELECT 62 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (4031312,4030825,4120657))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4031312,4030825,4120657))
) I
LEFT JOIN
(
  select concept_id from main.CONCEPT where (concept_id in (4311405,4249893,4304358,4250892,4300528,4138127,4042673,4272324,2108020,46273729,4230535))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4311405,4249893,4304358,4250892,4300528,4138127,4042673))
) E ON I.concept_id = E.concept_id
WHERE E.concept_id is null
) C UNION ALL 
SELECT 63 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (2006434,2006432,4297155,4001562,709956,709957,4183741,42733287,2103375,42732580,2740664,2106890,2106891,2006433,2006421,4337034,2102700,2102701,2102712,3177626,36676345,36676344,40480531,4127484,4302594,4231761,2102734,4301336,2106769,4301338,2106770))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (2006434,2006432,4297155,709956,709957,4183741,42733287,2103375,42732580,2740664,2106890,2106891,2006433,2006421,4337034))
) I
LEFT JOIN
(
  select concept_id from main.CONCEPT where (concept_id in (4311405,4249893,4304358,4250892,4300528,4138127,4042673))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4311405,4249893,4304358,4250892,4300528,4138127,4042673))
) E ON I.concept_id = E.concept_id
WHERE E.concept_id is null
) C UNION ALL 
SELECT 64 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (4203442,4121242,2002549,2108974,40483096))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4203442,4121242,2002549,2108974,40483096))
) I
LEFT JOIN
(
  select concept_id from main.CONCEPT where (concept_id in (4170280,4311405,4249893,4304358,4250892,4300528,4138127,4042673,4272324,2108020,46273729,4230535))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4170280,4311405,4249893,4304358,4250892,4300528,4138127,4042673))
) E ON I.concept_id = E.concept_id
WHERE E.concept_id is null
) C UNION ALL 
SELECT 65 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (4141456,2003225,4141110))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4141456,2003225,4141110))
) I
LEFT JOIN
(
  select concept_id from main.CONCEPT where (concept_id in (4311405,4249893,4304358,4250892,4300528,4138127,4042673,4272324,2108020,46273729,4230535))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4311405,4249893,4304358,4250892,4300528,4138127,4042673))
) E ON I.concept_id = E.concept_id
WHERE E.concept_id is null
) C UNION ALL 
SELECT 66 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (4187533,725062,2108937))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4187533,725062,2108937))
) I
LEFT JOIN
(
  select concept_id from main.CONCEPT where (concept_id in (4311405,4249893,4304358,4250892,4300528,4138127,4042673,4272324,2108020,46273729,4230535))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4311405,4249893,4304358,4250892,4300528,4138127,4042673))
) E ON I.concept_id = E.concept_id
WHERE E.concept_id is null
) C UNION ALL 
SELECT 67 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (2108486,4120425,2108645,2108640,2108475,42735001,2102721,42733288,42732332,2110038,2108490,42742521,2110372,2006421,4125465,2108482,4012004,4100937,2108485,2102736,4229920,4208330))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (2108486,2108645,2108640,2108475,42735001,2102721,42733288,42732332,2110038,2108490,42742521,2110372,2006421))
) I
LEFT JOIN
(
  select concept_id from main.CONCEPT where (concept_id in (4311405,4249893,4304358,4250892,4300528,4138127,4042673,4272324,2108020,46273729,4230535))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4311405,4249893,4304358,4250892,4300528,4138127,4042673))
) E ON I.concept_id = E.concept_id
WHERE E.concept_id is null
) C UNION ALL 
SELECT 68 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (4000882))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4000882))
) I
LEFT JOIN
(
  select concept_id from main.CONCEPT where (concept_id in (4311405,4249893,4304358,4250892,4300528,4138127,4042673))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4311405,4249893,4304358,4250892,4300528,4138127,4042673))
) E ON I.concept_id = E.concept_id
WHERE E.concept_id is null
) C UNION ALL 
SELECT 69 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (4321450,4159841,4076334,2005415,2104897,2104896,2104912,2104919,2105149,4002375,2798725,2819144,2852443,2819622,2832272,2886663,2865770,2824740,2860039,2886683,2891748,2865764,4211987,2005457,2104900,2104899,2104943,2104914,2105166,2105165,2104920,2104934,2104898,4297365,2105163,2104913,2005882,4090926,2104839,2104840,4075926,4203771,2104918,2104917,2108020))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4321450,4159841,4076334,2005415,2104897,2104896,2104912,2104919,2105149,4002375,2798725,2819144,2852443,2819622,2832272,2886663,2865770,2824740,2860039,2886683,2891748,2865764,4211987,2005457,2104900,2104899,2104943,2104914,2105166,2105165,2104920,2104934,2104898,4297365,2105163,2104913,2005882,4090926,2104839,2104840,4075926,4203771,2104918,2104917))
) I
LEFT JOIN
(
  select concept_id from main.CONCEPT where (concept_id in (4311405,4249893,4304358,4250892,4138127,4042673,4272324,4300528,46273729,4230535))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4311405,4249893,4304358,4250892,4138127,4042673))
) E ON I.concept_id = E.concept_id
WHERE E.concept_id is null
) C UNION ALL 
SELECT 70 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (4315036,4286744,43531416))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4315036,4286744,43531416))
) I
LEFT JOIN
(
  select concept_id from main.CONCEPT where (concept_id in (4311405,4249893,4304358,4250892,4300528,4138127,4042673,4272324,2108020,46273729,4230535))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4311405,4249893,4304358,4250892,4300528,4138127,4042673))
) E ON I.concept_id = E.concept_id
WHERE E.concept_id is null
) C UNION ALL 
SELECT 71 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (4219032,2006227,2006228,4338257,4264289,2105446,2105448,2105450,2105449,2784251,2784250,4143795,2006242,2105451,4119910))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (2006227,2006228))
) I
LEFT JOIN
(
  select concept_id from main.CONCEPT where (concept_id in (4311405,4249893,4304358,4250892,4300528,4138127,4042673,4272324,2108020,46273729,4230535))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4311405,4249893,4304358,4250892,4300528,4138127,4042673))
) E ON I.concept_id = E.concept_id
WHERE E.concept_id is null
) C UNION ALL 
SELECT 72 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (4000882,2001339,2001342,42742510))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4000882,2001339,2001342,42742510))
) I
LEFT JOIN
(
  select concept_id from main.CONCEPT where (concept_id in (4311405,4249893,4304358,4250892,4300528,4138127,4042673,4272324,2108020,46273729,4230535))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4311405,4249893,4304358,4250892,4300528,4138127,4042673))
) E ON I.concept_id = E.concept_id
WHERE E.concept_id is null
) C UNION ALL 
SELECT 73 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (2722219,4241198))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (2722219,4241198))
) I
LEFT JOIN
(
  select concept_id from main.CONCEPT where (concept_id in (4311405,4249893,4304358,4250892,4300528,4138127,4042673,4272324,2108020,46273729,4230535))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4311405,4249893,4304358,4250892,4300528,4138127,4042673))
) E ON I.concept_id = E.concept_id
WHERE E.concept_id is null
) C UNION ALL 
SELECT 74 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (2109616,4047234))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (2109616,4047234))
) I
LEFT JOIN
(
  select concept_id from main.CONCEPT where (concept_id in (4311405,4249893,4304358,4250892,4300528,4138127,2752031,2752037,4042673,4272324,2108020,46273729,4230535))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4311405,4249893,4304358,4250892,4300528,4138127,2752031,2752037,4042673))
) E ON I.concept_id = E.concept_id
WHERE E.concept_id is null
) C UNION ALL 
SELECT 75 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (38001216,38001215,38001217,38001213,38001212,38001214,2003223,38001219,38001218,38001220,2003224,2722214,2722215,2722216,4059308,4242997,2109368,4033555,4240962,4134888))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (38001216,38001215,38001217,38001213,38001212,38001214,2003223,38001219,38001218,38001220,2003224,2722214,2722215,2722216))
) I
LEFT JOIN
(
  select concept_id from main.CONCEPT where (concept_id in (4311405,4249893,4304358,4250892,4138127,4179071,4042673,4272324,4300528,2108020,46273729,4230535))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4311405,4249893,4304358,4250892,4138127,4179071,4042673))
) E ON I.concept_id = E.concept_id
WHERE E.concept_id is null
) C UNION ALL 
SELECT 76 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (4216472,44809456,44809457,4264332,2102759,4032640,4216471,4064295,2102773,2102761))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (4216472,44809456,44809457,4264332,2102759,4032640,4216471,4064295,2102773,2102761))
) I
) C UNION ALL 
SELECT 78 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (2003708,2776176,2775402,2775928,2775927,2774933,2775894,2775672,2775671,2775665,2109680,2003724,4077764,4235644,4107717,2109657,4022960,4029570,4184788,4022807,4021108,4304536,2773682,2773695,2109617,2773685,2109574,4288997,4027426,4270496,4118715,4010266,2109723,2109724,2109715,2773934,2109722,2773938))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (2003708,2776176,2775402,2775928,2775927,2774933,2775894,2775672,2775671,2775665,2109680,2003724,4077764,4235644,4107717,2109657,4022960,4029570,4184788,4270496,2109723,2109724,2109715,2109722))
) I
) C UNION ALL 
SELECT 79 as codeset_id, c.concept_id FROM (select distinct I.concept_id FROM
( 
  select concept_id from main.CONCEPT where (concept_id in (1201620,1154029,1174888,1126658,1103640,1110410,1124957,1125765,19026459,1103314,1102527))
UNION  select c.concept_id
  from main.CONCEPT c
  join main.CONCEPT_ANCESTOR ca on c.concept_id = ca.descendant_concept_id
  WHERE c.invalid_reason is null
  and (ca.ancestor_concept_id in (1201620,1154029,1174888,1126658,1103640,1110410,1124957,1125765,19026459,1103314,1102527))
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
select C.person_id, C.procedure_occurrence_id as event_id, C.start_date, C.end_date,
       C.visit_occurrence_id, C.start_date as sort_date
from 
(
  select po.person_id,po.procedure_occurrence_id,po.procedure_concept_id,po.visit_occurrence_id,po.quantity,po.procedure_date as start_date, (po.procedure_date + TO_DAYS(CAST(1 AS INTEGER))) as end_date 
  FROM main.PROCEDURE_OCCURRENCE po
JOIN Codesets cs on (po.procedure_concept_id = cs.concept_id and cs.codeset_id = 1)
) C
UNION ALL
select C.person_id, C.procedure_occurrence_id as event_id, C.start_date, C.end_date,
       C.visit_occurrence_id, C.start_date as sort_date
from 
(
  select po.person_id,po.procedure_occurrence_id,po.procedure_concept_id,po.visit_occurrence_id,po.quantity,po.procedure_date as start_date, (po.procedure_date + TO_DAYS(CAST(1 AS INTEGER))) as end_date 
  FROM main.PROCEDURE_OCCURRENCE po
JOIN Codesets cs on (po.procedure_concept_id = cs.concept_id and cs.codeset_id = 2)
) C
UNION ALL
select C.person_id, C.procedure_occurrence_id as event_id, C.start_date, C.end_date,
       C.visit_occurrence_id, C.start_date as sort_date
from 
(
  select po.person_id,po.procedure_occurrence_id,po.procedure_concept_id,po.visit_occurrence_id,po.quantity,po.procedure_date as start_date, (po.procedure_date + TO_DAYS(CAST(1 AS INTEGER))) as end_date 
  FROM main.PROCEDURE_OCCURRENCE po
JOIN Codesets cs on (po.procedure_concept_id = cs.concept_id and cs.codeset_id = 45)
) C
UNION ALL
select C.person_id, C.procedure_occurrence_id as event_id, C.start_date, C.end_date,
       C.visit_occurrence_id, C.start_date as sort_date
from 
(
  select po.person_id,po.procedure_occurrence_id,po.procedure_concept_id,po.visit_occurrence_id,po.quantity,po.procedure_date as start_date, (po.procedure_date + TO_DAYS(CAST(1 AS INTEGER))) as end_date 
  FROM main.PROCEDURE_OCCURRENCE po
JOIN Codesets cs on (po.procedure_concept_id = cs.concept_id and cs.codeset_id = 57)
) C
UNION ALL
select C.person_id, C.procedure_occurrence_id as event_id, C.start_date, C.end_date,
       C.visit_occurrence_id, C.start_date as sort_date
from 
(
  select po.person_id,po.procedure_occurrence_id,po.procedure_concept_id,po.visit_occurrence_id,po.quantity,po.procedure_date as start_date, (po.procedure_date + TO_DAYS(CAST(1 AS INTEGER))) as end_date 
  FROM main.PROCEDURE_OCCURRENCE po
JOIN Codesets cs on (po.procedure_concept_id = cs.concept_id and cs.codeset_id = 58)
) C
UNION ALL
select C.person_id, C.procedure_occurrence_id as event_id, C.start_date, C.end_date,
       C.visit_occurrence_id, C.start_date as sort_date
from 
(
  select po.person_id,po.procedure_occurrence_id,po.procedure_concept_id,po.visit_occurrence_id,po.quantity,po.procedure_date as start_date, (po.procedure_date + TO_DAYS(CAST(1 AS INTEGER))) as end_date 
  FROM main.PROCEDURE_OCCURRENCE po
JOIN Codesets cs on (po.procedure_concept_id = cs.concept_id and cs.codeset_id = 61)
) C
UNION ALL
select C.person_id, C.procedure_occurrence_id as event_id, C.start_date, C.end_date,
       C.visit_occurrence_id, C.start_date as sort_date
from 
(
  select po.person_id,po.procedure_occurrence_id,po.procedure_concept_id,po.visit_occurrence_id,po.quantity,po.procedure_date as start_date, (po.procedure_date + TO_DAYS(CAST(1 AS INTEGER))) as end_date 
  FROM main.PROCEDURE_OCCURRENCE po
JOIN Codesets cs on (po.procedure_concept_id = cs.concept_id and cs.codeset_id = 55)
) C
UNION ALL
select C.person_id, C.procedure_occurrence_id as event_id, C.start_date, C.end_date,
       C.visit_occurrence_id, C.start_date as sort_date
from 
(
  select po.person_id,po.procedure_occurrence_id,po.procedure_concept_id,po.visit_occurrence_id,po.quantity,po.procedure_date as start_date, (po.procedure_date + TO_DAYS(CAST(1 AS INTEGER))) as end_date 
  FROM main.PROCEDURE_OCCURRENCE po
JOIN Codesets cs on (po.procedure_concept_id = cs.concept_id and cs.codeset_id = 51)
) C
UNION ALL
select C.person_id, C.procedure_occurrence_id as event_id, C.start_date, C.end_date,
       C.visit_occurrence_id, C.start_date as sort_date
from 
(
  select po.person_id,po.procedure_occurrence_id,po.procedure_concept_id,po.visit_occurrence_id,po.quantity,po.procedure_date as start_date, (po.procedure_date + TO_DAYS(CAST(1 AS INTEGER))) as end_date 
  FROM main.PROCEDURE_OCCURRENCE po
JOIN Codesets cs on (po.procedure_concept_id = cs.concept_id and cs.codeset_id = 56)
) C
UNION ALL
select C.person_id, C.procedure_occurrence_id as event_id, C.start_date, C.end_date,
       C.visit_occurrence_id, C.start_date as sort_date
from 
(
  select po.person_id,po.procedure_occurrence_id,po.procedure_concept_id,po.visit_occurrence_id,po.quantity,po.procedure_date as start_date, (po.procedure_date + TO_DAYS(CAST(1 AS INTEGER))) as end_date 
  FROM main.PROCEDURE_OCCURRENCE po
JOIN Codesets cs on (po.procedure_concept_id = cs.concept_id and cs.codeset_id = 47)
) C
UNION ALL
select C.person_id, C.procedure_occurrence_id as event_id, C.start_date, C.end_date,
       C.visit_occurrence_id, C.start_date as sort_date
from 
(
  select po.person_id,po.procedure_occurrence_id,po.procedure_concept_id,po.visit_occurrence_id,po.quantity,po.procedure_date as start_date, (po.procedure_date + TO_DAYS(CAST(1 AS INTEGER))) as end_date 
  FROM main.PROCEDURE_OCCURRENCE po
JOIN Codesets cs on (po.procedure_concept_id = cs.concept_id and cs.codeset_id = 62)
) C
UNION ALL
select C.person_id, C.procedure_occurrence_id as event_id, C.start_date, C.end_date,
       C.visit_occurrence_id, C.start_date as sort_date
from 
(
  select po.person_id,po.procedure_occurrence_id,po.procedure_concept_id,po.visit_occurrence_id,po.quantity,po.procedure_date as start_date, (po.procedure_date + TO_DAYS(CAST(1 AS INTEGER))) as end_date 
  FROM main.PROCEDURE_OCCURRENCE po
JOIN Codesets cs on (po.procedure_concept_id = cs.concept_id and cs.codeset_id = 63)
) C
UNION ALL
select C.person_id, C.procedure_occurrence_id as event_id, C.start_date, C.end_date,
       C.visit_occurrence_id, C.start_date as sort_date
from 
(
  select po.person_id,po.procedure_occurrence_id,po.procedure_concept_id,po.visit_occurrence_id,po.quantity,po.procedure_date as start_date, (po.procedure_date + TO_DAYS(CAST(1 AS INTEGER))) as end_date 
  FROM main.PROCEDURE_OCCURRENCE po
JOIN Codesets cs on (po.procedure_concept_id = cs.concept_id and cs.codeset_id = 64)
) C
UNION ALL
select C.person_id, C.procedure_occurrence_id as event_id, C.start_date, C.end_date,
       C.visit_occurrence_id, C.start_date as sort_date
from 
(
  select po.person_id,po.procedure_occurrence_id,po.procedure_concept_id,po.visit_occurrence_id,po.quantity,po.procedure_date as start_date, (po.procedure_date + TO_DAYS(CAST(1 AS INTEGER))) as end_date 
  FROM main.PROCEDURE_OCCURRENCE po
JOIN Codesets cs on (po.procedure_concept_id = cs.concept_id and cs.codeset_id = 65)
) C
UNION ALL
select C.person_id, C.procedure_occurrence_id as event_id, C.start_date, C.end_date,
       C.visit_occurrence_id, C.start_date as sort_date
from 
(
  select po.person_id,po.procedure_occurrence_id,po.procedure_concept_id,po.visit_occurrence_id,po.quantity,po.procedure_date as start_date, (po.procedure_date + TO_DAYS(CAST(1 AS INTEGER))) as end_date 
  FROM main.PROCEDURE_OCCURRENCE po
JOIN Codesets cs on (po.procedure_concept_id = cs.concept_id and cs.codeset_id = 66)
) C
UNION ALL
select C.person_id, C.procedure_occurrence_id as event_id, C.start_date, C.end_date,
       C.visit_occurrence_id, C.start_date as sort_date
from 
(
  select po.person_id,po.procedure_occurrence_id,po.procedure_concept_id,po.visit_occurrence_id,po.quantity,po.procedure_date as start_date, (po.procedure_date + TO_DAYS(CAST(1 AS INTEGER))) as end_date 
  FROM main.PROCEDURE_OCCURRENCE po
JOIN Codesets cs on (po.procedure_concept_id = cs.concept_id and cs.codeset_id = 59)
) C
UNION ALL
select C.person_id, C.procedure_occurrence_id as event_id, C.start_date, C.end_date,
       C.visit_occurrence_id, C.start_date as sort_date
from 
(
  select po.person_id,po.procedure_occurrence_id,po.procedure_concept_id,po.visit_occurrence_id,po.quantity,po.procedure_date as start_date, (po.procedure_date + TO_DAYS(CAST(1 AS INTEGER))) as end_date 
  FROM main.PROCEDURE_OCCURRENCE po
JOIN Codesets cs on (po.procedure_concept_id = cs.concept_id and cs.codeset_id = 67)
) C
UNION ALL
select C.person_id, C.procedure_occurrence_id as event_id, C.start_date, C.end_date,
       C.visit_occurrence_id, C.start_date as sort_date
from 
(
  select po.person_id,po.procedure_occurrence_id,po.procedure_concept_id,po.visit_occurrence_id,po.quantity,po.procedure_date as start_date, (po.procedure_date + TO_DAYS(CAST(1 AS INTEGER))) as end_date 
  FROM main.PROCEDURE_OCCURRENCE po
JOIN Codesets cs on (po.procedure_concept_id = cs.concept_id and cs.codeset_id = 68)
) C
UNION ALL
select C.person_id, C.procedure_occurrence_id as event_id, C.start_date, C.end_date,
       C.visit_occurrence_id, C.start_date as sort_date
from 
(
  select po.person_id,po.procedure_occurrence_id,po.procedure_concept_id,po.visit_occurrence_id,po.quantity,po.procedure_date as start_date, (po.procedure_date + TO_DAYS(CAST(1 AS INTEGER))) as end_date 
  FROM main.PROCEDURE_OCCURRENCE po
JOIN Codesets cs on (po.procedure_concept_id = cs.concept_id and cs.codeset_id = 69)
) C
UNION ALL
select C.person_id, C.procedure_occurrence_id as event_id, C.start_date, C.end_date,
       C.visit_occurrence_id, C.start_date as sort_date
from 
(
  select po.person_id,po.procedure_occurrence_id,po.procedure_concept_id,po.visit_occurrence_id,po.quantity,po.procedure_date as start_date, (po.procedure_date + TO_DAYS(CAST(1 AS INTEGER))) as end_date 
  FROM main.PROCEDURE_OCCURRENCE po
JOIN Codesets cs on (po.procedure_concept_id = cs.concept_id and cs.codeset_id = 70)
) C
UNION ALL
select C.person_id, C.procedure_occurrence_id as event_id, C.start_date, C.end_date,
       C.visit_occurrence_id, C.start_date as sort_date
from 
(
  select po.person_id,po.procedure_occurrence_id,po.procedure_concept_id,po.visit_occurrence_id,po.quantity,po.procedure_date as start_date, (po.procedure_date + TO_DAYS(CAST(1 AS INTEGER))) as end_date 
  FROM main.PROCEDURE_OCCURRENCE po
JOIN Codesets cs on (po.procedure_concept_id = cs.concept_id and cs.codeset_id = 71)
) C
UNION ALL
select C.person_id, C.procedure_occurrence_id as event_id, C.start_date, C.end_date,
       C.visit_occurrence_id, C.start_date as sort_date
from 
(
  select po.person_id,po.procedure_occurrence_id,po.procedure_concept_id,po.visit_occurrence_id,po.quantity,po.procedure_date as start_date, (po.procedure_date + TO_DAYS(CAST(1 AS INTEGER))) as end_date 
  FROM main.PROCEDURE_OCCURRENCE po
JOIN Codesets cs on (po.procedure_concept_id = cs.concept_id and cs.codeset_id = 30)
) C
UNION ALL
select C.person_id, C.procedure_occurrence_id as event_id, C.start_date, C.end_date,
       C.visit_occurrence_id, C.start_date as sort_date
from 
(
  select po.person_id,po.procedure_occurrence_id,po.procedure_concept_id,po.visit_occurrence_id,po.quantity,po.procedure_date as start_date, (po.procedure_date + TO_DAYS(CAST(1 AS INTEGER))) as end_date 
  FROM main.PROCEDURE_OCCURRENCE po
JOIN Codesets cs on (po.procedure_concept_id = cs.concept_id and cs.codeset_id = 31)
) C
UNION ALL
select C.person_id, C.procedure_occurrence_id as event_id, C.start_date, C.end_date,
       C.visit_occurrence_id, C.start_date as sort_date
from 
(
  select po.person_id,po.procedure_occurrence_id,po.procedure_concept_id,po.visit_occurrence_id,po.quantity,po.procedure_date as start_date, (po.procedure_date + TO_DAYS(CAST(1 AS INTEGER))) as end_date 
  FROM main.PROCEDURE_OCCURRENCE po
JOIN Codesets cs on (po.procedure_concept_id = cs.concept_id and cs.codeset_id = 32)
) C
UNION ALL
select C.person_id, C.procedure_occurrence_id as event_id, C.start_date, C.end_date,
       C.visit_occurrence_id, C.start_date as sort_date
from 
(
  select po.person_id,po.procedure_occurrence_id,po.procedure_concept_id,po.visit_occurrence_id,po.quantity,po.procedure_date as start_date, (po.procedure_date + TO_DAYS(CAST(1 AS INTEGER))) as end_date 
  FROM main.PROCEDURE_OCCURRENCE po
JOIN Codesets cs on (po.procedure_concept_id = cs.concept_id and cs.codeset_id = 33)
) C
UNION ALL
select C.person_id, C.procedure_occurrence_id as event_id, C.start_date, C.end_date,
       C.visit_occurrence_id, C.start_date as sort_date
from 
(
  select po.person_id,po.procedure_occurrence_id,po.procedure_concept_id,po.visit_occurrence_id,po.quantity,po.procedure_date as start_date, (po.procedure_date + TO_DAYS(CAST(1 AS INTEGER))) as end_date 
  FROM main.PROCEDURE_OCCURRENCE po
JOIN Codesets cs on (po.procedure_concept_id = cs.concept_id and cs.codeset_id = 34)
) C
UNION ALL
select C.person_id, C.procedure_occurrence_id as event_id, C.start_date, C.end_date,
       C.visit_occurrence_id, C.start_date as sort_date
from 
(
  select po.person_id,po.procedure_occurrence_id,po.procedure_concept_id,po.visit_occurrence_id,po.quantity,po.procedure_date as start_date, (po.procedure_date + TO_DAYS(CAST(1 AS INTEGER))) as end_date 
  FROM main.PROCEDURE_OCCURRENCE po
JOIN Codesets cs on (po.procedure_concept_id = cs.concept_id and cs.codeset_id = 48)
) C
UNION ALL
select C.person_id, C.procedure_occurrence_id as event_id, C.start_date, C.end_date,
       C.visit_occurrence_id, C.start_date as sort_date
from 
(
  select po.person_id,po.procedure_occurrence_id,po.procedure_concept_id,po.visit_occurrence_id,po.quantity,po.procedure_date as start_date, (po.procedure_date + TO_DAYS(CAST(1 AS INTEGER))) as end_date 
  FROM main.PROCEDURE_OCCURRENCE po
JOIN Codesets cs on (po.procedure_concept_id = cs.concept_id and cs.codeset_id = 72)
) C
UNION ALL
select C.person_id, C.procedure_occurrence_id as event_id, C.start_date, C.end_date,
       C.visit_occurrence_id, C.start_date as sort_date
from 
(
  select po.person_id,po.procedure_occurrence_id,po.procedure_concept_id,po.visit_occurrence_id,po.quantity,po.procedure_date as start_date, (po.procedure_date + TO_DAYS(CAST(1 AS INTEGER))) as end_date 
  FROM main.PROCEDURE_OCCURRENCE po
JOIN Codesets cs on (po.procedure_concept_id = cs.concept_id and cs.codeset_id = 38)
) C
UNION ALL
select C.person_id, C.procedure_occurrence_id as event_id, C.start_date, C.end_date,
       C.visit_occurrence_id, C.start_date as sort_date
from 
(
  select po.person_id,po.procedure_occurrence_id,po.procedure_concept_id,po.visit_occurrence_id,po.quantity,po.procedure_date as start_date, (po.procedure_date + TO_DAYS(CAST(1 AS INTEGER))) as end_date 
  FROM main.PROCEDURE_OCCURRENCE po
JOIN Codesets cs on (po.procedure_concept_id = cs.concept_id and cs.codeset_id = 73)
) C
UNION ALL
select C.person_id, C.procedure_occurrence_id as event_id, C.start_date, C.end_date,
       C.visit_occurrence_id, C.start_date as sort_date
from 
(
  select po.person_id,po.procedure_occurrence_id,po.procedure_concept_id,po.visit_occurrence_id,po.quantity,po.procedure_date as start_date, (po.procedure_date + TO_DAYS(CAST(1 AS INTEGER))) as end_date 
  FROM main.PROCEDURE_OCCURRENCE po
JOIN Codesets cs on (po.procedure_concept_id = cs.concept_id and cs.codeset_id = 74)
) C
UNION ALL
select C.person_id, C.procedure_occurrence_id as event_id, C.start_date, C.end_date,
       C.visit_occurrence_id, C.start_date as sort_date
from 
(
  select po.person_id,po.procedure_occurrence_id,po.procedure_concept_id,po.visit_occurrence_id,po.quantity,po.procedure_date as start_date, (po.procedure_date + TO_DAYS(CAST(1 AS INTEGER))) as end_date 
  FROM main.PROCEDURE_OCCURRENCE po
JOIN Codesets cs on (po.procedure_concept_id = cs.concept_id and cs.codeset_id = 46)
) C
UNION ALL
select C.person_id, C.procedure_occurrence_id as event_id, C.start_date, C.end_date,
       C.visit_occurrence_id, C.start_date as sort_date
from 
(
  select po.person_id,po.procedure_occurrence_id,po.procedure_concept_id,po.visit_occurrence_id,po.quantity,po.procedure_date as start_date, (po.procedure_date + TO_DAYS(CAST(1 AS INTEGER))) as end_date 
  FROM main.PROCEDURE_OCCURRENCE po
JOIN Codesets cs on (po.procedure_concept_id = cs.concept_id and cs.codeset_id = 75)
) C
UNION ALL
select C.person_id, C.procedure_occurrence_id as event_id, C.start_date, C.end_date,
       C.visit_occurrence_id, C.start_date as sort_date
from 
(
  select po.person_id,po.procedure_occurrence_id,po.procedure_concept_id,po.visit_occurrence_id,po.quantity,po.procedure_date as start_date, (po.procedure_date + TO_DAYS(CAST(1 AS INTEGER))) as end_date 
  FROM main.PROCEDURE_OCCURRENCE po
JOIN Codesets cs on (po.procedure_concept_id = cs.concept_id and cs.codeset_id = 76)
) C
UNION ALL
select C.person_id, C.procedure_occurrence_id as event_id, C.start_date, C.end_date,
       C.visit_occurrence_id, C.start_date as sort_date
from 
(
  select po.person_id,po.procedure_occurrence_id,po.procedure_concept_id,po.visit_occurrence_id,po.quantity,po.procedure_date as start_date, (po.procedure_date + TO_DAYS(CAST(1 AS INTEGER))) as end_date 
  FROM main.PROCEDURE_OCCURRENCE po
JOIN Codesets cs on (po.procedure_concept_id = cs.concept_id and cs.codeset_id = 78)
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
SELECT 0 as index_id, e.person_id, e.event_id
FROM qualified_events E
JOIN main.PERSON P ON P.PERSON_ID = E.PERSON_ID
WHERE YEAR(CAST(E.start_date AS DATE)) - P.year_of_birth >= 18
GROUP BY e.person_id, e.event_id
  ) CQ on E.person_id = CQ.person_id and E.event_id = CQ.event_id
  GROUP BY E.person_id, E.event_id
  HAVING COUNT(index_id) = 1
) G
) AC on AC.person_id = pe.person_id AND AC.event_id = pe.event_id
) Results
),
Inclusion_1 AS (
SELECT
1 as inclusion_rule_id, person_id, event_id
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
select 0 as index_id, cc.person_id, cc.event_id
from (SELECT p.person_id, p.event_id 
FROM qualified_events P
JOIN (
  select PE.person_id, PE.event_id, PE.start_date, PE.end_date, PE.visit_occurrence_id, PE.sort_date FROM (
select C.person_id, C.visit_occurrence_id as event_id, C.start_date, C.end_date,
       C.visit_occurrence_id, C.start_date as sort_date
from 
(
  select vo.person_id,vo.visit_occurrence_id,vo.visit_concept_id,vo.visit_start_date as start_date, vo.visit_end_date as end_date 
  FROM main.VISIT_OCCURRENCE vo
JOIN Codesets cs on (vo.visit_concept_id = cs.concept_id and cs.codeset_id = 0)
) C
) PE
JOIN (
select 0 as index_id, person_id, event_id
FROM
(
  select E.person_id, E.event_id 
  FROM (SELECT Q.person_id, Q.event_id, Q.start_date, Q.end_date, Q.visit_occurrence_id, OP.observation_period_start_date as op_start_date, OP.observation_period_end_date as op_end_date
FROM (-- Begin Visit Occurrence Criteria
select C.person_id, C.visit_occurrence_id as event_id, C.start_date, C.end_date,
       C.visit_occurrence_id, C.start_date as sort_date
from 
(
  select vo.person_id,vo.visit_occurrence_id,vo.visit_concept_id,vo.visit_start_date as start_date, vo.visit_end_date as end_date 
  FROM main.VISIT_OCCURRENCE vo
JOIN Codesets cs on (vo.visit_concept_id = cs.concept_id and cs.codeset_id = 0)
) C
) Q
JOIN main.OBSERVATION_PERIOD OP on Q.person_id = OP.person_id 
  and OP.observation_period_start_date <= Q.start_date and OP.observation_period_end_date >= Q.start_date
) E
  INNER JOIN
  (
select 0 as index_id, p.person_id, p.event_id
from (SELECT Q.person_id, Q.event_id, Q.start_date, Q.end_date, Q.visit_occurrence_id, OP.observation_period_start_date as op_start_date, OP.observation_period_end_date as op_end_date
FROM (-- Begin Visit Occurrence Criteria
select C.person_id, C.visit_occurrence_id as event_id, C.start_date, C.end_date,
       C.visit_occurrence_id, C.start_date as sort_date
from 
(
  select vo.person_id,vo.visit_occurrence_id,vo.visit_concept_id,vo.visit_start_date as start_date, vo.visit_end_date as end_date 
  FROM main.VISIT_OCCURRENCE vo
JOIN Codesets cs on (vo.visit_concept_id = cs.concept_id and cs.codeset_id = 0)
) C
) Q
JOIN main.OBSERVATION_PERIOD OP on Q.person_id = OP.person_id 
  and OP.observation_period_start_date <= Q.start_date and OP.observation_period_end_date >= Q.start_date
) p
LEFT JOIN (
SELECT p.person_id, p.event_id 
FROM (SELECT Q.person_id, Q.event_id, Q.start_date, Q.end_date, Q.visit_occurrence_id, OP.observation_period_start_date as op_start_date, OP.observation_period_end_date as op_end_date
FROM (-- Begin Visit Occurrence Criteria
select C.person_id, C.visit_occurrence_id as event_id, C.start_date, C.end_date,
       C.visit_occurrence_id, C.start_date as sort_date
from 
(
  select vo.person_id,vo.visit_occurrence_id,vo.visit_concept_id,vo.visit_start_date as start_date, vo.visit_end_date as end_date 
  FROM main.VISIT_OCCURRENCE vo
JOIN Codesets cs on (vo.visit_concept_id = cs.concept_id and cs.codeset_id = 0)
) C
) Q
JOIN main.OBSERVATION_PERIOD OP on Q.person_id = OP.person_id 
  and OP.observation_period_start_date <= Q.start_date and OP.observation_period_end_date >= Q.start_date
) P
JOIN (
select C.person_id, C.visit_occurrence_id as event_id, C.start_date, C.end_date,
       C.visit_occurrence_id, C.start_date as sort_date
from 
(
  select vo.person_id,vo.visit_occurrence_id,vo.visit_concept_id,vo.visit_start_date as start_date, vo.visit_end_date as end_date 
  FROM main.VISIT_OCCURRENCE vo
JOIN Codesets cs on (vo.visit_concept_id = cs.concept_id and cs.codeset_id = 43)
) C
) A on A.person_id = P.person_id  AND A.START_DATE >= P.OP_START_DATE AND A.START_DATE <= P.OP_END_DATE AND A.START_DATE >= (P.START_DATE + TO_DAYS(CAST(-2 AS INTEGER))) AND A.START_DATE <= (P.START_DATE + TO_DAYS(CAST(0 AS INTEGER))) ) cc on p.person_id = cc.person_id and p.event_id = cc.event_id
GROUP BY p.person_id, p.event_id
HAVING COUNT(cc.event_id) = 0
  ) CQ on E.person_id = CQ.person_id and E.event_id = CQ.event_id
  GROUP BY E.person_id, E.event_id
  HAVING COUNT(index_id) = 1
) G
) AC on AC.person_id = pe.person_id and AC.event_id = pe.event_id
) A on A.person_id = P.person_id  AND A.START_DATE >= P.OP_START_DATE AND A.START_DATE <= P.OP_END_DATE AND A.START_DATE >= (P.START_DATE + TO_DAYS(CAST(-1 AS INTEGER))) AND A.START_DATE <= (P.START_DATE + TO_DAYS(CAST(0 AS INTEGER))) AND A.END_DATE >= (P.START_DATE + TO_DAYS(CAST(1 AS INTEGER))) AND A.END_DATE <= P.OP_END_DATE ) cc 
GROUP BY cc.person_id, cc.event_id
HAVING COUNT(cc.event_id) >= 1
  ) CQ on E.person_id = CQ.person_id and E.event_id = CQ.event_id
  GROUP BY E.person_id, E.event_id
  HAVING COUNT(index_id) = 1
) G
) AC on AC.person_id = pe.person_id AND AC.event_id = pe.event_id
) Results
),
Inclusion_2 AS (
SELECT
2 as inclusion_rule_id, person_id, event_id
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
select C.person_id, C.drug_exposure_id as event_id, C.start_date, C.end_date,
  C.visit_occurrence_id,C.start_date as sort_date
from 
(
  select de.person_id,de.drug_exposure_id,de.drug_concept_id,de.visit_occurrence_id,days_supply,quantity,refills,de.drug_exposure_start_date as start_date, COALESCE(de.drug_exposure_end_date, (de.drug_exposure_start_date + TO_DAYS(CAST(de.days_supply AS INTEGER))), (de.drug_exposure_start_date + TO_DAYS(CAST(1 AS INTEGER)))) as end_date 
  FROM main.DRUG_EXPOSURE de
JOIN Codesets cs on (de.drug_concept_id = cs.concept_id and cs.codeset_id = 79)
) C
) A on A.person_id = P.person_id  AND A.START_DATE >= P.OP_START_DATE AND A.START_DATE <= P.OP_END_DATE AND A.START_DATE >= (P.START_DATE + TO_DAYS(CAST(-365 AS INTEGER))) AND A.START_DATE <= (P.START_DATE + TO_DAYS(CAST(-14 AS INTEGER))) ) cc on p.person_id = cc.person_id and p.event_id = cc.event_id
GROUP BY p.person_id, p.event_id
HAVING COUNT(cc.event_id) = 0
  ) CQ on E.person_id = CQ.person_id and E.event_id = CQ.event_id
  GROUP BY E.person_id, E.event_id
  HAVING COUNT(index_id) = 1
) G
) AC on AC.person_id = pe.person_id AND AC.event_id = pe.event_id
) Results
),
inclusion_events AS (
SELECT
inclusion_rule_id, person_id, event_id
FROM
(select inclusion_rule_id, person_id, event_id from Inclusion_0
UNION ALL
select inclusion_rule_id, person_id, event_id from Inclusion_1
UNION ALL
select inclusion_rule_id, person_id, event_id from Inclusion_2) I
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
  WHERE (MG.inclusion_rule_mask = POWER(cast(2 as bigint),3)-1)
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
