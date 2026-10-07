-- mimic-code concepts_duckdb: kdigo_stages, with its 5 upstream concepts inlined as CTEs
WITH crrt AS (
    -- treatment/crrt.sql
    WITH crrt_settings AS (
      SELECT
        ce.stay_id,
        ce.charttime,
        CASE WHEN ce.itemid = 227290 THEN ce.value END AS crrt_mode,
        CASE WHEN ce.itemid = 224149 THEN ce.valuenum ELSE NULL END AS accesspressure,
        CASE WHEN ce.itemid = 224144 THEN ce.valuenum ELSE NULL END AS bloodflow,
        CASE WHEN ce.itemid = 228004 THEN ce.valuenum ELSE NULL END AS citrate,
        CASE WHEN ce.itemid = 225183 THEN ce.valuenum ELSE NULL END AS currentgoal,
        CASE WHEN ce.itemid = 225977 THEN ce.value ELSE NULL END AS dialysatefluid,
        CASE WHEN ce.itemid = 224154 THEN ce.valuenum ELSE NULL END AS dialysaterate,
        CASE WHEN ce.itemid = 224151 THEN ce.valuenum ELSE NULL END AS effluentpressure,
        CASE WHEN ce.itemid = 224150 THEN ce.valuenum ELSE NULL END AS filterpressure,
        CASE WHEN ce.itemid = 225958 THEN ce.value ELSE NULL END AS heparinconcentration,
        CASE WHEN ce.itemid = 224145 THEN ce.valuenum ELSE NULL END AS heparindose,
        CASE WHEN ce.itemid = 224191 THEN ce.valuenum ELSE NULL END AS hourlypatientfluidremoval,
        CASE WHEN ce.itemid = 228005 THEN ce.valuenum ELSE NULL END AS prefilterreplacementrate,
        CASE WHEN ce.itemid = 228006 THEN ce.valuenum ELSE NULL END AS postfilterreplacementrate,
        CASE WHEN ce.itemid = 225976 THEN ce.value ELSE NULL END AS replacementfluid,
        CASE WHEN ce.itemid = 224153 THEN ce.valuenum ELSE NULL END AS replacementrate,
        CASE WHEN ce.itemid = 224152 THEN ce.valuenum ELSE NULL END AS returnpressure,
        CASE WHEN ce.itemid = 226457 THEN ce.valuenum END AS ultrafiltrateoutput,
        CASE
          WHEN ce.itemid = 224146
          AND ce.value IN ('Active', 'Initiated', 'Reinitiated', 'New Filter')
          THEN 1
          WHEN ce.itemid = 224146 AND ce.value IN ('Recirculating', 'Discontinued')
          THEN 0
          ELSE NULL
        END AS system_active,
        CASE
          WHEN ce.itemid = 224146 AND ce.value IN ('Clots Present', 'Clots Present')
          THEN 1
          WHEN ce.itemid = 224146 AND ce.value IN ('No Clot Present', 'No Clot Present')
          THEN 0
          ELSE NULL
        END AS clots,
        CASE
          WHEN ce.itemid = 224146 AND ce.value IN ('Clots Increasing', 'Clot Increasing')
          THEN 1
          ELSE NULL
        END AS clots_increasing,
        CASE WHEN ce.itemid = 224146 AND ce.value IN ('Clotted') THEN 1 ELSE NULL END AS clotted
      FROM mimiciv_icu.chartevents AS ce
      WHERE
        ce.itemid IN (
          227290,
          224146,
          224149,
          224144,
          228004,
          225183,
          225977,
          224154,
          224151,
          224150,
          225958,
          224145,
          224191,
          228005,
          228006,
          225976,
          224153,
          224152,
          226457
        )
        AND NOT ce.value IS NULL
    )
    SELECT
      stay_id,
      charttime,
      MAX(crrt_mode) AS crrt_mode,
      MAX(accesspressure) AS access_pressure,
      MAX(bloodflow) AS blood_flow,
      MAX(citrate) AS citrate,
      MAX(currentgoal) AS current_goal,
      MAX(dialysatefluid) AS dialysate_fluid,
      MAX(dialysaterate) AS dialysate_rate,
      MAX(effluentpressure) AS effluent_pressure,
      MAX(filterpressure) AS filter_pressure,
      MAX(heparinconcentration) AS heparin_concentration,
      MAX(heparindose) AS heparin_dose,
      MAX(hourlypatientfluidremoval) AS hourly_patient_fluid_removal,
      MAX(prefilterreplacementrate) AS prefilter_replacement_rate,
      MAX(postfilterreplacementrate) AS postfilter_replacement_rate,
      MAX(replacementfluid) AS replacement_fluid,
      MAX(replacementrate) AS replacement_rate,
      MAX(returnpressure) AS return_pressure,
      MAX(ultrafiltrateoutput) AS ultrafiltrate_output,
      MAX(system_active) AS system_active,
      MAX(clots) AS clots,
      MAX(clots_increasing) AS clots_increasing,
      MAX(clotted) AS clotted
    FROM crrt_settings
    GROUP BY
      stay_id,
      charttime
),
kdigo_creatinine AS (
    -- organfailure/kdigo_creatinine.sql
    WITH cr AS (
      SELECT
        ie.hadm_id,
        ie.stay_id,
        le.charttime,
        AVG(le.valuenum) AS creat
      FROM mimiciv_icu.icustays AS ie
      LEFT JOIN mimiciv_hosp.labevents AS le
        ON ie.subject_id = le.subject_id
        AND le.itemid = 50912
        AND NOT le.valuenum IS NULL
        AND le.valuenum <= 150
        AND le.charttime >= ie.intime - INTERVAL '7' DAY
        AND le.charttime <= ie.outtime
      GROUP BY
        ie.hadm_id,
        ie.stay_id,
        le.charttime
    ), cr48 AS (
      SELECT
        cr.stay_id,
        cr.charttime,
        MIN(cr48.creat) AS creat_low_past_48hr
      FROM cr
      LEFT JOIN cr AS cr48
        ON cr.stay_id = cr48.stay_id
        AND cr48.charttime < cr.charttime
        AND cr48.charttime >= cr.charttime - INTERVAL '48' HOUR
      GROUP BY
        cr.stay_id,
        cr.charttime
    ), cr7 AS (
      SELECT
        cr.stay_id,
        cr.charttime,
        MIN(cr7.creat) AS creat_low_past_7day
      FROM cr
      LEFT JOIN cr AS cr7
        ON cr.stay_id = cr7.stay_id
        AND cr7.charttime < cr.charttime
        AND cr7.charttime >= cr.charttime - INTERVAL '7' DAY
      GROUP BY
        cr.stay_id,
        cr.charttime
    )
    SELECT
      cr.hadm_id,
      cr.stay_id,
      cr.charttime,
      cr.creat,
      cr48.creat_low_past_48hr,
      cr7.creat_low_past_7day
    FROM cr
    LEFT JOIN cr48
      ON cr.stay_id = cr48.stay_id AND cr.charttime = cr48.charttime
    LEFT JOIN cr7
      ON cr.stay_id = cr7.stay_id AND cr.charttime = cr7.charttime
),
urine_output AS (
    -- measurement/urine_output.sql
    WITH uo AS (
      SELECT
        oe.stay_id,
        oe.charttime,
        CASE WHEN oe.itemid = 227488 AND oe.value > 0 THEN -1 * oe.value ELSE oe.value END AS urineoutput
      FROM mimiciv_icu.outputevents AS oe
      WHERE
        itemid IN (
          226559,
          226560,
          226561,
          226584,
          226563,
          226564,
          226565,
          226567,
          226557,
          226558,
          227488,
          227489
        )
    )
    SELECT
      stay_id,
      charttime,
      SUM(urineoutput) AS urineoutput
    FROM uo
    GROUP BY
      stay_id,
      charttime
),
weight_durations AS (
    -- demographics/weight_durations.sql
    WITH wt_stg AS (
      SELECT
        c.stay_id,
        c.charttime,
        CASE WHEN c.itemid = 226512 THEN 'admit' ELSE 'daily' END AS weight_type,
        ROUND(CAST(c.valuenum AS DECIMAL(38, 9)), 3) AS weight
      FROM mimiciv_icu.chartevents AS c
      WHERE
        NOT c.valuenum IS NULL
        AND c.itemid IN (226512, 224639)
        AND c.valuenum > 0
        AND c.valuenum < 1500
    ), wt_stg1 AS (
      SELECT
        stay_id,
        charttime,
        weight_type,
        weight,
        ROW_NUMBER() OVER (PARTITION BY stay_id, weight_type ORDER BY charttime NULLS FIRST) AS rn
      FROM wt_stg
      WHERE
        NOT weight IS NULL
    ), wt_stg2 AS (
      SELECT
        wt_stg1.stay_id,
        ie.intime,
        ie.outtime,
        wt_stg1.weight_type,
        CASE
          WHEN wt_stg1.weight_type = 'admit' AND wt_stg1.rn = 1
          THEN ie.intime - INTERVAL '2' HOUR
          ELSE wt_stg1.charttime
        END AS starttime,
        wt_stg1.weight
      FROM wt_stg1
      INNER JOIN mimiciv_icu.icustays AS ie
        ON ie.stay_id = wt_stg1.stay_id
    ), wt_stg3 AS (
      SELECT
        stay_id,
        intime,
        outtime,
        starttime,
        COALESCE(
          LEAD(starttime) OVER (PARTITION BY stay_id ORDER BY starttime NULLS FIRST),
          outtime + INTERVAL '2' HOUR
        ) AS endtime,
        weight,
        weight_type
      FROM wt_stg2
    ), wt1 AS (
      SELECT
        stay_id,
        starttime,
        COALESCE(
          endtime,
          LEAD(starttime) OVER (PARTITION BY stay_id ORDER BY starttime NULLS FIRST),
          outtime + INTERVAL '2' HOUR
        ) AS endtime,
        weight,
        weight_type
      FROM wt_stg3
    ), wt_fix AS (
      SELECT
        ie.stay_id,
        ie.intime - INTERVAL '2' HOUR AS starttime,
        wt.starttime AS endtime,
        wt.weight,
        wt.weight_type
      FROM mimiciv_icu.icustays AS ie
      INNER JOIN (
        SELECT
          wt1.stay_id,
          wt1.starttime,
          wt1.weight,
          weight_type,
          ROW_NUMBER() OVER (PARTITION BY wt1.stay_id ORDER BY wt1.starttime NULLS FIRST) AS rn
        FROM wt1
      ) AS wt
        ON ie.stay_id = wt.stay_id AND wt.rn = 1 AND ie.intime < wt.starttime
    )
    SELECT
      wt1.stay_id,
      wt1.starttime,
      wt1.endtime,
      wt1.weight,
      wt1.weight_type
    FROM wt1
    UNION ALL
    SELECT
      wt_fix.stay_id,
      wt_fix.starttime,
      wt_fix.endtime,
      wt_fix.weight,
      wt_fix.weight_type
    FROM wt_fix
),
kdigo_uo AS (
    -- organfailure/kdigo_uo.sql
    WITH uo_stg1 AS (
      SELECT
        ie.stay_id,
        uo.charttime,
        CAST(DATE_DIFF('SECOND', intime, charttime) AS INT) AS seconds_since_admit,
        COALESCE(
          DATE_DIFF(
            'SECOND',
            LAG(charttime) OVER (PARTITION BY ie.stay_id ORDER BY charttime NULLS FIRST),
            charttime
          ) / 3600.0,
          1
        ) AS hours_since_previous_row,
        urineoutput
      FROM mimiciv_icu.icustays AS ie
      INNER JOIN urine_output AS uo
        ON ie.stay_id = uo.stay_id
    ), uo_stg2 AS (
      SELECT
        stay_id,
        charttime,
        hours_since_previous_row,
        urineoutput,
        SUM(urineoutput) OVER (
          PARTITION BY stay_id
          ORDER BY seconds_since_admit NULLS FIRST
          RANGE BETWEEN 21600 PRECEDING AND CURRENT ROW
        ) AS urineoutput_6hr,
        SUM(urineoutput) OVER (
          PARTITION BY stay_id
          ORDER BY seconds_since_admit NULLS FIRST
          RANGE BETWEEN 43200 PRECEDING AND CURRENT ROW
        ) AS urineoutput_12hr,
        SUM(urineoutput) OVER (
          PARTITION BY stay_id
          ORDER BY seconds_since_admit NULLS FIRST
          RANGE BETWEEN 86400 PRECEDING AND CURRENT ROW
        ) AS urineoutput_24hr,
        ROUND(
          CAST(SUM(hours_since_previous_row) OVER (
            PARTITION BY stay_id
            ORDER BY seconds_since_admit NULLS FIRST
            RANGE BETWEEN 21600 PRECEDING AND CURRENT ROW
          ) AS DECIMAL(38, 9)),
          6
        ) AS uo_tm_6hr,
        ROUND(
          CAST(SUM(hours_since_previous_row) OVER (
            PARTITION BY stay_id
            ORDER BY seconds_since_admit NULLS FIRST
            RANGE BETWEEN 43200 PRECEDING AND CURRENT ROW
          ) AS DECIMAL(38, 9)),
          6
        ) AS uo_tm_12hr,
        ROUND(
          CAST(SUM(hours_since_previous_row) OVER (
            PARTITION BY stay_id
            ORDER BY seconds_since_admit NULLS FIRST
            RANGE BETWEEN 86400 PRECEDING AND CURRENT ROW
          ) AS DECIMAL(38, 9)),
          6
        ) AS uo_tm_24hr
      FROM uo_stg1
    )
    SELECT
      ur.stay_id,
      ur.charttime,
      wd.weight,
      ur.urineoutput_6hr,
      ur.urineoutput_12hr,
      ur.urineoutput_24hr,
      CASE
        WHEN uo_tm_6hr >= 6 AND uo_tm_6hr < 12
        THEN ROUND(CAST((
          ur.urineoutput_6hr / wd.weight / uo_tm_6hr
        ) AS DECIMAL(38, 9)), 4)
        ELSE NULL
      END AS uo_rt_6hr,
      CASE
        WHEN uo_tm_12hr >= 12
        THEN ROUND(CAST((
          ur.urineoutput_12hr / wd.weight / uo_tm_12hr
        ) AS DECIMAL(38, 9)), 4)
        ELSE NULL
      END AS uo_rt_12hr,
      CASE
        WHEN uo_tm_24hr >= 24
        THEN ROUND(CAST((
          ur.urineoutput_24hr / wd.weight / uo_tm_24hr
        ) AS DECIMAL(38, 9)), 4)
        ELSE NULL
      END AS uo_rt_24hr,
      uo_tm_6hr,
      uo_tm_12hr,
      uo_tm_24hr
    FROM uo_stg2 AS ur
    LEFT JOIN weight_durations AS wd
      ON ur.stay_id = wd.stay_id
      AND ur.charttime >= wd.starttime
      AND ur.charttime < wd.endtime
)
-- organfailure/kdigo_stages.sql
SELECT * FROM (
    WITH cr_stg AS (
      SELECT
        cr.stay_id,
        cr.charttime,
        cr.creat_low_past_7day,
        cr.creat_low_past_48hr,
        cr.creat,
        CASE
          WHEN cr.creat >= (
            cr.creat_low_past_7day * 3.0
          )
          THEN 3
          WHEN cr.creat >= 4
          AND (
            cr.creat >= (
              cr.creat_low_past_48hr + 0.3
            )
            OR cr.creat >= (
              1.5 * cr.creat_low_past_7day
            )
          )
          THEN 3
          WHEN cr.creat >= (
            cr.creat_low_past_7day * 2.0
          )
          THEN 2
          WHEN cr.creat >= (
            cr.creat_low_past_48hr + 0.3
          )
          THEN 1
          WHEN cr.creat >= (
            cr.creat_low_past_7day * 1.5
          )
          THEN 1
          ELSE 0
        END AS aki_stage_creat
      FROM kdigo_creatinine AS cr
    ), uo_stg AS (
      SELECT
        uo.stay_id,
        uo.charttime,
        uo.weight,
        uo.uo_rt_6hr,
        uo.uo_rt_12hr,
        uo.uo_rt_24hr,
        CASE
          WHEN uo.uo_rt_6hr IS NULL
          THEN NULL
          WHEN uo.charttime <= ie.intime + INTERVAL '6' HOUR
          THEN 0
          WHEN uo.uo_tm_24hr >= 24 AND uo.uo_rt_24hr < 0.3
          THEN 3
          WHEN uo.uo_tm_12hr >= 12 AND uo.uo_rt_12hr = 0
          THEN 3
          WHEN uo.uo_tm_12hr >= 12 AND uo.uo_rt_12hr < 0.5
          THEN 2
          WHEN uo.uo_tm_6hr >= 6 AND uo.uo_rt_6hr < 0.5
          THEN 1
          ELSE 0
        END AS aki_stage_uo
      FROM kdigo_uo AS uo
      INNER JOIN mimiciv_icu.icustays AS ie
        ON uo.stay_id = ie.stay_id
    ), crrt_stg AS (
      SELECT
        stay_id,
        charttime,
        CASE WHEN NOT charttime IS NULL THEN 3 ELSE NULL END AS aki_stage_crrt
      FROM crrt
      WHERE
        NOT crrt_mode IS NULL
    ), tm_stg AS (
      SELECT
        stay_id,
        charttime
      FROM cr_stg
      UNION
      SELECT
        stay_id,
        charttime
      FROM uo_stg
      UNION
      SELECT
        stay_id,
        charttime
      FROM crrt_stg
    )
    SELECT
      ie.subject_id,
      ie.hadm_id,
      ie.stay_id,
      tm.charttime,
      cr.creat_low_past_7day,
      cr.creat_low_past_48hr,
      cr.creat,
      cr.aki_stage_creat,
      uo.uo_rt_6hr,
      uo.uo_rt_12hr,
      uo.uo_rt_24hr,
      uo.aki_stage_uo,
      crrt.aki_stage_crrt,
      CASE
        WHEN COALESCE(cr.aki_stage_creat, 0) IS NULL
        OR COALESCE(uo.aki_stage_uo, 0) IS NULL
        OR COALESCE(crrt.aki_stage_crrt, 0) IS NULL
        THEN NULL
        ELSE GREATEST(
          COALESCE(cr.aki_stage_creat, 0),
          COALESCE(uo.aki_stage_uo, 0),
          COALESCE(crrt.aki_stage_crrt, 0)
        )
      END AS aki_stage,
      MAX(
        CASE
          WHEN COALESCE(cr.aki_stage_creat, 0) IS NULL
          OR COALESCE(uo.aki_stage_uo, 0) IS NULL
          OR COALESCE(crrt.aki_stage_crrt, 0) IS NULL
          THEN NULL
          ELSE GREATEST(
            COALESCE(cr.aki_stage_creat, 0),
            COALESCE(uo.aki_stage_uo, 0),
            COALESCE(crrt.aki_stage_crrt, 0)
          )
        END
      ) OVER (
        PARTITION BY ie.subject_id
        ORDER BY DATE_DIFF('SECOND', ie.intime, tm.charttime) NULLS FIRST
        RANGE BETWEEN 21600 PRECEDING AND CURRENT ROW
      ) AS aki_stage_smoothed
    FROM mimiciv_icu.icustays AS ie
    LEFT JOIN tm_stg AS tm
      ON ie.stay_id = tm.stay_id
    LEFT JOIN cr_stg AS cr
      ON ie.stay_id = cr.stay_id AND tm.charttime = cr.charttime
    LEFT JOIN uo_stg AS uo
      ON ie.stay_id = uo.stay_id AND tm.charttime = uo.charttime
    LEFT JOIN crrt_stg AS crrt
      ON ie.stay_id = crrt.stay_id AND tm.charttime = crrt.charttime
) AS kdigo_stages_
