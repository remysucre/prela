use crate::concepts::encounters__patient_data_source_id::EncountersPatientDataSourceId;
use crate::concepts::normalized__medical_claim::NormalizedMedicalClaim;
use crate::schema::*;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct EncountersStgMedicalClaim {
    pub person_id: Str,
    pub patient_data_source_id: Str,
    pub claim_id: Str,
    pub claim_line_number: i64,
    pub claim_line_id: Str,
    pub claim_type: Str,
    pub start_date: Option<Date>,
    pub end_date: Option<Date>,
    pub admission_date: Option<Date>,
    pub discharge_date: Option<Date>,
    pub claim_start_date: Option<Date>,
    pub claim_end_date: Option<Date>,
    pub claim_line_start_date: Option<Date>,
    pub claim_line_end_date: Option<Date>,
    pub service_category_1: Str,
    pub service_category_2: Str,
    pub service_category_3: Str,
    pub bill_type_code: Option<Str>,
    pub bill_type_description: Option<Str>,
    pub hcpcs_code: Option<Str>,
    pub hcpcs_modifier_1: Option<Str>,
    pub hcpcs_modifier_2: Option<Str>,
    pub hcpcs_modifier_3: Option<Str>,
    pub hcpcs_modifier_4: Option<Str>,
    pub hcpcs_modifier_5: Option<Str>,
    pub ccs_category: Option<Str>,
    pub ccs_category_description: Option<Str>,
    pub drg_code_type: Str,
    pub drg_code: Option<Str>,
    pub drg_description: Option<Str>,
    pub admit_source_code: Option<Str>,
    pub admit_type_code: Option<Str>,
    pub place_of_service_code: Option<Str>,
    pub place_of_service_description: Option<Str>,
    pub revenue_center_code: Option<Str>,
    pub revenue_center_description: Option<Str>,
    pub diagnosis_code_type: Option<Str>,
    pub diagnosis_code_1: Option<Str>,
    pub primary_taxonomy_code: Option<Str>,
    pub primary_specialty_description: Option<Str>,
    pub modality: Option<Str>,
    pub billing_npi: Option<Str>,
    pub rendering_npi: Option<Str>,
    pub rend_primary_specialty_description: Option<Str>,
    pub facility_npi: Option<Str>,
    pub discharge_disposition_code: Option<Str>,
    pub paid_amount: Option<f64>,
    pub charge_amount: f64,
    pub allowed_amount: Option<f64>,
    pub data_source: Str,
    pub tuva_last_run: Ts,
}

fn leak(s: String) -> Str {
    Box::leak(s.into_boxed_str())
}

type M = NormalizedMedicalClaim;
type Ccs = ((((Str, Str), i64), Date), Date);

pub fn encounters__stg_medical_claim(
    db: &'static Db,
    nmc: &[M],
    pdsi: &[EncountersPatientDataSourceId],
) -> Vec<EncountersStgMedicalClaim> {
    let c = &db.ccs_services_procedures;
    let max_ry = (&c.release_year).inv().map(|_| ()).inv().fold(i64::MIN, |a, y: i64| a.max(y));
    let max_ry: Option<i64> = drain(&max_ry).into_iter().map(|(_, y)| y).next();

    let m = rel(nmc.to_vec());
    let d = rel(pdsi.to_vec());
    let dx: HashIdx<(Str, Str), usize> = (&d).map(|x: EncountersPatientDataSourceId| (x.person_id, x.data_source)).inv().collect();
    let cd = &db.int_claim_level_diagnosis;
    let cdx: HashIdx<(Str, Str), _> = (&cd.claim_id).and(&cd.data_source).inv().collect();
    let g = &db.service_category_grouper;
    let gx: HashIdx<((Str, i64), Str), _> = (&g.id)
        .with((&g.duplicate_row_number).filt(|x: i64| x == 1))
        .select((&g.claim_id).and(&g.claim_line_number).and(&g.data_source))
        .inv()
        .collect();
    let p = &db.provider;
    let px: HashIdx<Str, _> = (&p.npi).inv().collect();
    let rx: HashIdx<Str, _> = (&p.npi).inv().collect();
    let nx: HashIdx<Str, _> = (&db.nitos.hcpcs_code).inv().collect();
    let msx: HashIdx<Str, _> = (&db.ms_drg.ms_drg_code).inv().collect();
    let apx: HashIdx<Str, _> = (&db.apr_drg.apr_drg_code).inv().collect();
    let rcx: HashIdx<Str, _> = (&db.revenue_center.revenue_center_code).inv().collect();
    let posx: HashIdx<Str, _> = (&db.place_of_service.place_of_service_code).inv().collect();
    let btx: HashIdx<Str, _> = (&db.bill_type.bill_type_code).inv().collect();

    let j = (&m)
        .and((&m).map(|x: M| (x.claim_id, x.data_source)).select(&cdx).select((&cd.diagnosis_code_1).opt()).opt())
        .and(
            (&m).map(|x: M| ((x.claim_id, x.claim_line_number), x.data_source))
                .select(&gx)
                .select((&g.service_category_1).and(&g.service_category_2).and(&g.service_category_3)),
        )
        .and((&m).map(|x: M| (x.person_id, x.data_source)).select(&dx).select(&d))
        .and(
            (&m).flat_map(|x: M| x.facility_npi)
                .select(&px)
                .select((&p.primary_taxonomy_code).opt().and((&p.primary_specialty_description).opt()))
                .opt(),
        )
        .and((&m).flat_map(|x: M| x.hcpcs_code).select(&nx).select(&db.nitos.modality).opt())
        .and(
            (&m).filt(|x: M| x.drg_code_type == "ms-drg")
                .flat_map(|x: M| x.drg_code)
                .select(&msx)
                .select(&db.ms_drg.ms_drg_description)
                .opt(),
        )
        .and(
            (&m).filt(|x: M| x.drg_code_type == "apr-drg")
                .flat_map(|x: M| x.drg_code)
                .select(&apx)
                .select(&db.apr_drg.apr_drg_description)
                .opt(),
        )
        .and((&m).flat_map(|x: M| x.revenue_center_code).select(&rcx).select(&db.revenue_center.revenue_center_description).opt())
        .and((&m).flat_map(|x: M| x.place_of_service_code).select(&posx).select(&db.place_of_service.place_of_service_description).opt())
        .and((&m).flat_map(|x: M| x.bill_type_code).select(&btx).select(&db.bill_type.bill_type_description).opt())
        .and(
            (&m).flat_map(|x: M| x.rendering_npi)
                .select(&rx)
                .select((&p.primary_specialty_description).opt())
                .opt(),
        );
    let run = ts(2026, 1, 1, 0, 0, 0);
    let fin: Vec<EncountersStgMedicalClaim> = drain(&j)
        .into_iter()
        .map(|(_, (((((((((((m, cdx), ((s1, s2), s3)), d), fac), nit), ms), apr), rc), pos), bt), rend))| {
            EncountersStgMedicalClaim {
                person_id: m.person_id,
                patient_data_source_id: d.patient_data_source_id,
                claim_id: m.claim_id,
                claim_line_number: m.claim_line_number,
                claim_line_id: leak(format!("{}|{}", m.claim_id, m.claim_line_number)),
                claim_type: m.claim_type,
                start_date: m.admission_date.or(m.claim_line_start_date).or(m.claim_start_date),
                end_date: m.discharge_date.or(m.claim_line_end_date).or(m.claim_end_date),
                admission_date: m.admission_date,
                discharge_date: m.discharge_date,
                claim_start_date: m.claim_start_date,
                claim_end_date: m.claim_end_date,
                claim_line_start_date: m.claim_line_start_date,
                claim_line_end_date: m.claim_line_end_date,
                service_category_1: s1,
                service_category_2: s2,
                service_category_3: s3,
                bill_type_code: m.bill_type_code,
                bill_type_description: bt,
                hcpcs_code: m.hcpcs_code,
                hcpcs_modifier_1: m.hcpcs_modifier_1,
                hcpcs_modifier_2: m.hcpcs_modifier_2,
                hcpcs_modifier_3: m.hcpcs_modifier_3,
                hcpcs_modifier_4: m.hcpcs_modifier_4,
                hcpcs_modifier_5: m.hcpcs_modifier_5,
                ccs_category: None,
                ccs_category_description: None,
                drg_code_type: m.drg_code_type,
                drg_code: m.drg_code,
                drg_description: ms.or(apr),
                admit_source_code: m.admit_source_code,
                admit_type_code: m.admit_type_code,
                place_of_service_code: m.place_of_service_code,
                place_of_service_description: pos,
                revenue_center_code: m.revenue_center_code,
                revenue_center_description: rc,
                diagnosis_code_type: m.diagnosis_code_type,
                diagnosis_code_1: cdx.flatten().or(m.diagnosis_code_1),
                primary_taxonomy_code: fac.and_then(|x| x.0),
                primary_specialty_description: fac.and_then(|x| x.1),
                modality: nit,
                billing_npi: m.billing_npi,
                rendering_npi: m.rendering_npi,
                rend_primary_specialty_description: rend.flatten(),
                facility_npi: m.facility_npi,
                discharge_disposition_code: m.discharge_disposition_code,
                paid_amount: m.paid_amount,
                charge_amount: m.charge_amount,
                allowed_amount: m.allowed_amount,
                data_source: m.data_source,
                tuva_last_run: run,
            }
        })
        .collect();

    let f = rel(fin);
    let cx: HashIdx<Str, _> = (&c.hcpcs_code).inv().collect();
    let hit: HashIdx<usize, (Str, Str)> = (&f)
        .and(
            (&f).flat_map(|x: EncountersStgMedicalClaim| x.hcpcs_code)
                .select(&cx)
                .select((&c.ccs_category).and(&c.ccs_category_description).and(&c.release_year).and(&c.start_valid_date).and(&c.end_valid_date)),
        )
        .filt(move |(x, ((((_, _), ry), sv), ev)): (EncountersStgMedicalClaim, Ccs)| match x.start_date {
            Some(s) => (s >= sv && s <= ev) || max_ry.is_some_and(|my| year(s) > my && ry == my),
            None => false,
        })
        .map(|(_, ((((cat, desc), _), _), _)): (EncountersStgMedicalClaim, Ccs)| (cat, desc))
        .collect();
    let out = (&f).and((&hit).opt());
    drain(&out)
        .into_iter()
        .map(|(_, (x, cc))| EncountersStgMedicalClaim {
            ccs_category: cc.map(|c| c.0),
            ccs_category_description: cc.map(|c| c.1),
            ..x
        })
        .collect()
}

pub fn fmt(v: &EncountersStgMedicalClaim) -> String {
    row(vec![
        V::S(v.person_id),
        V::S(v.patient_data_source_id),
        V::S(v.claim_id),
        V::I(v.claim_line_number),
        V::S(v.claim_line_id),
        V::S(v.claim_type),
        odate(v.start_date),
        odate(v.end_date),
        odate(v.admission_date),
        odate(v.discharge_date),
        odate(v.claim_start_date),
        odate(v.claim_end_date),
        odate(v.claim_line_start_date),
        odate(v.claim_line_end_date),
        V::S(v.service_category_1),
        V::S(v.service_category_2),
        V::S(v.service_category_3),
        ostr(v.bill_type_code),
        ostr(v.bill_type_description),
        ostr(v.hcpcs_code),
        ostr(v.hcpcs_modifier_1),
        ostr(v.hcpcs_modifier_2),
        ostr(v.hcpcs_modifier_3),
        ostr(v.hcpcs_modifier_4),
        ostr(v.hcpcs_modifier_5),
        ostr(v.ccs_category),
        ostr(v.ccs_category_description),
        V::S(v.drg_code_type),
        ostr(v.drg_code),
        ostr(v.drg_description),
        ostr(v.admit_source_code),
        ostr(v.admit_type_code),
        ostr(v.place_of_service_code),
        ostr(v.place_of_service_description),
        ostr(v.revenue_center_code),
        ostr(v.revenue_center_description),
        ostr(v.diagnosis_code_type),
        ostr(v.diagnosis_code_1),
        ostr(v.primary_taxonomy_code),
        ostr(v.primary_specialty_description),
        ostr(v.modality),
        ostr(v.billing_npi),
        ostr(v.rendering_npi),
        ostr(v.rend_primary_specialty_description),
        ostr(v.facility_npi),
        ostr(v.discharge_disposition_code),
        ofloat(v.paid_amount),
        V::F(v.charge_amount),
        ofloat(v.allowed_amount),
        V::S(v.data_source),
        V::T(v.tuva_last_run),
    ])
}

pub fn q(db: &'static Db) -> String {
    let mc = crate::concepts::medical_claim::medical_claim(db);
    let il = crate::concepts::input_layer__medical_claim::input_layer__medical_claim(db, &mc);
    let stg = crate::concepts::normalized_input__stg_medical_claim::normalized_input__stg_medical_claim(db, &il);
    let nmc = crate::concepts::normalized__medical_claim::normalized__medical_claim_from_stg(db, &stg);
    let ne = crate::concepts::encounters__patient_data_source_id::normalized_eligibility(db);
    let pdsi = crate::concepts::encounters__patient_data_source_id::encounters__patient_data_source_id(db, &nmc, &ne);
    rows(encounters__stg_medical_claim(db, &nmc, &pdsi).iter().map(fmt))
}
