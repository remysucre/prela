use crate::concepts::suspicion_of_infection::*;
use crate::schema::*;
use crate::queries::sofa::*;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct Sepsis3 {
    pub subject_id: i64,
    pub stay_id: i64,
    pub antibiotic_time: Ts,
    pub culture_time: Option<Ts>,
    pub suspected_infection_time: Option<Ts>,
    pub sofa_time: Ts,
    pub sofa_score: i64,
    pub respiration: i64,
    pub coagulation: i64,
    pub liver: i64,
    pub cardiovascular: i64,
    pub cns: i64,
    pub renal: i64,
    pub sepsis3: bool,
}

const HOUR_US: i64 = 3600 * US;

type S1 = (SuspicionOfInfection, Sofa);

pub fn sepsis3(_db: &'static Db, sofa: &[Sofa], soi: &[SuspicionOfInfection]) -> Vec<Sepsis3> {
    let sr = rel(sofa.to_vec());
    let by: HashIdx<Option<i64>, usize> =
        (&sr).filt(|s: Sofa| s.sofa_24hours >= 2).map(|s: Sofa| Some(s.stay_id)).inv().collect();
    let ir = rel(soi.to_vec());
    let j = (&ir)
        .and((&ir).map(|i: SuspicionOfInfection| i.stay_id).select(&by).select(&sr))
        .filt(|(i, s): S1| {
            i.suspected_infection_time
                .is_some_and(|t| s.endtime >= t - 48 * HOUR_US && s.endtime <= t + 24 * HOUR_US)
        });
    let s1 = rel(drain(&j).into_iter().map(|(_, p)| p).collect::<Vec<S1>>());
    let w = (&s1).map(|(i, _): S1| i.stay_id).inv().select(&s1).window(
        |s: &[(_, S1)], out: &mut Vec<i64>| out.extend(1..=s.len() as i64),
        |(i, s): S1| (i.suspected_infection_time, i.antibiotic_time, i.culture_time, s.endtime),
        |a, b| a.cmp(b),
    );
    let first = (&w).filt(|(_, rn): (S1, i64)| rn == 1);
    drain(&first)
        .into_iter()
        .map(|(_, ((i, s), _))| Sepsis3 {
            subject_id: i.subject_id,
            stay_id: s.stay_id,
            antibiotic_time: i.antibiotic_time,
            culture_time: i.culture_time,
            suspected_infection_time: i.suspected_infection_time,
            sofa_time: s.endtime,
            sofa_score: s.sofa_24hours,
            respiration: s.respiration_24hours,
            coagulation: s.coagulation_24hours,
            liver: s.liver_24hours,
            cardiovascular: s.cardiovascular_24hours,
            cns: s.cns_24hours,
            renal: s.renal_24hours,
            sepsis3: s.sofa_24hours >= 2 && i.suspected_infection == 1,
        })
        .collect()
}

pub fn q(db: &'static Db) -> String {
    use crate::concepts;
    let bg = concepts::bg::bg(db);
    let chem = concepts::chemistry::chemistry(db);
    let cbc = concepts::complete_blood_count::complete_blood_count(db);
    let dob = concepts::dobutamine::dobutamine(db);
    let dop = concepts::dopamine::dopamine(db);
    let enz = concepts::enzyme::enzyme(db);
    let epi = concepts::epinephrine::epinephrine(db);
    let gcs = concepts::gcs::gcs(db);
    let it = concepts::icustay_times::icustay_times(db);
    let ih = concepts::icustay_hourly::icustay_hourly(db, &it);
    let nor = concepts::norepinephrine::norepinephrine(db);
    let uo = concepts::urine_output::urine_output(db);
    let wd = concepts::weight_durations::weight_durations(db);
    let uor = concepts::urine_output_rate::urine_output_rate(db, &uo, &wd);
    let od = concepts::oxygen_delivery::oxygen_delivery(db);
    let vset = concepts::ventilator_setting::ventilator_setting(db);
    let vent = concepts::ventilation::ventilation(db, &vset, &od);
    let vs = concepts::vitalsign::vitalsign(db);
    let so = sofa(db, &ih, &bg, &vent, &vs, &gcs, &enz, &chem, &cbc, &uor, &epi, &nor, &dop, &dob);
    let abx = concepts::antibiotic::antibiotic(db);
    let soi = concepts::suspicion_of_infection::suspicion_of_infection(db, &abx);
    rows(sepsis3(db, &so, &soi).into_iter().map(|v| {
        row(vec![
            V::I(v.subject_id),
            V::I(v.stay_id),
            V::T(v.antibiotic_time),
            ots(v.culture_time),
            ots(v.suspected_infection_time),
            V::T(v.sofa_time),
            V::I(v.sofa_score),
            V::I(v.respiration),
            V::I(v.coagulation),
            V::I(v.liver),
            V::I(v.cardiovascular),
            V::I(v.cns),
            V::I(v.renal),
            V::B(v.sepsis3),
        ])
    }))
}
