-- mimic-code concepts_duckdb: lods, with its 16 upstream concepts inlined as CTEs
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
blood_differential AS (
    -- measurement/blood_differential.sql
    WITH blood_diff AS (
      SELECT
        MAX(subject_id) AS subject_id,
        MAX(hadm_id) AS hadm_id,
        MAX(charttime) AS charttime,
        le.specimen_id,
        MAX(CASE WHEN itemid IN (51300, 51301, 51755) THEN valuenum ELSE NULL END) AS wbc,
        MAX(CASE WHEN itemid = 52069 THEN valuenum ELSE NULL END) AS basophils_abs,
        MAX(
          CASE
            WHEN itemid = 52073
            THEN valuenum
            WHEN itemid = 51199
            THEN valuenum / 1000.0
            ELSE NULL
          END
        ) AS eosinophils_abs,
        MAX(
          CASE
            WHEN itemid = 51133
            THEN valuenum
            WHEN itemid = 52769
            THEN valuenum / 1000.0
            ELSE NULL
          END
        ) AS lymphocytes_abs,
        MAX(
          CASE
            WHEN itemid = 52074
            THEN valuenum
            WHEN itemid = 51253
            THEN valuenum / 1000.0
            ELSE NULL
          END
        ) AS monocytes_abs,
        MAX(CASE WHEN itemid = 52075 THEN valuenum ELSE NULL END) AS neutrophils_abs,
        MAX(CASE WHEN itemid = 51218 THEN valuenum / 1000.0 ELSE NULL END) AS granulocytes_abs,
        MAX(CASE WHEN itemid = 51146 THEN valuenum ELSE NULL END) AS basophils,
        MAX(CASE WHEN itemid = 51200 THEN valuenum ELSE NULL END) AS eosinophils,
        MAX(CASE WHEN itemid IN (51244, 51245) THEN valuenum ELSE NULL END) AS lymphocytes,
        MAX(CASE WHEN itemid = 51254 THEN valuenum ELSE NULL END) AS monocytes,
        MAX(CASE WHEN itemid = 51256 THEN valuenum ELSE NULL END) AS neutrophils,
        MAX(CASE WHEN itemid = 51143 THEN valuenum ELSE NULL END) AS atypical_lymphocytes,
        MAX(CASE WHEN itemid = 51144 THEN valuenum ELSE NULL END) AS bands,
        MAX(CASE WHEN itemid = 52135 THEN valuenum ELSE NULL END) AS immature_granulocytes,
        MAX(CASE WHEN itemid = 51251 THEN valuenum ELSE NULL END) AS metamyelocytes,
        MAX(CASE WHEN itemid = 51257 THEN valuenum ELSE NULL END) AS nrbc,
        CASE
          WHEN MAX(CASE WHEN itemid IN (51300, 51301, 51755) THEN valuenum ELSE NULL END) > 0
          AND SUM(
            CASE
              WHEN itemid IN (51146, 51200, 51244, 51245, 51254, 51256)
              THEN valuenum
              ELSE NULL
            END
          ) > 0
          THEN 1
          ELSE 0
        END AS impute_abs
      FROM mimiciv_hosp.labevents AS le
      WHERE
        le.itemid IN (
          51146,
          52069,
          51199,
          51200,
          52073,
          51244,
          51245,
          51133,
          52769,
          51253,
          51254,
          52074,
          51256,
          52075,
          51143,
          51144,
          51218,
          52135,
          51251,
          51257,
          51300,
          51301,
          51755
        )
        AND NOT valuenum IS NULL
        AND valuenum >= 0
      GROUP BY
        le.specimen_id
    )
    SELECT
      subject_id,
      hadm_id,
      charttime,
      specimen_id,
      wbc,
      ROUND(
        CAST(CASE
          WHEN basophils_abs IS NULL AND NOT basophils IS NULL AND impute_abs = 1
          THEN basophils * wbc / 100
          ELSE basophils_abs
        END AS DECIMAL(38, 9)),
        4
      ) AS basophils_abs,
      ROUND(
        CAST(CASE
          WHEN eosinophils_abs IS NULL AND NOT eosinophils IS NULL AND impute_abs = 1
          THEN eosinophils * wbc / 100
          ELSE eosinophils_abs
        END AS DECIMAL(38, 9)),
        4
      ) AS eosinophils_abs,
      ROUND(
        CAST(CASE
          WHEN lymphocytes_abs IS NULL AND NOT lymphocytes IS NULL AND impute_abs = 1
          THEN lymphocytes * wbc / 100
          ELSE lymphocytes_abs
        END AS DECIMAL(38, 9)),
        4
      ) AS lymphocytes_abs,
      ROUND(
        CAST(CASE
          WHEN monocytes_abs IS NULL AND NOT monocytes IS NULL AND impute_abs = 1
          THEN monocytes * wbc / 100
          ELSE monocytes_abs
        END AS DECIMAL(38, 9)),
        4
      ) AS monocytes_abs,
      ROUND(
        CAST(CASE
          WHEN neutrophils_abs IS NULL AND NOT neutrophils IS NULL AND impute_abs = 1
          THEN neutrophils * wbc / 100
          ELSE neutrophils_abs
        END AS DECIMAL(38, 9)),
        4
      ) AS neutrophils_abs,
      basophils,
      eosinophils,
      lymphocytes,
      monocytes,
      neutrophils,
      atypical_lymphocytes,
      bands,
      immature_granulocytes,
      metamyelocytes,
      nrbc
    FROM blood_diff
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
coagulation AS (
    -- measurement/coagulation.sql
    SELECT
      MAX(subject_id) AS subject_id,
      MAX(hadm_id) AS hadm_id,
      MAX(charttime) AS charttime,
      le.specimen_id,
      MAX(CASE WHEN itemid = 51196 THEN valuenum ELSE NULL END) AS d_dimer,
      MAX(CASE WHEN itemid = 51214 THEN valuenum ELSE NULL END) AS fibrinogen,
      MAX(CASE WHEN itemid = 51297 THEN valuenum ELSE NULL END) AS thrombin,
      MAX(CASE WHEN itemid = 51237 THEN valuenum ELSE NULL END) AS inr,
      MAX(CASE WHEN itemid = 51274 THEN valuenum ELSE NULL END) AS pt,
      MAX(CASE WHEN itemid = 51275 THEN valuenum ELSE NULL END) AS ptt
    FROM mimiciv_hosp.labevents AS le
    WHERE
      le.itemid IN (51196, 51214, 51297, 51237, 51274, 51275) AND NOT valuenum IS NULL
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
first_day_lab AS (
    -- firstday/first_day_lab.sql
    WITH cbc AS (
      SELECT
        ie.stay_id,
        MIN(hematocrit) AS hematocrit_min,
        MAX(hematocrit) AS hematocrit_max,
        MIN(hemoglobin) AS hemoglobin_min,
        MAX(hemoglobin) AS hemoglobin_max,
        MIN(platelet) AS platelets_min,
        MAX(platelet) AS platelets_max,
        MIN(wbc) AS wbc_min,
        MAX(wbc) AS wbc_max
      FROM mimiciv_icu.icustays AS ie
      LEFT JOIN complete_blood_count AS le
        ON le.subject_id = ie.subject_id
        AND le.charttime >= ie.intime - INTERVAL '6' HOUR
        AND le.charttime <= ie.intime + INTERVAL '1' DAY
      GROUP BY
        ie.stay_id
    ), chem AS (
      SELECT
        ie.stay_id,
        MIN(albumin) AS albumin_min,
        MAX(albumin) AS albumin_max,
        MIN(globulin) AS globulin_min,
        MAX(globulin) AS globulin_max,
        MIN(total_protein) AS total_protein_min,
        MAX(total_protein) AS total_protein_max,
        MIN(aniongap) AS aniongap_min,
        MAX(aniongap) AS aniongap_max,
        MIN(bicarbonate) AS bicarbonate_min,
        MAX(bicarbonate) AS bicarbonate_max,
        MIN(bun) AS bun_min,
        MAX(bun) AS bun_max,
        MIN(calcium) AS calcium_min,
        MAX(calcium) AS calcium_max,
        MIN(chloride) AS chloride_min,
        MAX(chloride) AS chloride_max,
        MIN(creatinine) AS creatinine_min,
        MAX(creatinine) AS creatinine_max,
        MIN(glucose) AS glucose_min,
        MAX(glucose) AS glucose_max,
        MIN(sodium) AS sodium_min,
        MAX(sodium) AS sodium_max,
        MIN(potassium) AS potassium_min,
        MAX(potassium) AS potassium_max
      FROM mimiciv_icu.icustays AS ie
      LEFT JOIN chemistry AS le
        ON le.subject_id = ie.subject_id
        AND le.charttime >= ie.intime - INTERVAL '6' HOUR
        AND le.charttime <= ie.intime + INTERVAL '1' DAY
      GROUP BY
        ie.stay_id
    ), diff AS (
      SELECT
        ie.stay_id,
        MIN(basophils_abs) AS abs_basophils_min,
        MAX(basophils_abs) AS abs_basophils_max,
        MIN(eosinophils_abs) AS abs_eosinophils_min,
        MAX(eosinophils_abs) AS abs_eosinophils_max,
        MIN(lymphocytes_abs) AS abs_lymphocytes_min,
        MAX(lymphocytes_abs) AS abs_lymphocytes_max,
        MIN(monocytes_abs) AS abs_monocytes_min,
        MAX(monocytes_abs) AS abs_monocytes_max,
        MIN(neutrophils_abs) AS abs_neutrophils_min,
        MAX(neutrophils_abs) AS abs_neutrophils_max,
        MIN(atypical_lymphocytes) AS atyps_min,
        MAX(atypical_lymphocytes) AS atyps_max,
        MIN(bands) AS bands_min,
        MAX(bands) AS bands_max,
        MIN(immature_granulocytes) AS imm_granulocytes_min,
        MAX(immature_granulocytes) AS imm_granulocytes_max,
        MIN(metamyelocytes) AS metas_min,
        MAX(metamyelocytes) AS metas_max,
        MIN(nrbc) AS nrbc_min,
        MAX(nrbc) AS nrbc_max
      FROM mimiciv_icu.icustays AS ie
      LEFT JOIN blood_differential AS le
        ON le.subject_id = ie.subject_id
        AND le.charttime >= ie.intime - INTERVAL '6' HOUR
        AND le.charttime <= ie.intime + INTERVAL '1' DAY
      GROUP BY
        ie.stay_id
    ), coag AS (
      SELECT
        ie.stay_id,
        MIN(d_dimer) AS d_dimer_min,
        MAX(d_dimer) AS d_dimer_max,
        MIN(fibrinogen) AS fibrinogen_min,
        MAX(fibrinogen) AS fibrinogen_max,
        MIN(thrombin) AS thrombin_min,
        MAX(thrombin) AS thrombin_max,
        MIN(inr) AS inr_min,
        MAX(inr) AS inr_max,
        MIN(pt) AS pt_min,
        MAX(pt) AS pt_max,
        MIN(ptt) AS ptt_min,
        MAX(ptt) AS ptt_max
      FROM mimiciv_icu.icustays AS ie
      LEFT JOIN coagulation AS le
        ON le.subject_id = ie.subject_id
        AND le.charttime >= ie.intime - INTERVAL '6' HOUR
        AND le.charttime <= ie.intime + INTERVAL '1' DAY
      GROUP BY
        ie.stay_id
    ), enz AS (
      SELECT
        ie.stay_id,
        MIN(alt) AS alt_min,
        MAX(alt) AS alt_max,
        MIN(alp) AS alp_min,
        MAX(alp) AS alp_max,
        MIN(ast) AS ast_min,
        MAX(ast) AS ast_max,
        MIN(amylase) AS amylase_min,
        MAX(amylase) AS amylase_max,
        MIN(bilirubin_total) AS bilirubin_total_min,
        MAX(bilirubin_total) AS bilirubin_total_max,
        MIN(bilirubin_direct) AS bilirubin_direct_min,
        MAX(bilirubin_direct) AS bilirubin_direct_max,
        MIN(bilirubin_indirect) AS bilirubin_indirect_min,
        MAX(bilirubin_indirect) AS bilirubin_indirect_max,
        MIN(ck_cpk) AS ck_cpk_min,
        MAX(ck_cpk) AS ck_cpk_max,
        MIN(ck_mb) AS ck_mb_min,
        MAX(ck_mb) AS ck_mb_max,
        MIN(ggt) AS ggt_min,
        MAX(ggt) AS ggt_max,
        MIN(ld_ldh) AS ld_ldh_min,
        MAX(ld_ldh) AS ld_ldh_max
      FROM mimiciv_icu.icustays AS ie
      LEFT JOIN enzyme AS le
        ON le.subject_id = ie.subject_id
        AND le.charttime >= ie.intime - INTERVAL '6' HOUR
        AND le.charttime <= ie.intime + INTERVAL '1' DAY
      GROUP BY
        ie.stay_id
    )
    SELECT
      ie.subject_id,
      ie.stay_id,
      hematocrit_min,
      hematocrit_max,
      hemoglobin_min,
      hemoglobin_max,
      platelets_min,
      platelets_max,
      wbc_min,
      wbc_max,
      albumin_min,
      albumin_max,
      globulin_min,
      globulin_max,
      total_protein_min,
      total_protein_max,
      aniongap_min,
      aniongap_max,
      bicarbonate_min,
      bicarbonate_max,
      bun_min,
      bun_max,
      calcium_min,
      calcium_max,
      chloride_min,
      chloride_max,
      creatinine_min,
      creatinine_max,
      glucose_min,
      glucose_max,
      sodium_min,
      sodium_max,
      potassium_min,
      potassium_max,
      abs_basophils_min,
      abs_basophils_max,
      abs_eosinophils_min,
      abs_eosinophils_max,
      abs_lymphocytes_min,
      abs_lymphocytes_max,
      abs_monocytes_min,
      abs_monocytes_max,
      abs_neutrophils_min,
      abs_neutrophils_max,
      atyps_min,
      atyps_max,
      bands_min,
      bands_max,
      imm_granulocytes_min,
      imm_granulocytes_max,
      metas_min,
      metas_max,
      nrbc_min,
      nrbc_max,
      d_dimer_min,
      d_dimer_max,
      fibrinogen_min,
      fibrinogen_max,
      thrombin_min,
      thrombin_max,
      inr_min,
      inr_max,
      pt_min,
      pt_max,
      ptt_min,
      ptt_max,
      alt_min,
      alt_max,
      alp_min,
      alp_max,
      ast_min,
      ast_max,
      amylase_min,
      amylase_max,
      bilirubin_total_min,
      bilirubin_total_max,
      bilirubin_direct_min,
      bilirubin_direct_max,
      bilirubin_indirect_min,
      bilirubin_indirect_max,
      ck_cpk_min,
      ck_cpk_max,
      ck_mb_min,
      ck_mb_max,
      ggt_min,
      ggt_max,
      ld_ldh_min,
      ld_ldh_max
    FROM mimiciv_icu.icustays AS ie
    LEFT JOIN cbc
      ON ie.stay_id = cbc.stay_id
    LEFT JOIN chem
      ON ie.stay_id = chem.stay_id
    LEFT JOIN diff
      ON ie.stay_id = diff.stay_id
    LEFT JOIN coag
      ON ie.stay_id = coag.stay_id
    LEFT JOIN enz
      ON ie.stay_id = enz.stay_id
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
-- score/lods.sql
SELECT * FROM (
    WITH cpap AS (
      SELECT
        ie.stay_id,
        MIN(charttime - INTERVAL '1' HOUR) AS starttime,
        MAX(charttime + INTERVAL '4' HOUR) AS endtime,
        MAX(
          CASE
            WHEN LOWER(ce.value) LIKE '%cpap%'
            THEN 1
            WHEN LOWER(ce.value) LIKE '%bipap mask%'
            THEN 1
            ELSE 0
          END
        ) AS cpap
      FROM mimiciv_icu.icustays AS ie
      INNER JOIN mimiciv_icu.chartevents AS ce
        ON ie.stay_id = ce.stay_id
        AND ce.charttime >= ie.intime
        AND ce.charttime <= ie.intime + INTERVAL '1' DAY
      WHERE
        itemid = 226732
        AND (
          LOWER(ce.value) LIKE '%cpap%' OR LOWER(ce.value) LIKE '%bipap mask%'
        )
      GROUP BY
        ie.stay_id
    ), pafi1 AS (
      SELECT
        ie.stay_id,
        bg.charttime,
        pao2fio2ratio,
        CASE WHEN NOT vd.stay_id IS NULL THEN 1 ELSE 0 END AS vent,
        CASE WHEN NOT cp.stay_id IS NULL THEN 1 ELSE 0 END AS cpap
      FROM bg AS bg
      INNER JOIN mimiciv_icu.icustays AS ie
        ON bg.hadm_id = ie.hadm_id
        AND bg.charttime >= ie.intime
        AND bg.charttime < ie.outtime
      LEFT JOIN ventilation AS vd
        ON ie.stay_id = vd.stay_id
        AND bg.charttime >= vd.starttime
        AND bg.charttime <= vd.endtime
        AND vd.ventilation_status = 'InvasiveVent'
      LEFT JOIN cpap AS cp
        ON ie.stay_id = cp.stay_id
        AND bg.charttime >= cp.starttime
        AND bg.charttime <= cp.endtime
    ), pafi2 AS (
      SELECT
        stay_id,
        MIN(pao2fio2ratio) AS pao2fio2_vent_min
      FROM pafi1
      WHERE
        vent = 1 OR cpap = 1
      GROUP BY
        stay_id
    ), cohort AS (
      SELECT
        ie.subject_id,
        ie.hadm_id,
        ie.stay_id,
        ie.intime,
        ie.outtime,
        gcs.gcs_min,
        vital.heart_rate_max,
        vital.heart_rate_min,
        vital.sbp_max,
        vital.sbp_min,
        pf.pao2fio2_vent_min,
        labs.bun_max,
        labs.bun_min,
        labs.wbc_max,
        labs.wbc_min,
        labs.bilirubin_total_max AS bilirubin_max,
        labs.creatinine_max,
        labs.pt_min,
        labs.pt_max,
        labs.platelets_min AS platelet_min,
        uo.urineoutput
      FROM mimiciv_icu.icustays AS ie
      INNER JOIN mimiciv_hosp.admissions AS adm
        ON ie.hadm_id = adm.hadm_id
      INNER JOIN mimiciv_hosp.patients AS pat
        ON ie.subject_id = pat.subject_id
      LEFT JOIN pafi2 AS pf
        ON ie.stay_id = pf.stay_id
      LEFT JOIN first_day_gcs AS gcs
        ON ie.stay_id = gcs.stay_id
      LEFT JOIN first_day_vitalsign AS vital
        ON ie.stay_id = vital.stay_id
      LEFT JOIN first_day_urine_output AS uo
        ON ie.stay_id = uo.stay_id
      LEFT JOIN first_day_lab AS labs
        ON ie.stay_id = labs.stay_id
    ), scorecomp AS (
      SELECT
        cohort.*,
        CASE
          WHEN gcs_min IS NULL
          THEN NULL
          WHEN gcs_min < 3
          THEN NULL
          WHEN gcs_min <= 5
          THEN 5
          WHEN gcs_min <= 8
          THEN 3
          WHEN gcs_min <= 13
          THEN 1
          ELSE 0
        END AS neurologic,
        CASE
          WHEN heart_rate_max IS NULL AND sbp_min IS NULL
          THEN NULL
          WHEN heart_rate_min < 30
          THEN 5
          WHEN sbp_min < 40
          THEN 5
          WHEN sbp_min < 70
          THEN 3
          WHEN sbp_max >= 270
          THEN 3
          WHEN heart_rate_max >= 140
          THEN 1
          WHEN sbp_max >= 240
          THEN 1
          WHEN sbp_min < 90
          THEN 1
          ELSE 0
        END AS cardiovascular,
        CASE
          WHEN bun_max IS NULL OR urineoutput IS NULL OR creatinine_max IS NULL
          THEN NULL
          WHEN urineoutput < 500.0
          THEN 5
          WHEN bun_max >= 56.0
          THEN 5
          WHEN creatinine_max >= 1.60
          THEN 3
          WHEN urineoutput < 750.0
          THEN 3
          WHEN bun_max >= 28.0
          THEN 3
          WHEN urineoutput >= 10000.0
          THEN 3
          WHEN creatinine_max >= 1.20
          THEN 1
          WHEN bun_max >= 17.0
          THEN 1
          WHEN bun_max >= 7.50
          THEN 1
          ELSE 0
        END AS renal,
        CASE
          WHEN pao2fio2_vent_min IS NULL
          THEN 0
          WHEN pao2fio2_vent_min >= 150
          THEN 1
          WHEN pao2fio2_vent_min < 150
          THEN 3
          ELSE NULL
        END AS pulmonary,
        CASE
          WHEN wbc_max IS NULL AND platelet_min IS NULL
          THEN NULL
          WHEN wbc_min < 1.0
          THEN 3
          WHEN wbc_min < 2.5
          THEN 1
          WHEN platelet_min < 50.0
          THEN 1
          WHEN wbc_max >= 50.0
          THEN 1
          ELSE 0
        END AS hematologic,
        CASE
          WHEN pt_max IS NULL AND bilirubin_max IS NULL
          THEN NULL
          WHEN bilirubin_max >= 2.0
          THEN 1
          WHEN pt_max > (
            12 + 3
          )
          THEN 1
          WHEN pt_min < (
            12 * 0.25
          )
          THEN 1
          ELSE 0
        END AS hepatic
      FROM cohort
    )
    SELECT
      ie.subject_id,
      ie.hadm_id,
      ie.stay_id,
      COALESCE(neurologic, 0) + COALESCE(cardiovascular, 0) + COALESCE(renal, 0) + COALESCE(pulmonary, 0) + COALESCE(hematologic, 0) + COALESCE(hepatic, 0) AS lods,
      neurologic,
      cardiovascular,
      renal,
      pulmonary,
      hematologic,
      hepatic
    FROM mimiciv_icu.icustays AS ie
    LEFT JOIN scorecomp AS s
      ON ie.stay_id = s.stay_id
) AS lods_
