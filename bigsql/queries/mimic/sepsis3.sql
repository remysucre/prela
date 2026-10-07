-- mimic-code concepts_duckdb: sepsis3, with its 21 upstream concepts inlined as CTEs
WITH bg AS (
    -- measurement/bg.sql
    WITH bg AS (
      SELECT
        MAX(subject_id) AS subject_id,
        MAX(hadm_id) AS hadm_id,
        MAX(charttime) AS charttime,
        MAX(storetime) AS storetime,
        le.specimen_id,
        MAX(CASE WHEN itemid = 52033 THEN value ELSE NULL END) AS specimen,
        MAX(CASE WHEN itemid = 50801 THEN valuenum ELSE NULL END) AS aado2,
        MAX(CASE WHEN itemid = 50802 THEN valuenum ELSE NULL END) AS baseexcess,
        MAX(CASE WHEN itemid = 50803 THEN valuenum ELSE NULL END) AS bicarbonate,
        MAX(CASE WHEN itemid = 50804 THEN valuenum ELSE NULL END) AS totalco2,
        MAX(CASE WHEN itemid = 50805 THEN valuenum ELSE NULL END) AS carboxyhemoglobin,
        MAX(CASE WHEN itemid = 50806 THEN valuenum ELSE NULL END) AS chloride,
        MAX(CASE WHEN itemid = 50808 THEN valuenum ELSE NULL END) AS calcium,
        MAX(CASE WHEN itemid = 50809 AND valuenum <= 10000 THEN valuenum ELSE NULL END) AS glucose,
        MAX(CASE WHEN itemid = 50810 AND valuenum <= 100 THEN valuenum ELSE NULL END) AS hematocrit,
        MAX(CASE WHEN itemid = 50811 THEN valuenum ELSE NULL END) AS hemoglobin,
        MAX(CASE WHEN itemid = 50813 AND valuenum <= 10000 THEN valuenum ELSE NULL END) AS lactate,
        MAX(CASE WHEN itemid = 50814 THEN valuenum ELSE NULL END) AS methemoglobin,
        MAX(CASE WHEN itemid = 50815 THEN valuenum ELSE NULL END) AS o2flow,
        MAX(
          CASE
            WHEN itemid = 50816
            THEN CASE
              WHEN valuenum > 20 AND valuenum <= 100
              THEN valuenum
              WHEN valuenum > 0.2 AND valuenum <= 1.0
              THEN valuenum * 100.0
              ELSE NULL
            END
            ELSE NULL
          END
        ) AS fio2,
        MAX(CASE WHEN itemid = 50817 AND valuenum <= 100 THEN valuenum ELSE NULL END) AS so2,
        MAX(CASE WHEN itemid = 50818 THEN valuenum ELSE NULL END) AS pco2,
        MAX(CASE WHEN itemid = 50819 THEN valuenum ELSE NULL END) AS peep,
        MAX(CASE WHEN itemid = 50820 THEN valuenum ELSE NULL END) AS ph,
        MAX(CASE WHEN itemid = 50821 THEN valuenum ELSE NULL END) AS po2,
        MAX(CASE WHEN itemid = 50822 THEN valuenum ELSE NULL END) AS potassium,
        MAX(CASE WHEN itemid = 50823 THEN valuenum ELSE NULL END) AS requiredo2,
        MAX(CASE WHEN itemid = 50824 THEN valuenum ELSE NULL END) AS sodium,
        MAX(CASE WHEN itemid = 50825 THEN valuenum ELSE NULL END) AS temperature,
        MAX(CASE WHEN itemid = 50807 THEN value ELSE NULL END) AS comments
      FROM mimiciv_hosp.labevents AS le
      WHERE
        le.itemid IN (
          52033,
          50801,
          50802,
          50803,
          50804,
          50805,
          50806,
          50807,
          50808,
          50809,
          50810,
          50811,
          50813,
          50814,
          50815,
          50816,
          50817,
          50818,
          50819,
          50820,
          50821,
          50822,
          50823,
          50824,
          50825
        )
      GROUP BY
        le.specimen_id
    ), stg_spo2 AS (
      SELECT
        subject_id,
        charttime,
        AVG(valuenum) AS spo2
      FROM mimiciv_icu.chartevents
      WHERE
        itemid = 220277 AND valuenum > 0 AND valuenum <= 100
      GROUP BY
        subject_id,
        charttime
    ), stg_fio2 AS (
      SELECT
        subject_id,
        charttime,
        MAX(
          CASE
            WHEN valuenum > 0.2 AND valuenum <= 1
            THEN valuenum * 100
            WHEN valuenum > 1 AND valuenum < 20
            THEN NULL
            WHEN valuenum >= 20 AND valuenum <= 100
            THEN valuenum
            ELSE NULL
          END
        ) AS fio2_chartevents
      FROM mimiciv_icu.chartevents
      WHERE
        itemid = 223835 AND valuenum > 0 AND valuenum <= 100
      GROUP BY
        subject_id,
        charttime
    ), stg2 AS (
      SELECT
        bg.*,
        ROW_NUMBER() OVER (PARTITION BY bg.specimen_id ORDER BY s1.charttime DESC) AS lastrowspo2,
        s1.spo2
      FROM bg
      LEFT JOIN stg_spo2 AS s1
        ON bg.subject_id = s1.subject_id
        AND s1.charttime BETWEEN bg.charttime - INTERVAL '2' HOUR AND bg.charttime
      WHERE
        NOT bg.po2 IS NULL
    ), stg3 AS (
      SELECT
        bg.*,
        ROW_NUMBER() OVER (PARTITION BY bg.specimen_id ORDER BY s2.charttime DESC) AS lastrowfio2,
        s2.fio2_chartevents
      FROM stg2 AS bg
      LEFT JOIN stg_fio2 AS s2
        ON bg.subject_id = s2.subject_id
        AND s2.charttime >= bg.charttime - INTERVAL '4' HOUR
        AND s2.charttime <= bg.charttime
        AND s2.fio2_chartevents > 0
      WHERE
        bg.lastrowspo2 = 1
    )
    SELECT
      stg3.subject_id,
      stg3.hadm_id,
      stg3.charttime,
      specimen,
      so2,
      po2,
      pco2,
      fio2_chartevents,
      fio2,
      aado2,
      ROUND(
        CAST(CASE
          WHEN po2 IS NULL OR pco2 IS NULL
          THEN NULL
          WHEN NOT fio2 IS NULL
          THEN (
            fio2 / 100
          ) * (
            760 - 47
          ) - (
            pco2 / 0.8
          ) - po2
          WHEN NOT fio2_chartevents IS NULL
          THEN (
            fio2_chartevents / 100
          ) * (
            760 - 47
          ) - (
            pco2 / 0.8
          ) - po2
          ELSE NULL
        END AS DECIMAL(38, 9)),
        4
      ) AS aado2_calc,
      CASE
        WHEN po2 IS NULL
        THEN NULL
        WHEN NOT fio2 IS NULL
        THEN 100 * po2 / fio2
        WHEN NOT fio2_chartevents IS NULL
        THEN 100 * po2 / fio2_chartevents
        ELSE NULL
      END AS pao2fio2ratio,
      ph,
      baseexcess,
      bicarbonate,
      totalco2,
      hematocrit,
      hemoglobin,
      carboxyhemoglobin,
      methemoglobin,
      chloride,
      calcium,
      temperature,
      potassium,
      sodium,
      lactate,
      glucose
    FROM stg3
    WHERE
      lastrowfio2 = 1
),
chemistry AS (
    -- measurement/chemistry.sql
    SELECT
      MAX(subject_id) AS subject_id,
      MAX(hadm_id) AS hadm_id,
      MAX(charttime) AS charttime,
      le.specimen_id,
      MAX(CASE WHEN itemid = 50862 AND valuenum <= 10 THEN valuenum ELSE NULL END) AS albumin,
      MAX(CASE WHEN itemid = 50930 AND valuenum <= 10 THEN valuenum ELSE NULL END) AS globulin,
      MAX(CASE WHEN itemid = 50976 AND valuenum <= 20 THEN valuenum ELSE NULL END) AS total_protein,
      MAX(CASE WHEN itemid = 50868 AND valuenum <= 10000 THEN valuenum ELSE NULL END) AS aniongap,
      MAX(CASE WHEN itemid = 50882 AND valuenum <= 10000 THEN valuenum ELSE NULL END) AS bicarbonate,
      MAX(CASE WHEN itemid = 51006 AND valuenum <= 300 THEN valuenum ELSE NULL END) AS bun,
      MAX(CASE WHEN itemid = 50893 AND valuenum <= 10000 THEN valuenum ELSE NULL END) AS calcium,
      MAX(CASE WHEN itemid = 50902 AND valuenum <= 10000 THEN valuenum ELSE NULL END) AS chloride,
      MAX(CASE WHEN itemid = 50912 AND valuenum <= 150 THEN valuenum ELSE NULL END) AS creatinine,
      MAX(CASE WHEN itemid = 50931 AND valuenum <= 10000 THEN valuenum ELSE NULL END) AS glucose,
      MAX(CASE WHEN itemid = 50983 AND valuenum <= 200 THEN valuenum ELSE NULL END) AS sodium,
      MAX(CASE WHEN itemid = 50971 AND valuenum <= 30 THEN valuenum ELSE NULL END) AS potassium
    FROM mimiciv_hosp.labevents AS le
    WHERE
      le.itemid IN (
        50862,
        50930,
        50976,
        50868,
        50882,
        50893,
        50912,
        50902,
        50931,
        50971,
        50983,
        51006
      )
      AND NOT valuenum IS NULL
      AND (
        valuenum > 0 OR itemid = 50868
      )
    GROUP BY
      le.specimen_id
),
complete_blood_count AS (
    -- measurement/complete_blood_count.sql
    SELECT
      MAX(subject_id) AS subject_id,
      MAX(hadm_id) AS hadm_id,
      MAX(charttime) AS charttime,
      le.specimen_id,
      MAX(CASE WHEN itemid = 51221 THEN valuenum ELSE NULL END) AS hematocrit,
      MAX(CASE WHEN itemid = 51222 THEN valuenum ELSE NULL END) AS hemoglobin,
      MAX(CASE WHEN itemid = 51248 THEN valuenum ELSE NULL END) AS mch,
      MAX(CASE WHEN itemid = 51249 THEN valuenum ELSE NULL END) AS mchc,
      MAX(CASE WHEN itemid = 51250 THEN valuenum ELSE NULL END) AS mcv,
      MAX(CASE WHEN itemid = 51265 THEN valuenum ELSE NULL END) AS platelet,
      MAX(CASE WHEN itemid = 51279 THEN valuenum ELSE NULL END) AS rbc,
      MAX(CASE WHEN itemid = 51277 THEN valuenum ELSE NULL END) AS rdw,
      MAX(CASE WHEN itemid = 52159 THEN valuenum ELSE NULL END) AS rdwsd,
      MAX(CASE WHEN itemid = 51301 THEN valuenum ELSE NULL END) AS wbc
    FROM mimiciv_hosp.labevents AS le
    WHERE
      le.itemid IN (51221, 51222, 51248, 51249, 51250, 51265, 51279, 51277, 52159, 51301)
      AND NOT valuenum IS NULL
      AND valuenum > 0
    GROUP BY
      le.specimen_id
),
dobutamine AS (
    -- medication/dobutamine.sql
    SELECT
      stay_id,
      linkorderid,
      rate AS vaso_rate,
      amount AS vaso_amount,
      starttime,
      endtime
    FROM mimiciv_icu.inputevents
    WHERE
      itemid = 221653
),
dopamine AS (
    -- medication/dopamine.sql
    SELECT
      stay_id,
      linkorderid,
      rate AS vaso_rate,
      amount AS vaso_amount,
      starttime,
      endtime
    FROM mimiciv_icu.inputevents
    WHERE
      itemid = 221662
),
enzyme AS (
    -- measurement/enzyme.sql
    SELECT
      MAX(subject_id) AS subject_id,
      MAX(hadm_id) AS hadm_id,
      MAX(charttime) AS charttime,
      le.specimen_id,
      MAX(CASE WHEN itemid = 50861 THEN valuenum ELSE NULL END) AS alt,
      MAX(CASE WHEN itemid = 50863 THEN valuenum ELSE NULL END) AS alp,
      MAX(CASE WHEN itemid = 50878 THEN valuenum ELSE NULL END) AS ast,
      MAX(CASE WHEN itemid = 50867 THEN valuenum ELSE NULL END) AS amylase,
      MAX(CASE WHEN itemid = 50885 THEN valuenum ELSE NULL END) AS bilirubin_total,
      MAX(CASE WHEN itemid = 50883 THEN valuenum ELSE NULL END) AS bilirubin_direct,
      MAX(CASE WHEN itemid = 50884 THEN valuenum ELSE NULL END) AS bilirubin_indirect,
      MAX(CASE WHEN itemid = 50910 THEN valuenum ELSE NULL END) AS ck_cpk,
      MAX(CASE WHEN itemid = 50911 THEN valuenum ELSE NULL END) AS ck_mb,
      MAX(CASE WHEN itemid = 50927 THEN valuenum ELSE NULL END) AS ggt,
      MAX(CASE WHEN itemid = 50954 THEN valuenum ELSE NULL END) AS ld_ldh
    FROM mimiciv_hosp.labevents AS le
    WHERE
      le.itemid IN (50861, 50863, 50878, 50867, 50885, 50884, 50883, 50910, 50911, 50927, 50954)
      AND NOT valuenum IS NULL
      AND valuenum > 0
    GROUP BY
      le.specimen_id
),
epinephrine AS (
    -- medication/epinephrine.sql
    SELECT
      stay_id,
      linkorderid,
      rate AS vaso_rate,
      amount AS vaso_amount,
      starttime,
      endtime
    FROM mimiciv_icu.inputevents
    WHERE
      itemid = 221289
),
gcs_ AS (
    -- measurement/gcs.sql
    WITH base AS (
      SELECT
        subject_id,
        ce.stay_id,
        ce.charttime,
        MAX(CASE WHEN ce.itemid = 223901 THEN ce.valuenum ELSE NULL END) AS gcsmotor,
        MAX(
          CASE
            WHEN ce.itemid = 223900 AND ce.value = 'No Response-ETT'
            THEN 0
            WHEN ce.itemid = 223900
            THEN ce.valuenum
            ELSE NULL
          END
        ) AS gcsverbal,
        MAX(CASE WHEN ce.itemid = 220739 THEN ce.valuenum ELSE NULL END) AS gcseyes,
        MAX(CASE WHEN ce.itemid = 223900 AND ce.value = 'No Response-ETT' THEN 1 ELSE 0 END) AS endotrachflag,
        ROW_NUMBER() OVER (PARTITION BY ce.stay_id ORDER BY ce.charttime ASC NULLS FIRST) AS rn
      FROM mimiciv_icu.chartevents AS ce
      WHERE
        ce.itemid IN (223900, 223901, 220739)
      GROUP BY
        ce.subject_id,
        ce.stay_id,
        ce.charttime
    ), gcs AS (
      SELECT
        b.*,
        b2.gcsverbal AS gcsverbalprev,
        b2.gcsmotor AS gcsmotorprev,
        b2.gcseyes AS gcseyesprev,
        CASE
          WHEN b.gcsverbal = 0
          THEN 15
          WHEN b.gcsverbal IS NULL AND b2.gcsverbal = 0
          THEN 15
          WHEN b2.gcsverbal = 0
          THEN COALESCE(b.gcsmotor, 6) + COALESCE(b.gcsverbal, 5) + COALESCE(b.gcseyes, 4)
          ELSE COALESCE(b.gcsmotor, COALESCE(b2.gcsmotor, 6)) + COALESCE(b.gcsverbal, COALESCE(b2.gcsverbal, 5)) + COALESCE(b.gcseyes, COALESCE(b2.gcseyes, 4))
        END AS gcs
      FROM base AS b
      LEFT JOIN base AS b2
        ON b.stay_id = b2.stay_id
        AND b.rn = b2.rn + 1
        AND b2.charttime > b.charttime - INTERVAL '6' HOUR
    ), gcs_stg AS (
      SELECT
        subject_id,
        gs.stay_id,
        gs.charttime,
        gcs,
        COALESCE(gcsmotor, gcsmotorprev) AS gcsmotor,
        COALESCE(gcsverbal, gcsverbalprev) AS gcsverbal,
        COALESCE(gcseyes, gcseyesprev) AS gcseyes,
        CASE WHEN COALESCE(gcsmotor, gcsmotorprev) IS NULL THEN 0 ELSE 1 END + CASE WHEN COALESCE(gcsverbal, gcsverbalprev) IS NULL THEN 0 ELSE 1 END + CASE WHEN COALESCE(gcseyes, gcseyesprev) IS NULL THEN 0 ELSE 1 END AS components_measured,
        endotrachflag
      FROM gcs AS gs
    )
    SELECT
      gs.subject_id,
      gs.stay_id,
      gs.charttime,
      gcs,
      gcsmotor AS gcs_motor,
      gcsverbal AS gcs_verbal,
      gcseyes AS gcs_eyes,
      endotrachflag AS gcs_unable
    FROM gcs_stg AS gs
),
icustay_times AS (
    -- demographics/icustay_times.sql
    WITH t1 AS (
      SELECT
        ce.stay_id,
        MIN(charttime) AS intime_hr,
        MAX(charttime) AS outtime_hr
      FROM mimiciv_icu.chartevents AS ce
      WHERE
        ce.itemid = 220045
      GROUP BY
        ce.stay_id
    )
    SELECT
      ie.subject_id,
      ie.hadm_id,
      ie.stay_id,
      t1.intime_hr,
      t1.outtime_hr
    FROM mimiciv_icu.icustays AS ie
    LEFT JOIN t1
      ON ie.stay_id = t1.stay_id
),
icustay_hourly AS (
    -- demographics/icustay_hourly.sql
    WITH all_hours AS (
      SELECT
        it.stay_id,
        CASE
          WHEN DATE_TRUNC('HOUR', CAST(it.intime_hr AS TIMESTAMP)) = it.intime_hr
          THEN it.intime_hr
          ELSE DATE_TRUNC('HOUR', CAST(it.intime_hr AS TIMESTAMP)) + INTERVAL '1' HOUR
        END AS endtime,
        GENERATE_SERIES(-24, CAST(CEIL(DATE_DIFF('HOUR', it.intime_hr, it.outtime_hr)) AS INT)) AS hrs
      FROM icustay_times AS it
    )
    SELECT
      stay_id,
      CAST(hr_unnested AS BIGINT) AS hr,
      endtime + INTERVAL (CAST(hr_unnested AS BIGINT)) HOUR AS endtime
    FROM all_hours
    CROSS JOIN UNNEST(all_hours.hrs) AS _t0(hr_unnested)
),
norepinephrine AS (
    -- medication/norepinephrine.sql
    SELECT
      stay_id,
      linkorderid,
      CASE
        WHEN rateuom = 'mg/kg/min' AND patientweight = 1
        THEN rate
        WHEN rateuom = 'mg/kg/min'
        THEN rate * 1000.0
        ELSE rate
      END AS vaso_rate,
      amount AS vaso_amount,
      starttime,
      endtime
    FROM mimiciv_icu.inputevents
    WHERE
      itemid = 221906
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
urine_output_rate AS (
    -- measurement/urine_output_rate.sql
    WITH tm AS (
      SELECT
        ie.stay_id,
        MIN(charttime) AS intime_hr,
        MAX(charttime) AS outtime_hr
      FROM mimiciv_icu.icustays AS ie
      INNER JOIN mimiciv_icu.chartevents AS ce
        ON ie.stay_id = ce.stay_id
        AND ce.itemid = 220045
        AND ce.charttime > ie.intime - INTERVAL '1' MONTH
        AND ce.charttime < ie.outtime + INTERVAL '1' MONTH
      GROUP BY
        ie.stay_id
    ), uo_tm AS (
      SELECT
        tm.stay_id,
        CASE
          WHEN LAG(charttime) OVER w IS NULL
          THEN DATE_DIFF('MINUTE', intime_hr, charttime)
          ELSE DATE_DIFF('MINUTE', LAG(charttime) OVER w, charttime)
        END AS tm_since_last_uo,
        uo.charttime,
        uo.urineoutput
      FROM tm
      INNER JOIN urine_output AS uo
        ON tm.stay_id = uo.stay_id
      WINDOW w AS (PARTITION BY tm.stay_id ORDER BY charttime NULLS FIRST)
    ), ur_stg AS (
      SELECT
        io.stay_id,
        io.charttime,
        SUM(DISTINCT io.urineoutput) AS uo,
        SUM(
          CASE
            WHEN DATE_DIFF('HOUR', iosum.charttime, io.charttime) <= 5
            THEN iosum.urineoutput
            ELSE NULL
          END
        ) AS urineoutput_6hr,
        ROUND(
          CAST(SUM(
            CASE
              WHEN DATE_DIFF('HOUR', iosum.charttime, io.charttime) <= 5
              THEN iosum.tm_since_last_uo
              ELSE NULL
            END
          ) / 60.0 AS DECIMAL(38, 9)),
          6
        ) AS uo_tm_6hr,
        SUM(
          CASE
            WHEN DATE_DIFF('HOUR', iosum.charttime, io.charttime) <= 11
            THEN iosum.urineoutput
            ELSE NULL
          END
        ) AS urineoutput_12hr,
        ROUND(
          CAST(SUM(
            CASE
              WHEN DATE_DIFF('HOUR', iosum.charttime, io.charttime) <= 11
              THEN iosum.tm_since_last_uo
              ELSE NULL
            END
          ) / 60.0 AS DECIMAL(38, 9)),
          6
        ) AS uo_tm_12hr,
        SUM(iosum.urineoutput) AS urineoutput_24hr,
        ROUND(CAST(SUM(iosum.tm_since_last_uo) / 60.0 AS DECIMAL(38, 9)), 6) AS uo_tm_24hr
      FROM uo_tm AS io
      LEFT JOIN uo_tm AS iosum
        ON io.stay_id = iosum.stay_id
        AND io.charttime >= iosum.charttime
        AND io.charttime <= (
          iosum.charttime + INTERVAL '23' HOUR
        )
      GROUP BY
        io.stay_id,
        io.charttime
    )
    SELECT
      ur.stay_id,
      ur.charttime,
      wd.weight,
      ur.uo,
      ur.urineoutput_6hr,
      ur.urineoutput_12hr,
      ur.urineoutput_24hr,
      CASE
        WHEN uo_tm_6hr >= 6
        THEN ROUND(CAST((
          ur.urineoutput_6hr / wd.weight / uo_tm_6hr
        ) AS DECIMAL(38, 9)), 4)
      END AS uo_mlkghr_6hr,
      CASE
        WHEN uo_tm_12hr >= 12
        THEN ROUND(CAST((
          ur.urineoutput_12hr / wd.weight / uo_tm_12hr
        ) AS DECIMAL(38, 9)), 4)
      END AS uo_mlkghr_12hr,
      CASE
        WHEN uo_tm_24hr >= 24
        THEN ROUND(CAST((
          ur.urineoutput_24hr / wd.weight / uo_tm_24hr
        ) AS DECIMAL(38, 9)), 4)
      END AS uo_mlkghr_24hr,
      ROUND(CAST(uo_tm_6hr AS DECIMAL(38, 9)), 2) AS uo_tm_6hr,
      ROUND(CAST(uo_tm_12hr AS DECIMAL(38, 9)), 2) AS uo_tm_12hr,
      ROUND(CAST(uo_tm_24hr AS DECIMAL(38, 9)), 2) AS uo_tm_24hr
    FROM ur_stg AS ur
    LEFT JOIN weight_durations AS wd
      ON ur.stay_id = wd.stay_id
      AND ur.charttime > wd.starttime
      AND ur.charttime <= wd.endtime
      AND wd.weight > 0
),
oxygen_delivery AS (
    -- measurement/oxygen_delivery.sql
    WITH ce_stg1 AS (
      SELECT
        ce.subject_id,
        ce.stay_id,
        ce.charttime,
        CASE WHEN itemid IN (223834, 227582) THEN 223834 ELSE itemid END AS itemid,
        value,
        valuenum,
        valueuom,
        storetime
      FROM mimiciv_icu.chartevents AS ce
      WHERE
        NOT ce.value IS NULL AND ce.itemid IN (223834, 227582, 227287)
    ), ce_stg2 AS (
      SELECT
        ce.subject_id,
        ce.stay_id,
        ce.charttime,
        itemid,
        value,
        valuenum,
        valueuom,
        ROW_NUMBER() OVER (PARTITION BY subject_id, charttime, itemid ORDER BY storetime DESC, valuenum DESC) AS rn
      FROM ce_stg1 AS ce
    ), o2 AS (
      SELECT
        subject_id,
        stay_id,
        charttime,
        itemid,
        value AS o2_device,
        ROW_NUMBER() OVER (PARTITION BY subject_id, charttime, itemid ORDER BY storetime DESC, value DESC) AS rn
      FROM mimiciv_icu.chartevents
      WHERE
        itemid = 226732
    ), stg AS (
      SELECT
        COALESCE(ce.subject_id, o2.subject_id) AS subject_id,
        COALESCE(ce.stay_id, o2.stay_id) AS stay_id,
        COALESCE(ce.charttime, o2.charttime) AS charttime,
        COALESCE(ce.itemid, o2.itemid) AS itemid,
        ce.value,
        ce.valuenum,
        o2.o2_device,
        o2.rn
      FROM ce_stg2 AS ce
      FULL OUTER JOIN o2
        ON ce.subject_id = o2.subject_id AND ce.charttime = o2.charttime
      WHERE
        ce.rn = 1
    )
    SELECT
      subject_id,
      MAX(stay_id) AS stay_id,
      charttime,
      MAX(CASE WHEN itemid = 223834 THEN valuenum ELSE NULL END) AS o2_flow,
      MAX(CASE WHEN itemid = 227287 THEN valuenum ELSE NULL END) AS o2_flow_additional,
      MAX(CASE WHEN rn = 1 THEN o2_device ELSE NULL END) AS o2_delivery_device_1,
      MAX(CASE WHEN rn = 2 THEN o2_device ELSE NULL END) AS o2_delivery_device_2,
      MAX(CASE WHEN rn = 3 THEN o2_device ELSE NULL END) AS o2_delivery_device_3,
      MAX(CASE WHEN rn = 4 THEN o2_device ELSE NULL END) AS o2_delivery_device_4
    FROM stg
    GROUP BY
      subject_id,
      charttime
),
ventilator_setting AS (
    -- measurement/ventilator_setting.sql
    WITH ce AS (
      SELECT
        ce.subject_id,
        ce.stay_id,
        ce.charttime,
        itemid,
        value,
        CASE
          WHEN itemid = 223835
          THEN CASE
            WHEN valuenum >= 0.20 AND valuenum <= 1
            THEN valuenum * 100
            WHEN valuenum > 1 AND valuenum < 20
            THEN NULL
            WHEN valuenum >= 20 AND valuenum <= 100
            THEN valuenum
            ELSE NULL
          END
          WHEN itemid IN (220339, 224700)
          THEN CASE WHEN valuenum > 100 THEN NULL WHEN valuenum < 0 THEN NULL ELSE valuenum END
          ELSE valuenum
        END AS valuenum,
        valueuom,
        storetime
      FROM mimiciv_icu.chartevents AS ce
      WHERE
        NOT ce.value IS NULL
        AND NOT ce.stay_id IS NULL
        AND ce.itemid IN (
          224688,
          224689,
          224690,
          224687,
          224685,
          224684,
          224686,
          224696,
          220339,
          224700,
          223835,
          223849,
          229314,
          223848,
          224691
        )
    )
    SELECT
      subject_id,
      MAX(stay_id) AS stay_id,
      charttime,
      MAX(CASE WHEN itemid = 224688 THEN valuenum ELSE NULL END) AS respiratory_rate_set,
      MAX(CASE WHEN itemid = 224690 THEN valuenum ELSE NULL END) AS respiratory_rate_total,
      MAX(CASE WHEN itemid = 224689 THEN valuenum ELSE NULL END) AS respiratory_rate_spontaneous,
      MAX(CASE WHEN itemid = 224687 THEN valuenum ELSE NULL END) AS minute_volume,
      MAX(CASE WHEN itemid = 224684 THEN valuenum ELSE NULL END) AS tidal_volume_set,
      MAX(CASE WHEN itemid = 224685 THEN valuenum ELSE NULL END) AS tidal_volume_observed,
      MAX(CASE WHEN itemid = 224686 THEN valuenum ELSE NULL END) AS tidal_volume_spontaneous,
      MAX(CASE WHEN itemid = 224696 THEN valuenum ELSE NULL END) AS plateau_pressure,
      MAX(CASE WHEN itemid IN (220339, 224700) THEN valuenum ELSE NULL END) AS peep,
      MAX(CASE WHEN itemid = 223835 THEN valuenum ELSE NULL END) AS fio2,
      MAX(CASE WHEN itemid = 224691 THEN valuenum ELSE NULL END) AS flow_rate,
      MAX(CASE WHEN itemid = 223849 THEN value ELSE NULL END) AS ventilator_mode,
      MAX(CASE WHEN itemid = 229314 THEN value ELSE NULL END) AS ventilator_mode_hamilton,
      MAX(CASE WHEN itemid = 223848 THEN value ELSE NULL END) AS ventilator_type
    FROM ce
    GROUP BY
      subject_id,
      charttime
),
ventilation AS (
    -- treatment/ventilation.sql
    WITH tm AS (
      SELECT
        stay_id,
        charttime
      FROM ventilator_setting
      UNION
      SELECT
        stay_id,
        charttime
      FROM oxygen_delivery
    ), vs AS (
      SELECT
        tm.stay_id,
        tm.charttime,
        o2_delivery_device_1,
        COALESCE(ventilator_mode, ventilator_mode_hamilton) AS vent_mode,
        CASE
          WHEN o2_delivery_device_1 IN ('Tracheostomy tube', 'Trach mask ')
          THEN 'Tracheostomy'
          WHEN o2_delivery_device_1 IN ('Endotracheal tube')
          OR ventilator_mode IN (
            '(S) CMV',
            'APRV',
            'APRV/Biphasic+ApnPress',
            'APRV/Biphasic+ApnVol',
            'APV (cmv)',
            'Ambient',
            'Apnea Ventilation',
            'CMV',
            'CMV/ASSIST',
            'CMV/ASSIST/AutoFlow',
            'CMV/AutoFlow',
            'CPAP/PPS',
            'CPAP/PSV',
            'CPAP/PSV+Apn TCPL',
            'CPAP/PSV+ApnPres',
            'CPAP/PSV+ApnVol',
            'MMV',
            'MMV/AutoFlow',
            'MMV/PSV',
            'MMV/PSV/AutoFlow',
            'P-CMV',
            'PCV+',
            'PCV+/PSV',
            'PCV+Assist',
            'PRES/AC',
            'PRVC/AC',
            'PRVC/SIMV',
            'PSV/SBT',
            'SIMV',
            'SIMV/AutoFlow',
            'SIMV/PRES',
            'SIMV/PSV',
            'SIMV/PSV/AutoFlow',
            'SIMV/VOL',
            'SYNCHRON MASTER',
            'SYNCHRON SLAVE',
            'VOL/AC'
          )
          OR ventilator_mode_hamilton IN (
            'APRV',
            'APV (cmv)',
            'Ambient',
            '(S) CMV',
            'P-CMV',
            'SIMV',
            'APV (simv)',
            'P-SIMV',
            'VS',
            'ASV'
          )
          THEN 'InvasiveVent'
          WHEN o2_delivery_device_1 IN ('Bipap mask ', 'CPAP mask ')
          OR o2_delivery_device_2 IN ('Bipap mask ', 'CPAP mask ')
          OR o2_delivery_device_3 IN ('Bipap mask ', 'CPAP mask ')
          OR o2_delivery_device_4 IN ('Bipap mask ', 'CPAP mask ')
          OR ventilator_mode_hamilton IN ('DuoPaP', 'NIV', 'NIV-ST')
          THEN 'NonInvasiveVent'
          WHEN o2_delivery_device_1 IN ('High flow nasal cannula')
          THEN 'HFNC'
          WHEN o2_delivery_device_1 IN (
            'Non-rebreather',
            'Face tent',
            'Aerosol-cool',
            'Venti mask ',
            'Medium conc mask ',
            'Ultrasonic neb',
            'Vapomist',
            'Oxymizer',
            'High flow neb',
            'Nasal cannula'
          )
          THEN 'SupplementalOxygen'
          WHEN o2_delivery_device_1 IN ('None')
          THEN 'None'
          ELSE NULL
        END AS ventilation_status
      FROM tm
      LEFT JOIN ventilator_setting AS vs
        ON tm.stay_id = vs.stay_id AND tm.charttime = vs.charttime
      LEFT JOIN oxygen_delivery AS od
        ON tm.stay_id = od.stay_id AND tm.charttime = od.charttime
    ), vd0 AS (
      SELECT
        stay_id,
        charttime,
        LAG(charttime, 1) OVER (PARTITION BY stay_id, ventilation_status ORDER BY charttime NULLS FIRST) AS charttime_lag,
        LEAD(charttime, 1) OVER w AS charttime_lead,
        ventilation_status,
        LAG(ventilation_status, 1) OVER w AS ventilation_status_lag
      FROM vs
      WHERE
        NOT ventilation_status IS NULL
      WINDOW w AS (PARTITION BY stay_id ORDER BY charttime NULLS FIRST)
    ), vd1 AS (
      SELECT
        stay_id,
        charttime,
        charttime_lag,
        charttime_lead,
        ventilation_status,
        DATE_DIFF('MINUTE', charttime_lag, charttime) / 60 AS ventduration,
        CASE
          WHEN ventilation_status_lag IS NULL
          THEN 1
          WHEN DATE_DIFF('HOUR', charttime_lag, charttime) >= 14
          THEN 1
          WHEN ventilation_status_lag <> ventilation_status
          THEN 1
          ELSE 0
        END AS new_ventilation_event
      FROM vd0
    ), vd2 AS (
      SELECT
        vd1.stay_id,
        vd1.charttime,
        vd1.charttime_lead,
        vd1.ventilation_status,
        ventduration,
        new_ventilation_event,
        SUM(new_ventilation_event) OVER (PARTITION BY stay_id ORDER BY charttime NULLS FIRST) AS vent_seq
      FROM vd1
    )
    SELECT
      stay_id,
      MIN(charttime) AS starttime,
      MAX(
        CASE
          WHEN charttime_lead IS NULL OR DATE_DIFF('HOUR', charttime, charttime_lead) >= 14
          THEN charttime
          ELSE charttime_lead
        END
      ) AS endtime,
      MAX(ventilation_status) AS ventilation_status
    FROM vd2
    GROUP BY
      stay_id,
      vent_seq
    HAVING
      MIN(charttime) <> MAX(charttime)
),
vitalsign AS (
    -- measurement/vitalsign.sql
    SELECT
      ce.subject_id,
      ce.stay_id,
      ce.charttime,
      AVG(
        CASE WHEN itemid IN (220045) AND valuenum > 0 AND valuenum < 300 THEN valuenum END
      ) AS heart_rate,
      AVG(
        CASE
          WHEN itemid IN (220179, 220050, 225309) AND valuenum > 0 AND valuenum < 400
          THEN valuenum
        END
      ) AS sbp,
      AVG(
        CASE
          WHEN itemid IN (220180, 220051, 225310) AND valuenum > 0 AND valuenum < 300
          THEN valuenum
        END
      ) AS dbp,
      AVG(
        CASE
          WHEN itemid IN (220052, 220181, 225312) AND valuenum > 0 AND valuenum < 300
          THEN valuenum
        END
      ) AS mbp,
      AVG(CASE WHEN itemid = 220179 AND valuenum > 0 AND valuenum < 400 THEN valuenum END) AS sbp_ni,
      AVG(CASE WHEN itemid = 220180 AND valuenum > 0 AND valuenum < 300 THEN valuenum END) AS dbp_ni,
      AVG(CASE WHEN itemid = 220181 AND valuenum > 0 AND valuenum < 300 THEN valuenum END) AS mbp_ni,
      AVG(
        CASE
          WHEN itemid IN (220210, 224690) AND valuenum > 0 AND valuenum < 70
          THEN valuenum
        END
      ) AS resp_rate,
      ROUND(
        CAST(AVG(
          CASE
            WHEN itemid IN (223761) AND valuenum > 70 AND valuenum < 120
            THEN (
              valuenum - 32
            ) / 1.8
            WHEN itemid IN (223762) AND valuenum > 10 AND valuenum < 50
            THEN valuenum
          END
        ) AS DECIMAL(38, 9)),
        2
      ) AS temperature,
      MAX(CASE WHEN itemid = 224642 THEN value END) AS temperature_site,
      AVG(
        CASE WHEN itemid IN (220277) AND valuenum > 0 AND valuenum <= 100 THEN valuenum END
      ) AS spo2,
      AVG(CASE WHEN itemid IN (225664, 220621, 226537) AND valuenum > 0 THEN valuenum END) AS glucose
    FROM mimiciv_icu.chartevents AS ce
    WHERE
      NOT ce.stay_id IS NULL
      AND ce.itemid IN (
        220045,
        225309,
        225310,
        225312,
        220050,
        220051,
        220052,
        220179,
        220180,
        220181,
        220210,
        224690,
        220277,
        225664,
        220621,
        226537,
        223762,
        223761,
        224642
      )
    GROUP BY
      ce.subject_id,
      ce.stay_id,
      ce.charttime
),
sofa_ AS (
    -- score/sofa.sql
    WITH co AS (
      SELECT
        ih.stay_id,
        ie.hadm_id,
        hr,
        ih.endtime - INTERVAL '1' HOUR AS starttime,
        ih.endtime
      FROM icustay_hourly AS ih
      INNER JOIN mimiciv_icu.icustays AS ie
        ON ih.stay_id = ie.stay_id
    ), pafi AS (
      SELECT
        ie.stay_id,
        bg.charttime,
        CASE WHEN vd.stay_id IS NULL THEN pao2fio2ratio ELSE NULL END AS pao2fio2ratio_novent,
        CASE WHEN NOT vd.stay_id IS NULL THEN pao2fio2ratio ELSE NULL END AS pao2fio2ratio_vent
      FROM mimiciv_icu.icustays AS ie
      INNER JOIN bg AS bg
        ON ie.subject_id = bg.subject_id
      LEFT JOIN ventilation AS vd
        ON ie.stay_id = vd.stay_id
        AND bg.charttime >= vd.starttime
        AND bg.charttime <= vd.endtime
        AND vd.ventilation_status = 'InvasiveVent'
      WHERE
        specimen = 'ART.'
    ), vs AS (
      SELECT
        co.stay_id,
        co.hr,
        MIN(vs.mbp) AS meanbp_min
      FROM co
      LEFT JOIN vitalsign AS vs
        ON co.stay_id = vs.stay_id
        AND co.starttime < vs.charttime
        AND co.endtime >= vs.charttime
      GROUP BY
        co.stay_id,
        co.hr
    ), gcs AS (
      SELECT
        co.stay_id,
        co.hr,
        MIN(gcs.gcs) AS gcs_min
      FROM co
      LEFT JOIN gcs_ AS gcs
        ON co.stay_id = gcs.stay_id
        AND co.starttime < gcs.charttime
        AND co.endtime >= gcs.charttime
      GROUP BY
        co.stay_id,
        co.hr
    ), bili AS (
      SELECT
        co.stay_id,
        co.hr,
        MAX(enz.bilirubin_total) AS bilirubin_max
      FROM co
      LEFT JOIN enzyme AS enz
        ON co.hadm_id = enz.hadm_id
        AND co.starttime < enz.charttime
        AND co.endtime >= enz.charttime
      GROUP BY
        co.stay_id,
        co.hr
    ), cr AS (
      SELECT
        co.stay_id,
        co.hr,
        MAX(chem.creatinine) AS creatinine_max
      FROM co
      LEFT JOIN chemistry AS chem
        ON co.hadm_id = chem.hadm_id
        AND co.starttime < chem.charttime
        AND co.endtime >= chem.charttime
      GROUP BY
        co.stay_id,
        co.hr
    ), plt AS (
      SELECT
        co.stay_id,
        co.hr,
        MIN(cbc.platelet) AS platelet_min
      FROM co
      LEFT JOIN complete_blood_count AS cbc
        ON co.hadm_id = cbc.hadm_id
        AND co.starttime < cbc.charttime
        AND co.endtime >= cbc.charttime
      GROUP BY
        co.stay_id,
        co.hr
    ), pf AS (
      SELECT
        co.stay_id,
        co.hr,
        MIN(pafi.pao2fio2ratio_novent) AS pao2fio2ratio_novent,
        MIN(pafi.pao2fio2ratio_vent) AS pao2fio2ratio_vent
      FROM co
      LEFT JOIN pafi
        ON co.stay_id = pafi.stay_id
        AND co.starttime < pafi.charttime
        AND co.endtime >= pafi.charttime
      GROUP BY
        co.stay_id,
        co.hr
    ), uo AS (
      SELECT
        co.stay_id,
        co.hr,
        MAX(
          CASE
            WHEN uo.uo_tm_24hr >= 22 AND uo.uo_tm_24hr <= 30
            THEN uo.urineoutput_24hr / uo.uo_tm_24hr * 24
          END
        ) AS uo_24hr
      FROM co
      LEFT JOIN urine_output_rate AS uo
        ON co.stay_id = uo.stay_id
        AND co.starttime < uo.charttime
        AND co.endtime >= uo.charttime
      GROUP BY
        co.stay_id,
        co.hr
    ), vaso AS (
      SELECT
        co.stay_id,
        co.hr,
        MAX(epi.vaso_rate) AS rate_epinephrine,
        MAX(nor.vaso_rate) AS rate_norepinephrine,
        MAX(dop.vaso_rate) AS rate_dopamine,
        MAX(dob.vaso_rate) AS rate_dobutamine
      FROM co
      LEFT JOIN epinephrine AS epi
        ON co.stay_id = epi.stay_id
        AND co.endtime > epi.starttime
        AND co.endtime <= epi.endtime
      LEFT JOIN norepinephrine AS nor
        ON co.stay_id = nor.stay_id
        AND co.endtime > nor.starttime
        AND co.endtime <= nor.endtime
      LEFT JOIN dopamine AS dop
        ON co.stay_id = dop.stay_id
        AND co.endtime > dop.starttime
        AND co.endtime <= dop.endtime
      LEFT JOIN dobutamine AS dob
        ON co.stay_id = dob.stay_id
        AND co.endtime > dob.starttime
        AND co.endtime <= dob.endtime
      WHERE
        NOT epi.stay_id IS NULL
        OR NOT nor.stay_id IS NULL
        OR NOT dop.stay_id IS NULL
        OR NOT dob.stay_id IS NULL
      GROUP BY
        co.stay_id,
        co.hr
    ), scorecomp AS (
      SELECT
        co.stay_id,
        co.hr,
        co.starttime,
        co.endtime,
        pf.pao2fio2ratio_novent,
        pf.pao2fio2ratio_vent,
        vaso.rate_epinephrine,
        vaso.rate_norepinephrine,
        vaso.rate_dopamine,
        vaso.rate_dobutamine,
        vs.meanbp_min,
        gcs.gcs_min,
        uo.uo_24hr,
        bili.bilirubin_max,
        cr.creatinine_max,
        plt.platelet_min
      FROM co
      LEFT JOIN vs
        ON co.stay_id = vs.stay_id AND co.hr = vs.hr
      LEFT JOIN gcs
        ON co.stay_id = gcs.stay_id AND co.hr = gcs.hr
      LEFT JOIN bili
        ON co.stay_id = bili.stay_id AND co.hr = bili.hr
      LEFT JOIN cr
        ON co.stay_id = cr.stay_id AND co.hr = cr.hr
      LEFT JOIN plt
        ON co.stay_id = plt.stay_id AND co.hr = plt.hr
      LEFT JOIN pf
        ON co.stay_id = pf.stay_id AND co.hr = pf.hr
      LEFT JOIN uo
        ON co.stay_id = uo.stay_id AND co.hr = uo.hr
      LEFT JOIN vaso
        ON co.stay_id = vaso.stay_id AND co.hr = vaso.hr
    ), scorecalc AS (
      SELECT
        scorecomp.*,
        CASE
          WHEN pao2fio2ratio_vent < 100
          THEN 4
          WHEN pao2fio2ratio_vent < 200
          THEN 3
          WHEN pao2fio2ratio_novent < 300
          THEN 2
          WHEN pao2fio2ratio_vent < 300
          THEN 2
          WHEN pao2fio2ratio_novent < 400
          THEN 1
          WHEN pao2fio2ratio_vent < 400
          THEN 1
          WHEN COALESCE(pao2fio2ratio_vent, pao2fio2ratio_novent) IS NULL
          THEN NULL
          ELSE 0
        END AS respiration,
        CASE
          WHEN platelet_min < 20
          THEN 4
          WHEN platelet_min < 50
          THEN 3
          WHEN platelet_min < 100
          THEN 2
          WHEN platelet_min < 150
          THEN 1
          WHEN platelet_min IS NULL
          THEN NULL
          ELSE 0
        END AS coagulation,
        CASE
          WHEN bilirubin_max >= 12.0
          THEN 4
          WHEN bilirubin_max >= 6.0
          THEN 3
          WHEN bilirubin_max >= 2.0
          THEN 2
          WHEN bilirubin_max >= 1.2
          THEN 1
          WHEN bilirubin_max IS NULL
          THEN NULL
          ELSE 0
        END AS liver,
        CASE
          WHEN rate_dopamine > 15 OR rate_epinephrine > 0.1 OR rate_norepinephrine > 0.1
          THEN 4
          WHEN rate_dopamine > 5 OR rate_epinephrine <= 0.1 OR rate_norepinephrine <= 0.1
          THEN 3
          WHEN rate_dopamine > 0 OR rate_dobutamine > 0
          THEN 2
          WHEN meanbp_min < 70
          THEN 1
          WHEN COALESCE(meanbp_min, rate_dopamine, rate_dobutamine, rate_epinephrine, rate_norepinephrine) IS NULL
          THEN NULL
          ELSE 0
        END AS cardiovascular,
        CASE
          WHEN (
            gcs_min >= 13 AND gcs_min <= 14
          )
          THEN 1
          WHEN (
            gcs_min >= 10 AND gcs_min <= 12
          )
          THEN 2
          WHEN (
            gcs_min >= 6 AND gcs_min <= 9
          )
          THEN 3
          WHEN gcs_min < 6
          THEN 4
          WHEN gcs_min IS NULL
          THEN NULL
          ELSE 0
        END AS cns,
        CASE
          WHEN (
            creatinine_max >= 5.0
          )
          THEN 4
          WHEN uo_24hr < 200
          THEN 4
          WHEN (
            creatinine_max >= 3.5 AND creatinine_max < 5.0
          )
          THEN 3
          WHEN uo_24hr < 500
          THEN 3
          WHEN (
            creatinine_max >= 2.0 AND creatinine_max < 3.5
          )
          THEN 2
          WHEN (
            creatinine_max >= 1.2 AND creatinine_max < 2.0
          )
          THEN 1
          WHEN COALESCE(uo_24hr, creatinine_max) IS NULL
          THEN NULL
          ELSE 0
        END AS renal
      FROM scorecomp
    ), score_final AS (
      SELECT
        s.*,
        COALESCE(MAX(respiration) OVER w, 0) AS respiration_24hours,
        COALESCE(MAX(coagulation) OVER w, 0) AS coagulation_24hours,
        COALESCE(MAX(liver) OVER w, 0) AS liver_24hours,
        COALESCE(MAX(cardiovascular) OVER w, 0) AS cardiovascular_24hours,
        COALESCE(MAX(cns) OVER w, 0) AS cns_24hours,
        COALESCE(MAX(renal) OVER w, 0) AS renal_24hours,
        COALESCE(MAX(respiration) OVER w, 0) + COALESCE(MAX(coagulation) OVER w, 0) + COALESCE(MAX(liver) OVER w, 0) + COALESCE(MAX(cardiovascular) OVER w, 0) + COALESCE(MAX(cns) OVER w, 0) + COALESCE(MAX(renal) OVER w, 0) AS sofa_24hours
      FROM scorecalc AS s
      WINDOW w AS (
        PARTITION BY stay_id
        ORDER BY hr NULLS FIRST
        ROWS BETWEEN 23 PRECEDING AND 0 FOLLOWING
      )
    )
    SELECT
      *
    FROM score_final
    WHERE
      hr >= 0
),
antibiotic AS (
    -- medication/antibiotic.sql
    WITH abx AS (
      SELECT DISTINCT
        drug,
        route,
        CASE
          WHEN LOWER(drug) LIKE '%adoxa%'
          THEN 1
          WHEN LOWER(drug) LIKE '%ala-tet%'
          THEN 1
          WHEN LOWER(drug) LIKE '%alodox%'
          THEN 1
          WHEN LOWER(drug) LIKE '%amikacin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%amikin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%amoxicill%'
          THEN 1
          WHEN LOWER(drug) LIKE '%amphotericin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%anidulafungin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%ancef%'
          THEN 1
          WHEN LOWER(drug) LIKE '%clavulanate%'
          THEN 1
          WHEN LOWER(drug) LIKE '%ampicillin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%augmentin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%avelox%'
          THEN 1
          WHEN LOWER(drug) LIKE '%avidoxy%'
          THEN 1
          WHEN LOWER(drug) LIKE '%azactam%'
          THEN 1
          WHEN LOWER(drug) LIKE '%azithromycin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%aztreonam%'
          THEN 1
          WHEN LOWER(drug) LIKE '%axetil%'
          THEN 1
          WHEN LOWER(drug) LIKE '%bactocill%'
          THEN 1
          WHEN LOWER(drug) LIKE '%bactrim%'
          THEN 1
          WHEN LOWER(drug) LIKE '%bactroban%'
          THEN 1
          WHEN LOWER(drug) LIKE '%bethkis%'
          THEN 1
          WHEN LOWER(drug) LIKE '%biaxin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%bicillin l-a%'
          THEN 1
          WHEN LOWER(drug) LIKE '%cayston%'
          THEN 1
          WHEN LOWER(drug) LIKE '%cefazolin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%cedax%'
          THEN 1
          WHEN LOWER(drug) LIKE '%cefoxitin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%ceftazidime%'
          THEN 1
          WHEN LOWER(drug) LIKE '%cefaclor%'
          THEN 1
          WHEN LOWER(drug) LIKE '%cefadroxil%'
          THEN 1
          WHEN LOWER(drug) LIKE '%cefdinir%'
          THEN 1
          WHEN LOWER(drug) LIKE '%cefditoren%'
          THEN 1
          WHEN LOWER(drug) LIKE '%cefepime%'
          THEN 1
          WHEN LOWER(drug) LIKE '%cefotan%'
          THEN 1
          WHEN LOWER(drug) LIKE '%cefotetan%'
          THEN 1
          WHEN LOWER(drug) LIKE '%cefotaxime%'
          THEN 1
          WHEN LOWER(drug) LIKE '%ceftaroline%'
          THEN 1
          WHEN LOWER(drug) LIKE '%cefpodoxime%'
          THEN 1
          WHEN LOWER(drug) LIKE '%cefpirome%'
          THEN 1
          WHEN LOWER(drug) LIKE '%cefprozil%'
          THEN 1
          WHEN LOWER(drug) LIKE '%ceftibuten%'
          THEN 1
          WHEN LOWER(drug) LIKE '%ceftin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%ceftriaxone%'
          THEN 1
          WHEN LOWER(drug) LIKE '%cefuroxime%'
          THEN 1
          WHEN LOWER(drug) LIKE '%cephalexin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%cephalothin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%cephapririn%'
          THEN 1
          WHEN LOWER(drug) LIKE '%chloramphenicol%'
          THEN 1
          WHEN LOWER(drug) LIKE '%cipro%'
          THEN 1
          WHEN LOWER(drug) LIKE '%ciprofloxacin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%claforan%'
          THEN 1
          WHEN LOWER(drug) LIKE '%clarithromycin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%cleocin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%clindamycin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%cubicin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%dicloxacillin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%dirithromycin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%doryx%'
          THEN 1
          WHEN LOWER(drug) LIKE '%doxycy%'
          THEN 1
          WHEN LOWER(drug) LIKE '%duricef%'
          THEN 1
          WHEN LOWER(drug) LIKE '%dynacin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%ery-tab%'
          THEN 1
          WHEN LOWER(drug) LIKE '%eryped%'
          THEN 1
          WHEN LOWER(drug) LIKE '%eryc%'
          THEN 1
          WHEN LOWER(drug) LIKE '%erythrocin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%erythromycin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%factive%'
          THEN 1
          WHEN LOWER(drug) LIKE '%flagyl%'
          THEN 1
          WHEN LOWER(drug) LIKE '%fortaz%'
          THEN 1
          WHEN LOWER(drug) LIKE '%furadantin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%garamycin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%gentamicin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%kanamycin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%keflex%'
          THEN 1
          WHEN LOWER(drug) LIKE '%kefzol%'
          THEN 1
          WHEN LOWER(drug) LIKE '%ketek%'
          THEN 1
          WHEN LOWER(drug) LIKE '%levaquin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%levofloxacin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%lincocin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%linezolid%'
          THEN 1
          WHEN LOWER(drug) LIKE '%macrobid%'
          THEN 1
          WHEN LOWER(drug) LIKE '%macrodantin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%maxipime%'
          THEN 1
          WHEN LOWER(drug) LIKE '%mefoxin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%metronidazole%'
          THEN 1
          WHEN LOWER(drug) LIKE '%meropenem%'
          THEN 1
          WHEN LOWER(drug) LIKE '%methicillin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%minocin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%minocycline%'
          THEN 1
          WHEN LOWER(drug) LIKE '%monodox%'
          THEN 1
          WHEN LOWER(drug) LIKE '%monurol%'
          THEN 1
          WHEN LOWER(drug) LIKE '%morgidox%'
          THEN 1
          WHEN LOWER(drug) LIKE '%moxatag%'
          THEN 1
          WHEN LOWER(drug) LIKE '%moxifloxacin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%mupirocin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%myrac%'
          THEN 1
          WHEN LOWER(drug) LIKE '%nafcillin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%neomycin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%nicazel doxy 30%'
          THEN 1
          WHEN LOWER(drug) LIKE '%nitrofurantoin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%norfloxacin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%noroxin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%ocudox%'
          THEN 1
          WHEN LOWER(drug) LIKE '%ofloxacin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%omnicef%'
          THEN 1
          WHEN LOWER(drug) LIKE '%oracea%'
          THEN 1
          WHEN LOWER(drug) LIKE '%oraxyl%'
          THEN 1
          WHEN LOWER(drug) LIKE '%oxacillin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%pc pen vk%'
          THEN 1
          WHEN LOWER(drug) LIKE '%pce dispertab%'
          THEN 1
          WHEN LOWER(drug) LIKE '%panixine%'
          THEN 1
          WHEN LOWER(drug) LIKE '%pediazole%'
          THEN 1
          WHEN LOWER(drug) LIKE '%penicillin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%periostat%'
          THEN 1
          WHEN LOWER(drug) LIKE '%pfizerpen%'
          THEN 1
          WHEN LOWER(drug) LIKE '%piperacillin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%tazobactam%'
          THEN 1
          WHEN LOWER(drug) LIKE '%primsol%'
          THEN 1
          WHEN LOWER(drug) LIKE '%proquin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%raniclor%'
          THEN 1
          WHEN LOWER(drug) LIKE '%rifadin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%rifampin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%rocephin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%smz-tmp%'
          THEN 1
          WHEN LOWER(drug) LIKE '%septra%'
          THEN 1
          WHEN LOWER(drug) LIKE '%septra ds%'
          THEN 1
          WHEN LOWER(drug) LIKE '%septra%'
          THEN 1
          WHEN LOWER(drug) LIKE '%solodyn%'
          THEN 1
          WHEN LOWER(drug) LIKE '%spectracef%'
          THEN 1
          WHEN LOWER(drug) LIKE '%streptomycin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%sulfadiazine%'
          THEN 1
          WHEN LOWER(drug) LIKE '%sulfamethoxazole%'
          THEN 1
          WHEN LOWER(drug) LIKE '%trimethoprim%'
          THEN 1
          WHEN LOWER(drug) LIKE '%sulfatrim%'
          THEN 1
          WHEN LOWER(drug) LIKE '%sulfisoxazole%'
          THEN 1
          WHEN LOWER(drug) LIKE '%suprax%'
          THEN 1
          WHEN LOWER(drug) LIKE '%synercid%'
          THEN 1
          WHEN LOWER(drug) LIKE '%tazicef%'
          THEN 1
          WHEN LOWER(drug) LIKE '%tetracycline%'
          THEN 1
          WHEN LOWER(drug) LIKE '%timentin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%tobramycin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%trimethoprim%'
          THEN 1
          WHEN LOWER(drug) LIKE '%unasyn%'
          THEN 1
          WHEN LOWER(drug) LIKE '%vancocin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%vancomycin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%vantin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%vibativ%'
          THEN 1
          WHEN LOWER(drug) LIKE '%vibra-tabs%'
          THEN 1
          WHEN LOWER(drug) LIKE '%vibramycin%'
          THEN 1
          WHEN LOWER(drug) LIKE '%zinacef%'
          THEN 1
          WHEN LOWER(drug) LIKE '%zithromax%'
          THEN 1
          WHEN LOWER(drug) LIKE '%zosyn%'
          THEN 1
          WHEN LOWER(drug) LIKE '%zyvox%'
          THEN 1
          ELSE 0
        END AS antibiotic
      FROM mimiciv_hosp.prescriptions
      WHERE
        NOT drug_type IN ('BASE')
        AND NOT route IN ('OU', 'OS', 'OD', 'AU', 'AS', 'AD', 'TP')
        AND LOWER(route) NOT LIKE '%ear%'
        AND LOWER(route) NOT LIKE '%eye%'
        AND LOWER(drug) NOT LIKE '%cream%'
        AND LOWER(drug) NOT LIKE '%desensitization%'
        AND LOWER(drug) NOT LIKE '%ophth oint%'
        AND LOWER(drug) NOT LIKE '%gel%'
    )
    SELECT
      pr.subject_id,
      pr.hadm_id,
      ie.stay_id,
      pr.drug AS antibiotic,
      pr.route,
      pr.starttime,
      pr.stoptime
    FROM mimiciv_hosp.prescriptions AS pr
    INNER JOIN abx
      ON pr.drug = abx.drug AND pr.route = abx.route
    LEFT JOIN mimiciv_icu.icustays AS ie
      ON pr.hadm_id = ie.hadm_id
      AND pr.starttime >= ie.intime
      AND pr.starttime < ie.outtime
    WHERE
      abx.antibiotic = 1
),
suspicion_of_infection AS (
    -- sepsis/suspicion_of_infection.sql
    WITH ab_tbl AS (
      SELECT
        abx.subject_id,
        abx.hadm_id,
        abx.stay_id,
        abx.antibiotic,
        abx.starttime AS antibiotic_time,
        DATE_TRUNC('DAY', CAST(abx.starttime AS TIMESTAMP)) AS antibiotic_date,
        abx.stoptime AS antibiotic_stoptime,
        ROW_NUMBER() OVER (
          PARTITION BY subject_id
          ORDER BY starttime NULLS FIRST, stoptime NULLS FIRST, antibiotic NULLS FIRST, hadm_id NULLS FIRST, stay_id NULLS FIRST
        ) AS ab_id
      FROM antibiotic AS abx
    ), me AS (
      SELECT
        micro_specimen_id,
        MAX(subject_id) AS subject_id,
        MAX(hadm_id) AS hadm_id,
        CAST(MAX(chartdate) AS DATE) AS chartdate,
        MAX(charttime) AS charttime,
        MAX(spec_type_desc) AS spec_type_desc,
        MAX(
          CASE
            WHEN NOT org_name IS NULL
            AND NOT org_itemid IN (90856, 90760)
            AND org_name <> ''
            AND org_name <> 'CANCELLED'
            THEN 1
            ELSE 0
          END
        ) AS positiveculture
      FROM mimiciv_hosp.microbiologyevents
      GROUP BY
        micro_specimen_id
    ), me_then_ab AS (
      SELECT
        ab_tbl.subject_id,
        ab_tbl.hadm_id,
        ab_tbl.stay_id,
        ab_tbl.ab_id,
        me72.micro_specimen_id,
        COALESCE(me72.charttime, CAST(me72.chartdate AS TIMESTAMP)) AS last72_charttime,
        me72.positiveculture AS last72_positiveculture,
        me72.spec_type_desc AS last72_specimen,
        ROW_NUMBER() OVER (
          PARTITION BY ab_tbl.subject_id, ab_tbl.ab_id
          ORDER BY me72.chartdate NULLS FIRST, me72.charttime, me72.positiveculture DESC, me72.micro_specimen_id NULLS FIRST
        ) AS micro_seq
      FROM ab_tbl
      LEFT JOIN me AS me72
        ON ab_tbl.subject_id = me72.subject_id
        AND (
          (
            NOT me72.charttime IS NULL
            AND ab_tbl.antibiotic_time > me72.charttime
            AND ab_tbl.antibiotic_time <= me72.charttime + INTERVAL '72' HOUR
          )
          OR (
            me72.charttime IS NULL
            AND antibiotic_date >= me72.chartdate
            AND antibiotic_date <= me72.chartdate + INTERVAL '3' DAY
          )
        )
    ), ab_then_me AS (
      SELECT
        ab_tbl.subject_id,
        ab_tbl.hadm_id,
        ab_tbl.stay_id,
        ab_tbl.ab_id,
        me24.micro_specimen_id,
        COALESCE(me24.charttime, CAST(me24.chartdate AS TIMESTAMP)) AS next24_charttime,
        me24.positiveculture AS next24_positiveculture,
        me24.spec_type_desc AS next24_specimen,
        ROW_NUMBER() OVER (
          PARTITION BY ab_tbl.subject_id, ab_tbl.ab_id
          ORDER BY me24.chartdate NULLS FIRST, me24.charttime, me24.positiveculture DESC, me24.micro_specimen_id NULLS FIRST
        ) AS micro_seq
      FROM ab_tbl
      LEFT JOIN me AS me24
        ON ab_tbl.subject_id = me24.subject_id
        AND (
          (
            NOT me24.charttime IS NULL
            AND ab_tbl.antibiotic_time >= me24.charttime - INTERVAL '24' HOUR
            AND ab_tbl.antibiotic_time < me24.charttime
          )
          OR (
            me24.charttime IS NULL
            AND ab_tbl.antibiotic_date >= me24.chartdate - INTERVAL '1' DAY
            AND ab_tbl.antibiotic_date <= me24.chartdate
          )
        )
    )
    SELECT
      ab_tbl.subject_id,
      ab_tbl.stay_id,
      ab_tbl.hadm_id,
      ab_tbl.ab_id,
      ab_tbl.antibiotic,
      ab_tbl.antibiotic_time,
      CASE WHEN last72_specimen IS NULL AND next24_specimen IS NULL THEN 0 ELSE 1 END AS suspected_infection,
      CASE
        WHEN last72_specimen IS NULL AND next24_specimen IS NULL
        THEN NULL
        ELSE COALESCE(last72_charttime, antibiotic_time)
      END AS suspected_infection_time,
      COALESCE(last72_charttime, next24_charttime) AS culture_time,
      COALESCE(last72_specimen, next24_specimen) AS specimen,
      COALESCE(last72_positiveculture, next24_positiveculture) AS positive_culture
    FROM ab_tbl
    LEFT JOIN ab_then_me AS ab2me
      ON ab_tbl.subject_id = ab2me.subject_id
      AND ab_tbl.ab_id = ab2me.ab_id
      AND ab2me.micro_seq = 1
    LEFT JOIN me_then_ab AS me2ab
      ON ab_tbl.subject_id = me2ab.subject_id
      AND ab_tbl.ab_id = me2ab.ab_id
      AND me2ab.micro_seq = 1
)
-- sepsis/sepsis3.sql
SELECT * FROM (
    WITH sofa AS (
      SELECT
        stay_id,
        starttime,
        endtime,
        respiration_24hours AS respiration,
        coagulation_24hours AS coagulation,
        liver_24hours AS liver,
        cardiovascular_24hours AS cardiovascular,
        cns_24hours AS cns,
        renal_24hours AS renal,
        sofa_24hours AS sofa_score
      FROM sofa_
      WHERE
        sofa_24hours >= 2
    ), s1 AS (
      SELECT
        soi.subject_id,
        soi.stay_id,
        soi.ab_id,
        soi.antibiotic,
        soi.antibiotic_time,
        soi.culture_time,
        soi.suspected_infection,
        soi.suspected_infection_time,
        soi.specimen,
        soi.positive_culture,
        starttime,
        endtime,
        respiration,
        coagulation,
        liver,
        cardiovascular,
        cns,
        renal,
        sofa_score,
        sofa_score >= 2 AND suspected_infection = 1 AS sepsis3,
        ROW_NUMBER() OVER (
          PARTITION BY soi.stay_id
          ORDER BY suspected_infection_time NULLS FIRST, antibiotic_time NULLS FIRST, culture_time NULLS FIRST, endtime NULLS FIRST
        ) AS rn_sus
      FROM suspicion_of_infection AS soi
      INNER JOIN sofa
        ON soi.stay_id = sofa.stay_id
        AND sofa.endtime >= soi.suspected_infection_time - INTERVAL '48' HOUR
        AND sofa.endtime <= soi.suspected_infection_time + INTERVAL '24' HOUR
      WHERE
        NOT soi.stay_id IS NULL
    )
    SELECT
      subject_id,
      stay_id,
      antibiotic_time,
      culture_time,
      suspected_infection_time,
      endtime AS sofa_time,
      sofa_score,
      respiration,
      coagulation,
      liver,
      cardiovascular,
      cns,
      renal,
      sepsis3
    FROM s1
    WHERE
      rn_sus = 1
) AS sepsis3_
