use crate::concepts::{
    blood_differential::*, chemistry::*, coagulation::*, complete_blood_count::*, enzyme::*,
};
use crate::schema::*;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct FirstDayLab {
    pub subject_id: i64,
    pub stay_id: i64,
    pub hematocrit_min: Option<f64>,
    pub hematocrit_max: Option<f64>,
    pub hemoglobin_min: Option<f64>,
    pub hemoglobin_max: Option<f64>,
    pub platelets_min: Option<f64>,
    pub platelets_max: Option<f64>,
    pub wbc_min: Option<f64>,
    pub wbc_max: Option<f64>,
    pub albumin_min: Option<f64>,
    pub albumin_max: Option<f64>,
    pub globulin_min: Option<f64>,
    pub globulin_max: Option<f64>,
    pub total_protein_min: Option<f64>,
    pub total_protein_max: Option<f64>,
    pub aniongap_min: Option<f64>,
    pub aniongap_max: Option<f64>,
    pub bicarbonate_min: Option<f64>,
    pub bicarbonate_max: Option<f64>,
    pub bun_min: Option<f64>,
    pub bun_max: Option<f64>,
    pub calcium_min: Option<f64>,
    pub calcium_max: Option<f64>,
    pub chloride_min: Option<f64>,
    pub chloride_max: Option<f64>,
    pub creatinine_min: Option<f64>,
    pub creatinine_max: Option<f64>,
    pub glucose_min: Option<f64>,
    pub glucose_max: Option<f64>,
    pub sodium_min: Option<f64>,
    pub sodium_max: Option<f64>,
    pub potassium_min: Option<f64>,
    pub potassium_max: Option<f64>,
    pub abs_basophils_min: Option<f64>,
    pub abs_basophils_max: Option<f64>,
    pub abs_eosinophils_min: Option<f64>,
    pub abs_eosinophils_max: Option<f64>,
    pub abs_lymphocytes_min: Option<f64>,
    pub abs_lymphocytes_max: Option<f64>,
    pub abs_monocytes_min: Option<f64>,
    pub abs_monocytes_max: Option<f64>,
    pub abs_neutrophils_min: Option<f64>,
    pub abs_neutrophils_max: Option<f64>,
    pub atyps_min: Option<f64>,
    pub atyps_max: Option<f64>,
    pub bands_min: Option<f64>,
    pub bands_max: Option<f64>,
    pub imm_granulocytes_min: Option<f64>,
    pub imm_granulocytes_max: Option<f64>,
    pub metas_min: Option<f64>,
    pub metas_max: Option<f64>,
    pub nrbc_min: Option<f64>,
    pub nrbc_max: Option<f64>,
    pub d_dimer_min: Option<f64>,
    pub d_dimer_max: Option<f64>,
    pub fibrinogen_min: Option<f64>,
    pub fibrinogen_max: Option<f64>,
    pub thrombin_min: Option<f64>,
    pub thrombin_max: Option<f64>,
    pub inr_min: Option<f64>,
    pub inr_max: Option<f64>,
    pub pt_min: Option<f64>,
    pub pt_max: Option<f64>,
    pub ptt_min: Option<f64>,
    pub ptt_max: Option<f64>,
    pub alt_min: Option<f64>,
    pub alt_max: Option<f64>,
    pub alp_min: Option<f64>,
    pub alp_max: Option<f64>,
    pub ast_min: Option<f64>,
    pub ast_max: Option<f64>,
    pub amylase_min: Option<f64>,
    pub amylase_max: Option<f64>,
    pub bilirubin_total_min: Option<f64>,
    pub bilirubin_total_max: Option<f64>,
    pub bilirubin_direct_min: Option<f64>,
    pub bilirubin_direct_max: Option<f64>,
    pub bilirubin_indirect_min: Option<f64>,
    pub bilirubin_indirect_max: Option<f64>,
    pub ck_cpk_min: Option<f64>,
    pub ck_cpk_max: Option<f64>,
    pub ck_mb_min: Option<f64>,
    pub ck_mb_max: Option<f64>,
    pub ggt_min: Option<f64>,
    pub ggt_max: Option<f64>,
    pub ld_ldh_min: Option<f64>,
    pub ld_ldh_max: Option<f64>,
}

type MinMax = (Option<f64>, Option<f64>);

fn first_day<T: Copy, const N: usize>(
    db: &'static Db,
    v: &[T],
    subject: fn(&T) -> i64,
    time: fn(&T) -> Ts,
    vals: fn(&T) -> [Option<f64>; N],
) -> Fold<Id<IcuStay>, [MinMax; N]> {
    let icu = &db.icu_stay;
    let r = rel(v.to_vec());
    let by: HashIdx<i64, usize> = (&r).map(move |x: T| subject(&x)).inv().collect();
    (&icu.intime)
        .and((&icu.subject_id).select(&by).select(&r))
        .filt(move |(i, x): (Ts, T)| time(&x) >= i - 6 * 3600 * US && time(&x) <= i + DAY_US)
        .fold([(None, None); N], move |mut a, (_, x): (Ts, T)| {
            for (s, c) in a.iter_mut().zip(vals(&x)) {
                if let Some(c) = c {
                    s.0 = Some(s.0.map_or(c, |m: f64| m.min(c)));
                    s.1 = Some(s.1.map_or(c, |m: f64| m.max(c)));
                }
            }
            a
        })
}

pub fn first_day_lab(
    db: &'static Db,
    cbc: &[CompleteBloodCount],
    chem: &[Chemistry],
    diff: &[BloodDifferential],
    coag: &[Coagulation],
    enz: &[Enzyme],
) -> Vec<FirstDayLab> {
    let cbc_f = first_day(
        db,
        cbc,
        |x: &CompleteBloodCount| x.subject_id,
        |x: &CompleteBloodCount| x.charttime,
        |x: &CompleteBloodCount| [x.hematocrit, x.hemoglobin, x.platelet, x.wbc],
    );
    let chem_f = first_day(
        db,
        chem,
        |x: &Chemistry| x.subject_id,
        |x: &Chemistry| x.charttime,
        |x: &Chemistry| {
            [
                x.albumin,
                x.globulin,
                x.total_protein,
                x.aniongap,
                x.bicarbonate,
                x.bun,
                x.calcium,
                x.chloride,
                x.creatinine,
                x.glucose,
                x.sodium,
                x.potassium,
            ]
        },
    );
    let diff_f = first_day(
        db,
        diff,
        |x: &BloodDifferential| x.subject_id,
        |x: &BloodDifferential| x.charttime,
        |x: &BloodDifferential| {
            [
                x.basophils_abs,
                x.eosinophils_abs,
                x.lymphocytes_abs,
                x.monocytes_abs,
                x.neutrophils_abs,
                x.atypical_lymphocytes,
                x.bands,
                x.immature_granulocytes,
                x.metamyelocytes,
                x.nrbc,
            ]
        },
    );
    let coag_f = first_day(
        db,
        coag,
        |x: &Coagulation| x.subject_id,
        |x: &Coagulation| x.charttime,
        |x: &Coagulation| [x.d_dimer, x.fibrinogen, x.thrombin, x.inr, x.pt, x.ptt],
    );
    let enz_f = first_day(
        db,
        enz,
        |x: &Enzyme| x.subject_id,
        |x: &Enzyme| x.charttime,
        |x: &Enzyme| {
            [
                x.alt,
                x.alp,
                x.ast,
                x.amylase,
                x.bilirubin_total,
                x.bilirubin_direct,
                x.bilirubin_indirect,
                x.ck_cpk,
                x.ck_mb,
                x.ggt,
                x.ld_ldh,
            ]
        },
    );
    let icu = &db.icu_stay;
    let r = (&icu.subject_id)
        .and(&icu.stay_id)
        .and((&cbc_f).opt())
        .and((&chem_f).opt())
        .and((&diff_f).opt())
        .and((&coag_f).opt())
        .and((&enz_f).opt());
    drain(&r)
        .into_iter()
        .map(
            |(_, ((((((subject_id, stay_id), cbc), chem), diff), coag), enz))| {
                let cbc = cbc.unwrap_or([(None, None); 4]);
                let chem = chem.unwrap_or([(None, None); 12]);
                let diff = diff.unwrap_or([(None, None); 10]);
                let coag = coag.unwrap_or([(None, None); 6]);
                let enz = enz.unwrap_or([(None, None); 11]);
                FirstDayLab {
                    subject_id,
                    stay_id,
                    hematocrit_min: cbc[0].0,
                    hematocrit_max: cbc[0].1,
                    hemoglobin_min: cbc[1].0,
                    hemoglobin_max: cbc[1].1,
                    platelets_min: cbc[2].0,
                    platelets_max: cbc[2].1,
                    wbc_min: cbc[3].0,
                    wbc_max: cbc[3].1,
                    albumin_min: chem[0].0,
                    albumin_max: chem[0].1,
                    globulin_min: chem[1].0,
                    globulin_max: chem[1].1,
                    total_protein_min: chem[2].0,
                    total_protein_max: chem[2].1,
                    aniongap_min: chem[3].0,
                    aniongap_max: chem[3].1,
                    bicarbonate_min: chem[4].0,
                    bicarbonate_max: chem[4].1,
                    bun_min: chem[5].0,
                    bun_max: chem[5].1,
                    calcium_min: chem[6].0,
                    calcium_max: chem[6].1,
                    chloride_min: chem[7].0,
                    chloride_max: chem[7].1,
                    creatinine_min: chem[8].0,
                    creatinine_max: chem[8].1,
                    glucose_min: chem[9].0,
                    glucose_max: chem[9].1,
                    sodium_min: chem[10].0,
                    sodium_max: chem[10].1,
                    potassium_min: chem[11].0,
                    potassium_max: chem[11].1,
                    abs_basophils_min: diff[0].0,
                    abs_basophils_max: diff[0].1,
                    abs_eosinophils_min: diff[1].0,
                    abs_eosinophils_max: diff[1].1,
                    abs_lymphocytes_min: diff[2].0,
                    abs_lymphocytes_max: diff[2].1,
                    abs_monocytes_min: diff[3].0,
                    abs_monocytes_max: diff[3].1,
                    abs_neutrophils_min: diff[4].0,
                    abs_neutrophils_max: diff[4].1,
                    atyps_min: diff[5].0,
                    atyps_max: diff[5].1,
                    bands_min: diff[6].0,
                    bands_max: diff[6].1,
                    imm_granulocytes_min: diff[7].0,
                    imm_granulocytes_max: diff[7].1,
                    metas_min: diff[8].0,
                    metas_max: diff[8].1,
                    nrbc_min: diff[9].0,
                    nrbc_max: diff[9].1,
                    d_dimer_min: coag[0].0,
                    d_dimer_max: coag[0].1,
                    fibrinogen_min: coag[1].0,
                    fibrinogen_max: coag[1].1,
                    thrombin_min: coag[2].0,
                    thrombin_max: coag[2].1,
                    inr_min: coag[3].0,
                    inr_max: coag[3].1,
                    pt_min: coag[4].0,
                    pt_max: coag[4].1,
                    ptt_min: coag[5].0,
                    ptt_max: coag[5].1,
                    alt_min: enz[0].0,
                    alt_max: enz[0].1,
                    alp_min: enz[1].0,
                    alp_max: enz[1].1,
                    ast_min: enz[2].0,
                    ast_max: enz[2].1,
                    amylase_min: enz[3].0,
                    amylase_max: enz[3].1,
                    bilirubin_total_min: enz[4].0,
                    bilirubin_total_max: enz[4].1,
                    bilirubin_direct_min: enz[5].0,
                    bilirubin_direct_max: enz[5].1,
                    bilirubin_indirect_min: enz[6].0,
                    bilirubin_indirect_max: enz[6].1,
                    ck_cpk_min: enz[7].0,
                    ck_cpk_max: enz[7].1,
                    ck_mb_min: enz[8].0,
                    ck_mb_max: enz[8].1,
                    ggt_min: enz[9].0,
                    ggt_max: enz[9].1,
                    ld_ldh_min: enz[10].0,
                    ld_ldh_max: enz[10].1,
                }
            },
        )
        .collect()
}

pub fn q(db: &'static Db) -> String {
    let cbc = crate::concepts::complete_blood_count::complete_blood_count(db);
    let chem = crate::concepts::chemistry::chemistry(db);
    let diff = crate::concepts::blood_differential::blood_differential(db);
    let coag = crate::concepts::coagulation::coagulation(db);
    let enz = crate::concepts::enzyme::enzyme(db);
    rows(
        first_day_lab(db, &cbc, &chem, &diff, &coag, &enz)
            .into_iter()
            .map(|v| {
                row(vec![
                    V::I(v.subject_id),
                    V::I(v.stay_id),
                    ofloat(v.hematocrit_min),
                    ofloat(v.hematocrit_max),
                    ofloat(v.hemoglobin_min),
                    ofloat(v.hemoglobin_max),
                    ofloat(v.platelets_min),
                    ofloat(v.platelets_max),
                    ofloat(v.wbc_min),
                    ofloat(v.wbc_max),
                    ofloat(v.albumin_min),
                    ofloat(v.albumin_max),
                    ofloat(v.globulin_min),
                    ofloat(v.globulin_max),
                    ofloat(v.total_protein_min),
                    ofloat(v.total_protein_max),
                    ofloat(v.aniongap_min),
                    ofloat(v.aniongap_max),
                    ofloat(v.bicarbonate_min),
                    ofloat(v.bicarbonate_max),
                    ofloat(v.bun_min),
                    ofloat(v.bun_max),
                    ofloat(v.calcium_min),
                    ofloat(v.calcium_max),
                    ofloat(v.chloride_min),
                    ofloat(v.chloride_max),
                    ofloat(v.creatinine_min),
                    ofloat(v.creatinine_max),
                    ofloat(v.glucose_min),
                    ofloat(v.glucose_max),
                    ofloat(v.sodium_min),
                    ofloat(v.sodium_max),
                    ofloat(v.potassium_min),
                    ofloat(v.potassium_max),
                    ofloat(v.abs_basophils_min),
                    ofloat(v.abs_basophils_max),
                    ofloat(v.abs_eosinophils_min),
                    ofloat(v.abs_eosinophils_max),
                    ofloat(v.abs_lymphocytes_min),
                    ofloat(v.abs_lymphocytes_max),
                    ofloat(v.abs_monocytes_min),
                    ofloat(v.abs_monocytes_max),
                    ofloat(v.abs_neutrophils_min),
                    ofloat(v.abs_neutrophils_max),
                    ofloat(v.atyps_min),
                    ofloat(v.atyps_max),
                    ofloat(v.bands_min),
                    ofloat(v.bands_max),
                    ofloat(v.imm_granulocytes_min),
                    ofloat(v.imm_granulocytes_max),
                    ofloat(v.metas_min),
                    ofloat(v.metas_max),
                    ofloat(v.nrbc_min),
                    ofloat(v.nrbc_max),
                    ofloat(v.d_dimer_min),
                    ofloat(v.d_dimer_max),
                    ofloat(v.fibrinogen_min),
                    ofloat(v.fibrinogen_max),
                    ofloat(v.thrombin_min),
                    ofloat(v.thrombin_max),
                    ofloat(v.inr_min),
                    ofloat(v.inr_max),
                    ofloat(v.pt_min),
                    ofloat(v.pt_max),
                    ofloat(v.ptt_min),
                    ofloat(v.ptt_max),
                    ofloat(v.alt_min),
                    ofloat(v.alt_max),
                    ofloat(v.alp_min),
                    ofloat(v.alp_max),
                    ofloat(v.ast_min),
                    ofloat(v.ast_max),
                    ofloat(v.amylase_min),
                    ofloat(v.amylase_max),
                    ofloat(v.bilirubin_total_min),
                    ofloat(v.bilirubin_total_max),
                    ofloat(v.bilirubin_direct_min),
                    ofloat(v.bilirubin_direct_max),
                    ofloat(v.bilirubin_indirect_min),
                    ofloat(v.bilirubin_indirect_max),
                    ofloat(v.ck_cpk_min),
                    ofloat(v.ck_cpk_max),
                    ofloat(v.ck_mb_min),
                    ofloat(v.ck_mb_max),
                    ofloat(v.ggt_min),
                    ofloat(v.ggt_max),
                    ofloat(v.ld_ldh_min),
                    ofloat(v.ld_ldh_max),
                ])
            }),
    )
}
