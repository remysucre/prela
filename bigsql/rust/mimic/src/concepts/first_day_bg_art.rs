use crate::concepts::bg::*;
use crate::schema::*;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct FirstDayBgArt {
    pub subject_id: i64,
    pub stay_id: i64,
    pub lactate_min: Option<f64>,
    pub lactate_max: Option<f64>,
    pub ph_min: Option<f64>,
    pub ph_max: Option<f64>,
    pub so2_min: Option<f64>,
    pub so2_max: Option<f64>,
    pub po2_min: Option<f64>,
    pub po2_max: Option<f64>,
    pub pco2_min: Option<f64>,
    pub pco2_max: Option<f64>,
    pub aado2_min: Option<f64>,
    pub aado2_max: Option<f64>,
    pub aado2_calc_min: Option<f64>,
    pub aado2_calc_max: Option<f64>,
    pub pao2fio2ratio_min: Option<f64>,
    pub pao2fio2ratio_max: Option<f64>,
    pub baseexcess_min: Option<f64>,
    pub baseexcess_max: Option<f64>,
    pub bicarbonate_min: Option<f64>,
    pub bicarbonate_max: Option<f64>,
    pub totalco2_min: Option<f64>,
    pub totalco2_max: Option<f64>,
    pub hematocrit_min: Option<f64>,
    pub hematocrit_max: Option<f64>,
    pub hemoglobin_min: Option<f64>,
    pub hemoglobin_max: Option<f64>,
    pub carboxyhemoglobin_min: Option<f64>,
    pub carboxyhemoglobin_max: Option<f64>,
    pub methemoglobin_min: Option<f64>,
    pub methemoglobin_max: Option<f64>,
    pub temperature_min: Option<f64>,
    pub temperature_max: Option<f64>,
    pub chloride_min: Option<f64>,
    pub chloride_max: Option<f64>,
    pub calcium_min: Option<f64>,
    pub calcium_max: Option<f64>,
    pub glucose_min: Option<f64>,
    pub glucose_max: Option<f64>,
    pub potassium_min: Option<f64>,
    pub potassium_max: Option<f64>,
    pub sodium_min: Option<f64>,
    pub sodium_max: Option<f64>,
}

pub fn first_day_bg_art(db: &'static Db, bg: &[Bg]) -> Vec<FirstDayBgArt> {
    let icu = &db.icu_stay;
    let r = rel(bg.to_vec());
    let by: HashIdx<i64, usize> = (&r).map(|x: Bg| x.subject_id).inv().collect();
    let per = (&icu.intime)
        .and((&icu.subject_id).select(&by).select(&r))
        .filt(|(i, x): (Ts, Bg)| {
            x.specimen.is_some_and(|s| s == "ART.")
                && x.charttime >= i - 6 * 3600 * US
                && x.charttime <= i + DAY_US
        })
        .fold(
            [(None, None); 21],
            |mut a: [(Option<f64>, Option<f64>); 21], (_, x): (Ts, Bg)| {
                for (s, c) in a.iter_mut().zip([
                    x.lactate,
                    x.ph,
                    x.so2,
                    x.po2,
                    x.pco2,
                    x.aado2,
                    x.aado2_calc,
                    x.pao2fio2ratio,
                    x.baseexcess,
                    x.bicarbonate,
                    x.totalco2,
                    x.hematocrit,
                    x.hemoglobin,
                    x.carboxyhemoglobin,
                    x.methemoglobin,
                    x.temperature,
                    x.chloride,
                    x.calcium,
                    x.glucose,
                    x.potassium,
                    x.sodium,
                ]) {
                    if let Some(c) = c {
                        s.0 = Some(s.0.map_or(c, |m: f64| m.min(c)));
                        s.1 = Some(s.1.map_or(c, |m: f64| m.max(c)));
                    }
                }
                a
            },
        );
    let r = (&icu.subject_id).and(&icu.stay_id).and((&per).opt());
    drain(&r)
        .into_iter()
        .map(|(_, ((subject_id, stay_id), m))| {
            let m = m.unwrap_or([(None, None); 21]);
            FirstDayBgArt {
                subject_id,
                stay_id,
                lactate_min: m[0].0,
                lactate_max: m[0].1,
                ph_min: m[1].0,
                ph_max: m[1].1,
                so2_min: m[2].0,
                so2_max: m[2].1,
                po2_min: m[3].0,
                po2_max: m[3].1,
                pco2_min: m[4].0,
                pco2_max: m[4].1,
                aado2_min: m[5].0,
                aado2_max: m[5].1,
                aado2_calc_min: m[6].0,
                aado2_calc_max: m[6].1,
                pao2fio2ratio_min: m[7].0,
                pao2fio2ratio_max: m[7].1,
                baseexcess_min: m[8].0,
                baseexcess_max: m[8].1,
                bicarbonate_min: m[9].0,
                bicarbonate_max: m[9].1,
                totalco2_min: m[10].0,
                totalco2_max: m[10].1,
                hematocrit_min: m[11].0,
                hematocrit_max: m[11].1,
                hemoglobin_min: m[12].0,
                hemoglobin_max: m[12].1,
                carboxyhemoglobin_min: m[13].0,
                carboxyhemoglobin_max: m[13].1,
                methemoglobin_min: m[14].0,
                methemoglobin_max: m[14].1,
                temperature_min: m[15].0,
                temperature_max: m[15].1,
                chloride_min: m[16].0,
                chloride_max: m[16].1,
                calcium_min: m[17].0,
                calcium_max: m[17].1,
                glucose_min: m[18].0,
                glucose_max: m[18].1,
                potassium_min: m[19].0,
                potassium_max: m[19].1,
                sodium_min: m[20].0,
                sodium_max: m[20].1,
            }
        })
        .collect()
}

pub fn q(db: &'static Db) -> String {
    let bg = crate::concepts::bg::bg(db);
    rows(first_day_bg_art(db, &bg).into_iter().map(|v| {
        row(vec![
            V::I(v.subject_id),
            V::I(v.stay_id),
            ofloat(v.lactate_min),
            ofloat(v.lactate_max),
            ofloat(v.ph_min),
            ofloat(v.ph_max),
            ofloat(v.so2_min),
            ofloat(v.so2_max),
            ofloat(v.po2_min),
            ofloat(v.po2_max),
            ofloat(v.pco2_min),
            ofloat(v.pco2_max),
            ofloat(v.aado2_min),
            ofloat(v.aado2_max),
            ofloat(v.aado2_calc_min),
            ofloat(v.aado2_calc_max),
            ofloat(v.pao2fio2ratio_min),
            ofloat(v.pao2fio2ratio_max),
            ofloat(v.baseexcess_min),
            ofloat(v.baseexcess_max),
            ofloat(v.bicarbonate_min),
            ofloat(v.bicarbonate_max),
            ofloat(v.totalco2_min),
            ofloat(v.totalco2_max),
            ofloat(v.hematocrit_min),
            ofloat(v.hematocrit_max),
            ofloat(v.hemoglobin_min),
            ofloat(v.hemoglobin_max),
            ofloat(v.carboxyhemoglobin_min),
            ofloat(v.carboxyhemoglobin_max),
            ofloat(v.methemoglobin_min),
            ofloat(v.methemoglobin_max),
            ofloat(v.temperature_min),
            ofloat(v.temperature_max),
            ofloat(v.chloride_min),
            ofloat(v.chloride_max),
            ofloat(v.calcium_min),
            ofloat(v.calcium_max),
            ofloat(v.glucose_min),
            ofloat(v.glucose_max),
            ofloat(v.potassium_min),
            ofloat(v.potassium_max),
            ofloat(v.sodium_min),
            ofloat(v.sodium_max),
        ])
    }))
}
