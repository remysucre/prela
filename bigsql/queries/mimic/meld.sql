-- mimic-code concepts_duckdb: meld, with its 8 upstream concepts inlined as CTEs
WITH blood_differential AS (
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
rrt AS (
    -- treatment/rrt.sql
    WITH ce AS (
      SELECT
        ce.stay_id,
        ce.charttime,
        CASE
          WHEN ce.itemid IN (226118, 227357, 225725)
          THEN 1
          WHEN ce.itemid IN (
            226499,
            224154,
            225810,
            225959,
            227639,
            225183,
            227438,
            224191,
            225806,
            225807,
            228004,
            228005,
            228006,
            224144,
            224145,
            224149,
            224150,
            224151,
            224152,
            224153,
            224404,
            224406,
            226457
          )
          THEN 1
          WHEN ce.itemid IN (
            224135,
            224139,
            224146,
            225323,
            225740,
            225776,
            225951,
            225952,
            225953,
            225954,
            225956,
            225958,
            225961,
            225963,
            225965,
            225976,
            225977,
            227124,
            227290,
            227638,
            227640,
            227753
          )
          THEN 1
          ELSE 0
        END AS dialysis_present,
        CASE
          WHEN ce.itemid = 225965 AND value = 'In use'
          THEN 1
          WHEN ce.itemid IN (
            226499,
            224154,
            225183,
            227438,
            224191,
            225806,
            225807,
            228004,
            228005,
            228006,
            224144,
            224145,
            224153,
            226457
          )
          THEN 1
          ELSE 0
        END AS dialysis_active,
        CASE
          WHEN ce.itemid = 227290
          THEN value
          WHEN ce.itemid IN (
            225810,
            225806,
            225807,
            225810,
            227639,
            225959,
            225951,
            225952,
            225961,
            225953,
            225963,
            225965,
            227638,
            227640
          )
          THEN 'Peritoneal'
          WHEN ce.itemid = 226499
          THEN 'IHD'
          ELSE NULL
        END AS dialysis_type
      FROM mimiciv_icu.chartevents AS ce
      WHERE
        ce.itemid IN (
          226118,
          227357,
          225725,
          226499,
          224154,
          225810,
          227639,
          225183,
          227438,
          224191,
          225806,
          225807,
          228004,
          228005,
          228006,
          224144,
          224145,
          224149,
          224150,
          224151,
          224152,
          224153,
          224404,
          224406,
          226457,
          225959,
          224135,
          224139,
          224146,
          225323,
          225740,
          225776,
          225951,
          225952,
          225953,
          225954,
          225956,
          225958,
          225961,
          225963,
          225965,
          225976,
          225977,
          227124,
          227290,
          227638,
          227640,
          227753
        )
        AND NOT ce.value IS NULL
    ), mv_ranges AS (
      SELECT
        stay_id,
        starttime,
        endtime,
        1 AS dialysis_present,
        1 AS dialysis_active,
        'CRRT' AS dialysis_type
      FROM mimiciv_icu.inputevents
      WHERE
        itemid IN (227536, 227525) AND amount > 0
      UNION
      SELECT
        stay_id,
        starttime,
        endtime,
        1 AS dialysis_present,
        CASE WHEN NOT itemid IN (224270, 225436) THEN 1 ELSE 0 END AS dialysis_active,
        CASE
          WHEN itemid = 225441
          THEN 'IHD'
          WHEN itemid = 225802
          THEN 'CRRT'
          WHEN itemid = 225803
          THEN 'CVVHD'
          WHEN itemid = 225805
          THEN 'Peritoneal'
          WHEN itemid = 225809
          THEN 'CVVHDF'
          WHEN itemid = 225955
          THEN 'SCUF'
          ELSE NULL
        END AS dialysis_type
      FROM mimiciv_icu.procedureevents
      WHERE
        itemid IN (225441, 225802, 225803, 225805, 224270, 225809, 225955, 225436)
        AND NOT value IS NULL
    ), stg0 AS (
      SELECT
        stay_id,
        charttime,
        dialysis_present,
        dialysis_active,
        dialysis_type
      FROM ce
      WHERE
        dialysis_present = 1
      UNION
      SELECT
        stay_id,
        starttime AS charttime,
        dialysis_present,
        dialysis_active,
        dialysis_type
      FROM mv_ranges
    )
    SELECT
      stg0.stay_id,
      charttime,
      COALESCE(mv.dialysis_present, stg0.dialysis_present) AS dialysis_present,
      COALESCE(mv.dialysis_active, stg0.dialysis_active) AS dialysis_active,
      COALESCE(mv.dialysis_type, stg0.dialysis_type) AS dialysis_type
    FROM stg0
    LEFT JOIN mv_ranges AS mv
      ON stg0.stay_id = mv.stay_id
      AND stg0.charttime >= mv.starttime
      AND stg0.charttime <= mv.endtime
),
first_day_rrt AS (
    -- firstday/first_day_rrt.sql
    SELECT
      ie.subject_id,
      ie.stay_id,
      MAX(dialysis_present) AS dialysis_present,
      MAX(dialysis_active) AS dialysis_active,
      LISTAGG(DISTINCT dialysis_type, ', '
      ORDER BY
        dialysis_type NULLS FIRST) AS dialysis_type
    FROM mimiciv_icu.icustays AS ie
    LEFT JOIN rrt AS rrt
      ON ie.stay_id = rrt.stay_id
      AND rrt.charttime >= ie.intime - INTERVAL '6' HOUR
      AND rrt.charttime <= ie.intime + INTERVAL '1' DAY
    GROUP BY
      ie.subject_id,
      ie.stay_id
)
-- organfailure/meld.sql
SELECT * FROM (
    WITH cohort AS (
      SELECT
        ie.subject_id,
        ie.hadm_id,
        ie.stay_id,
        ie.intime,
        ie.outtime,
        labs.creatinine_max,
        labs.bilirubin_total_max,
        labs.inr_max,
        labs.sodium_min,
        r.dialysis_present AS rrt
      FROM mimiciv_icu.icustays AS ie
      LEFT JOIN first_day_lab AS labs
        ON ie.stay_id = labs.stay_id
      LEFT JOIN first_day_rrt AS r
        ON ie.stay_id = r.stay_id
    ), score AS (
      SELECT
        subject_id,
        hadm_id,
        stay_id,
        rrt,
        creatinine_max,
        bilirubin_total_max,
        inr_max,
        sodium_min,
        CASE
          WHEN sodium_min IS NULL
          THEN 0.0
          WHEN sodium_min > 137
          THEN 0.0
          WHEN sodium_min < 125
          THEN 12.0
          ELSE 137.0 - sodium_min
        END AS sodium_score,
        CASE
          WHEN rrt = 1 OR creatinine_max > 4.0
          THEN (
            0.957 * LN(4)
          )
          WHEN creatinine_max < 1
          THEN (
            0.957 * LN(1)
          )
          ELSE 0.957 * COALESCE(LN(creatinine_max), LN(1))
        END AS creatinine_score,
        CASE
          WHEN bilirubin_total_max < 1
          THEN 0.378 * LN(1)
          ELSE 0.378 * COALESCE(LN(bilirubin_total_max), LN(1))
        END AS bilirubin_score,
        CASE
          WHEN inr_max < 1
          THEN (
            1.120 * LN(1) + 0.643
          )
          ELSE (
            1.120 * COALESCE(LN(inr_max), LN(1)) + 0.643
          )
        END AS inr_score
      FROM cohort
    ), score2 AS (
      SELECT
        subject_id,
        hadm_id,
        stay_id,
        rrt,
        creatinine_max,
        bilirubin_total_max,
        inr_max,
        sodium_min,
        creatinine_score,
        sodium_score,
        bilirubin_score,
        inr_score,
        CASE
          WHEN (
            creatinine_score + bilirubin_score + inr_score
          ) > 4
          THEN 40.0
          ELSE ROUND(CAST(creatinine_score + bilirubin_score + inr_score AS DECIMAL(38, 9)), 1) * 10
        END AS meld_initial
      FROM score
    )
    SELECT
      subject_id,
      hadm_id,
      stay_id,
      meld_initial,
      CASE
        WHEN meld_initial > 11
        THEN meld_initial + 1.32 * sodium_score - 0.033 * meld_initial * sodium_score
        ELSE meld_initial
      END AS meld,
      rrt,
      creatinine_max,
      bilirubin_total_max,
      inr_max,
      sodium_min
    FROM score2
) AS meld_
