use crate::schema::*;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct Antibiotic {
    pub subject_id: i64,
    pub hadm_id: i64,
    pub stay_id: Option<i64>,
    pub antibiotic: Str,
    pub route: Str,
    pub starttime: Ts,
    pub stoptime: Option<Ts>,
}

const ABX: [&str; 154] = [
    "%adoxa%",
    "%ala-tet%",
    "%alodox%",
    "%amikacin%",
    "%amikin%",
    "%amoxicill%",
    "%amphotericin%",
    "%anidulafungin%",
    "%ancef%",
    "%clavulanate%",
    "%ampicillin%",
    "%augmentin%",
    "%avelox%",
    "%avidoxy%",
    "%azactam%",
    "%azithromycin%",
    "%aztreonam%",
    "%axetil%",
    "%bactocill%",
    "%bactrim%",
    "%bactroban%",
    "%bethkis%",
    "%biaxin%",
    "%bicillin l-a%",
    "%cayston%",
    "%cefazolin%",
    "%cedax%",
    "%cefoxitin%",
    "%ceftazidime%",
    "%cefaclor%",
    "%cefadroxil%",
    "%cefdinir%",
    "%cefditoren%",
    "%cefepime%",
    "%cefotan%",
    "%cefotetan%",
    "%cefotaxime%",
    "%ceftaroline%",
    "%cefpodoxime%",
    "%cefpirome%",
    "%cefprozil%",
    "%ceftibuten%",
    "%ceftin%",
    "%ceftriaxone%",
    "%cefuroxime%",
    "%cephalexin%",
    "%cephalothin%",
    "%cephapririn%",
    "%chloramphenicol%",
    "%cipro%",
    "%ciprofloxacin%",
    "%claforan%",
    "%clarithromycin%",
    "%cleocin%",
    "%clindamycin%",
    "%cubicin%",
    "%dicloxacillin%",
    "%dirithromycin%",
    "%doryx%",
    "%doxycy%",
    "%duricef%",
    "%dynacin%",
    "%ery-tab%",
    "%eryped%",
    "%eryc%",
    "%erythrocin%",
    "%erythromycin%",
    "%factive%",
    "%flagyl%",
    "%fortaz%",
    "%furadantin%",
    "%garamycin%",
    "%gentamicin%",
    "%kanamycin%",
    "%keflex%",
    "%kefzol%",
    "%ketek%",
    "%levaquin%",
    "%levofloxacin%",
    "%lincocin%",
    "%linezolid%",
    "%macrobid%",
    "%macrodantin%",
    "%maxipime%",
    "%mefoxin%",
    "%metronidazole%",
    "%meropenem%",
    "%methicillin%",
    "%minocin%",
    "%minocycline%",
    "%monodox%",
    "%monurol%",
    "%morgidox%",
    "%moxatag%",
    "%moxifloxacin%",
    "%mupirocin%",
    "%myrac%",
    "%nafcillin%",
    "%neomycin%",
    "%nicazel doxy 30%",
    "%nitrofurantoin%",
    "%norfloxacin%",
    "%noroxin%",
    "%ocudox%",
    "%ofloxacin%",
    "%omnicef%",
    "%oracea%",
    "%oraxyl%",
    "%oxacillin%",
    "%pc pen vk%",
    "%pce dispertab%",
    "%panixine%",
    "%pediazole%",
    "%penicillin%",
    "%periostat%",
    "%pfizerpen%",
    "%piperacillin%",
    "%tazobactam%",
    "%primsol%",
    "%proquin%",
    "%raniclor%",
    "%rifadin%",
    "%rifampin%",
    "%rocephin%",
    "%smz-tmp%",
    "%septra%",
    "%septra ds%",
    "%septra%",
    "%solodyn%",
    "%spectracef%",
    "%streptomycin%",
    "%sulfadiazine%",
    "%sulfamethoxazole%",
    "%trimethoprim%",
    "%sulfatrim%",
    "%sulfisoxazole%",
    "%suprax%",
    "%synercid%",
    "%tazicef%",
    "%tetracycline%",
    "%timentin%",
    "%tobramycin%",
    "%trimethoprim%",
    "%unasyn%",
    "%vancocin%",
    "%vancomycin%",
    "%vantin%",
    "%vibativ%",
    "%vibra-tabs%",
    "%vibramycin%",
    "%zinacef%",
    "%zithromax%",
    "%zosyn%",
    "%zyvox%",
];

const NOT_ROUTES: [&str; 7] = ["OU", "OS", "OD", "AU", "AS", "AD", "TP"];

fn is_abx(drug: &str) -> bool {
    let d = drug.to_lowercase();
    ABX.iter().any(|p| like(&d, p))
}

pub fn antibiotic(db: &'static Db) -> Vec<Antibiotic> {
    let pr = &db.prescription;
    let icu = &db.icu_stay;
    let abx: HashIdx<(Str, Str), Id<Prescription>> = (&pr.id)
        .with((&pr.drug_type).filt(|t| t != "BASE"))
        .with((&pr.route).filt(|r: Str| {
            let l = r.to_lowercase();
            !NOT_ROUTES.contains(&r) && !like(&l, "%ear%") && !like(&l, "%eye%")
        }))
        .with((&pr.drug).filt(|d: Str| {
            let l = d.to_lowercase();
            !like(&l, "%cream%") && !like(&l, "%desensitization%") && !like(&l, "%ophth oint%") && !like(&l, "%gel%")
        }))
        .select((&pr.drug).and(&pr.route))
        .filt(|(d, _): (Str, Str)| is_abx(d))
        .inv()
        .collect();
    let by_adm: HashIdx<Id<Admission>, Id<IcuStay>> = (&icu.hadm).inv().collect();
    let stay: HashIdx<Id<Prescription>, i64> = (&pr.starttime)
        .and((&pr.hadm).select(&by_adm).select((&icu.intime).and(&icu.outtime).and(&icu.stay_id)))
        .filt(|(t, ((i, o), _)): (Ts, ((Ts, Ts), i64))| t >= i && t < o)
        .map(|(_, (_, s)): (Ts, ((Ts, Ts), i64))| s)
        .collect();
    let r = (&pr.drug)
        .and(&pr.route)
        .with(&abx)
        .and(
            (&pr.subject_id)
                .and(&pr.hadm_id)
                .and((&stay).opt())
                .and(&pr.starttime)
                .and((&pr.stoptime).opt()),
        );
    drain(&r)
        .into_iter()
        .map(|(_, ((antibiotic, route), ((((subject_id, hadm_id), stay_id), starttime), stoptime)))| Antibiotic {
            subject_id,
            hadm_id,
            stay_id,
            antibiotic,
            route,
            starttime,
            stoptime,
        })
        .collect()
}

pub fn q(db: &'static Db) -> String {
    rows(antibiotic(db).into_iter().map(|v| {
        row(vec![
            V::I(v.subject_id),
            V::I(v.hadm_id),
            oint(v.stay_id),
            V::S(v.antibiotic),
            V::S(v.route),
            V::T(v.starttime),
            ots(v.stoptime),
        ])
    }))
}
