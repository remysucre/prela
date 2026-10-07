-- mimic-code concepts_duckdb: sapsii, with its 11 upstream concepts inlined as CTEs
WITH age AS (
    -- demographics/age.sql
    SELECT
      ad.subject_id,
      ad.hadm_id,
      ad.admittime,
      pa.anchor_age,
      pa.anchor_year,
      pa.anchor_age + DATE_DIFF('YEAR', MAKE_TIMESTAMP(pa.anchor_year, 1, 1, 0, 0, 0), ad.admittime) AS age
    FROM mimiciv_hosp.admissions AS ad
    INNER JOIN mimiciv_hosp.patients AS pa
      ON ad.subject_id = pa.subject_id
),
bg AS (
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
)
-- score/sapsii.sql
SELECT * FROM (
    WITH co AS (
      SELECT
        subject_id,
        hadm_id,
        stay_id,
        intime AS starttime,
        intime + INTERVAL '24' HOUR AS endtime
      FROM mimiciv_icu.icustays
    ), cpap AS (
      SELECT
        co.subject_id,
        co.stay_id,
        CASE
          WHEN MIN(charttime - INTERVAL '1' HOUR) IS NULL OR co.starttime IS NULL
          THEN NULL
          ELSE GREATEST(MIN(charttime - INTERVAL '1' HOUR), co.starttime)
        END AS starttime,
        CASE
          WHEN MAX(charttime + INTERVAL '4' HOUR) IS NULL OR co.endtime IS NULL
          THEN NULL
          ELSE LEAST(MAX(charttime + INTERVAL '4' HOUR), co.endtime)
        END AS endtime,
        MAX(CASE WHEN REGEXP_MATCHES(LOWER(ce.value), '(cpap mask|bipap)') THEN 1 ELSE 0 END) AS cpap
      FROM co
      INNER JOIN mimiciv_icu.chartevents AS ce
        ON co.stay_id = ce.stay_id
        AND ce.charttime > co.starttime
        AND ce.charttime <= co.endtime
      WHERE
        ce.itemid = 226732 AND REGEXP_MATCHES(LOWER(ce.value), '(cpap mask|bipap)')
      GROUP BY
        co.subject_id,
        co.stay_id,
        co.starttime,
        co.endtime
    ), surgflag AS (
      SELECT
        adm.hadm_id,
        CASE WHEN LOWER(curr_service) LIKE '%surg%' THEN 1 ELSE 0 END AS surgical,
        ROW_NUMBER() OVER (PARTITION BY adm.hadm_id ORDER BY transfertime NULLS FIRST) AS serviceorder
      FROM mimiciv_hosp.admissions AS adm
      LEFT JOIN mimiciv_hosp.services AS se
        ON adm.hadm_id = se.hadm_id
    ), comorb AS (
      SELECT
        hadm_id,
        MAX(
          CASE
            WHEN icd_version = 9 AND SUBSTRING(icd_code, 1, 3) BETWEEN '042' AND '044'
            THEN 1
            WHEN icd_version = 10 AND SUBSTRING(icd_code, 1, 3) BETWEEN 'B20' AND 'B22'
            THEN 1
            WHEN icd_version = 10 AND SUBSTRING(icd_code, 1, 3) = 'B24'
            THEN 1
            ELSE 0
          END
        ) AS aids,
        MAX(
          CASE
            WHEN icd_version = 9
            THEN CASE
              WHEN SUBSTRING(icd_code, 1, 5) BETWEEN '20000' AND '20238'
              THEN 1
              WHEN SUBSTRING(icd_code, 1, 5) BETWEEN '20240' AND '20248'
              THEN 1
              WHEN SUBSTRING(icd_code, 1, 5) BETWEEN '20250' AND '20302'
              THEN 1
              WHEN SUBSTRING(icd_code, 1, 5) BETWEEN '20310' AND '20312'
              THEN 1
              WHEN SUBSTRING(icd_code, 1, 5) BETWEEN '20302' AND '20382'
              THEN 1
              WHEN SUBSTRING(icd_code, 1, 5) BETWEEN '20400' AND '20522'
              THEN 1
              WHEN SUBSTRING(icd_code, 1, 5) BETWEEN '20580' AND '20702'
              THEN 1
              WHEN SUBSTRING(icd_code, 1, 5) BETWEEN '20720' AND '20892'
              THEN 1
              WHEN SUBSTRING(icd_code, 1, 4) IN ('2386', '2733')
              THEN 1
              ELSE 0
            END
            WHEN icd_version = 10 AND SUBSTRING(icd_code, 1, 3) BETWEEN 'C81' AND 'C96'
            THEN 1
            ELSE 0
          END
        ) AS hem,
        MAX(
          CASE
            WHEN icd_version = 9
            THEN CASE
              WHEN SUBSTRING(icd_code, 1, 4) BETWEEN '1960' AND '1991'
              THEN 1
              WHEN SUBSTRING(icd_code, 1, 5) BETWEEN '20970' AND '20975'
              THEN 1
              WHEN SUBSTRING(icd_code, 1, 5) IN ('20979', '78951')
              THEN 1
              ELSE 0
            END
            WHEN icd_version = 10 AND SUBSTRING(icd_code, 1, 3) BETWEEN 'C77' AND 'C79'
            THEN 1
            WHEN icd_version = 10 AND SUBSTRING(icd_code, 1, 4) = 'C800'
            THEN 1
            ELSE 0
          END
        ) AS mets
      FROM mimiciv_hosp.diagnoses_icd
      GROUP BY
        hadm_id
    ), pafi1 AS (
      SELECT
        co.stay_id,
        bg.charttime,
        pao2fio2ratio AS pao2fio2,
        CASE WHEN NOT vd.stay_id IS NULL THEN 1 ELSE 0 END AS vent,
        CASE WHEN NOT cp.subject_id IS NULL THEN 1 ELSE 0 END AS cpap
      FROM co
      LEFT JOIN bg AS bg
        ON co.subject_id = bg.subject_id
        AND bg.specimen = 'ART.'
        AND bg.charttime > co.starttime
        AND bg.charttime <= co.endtime
      LEFT JOIN ventilation AS vd
        ON co.stay_id = vd.stay_id
        AND bg.charttime > vd.starttime
        AND bg.charttime <= vd.endtime
        AND vd.ventilation_status = 'InvasiveVent'
      LEFT JOIN cpap AS cp
        ON bg.subject_id = cp.subject_id
        AND bg.charttime > cp.starttime
        AND bg.charttime <= cp.endtime
    ), pafi2 AS (
      SELECT
        stay_id,
        MIN(pao2fio2) AS pao2fio2_vent_min
      FROM pafi1
      WHERE
        vent = 1 OR cpap = 1
      GROUP BY
        stay_id
    ), gcs AS (
      SELECT
        co.stay_id,
        MIN(gcs.gcs) AS mingcs
      FROM co
      LEFT JOIN gcs_ AS gcs
        ON co.stay_id = gcs.stay_id
        AND co.starttime < gcs.charttime
        AND gcs.charttime <= co.endtime
      GROUP BY
        co.stay_id
    ), vital AS (
      SELECT
        co.stay_id,
        MIN(vital.heart_rate) AS heartrate_min,
        MAX(vital.heart_rate) AS heartrate_max,
        MIN(vital.sbp) AS sysbp_min,
        MAX(vital.sbp) AS sysbp_max,
        MIN(vital.temperature) AS tempc_min,
        MAX(vital.temperature) AS tempc_max
      FROM co
      LEFT JOIN vitalsign AS vital
        ON co.subject_id = vital.subject_id
        AND co.starttime < vital.charttime
        AND co.endtime >= vital.charttime
      GROUP BY
        co.stay_id
    ), uo AS (
      SELECT
        co.stay_id,
        SUM(uo.urineoutput) AS urineoutput
      FROM co
      LEFT JOIN urine_output AS uo
        ON co.stay_id = uo.stay_id
        AND co.starttime < uo.charttime
        AND co.endtime >= uo.charttime
      GROUP BY
        co.stay_id
    ), labs AS (
      SELECT
        co.stay_id,
        MIN(labs.bun) AS bun_min,
        MAX(labs.bun) AS bun_max,
        MIN(labs.potassium) AS potassium_min,
        MAX(labs.potassium) AS potassium_max,
        MIN(labs.sodium) AS sodium_min,
        MAX(labs.sodium) AS sodium_max,
        MIN(labs.bicarbonate) AS bicarbonate_min,
        MAX(labs.bicarbonate) AS bicarbonate_max
      FROM co
      LEFT JOIN chemistry AS labs
        ON co.subject_id = labs.subject_id
        AND co.starttime < labs.charttime
        AND co.endtime >= labs.charttime
      GROUP BY
        co.stay_id
    ), cbc AS (
      SELECT
        co.stay_id,
        MIN(cbc.wbc) AS wbc_min,
        MAX(cbc.wbc) AS wbc_max
      FROM co
      LEFT JOIN complete_blood_count AS cbc
        ON co.subject_id = cbc.subject_id
        AND co.starttime < cbc.charttime
        AND co.endtime >= cbc.charttime
      GROUP BY
        co.stay_id
    ), enz AS (
      SELECT
        co.stay_id,
        MIN(enz.bilirubin_total) AS bilirubin_min,
        MAX(enz.bilirubin_total) AS bilirubin_max
      FROM co
      LEFT JOIN enzyme AS enz
        ON co.subject_id = enz.subject_id
        AND co.starttime < enz.charttime
        AND co.endtime >= enz.charttime
      GROUP BY
        co.stay_id
    ), cohort AS (
      SELECT
        ie.subject_id,
        ie.hadm_id,
        ie.stay_id,
        ie.intime,
        ie.outtime,
        va.age,
        co.starttime,
        co.endtime,
        vital.heartrate_max,
        vital.heartrate_min,
        vital.sysbp_max,
        vital.sysbp_min,
        vital.tempc_max,
        vital.tempc_min,
        pf.pao2fio2_vent_min,
        uo.urineoutput,
        labs.bun_min,
        labs.bun_max,
        cbc.wbc_min,
        cbc.wbc_max,
        labs.potassium_min,
        labs.potassium_max,
        labs.sodium_min,
        labs.sodium_max,
        labs.bicarbonate_min,
        labs.bicarbonate_max,
        enz.bilirubin_min,
        enz.bilirubin_max,
        gcs.mingcs,
        comorb.aids,
        comorb.hem,
        comorb.mets,
        CASE
          WHEN adm.admission_type = 'ELECTIVE' AND sf.surgical = 1
          THEN 'ScheduledSurgical'
          WHEN adm.admission_type <> 'ELECTIVE' AND sf.surgical = 1
          THEN 'UnscheduledSurgical'
          ELSE 'Medical'
        END AS admissiontype
      FROM mimiciv_icu.icustays AS ie
      INNER JOIN mimiciv_hosp.admissions AS adm
        ON ie.hadm_id = adm.hadm_id
      LEFT JOIN age AS va
        ON ie.hadm_id = va.hadm_id
      INNER JOIN co
        ON ie.stay_id = co.stay_id
      LEFT JOIN pafi2 AS pf
        ON ie.stay_id = pf.stay_id
      LEFT JOIN surgflag AS sf
        ON adm.hadm_id = sf.hadm_id AND sf.serviceorder = 1
      LEFT JOIN comorb
        ON ie.hadm_id = comorb.hadm_id
      LEFT JOIN gcs AS gcs
        ON ie.stay_id = gcs.stay_id
      LEFT JOIN vital
        ON ie.stay_id = vital.stay_id
      LEFT JOIN uo
        ON ie.stay_id = uo.stay_id
      LEFT JOIN labs
        ON ie.stay_id = labs.stay_id
      LEFT JOIN cbc
        ON ie.stay_id = cbc.stay_id
      LEFT JOIN enz
        ON ie.stay_id = enz.stay_id
    ), scorecomp AS (
      SELECT
        cohort.*,
        CASE
          WHEN age IS NULL
          THEN NULL
          WHEN age < 40
          THEN 0
          WHEN age < 60
          THEN 7
          WHEN age < 70
          THEN 12
          WHEN age < 75
          THEN 15
          WHEN age < 80
          THEN 16
          WHEN age >= 80
          THEN 18
        END AS age_score,
        CASE
          WHEN heartrate_max IS NULL
          THEN NULL
          WHEN heartrate_min < 40
          THEN 11
          WHEN heartrate_max >= 160
          THEN 7
          WHEN heartrate_max >= 120
          THEN 4
          WHEN heartrate_min < 70
          THEN 2
          WHEN heartrate_max >= 70
          AND heartrate_max < 120
          AND heartrate_min >= 70
          AND heartrate_min < 120
          THEN 0
        END AS hr_score,
        CASE
          WHEN sysbp_min IS NULL
          THEN NULL
          WHEN sysbp_min < 70
          THEN 13
          WHEN sysbp_min < 100
          THEN 5
          WHEN sysbp_max >= 200
          THEN 2
          WHEN sysbp_max >= 100 AND sysbp_max < 200 AND sysbp_min >= 100 AND sysbp_min < 200
          THEN 0
        END AS sysbp_score,
        CASE
          WHEN tempc_max IS NULL
          THEN NULL
          WHEN tempc_max >= 39.0
          THEN 3
          WHEN tempc_min < 39.0
          THEN 0
        END AS temp_score,
        CASE
          WHEN pao2fio2_vent_min IS NULL
          THEN NULL
          WHEN pao2fio2_vent_min < 100
          THEN 11
          WHEN pao2fio2_vent_min < 200
          THEN 9
          WHEN pao2fio2_vent_min >= 200
          THEN 6
        END AS pao2fio2_score,
        CASE
          WHEN urineoutput IS NULL
          THEN NULL
          WHEN urineoutput < 500.0
          THEN 11
          WHEN urineoutput < 1000.0
          THEN 4
          WHEN urineoutput >= 1000.0
          THEN 0
        END AS uo_score,
        CASE
          WHEN bun_max IS NULL
          THEN NULL
          WHEN bun_max < 28.0
          THEN 0
          WHEN bun_max < 84.0
          THEN 6
          WHEN bun_max >= 84.0
          THEN 10
        END AS bun_score,
        CASE
          WHEN wbc_max IS NULL
          THEN NULL
          WHEN wbc_min < 1.0
          THEN 12
          WHEN wbc_max >= 20.0
          THEN 3
          WHEN wbc_max >= 1.0 AND wbc_max < 20.0 AND wbc_min >= 1.0 AND wbc_min < 20.0
          THEN 0
        END AS wbc_score,
        CASE
          WHEN potassium_max IS NULL
          THEN NULL
          WHEN potassium_min < 3.0
          THEN 3
          WHEN potassium_max >= 5.0
          THEN 3
          WHEN potassium_max >= 3.0
          AND potassium_max < 5.0
          AND potassium_min >= 3.0
          AND potassium_min < 5.0
          THEN 0
        END AS potassium_score,
        CASE
          WHEN sodium_max IS NULL
          THEN NULL
          WHEN sodium_min < 125
          THEN 5
          WHEN sodium_max >= 145
          THEN 1
          WHEN sodium_max >= 125 AND sodium_max < 145 AND sodium_min >= 125 AND sodium_min < 145
          THEN 0
        END AS sodium_score,
        CASE
          WHEN bicarbonate_max IS NULL
          THEN NULL
          WHEN bicarbonate_min < 15.0
          THEN 6
          WHEN bicarbonate_min < 20.0
          THEN 3
          WHEN bicarbonate_max >= 20.0 AND bicarbonate_min >= 20.0
          THEN 0
        END AS bicarbonate_score,
        CASE
          WHEN bilirubin_max IS NULL
          THEN NULL
          WHEN bilirubin_max < 4.0
          THEN 0
          WHEN bilirubin_max < 6.0
          THEN 4
          WHEN bilirubin_max >= 6.0
          THEN 9
        END AS bilirubin_score,
        CASE
          WHEN mingcs IS NULL
          THEN NULL
          WHEN mingcs < 3
          THEN NULL
          WHEN mingcs < 6
          THEN 26
          WHEN mingcs < 9
          THEN 13
          WHEN mingcs < 11
          THEN 7
          WHEN mingcs < 14
          THEN 5
          WHEN mingcs >= 14 AND mingcs <= 15
          THEN 0
        END AS gcs_score,
        CASE WHEN aids = 1 THEN 17 WHEN hem = 1 THEN 10 WHEN mets = 1 THEN 9 ELSE 0 END AS comorbidity_score,
        CASE
          WHEN admissiontype = 'ScheduledSurgical'
          THEN 0
          WHEN admissiontype = 'Medical'
          THEN 6
          WHEN admissiontype = 'UnscheduledSurgical'
          THEN 8
          ELSE NULL
        END AS admissiontype_score
      FROM cohort
    ), score AS (
      SELECT
        s.*,
        COALESCE(age_score, 0) + COALESCE(hr_score, 0) + COALESCE(sysbp_score, 0) + COALESCE(temp_score, 0) + COALESCE(pao2fio2_score, 0) + COALESCE(uo_score, 0) + COALESCE(bun_score, 0) + COALESCE(wbc_score, 0) + COALESCE(potassium_score, 0) + COALESCE(sodium_score, 0) + COALESCE(bicarbonate_score, 0) + COALESCE(bilirubin_score, 0) + COALESCE(gcs_score, 0) + COALESCE(comorbidity_score, 0) + COALESCE(admissiontype_score, 0) AS sapsii
      FROM scorecomp AS s
    )
    SELECT
      s.subject_id,
      s.hadm_id,
      s.stay_id,
      s.starttime,
      s.endtime,
      sapsii,
      1 / (
        1 + EXP(-(
          -7.7631 + 0.0737 * (
            sapsii
          ) + 0.9971 * (
            LN(sapsii + 1)
          )
        ))
      ) AS sapsii_prob,
      age_score,
      hr_score,
      sysbp_score,
      temp_score,
      pao2fio2_score,
      uo_score,
      bun_score,
      wbc_score,
      potassium_score,
      sodium_score,
      bicarbonate_score,
      bilirubin_score,
      gcs_score,
      comorbidity_score,
      admissiontype_score
    FROM score AS s
) AS sapsii_
