use crate::concepts::anchor_tmpl_b as k;
use crate::concepts::asc__anchor_events::AscAnchorEvents;
use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::schema::Db;
use harness::prelude::*;

type M = EncountersStgMedicalClaim;
type B = (Str, Str, Option<Date>, Option<Date>, Str);
type O = (bool, Option<Date>, Str);

#[derive(Clone, Copy)]
pub struct AscGenerateEncounterId {
    pub patient_data_source_id: Str,
    pub data_source: Str,
    pub start_date: Option<Date>,
    pub end_date: Option<Date>,
    pub claim_id: Str,
    pub old_encounter_id: Str,
}

pub fn asc__generate_encounter_id(_db: &'static Db, stg: &[M], anchors: &[AscAnchorEvents]) -> Vec<AscGenerateEncounterId> {
    let m = rel(stg.to_vec());
    let u = rel(k::anchor_pairs(anchors));
    let ux: HashIdx<(Str, Str), usize> = (&u).inv().collect();
    let bd = (&m)
        .and((&m).map(|x: M| (x.claim_id, x.data_source)).select(&ux))
        .map(|(x, _): (M, usize)| (x.patient_data_source_id, x.data_source, x.start_date, x.end_date, x.claim_id))
        .inv()
        .map(|_| ())
        .fold((), |a, _| a);
    let bd = rel(drain(&bd).into_iter().map(|(b, _)| b).collect::<Vec<B>>());
    let floor = date(1900, 1, 1);
    let nd = (&bd).map(|x: B| x.0).inv().select(&bd).window(
        move |g: &[(O, B)], out: &mut Vec<i64>| {
            let mut prev: Option<Date> = None;
            let mut grp = 0i64;
            for (_, b) in g {
                let flag = match b.2 {
                    Some(s) => s > prev.unwrap_or(floor),
                    None => false,
                };
                grp += flag as i64;
                out.push(grp);
                if let Some(e) = b.3 {
                    prev = Some(prev.map_or(e, |p| p.max(e)));
                }
            }
        },
        |x: B| (x.2.is_none(), x.2, x.4),
        |a: &O, b: &O| a.cmp(b),
    );
    let nd = rel(drain(&nd).into_iter().map(|(_, r)| r).collect::<Vec<(B, i64)>>());
    let ue = (&nd)
        .map(|(b, g): (B, i64)| (b.0, g))
        .inv()
        .select(&nd)
        .fold(None, |a: Option<Date>, (b, _): (B, i64)| match (a, b.2) {
            (Some(x), Some(y)) => Some(x.min(y)),
            (x, y) => x.or(y),
        });
    let ne = rel(
        drain(&ue)
            .into_iter()
            .map(|((p, g), sd)| (p, g, k::surrogate(&[Some("ambulatory surgery center"), Some(p), k::date_text(sd).as_deref()])))
            .collect::<Vec<(Str, i64, Str)>>(),
    );
    let nex: HashIdx<(Str, i64), usize> = (&ne).map(|(p, g, _): (Str, i64, Str)| (p, g)).inv().collect();
    let j = (&nd).and((&nd).map(|(b, g): (B, i64)| (b.0, g)).select(&nex).select(&ne));
    drain(&j)
        .into_iter()
        .map(|(_, ((b, _), (_, _, e)))| AscGenerateEncounterId {
            patient_data_source_id: b.0,
            data_source: b.1,
            start_date: b.2,
            end_date: b.3,
            claim_id: b.4,
            old_encounter_id: e,
        })
        .collect()
}

pub fn fmt(v: &AscGenerateEncounterId) -> String {
    row(vec![
        V::S(v.patient_data_source_id),
        V::S(v.data_source),
        odate(v.start_date),
        odate(v.end_date),
        V::S(v.claim_id),
        V::S(v.old_encounter_id),
    ])
}

pub fn q(db: &'static Db) -> String {
    let s = k::stg(db);
    let a = crate::concepts::asc__anchor_events::asc__anchor_events(db, &s);
    rows(asc__generate_encounter_id(db, &s, &a).iter().map(fmt))
}
