-- mimic-code concepts_duckdb: oasis, with its 10 upstream concepts inlined as CTEs
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
gcs AS (
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
first_day_gcs AS (
    -- firstday/first_day_gcs.sql
    WITH gcs_final AS (
      SELECT
        ie.subject_id,
        ie.stay_id,
        g.gcs,
        g.gcs_motor,
        g.gcs_verbal,
        g.gcs_eyes,
        g.gcs_unable,
        ROW_NUMBER() OVER (PARTITION BY g.stay_id ORDER BY g.gcs ASC, g.charttime DESC) AS gcs_seq
      FROM mimiciv_icu.icustays AS ie
      LEFT JOIN gcs AS g
        ON ie.stay_id = g.stay_id
        AND g.charttime >= ie.intime - INTERVAL '6' HOUR
        AND g.charttime <= ie.intime + INTERVAL '1' DAY
    )
    SELECT
      ie.subject_id,
      ie.stay_id,
      gcs AS gcs_min,
      gcs_motor,
      gcs_verbal,
      gcs_eyes,
      gcs_unable
    FROM mimiciv_icu.icustays AS ie
    LEFT JOIN gcs_final AS gs
      ON ie.stay_id = gs.stay_id AND gs.gcs_seq = 1
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
first_day_urine_output AS (
    -- firstday/first_day_urine_output.sql
    SELECT
      ie.subject_id,
      ie.stay_id,
      SUM(urineoutput) AS urineoutput
    FROM mimiciv_icu.icustays AS ie
    LEFT JOIN urine_output AS uo
      ON ie.stay_id = uo.stay_id
      AND uo.charttime >= ie.intime
      AND uo.charttime <= ie.intime + INTERVAL '1' DAY
    GROUP BY
      ie.subject_id,
      ie.stay_id
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
first_day_vitalsign AS (
    -- firstday/first_day_vitalsign.sql
    SELECT
      ie.subject_id,
      ie.stay_id,
      MIN(heart_rate) AS heart_rate_min,
      MAX(heart_rate) AS heart_rate_max,
      AVG(heart_rate) AS heart_rate_mean,
      MIN(sbp) AS sbp_min,
      MAX(sbp) AS sbp_max,
      AVG(sbp) AS sbp_mean,
      MIN(dbp) AS dbp_min,
      MAX(dbp) AS dbp_max,
      AVG(dbp) AS dbp_mean,
      MIN(mbp) AS mbp_min,
      MAX(mbp) AS mbp_max,
      AVG(mbp) AS mbp_mean,
      MIN(resp_rate) AS resp_rate_min,
      MAX(resp_rate) AS resp_rate_max,
      AVG(resp_rate) AS resp_rate_mean,
      MIN(temperature) AS temperature_min,
      MAX(temperature) AS temperature_max,
      AVG(temperature) AS temperature_mean,
      MIN(spo2) AS spo2_min,
      MAX(spo2) AS spo2_max,
      AVG(spo2) AS spo2_mean,
      MIN(glucose) AS glucose_min,
      MAX(glucose) AS glucose_max,
      AVG(glucose) AS glucose_mean
    FROM mimiciv_icu.icustays AS ie
    LEFT JOIN vitalsign AS ce
      ON ie.stay_id = ce.stay_id
      AND ce.charttime >= ie.intime - INTERVAL '6' HOUR
      AND ce.charttime <= ie.intime + INTERVAL '1' DAY
    GROUP BY
      ie.subject_id,
      ie.stay_id
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
)
-- score/oasis.sql
SELECT * FROM (
    WITH surgflag AS (
      SELECT
        ie.stay_id,
        MAX(
          CASE
            WHEN LOWER(curr_service) LIKE '%surg%'
            THEN 1
            WHEN curr_service = 'ORTHO'
            THEN 1
            ELSE 0
          END
        ) AS surgical
      FROM mimiciv_icu.icustays AS ie
      LEFT JOIN mimiciv_hosp.services AS se
        ON ie.hadm_id = se.hadm_id AND se.transfertime < ie.intime + INTERVAL '1' DAY
      GROUP BY
        ie.stay_id
    ), vent AS (
      SELECT
        ie.stay_id,
        MAX(CASE WHEN NOT v.stay_id IS NULL THEN 1 ELSE 0 END) AS vent
      FROM mimiciv_icu.icustays AS ie
      LEFT JOIN ventilation AS v
        ON ie.stay_id = v.stay_id
        AND v.ventilation_status = 'InvasiveVent'
        AND (
          (
            v.starttime >= ie.intime AND v.starttime <= ie.intime + INTERVAL '1' DAY
          )
          OR (
            v.endtime >= ie.intime AND v.endtime <= ie.intime + INTERVAL '1' DAY
          )
          OR (
            v.starttime <= ie.intime AND v.endtime >= ie.intime + INTERVAL '1' DAY
          )
        )
      GROUP BY
        ie.stay_id
    ), cohort AS (
      SELECT
        ie.subject_id,
        ie.hadm_id,
        ie.stay_id,
        ie.intime,
        ie.outtime,
        adm.deathtime,
        DATE_DIFF('MINUTE', adm.admittime, ie.intime) AS preiculos,
        ag.age,
        gcs.gcs_min,
        vital.heart_rate_max,
        vital.heart_rate_min,
        vital.mbp_max,
        vital.mbp_min,
        vital.resp_rate_max,
        vital.resp_rate_min,
        vital.temperature_max,
        vital.temperature_min,
        vent.vent AS mechvent,
        uo.urineoutput,
        CASE
          WHEN adm.admission_type = 'ELECTIVE' AND sf.surgical = 1
          THEN 1
          WHEN adm.admission_type IS NULL OR sf.surgical IS NULL
          THEN NULL
          ELSE 0
        END AS electivesurgery,
        CASE
          WHEN adm.deathtime BETWEEN ie.intime AND ie.outtime
          THEN 1
          WHEN adm.deathtime <= ie.intime
          THEN 1
          WHEN adm.dischtime <= ie.outtime AND adm.discharge_location = 'DEAD/EXPIRED'
          THEN 1
          ELSE 0
        END AS icustay_expire_flag,
        adm.hospital_expire_flag
      FROM mimiciv_icu.icustays AS ie
      INNER JOIN mimiciv_hosp.admissions AS adm
        ON ie.hadm_id = adm.hadm_id
      INNER JOIN mimiciv_hosp.patients AS pat
        ON ie.subject_id = pat.subject_id
      LEFT JOIN age AS ag
        ON ie.hadm_id = ag.hadm_id
      LEFT JOIN surgflag AS sf
        ON ie.stay_id = sf.stay_id
      LEFT JOIN first_day_gcs AS gcs
        ON ie.stay_id = gcs.stay_id
      LEFT JOIN first_day_vitalsign AS vital
        ON ie.stay_id = vital.stay_id
      LEFT JOIN first_day_urine_output AS uo
        ON ie.stay_id = uo.stay_id
      LEFT JOIN vent
        ON ie.stay_id = vent.stay_id
    ), scorecomp AS (
      SELECT
        co.subject_id,
        co.hadm_id,
        co.stay_id,
        co.icustay_expire_flag,
        co.hospital_expire_flag,
        CASE
          WHEN preiculos IS NULL
          THEN NULL
          WHEN preiculos < 10.2
          THEN 5
          WHEN preiculos < 297
          THEN 3
          WHEN preiculos < 1440
          THEN 0
          WHEN preiculos < 18708
          THEN 2
          ELSE 1
        END AS preiculos_score,
        CASE
          WHEN age IS NULL
          THEN NULL
          WHEN age < 24
          THEN 0
          WHEN age <= 53
          THEN 3
          WHEN age <= 77
          THEN 6
          WHEN age <= 89
          THEN 9
          WHEN age >= 90
          THEN 7
          ELSE 0
        END AS age_score,
        CASE
          WHEN gcs_min IS NULL
          THEN NULL
          WHEN gcs_min <= 7
          THEN 10
          WHEN gcs_min < 14
          THEN 4
          WHEN gcs_min = 14
          THEN 3
          ELSE 0
        END AS gcs_score,
        CASE
          WHEN heart_rate_max IS NULL
          THEN NULL
          WHEN heart_rate_max > 125
          THEN 6
          WHEN heart_rate_min < 33
          THEN 4
          WHEN heart_rate_max >= 107 AND heart_rate_max <= 125
          THEN 3
          WHEN heart_rate_max >= 89 AND heart_rate_max <= 106
          THEN 1
          ELSE 0
        END AS heart_rate_score,
        CASE
          WHEN mbp_min IS NULL
          THEN NULL
          WHEN mbp_min < 20.65
          THEN 4
          WHEN mbp_min < 51
          THEN 3
          WHEN mbp_max > 143.44
          THEN 3
          WHEN mbp_min >= 51 AND mbp_min < 61.33
          THEN 2
          ELSE 0
        END AS mbp_score,
        CASE
          WHEN resp_rate_min IS NULL
          THEN NULL
          WHEN resp_rate_min < 6
          THEN 10
          WHEN resp_rate_max > 44
          THEN 9
          WHEN resp_rate_max > 30
          THEN 6
          WHEN resp_rate_max > 22
          THEN 1
          WHEN resp_rate_min < 13
          THEN 1
          ELSE 0
        END AS resp_rate_score,
        CASE
          WHEN temperature_max IS NULL
          THEN NULL
          WHEN temperature_max > 39.88
          THEN 6
          WHEN temperature_min >= 33.22 AND temperature_min <= 35.93
          THEN 4
          WHEN temperature_max >= 33.22 AND temperature_max <= 35.93
          THEN 4
          WHEN temperature_min < 33.22
          THEN 3
          WHEN temperature_min > 35.93 AND temperature_min <= 36.39
          THEN 2
          WHEN temperature_max >= 36.89 AND temperature_max <= 39.88
          THEN 2
          ELSE 0
        END AS temp_score,
        CASE
          WHEN urineoutput IS NULL
          THEN NULL
          WHEN urineoutput < 671.09
          THEN 10
          WHEN urineoutput > 6896.80
          THEN 8
          WHEN urineoutput >= 671.09 AND urineoutput <= 1426.99
          THEN 5
          WHEN urineoutput >= 1427.00 AND urineoutput <= 2544.14
          THEN 1
          ELSE 0
        END AS urineoutput_score,
        CASE WHEN mechvent IS NULL THEN NULL WHEN mechvent = 1 THEN 9 ELSE 0 END AS mechvent_score,
        CASE WHEN electivesurgery IS NULL THEN NULL WHEN electivesurgery = 1 THEN 0 ELSE 6 END AS electivesurgery_score,
        preiculos,
        age,
        gcs_min AS gcs,
        CASE
          WHEN heart_rate_max IS NULL
          THEN NULL
          WHEN heart_rate_max > 125
          THEN heart_rate_max
          WHEN heart_rate_min < 33
          THEN heart_rate_min
          WHEN heart_rate_max >= 107 AND heart_rate_max <= 125
          THEN heart_rate_max
          WHEN heart_rate_max >= 89 AND heart_rate_max <= 106
          THEN heart_rate_max
          ELSE (
            heart_rate_min + heart_rate_max
          ) / 2
        END AS heartrate,
        CASE
          WHEN mbp_min IS NULL
          THEN NULL
          WHEN mbp_min < 20.65
          THEN mbp_min
          WHEN mbp_min < 51
          THEN mbp_min
          WHEN mbp_max > 143.44
          THEN mbp_max
          WHEN mbp_min >= 51 AND mbp_min < 61.33
          THEN mbp_min
          ELSE (
            mbp_min + mbp_max
          ) / 2
        END AS meanbp,
        CASE
          WHEN resp_rate_min IS NULL
          THEN NULL
          WHEN resp_rate_min < 6
          THEN resp_rate_min
          WHEN resp_rate_max > 44
          THEN resp_rate_max
          WHEN resp_rate_max > 30
          THEN resp_rate_max
          WHEN resp_rate_max > 22
          THEN resp_rate_max
          WHEN resp_rate_min < 13
          THEN resp_rate_min
          ELSE (
            resp_rate_min + resp_rate_max
          ) / 2
        END AS resprate,
        CASE
          WHEN temperature_max IS NULL
          THEN NULL
          WHEN temperature_max > 39.88
          THEN temperature_max
          WHEN temperature_min >= 33.22 AND temperature_min <= 35.93
          THEN temperature_min
          WHEN temperature_max >= 33.22 AND temperature_max <= 35.93
          THEN temperature_max
          WHEN temperature_min < 33.22
          THEN temperature_min
          WHEN temperature_min > 35.93 AND temperature_min <= 36.39
          THEN temperature_min
          WHEN temperature_max >= 36.89 AND temperature_max <= 39.88
          THEN temperature_max
          ELSE (
            temperature_min + temperature_max
          ) / 2
        END AS temp,
        urineoutput,
        mechvent,
        electivesurgery
      FROM cohort AS co
    ), score AS (
      SELECT
        s.*,
        COALESCE(age_score, 0) + COALESCE(preiculos_score, 0) + COALESCE(gcs_score, 0) + COALESCE(heart_rate_score, 0) + COALESCE(mbp_score, 0) + COALESCE(resp_rate_score, 0) + COALESCE(temp_score, 0) + COALESCE(urineoutput_score, 0) + COALESCE(mechvent_score, 0) + COALESCE(electivesurgery_score, 0) AS oasis
      FROM scorecomp AS s
    )
    SELECT
      subject_id,
      hadm_id,
      stay_id,
      oasis,
      1 / (
        1 + EXP(-(
          -6.1746 + 0.1275 * (
            oasis
          )
        ))
      ) AS oasis_prob,
      age,
      age_score,
      preiculos,
      preiculos_score,
      gcs,
      gcs_score,
      heartrate,
      heart_rate_score,
      meanbp,
      mbp_score,
      resprate,
      resp_rate_score,
      temp,
      temp_score,
      urineoutput,
      urineoutput_score,
      mechvent,
      mechvent_score,
      electivesurgery,
      electivesurgery_score
    FROM score
) AS oasis_
