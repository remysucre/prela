-- mimic-code concepts_duckdb: sirs, with its 10 upstream concepts inlined as CTEs
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
first_day_bg_art AS (
    -- firstday/first_day_bg_art.sql
    SELECT
      ie.subject_id,
      ie.stay_id,
      MIN(lactate) AS lactate_min,
      MAX(lactate) AS lactate_max,
      MIN(ph) AS ph_min,
      MAX(ph) AS ph_max,
      MIN(so2) AS so2_min,
      MAX(so2) AS so2_max,
      MIN(po2) AS po2_min,
      MAX(po2) AS po2_max,
      MIN(pco2) AS pco2_min,
      MAX(pco2) AS pco2_max,
      MIN(aado2) AS aado2_min,
      MAX(aado2) AS aado2_max,
      MIN(aado2_calc) AS aado2_calc_min,
      MAX(aado2_calc) AS aado2_calc_max,
      MIN(pao2fio2ratio) AS pao2fio2ratio_min,
      MAX(pao2fio2ratio) AS pao2fio2ratio_max,
      MIN(baseexcess) AS baseexcess_min,
      MAX(baseexcess) AS baseexcess_max,
      MIN(bicarbonate) AS bicarbonate_min,
      MAX(bicarbonate) AS bicarbonate_max,
      MIN(totalco2) AS totalco2_min,
      MAX(totalco2) AS totalco2_max,
      MIN(hematocrit) AS hematocrit_min,
      MAX(hematocrit) AS hematocrit_max,
      MIN(hemoglobin) AS hemoglobin_min,
      MAX(hemoglobin) AS hemoglobin_max,
      MIN(carboxyhemoglobin) AS carboxyhemoglobin_min,
      MAX(carboxyhemoglobin) AS carboxyhemoglobin_max,
      MIN(methemoglobin) AS methemoglobin_min,
      MAX(methemoglobin) AS methemoglobin_max,
      MIN(temperature) AS temperature_min,
      MAX(temperature) AS temperature_max,
      MIN(chloride) AS chloride_min,
      MAX(chloride) AS chloride_max,
      MIN(calcium) AS calcium_min,
      MAX(calcium) AS calcium_max,
      MIN(glucose) AS glucose_min,
      MAX(glucose) AS glucose_max,
      MIN(potassium) AS potassium_min,
      MAX(potassium) AS potassium_max,
      MIN(sodium) AS sodium_min,
      MAX(sodium) AS sodium_max
    FROM mimiciv_icu.icustays AS ie
    LEFT JOIN bg AS bg
      ON ie.subject_id = bg.subject_id
      AND bg.specimen = 'ART.'
      AND bg.charttime >= ie.intime - INTERVAL '6' HOUR
      AND bg.charttime <= ie.intime + INTERVAL '1' DAY
    GROUP BY
      ie.subject_id,
      ie.stay_id
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
)
-- score/sirs.sql
SELECT * FROM (
    WITH scorecomp AS (
      SELECT
        ie.stay_id,
        v.temperature_min,
        v.temperature_max,
        v.heart_rate_max,
        v.resp_rate_max,
        bg.pco2_min AS paco2_min,
        l.wbc_min,
        l.wbc_max,
        l.bands_max
      FROM mimiciv_icu.icustays AS ie
      LEFT JOIN first_day_bg_art AS bg
        ON ie.stay_id = bg.stay_id
      LEFT JOIN first_day_vitalsign AS v
        ON ie.stay_id = v.stay_id
      LEFT JOIN first_day_lab AS l
        ON ie.stay_id = l.stay_id
    ), scorecalc AS (
      SELECT
        stay_id,
        CASE
          WHEN temperature_min < 36.0
          THEN 1
          WHEN temperature_max > 38.0
          THEN 1
          WHEN temperature_min IS NULL
          THEN NULL
          ELSE 0
        END AS temp_score,
        CASE
          WHEN heart_rate_max > 90.0
          THEN 1
          WHEN heart_rate_max IS NULL
          THEN NULL
          ELSE 0
        END AS heart_rate_score,
        CASE
          WHEN resp_rate_max > 20.0
          THEN 1
          WHEN paco2_min < 32.0
          THEN 1
          WHEN COALESCE(resp_rate_max, paco2_min) IS NULL
          THEN NULL
          ELSE 0
        END AS resp_score,
        CASE
          WHEN wbc_min < 4.0
          THEN 1
          WHEN wbc_max > 12.0
          THEN 1
          WHEN bands_max > 10
          THEN 1
          WHEN COALESCE(wbc_min, bands_max) IS NULL
          THEN NULL
          ELSE 0
        END AS wbc_score
      FROM scorecomp
    )
    SELECT
      ie.subject_id,
      ie.hadm_id,
      ie.stay_id,
      COALESCE(temp_score, 0) + COALESCE(heart_rate_score, 0) + COALESCE(resp_score, 0) + COALESCE(wbc_score, 0) AS sirs,
      temp_score,
      heart_rate_score,
      resp_score,
      wbc_score
    FROM mimiciv_icu.icustays AS ie
    LEFT JOIN scorecalc AS s
      ON ie.stay_id = s.stay_id
) AS sirs_
