use crate::concepts::int_eligibility_casting::IntEligibilityCasting;
use crate::schema::Db;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct IntEligibilityStateNormalized {
    pub person_id: Str,
    pub person_id_key: Str,
    pub normalized_state_name: Option<Str>,
    pub fips_state_code: Option<Str>,
    pub fips_state_abbreviation: Option<Str>,
}

type Ansi = ((Str, Str), Str);

fn norm(s: &str) -> String {
    s.to_lowercase().trim_matches(' ').to_string()
}

pub fn int_eligibility_state_normalized(db: &'static Db, ec: &[IntEligibilityCasting]) -> Vec<IntEligibilityStateNormalized> {
    let a = &db.ansi_fips_state;
    let src = rel(ec.to_vec());
    let ansi = (&a.id).select((&a.ansi_fips_state_name).and(&a.ansi_fips_state_code).and(&a.ansi_fips_state_abbreviation));
    let hit: HashIdx<usize, Ansi> = (&src)
        .cross(&ansi)
        .filt(|(e, ((name, code), abbr)): (IntEligibilityCasting, Ansi)| {
            let s = norm(e.state);
            s == norm(abbr) || s == norm(code) || s == norm(name)
        })
        .map(|(_, r): (IntEligibilityCasting, Ansi)| r)
        .inv()
        .map(|(i, _)| i)
        .inv()
        .collect();
    let d = (&src)
        .and((&hit).opt())
        .map(|(e, m): (IntEligibilityCasting, Option<Ansi>)| {
            (e.person_id, e.person_id_key, m.map(|r| r.0.0), m.map(|r| r.0.1), m.map(|r| r.1))
        })
        .inv()
        .map(|_| ())
        .fold((), |a, _| a);
    drain(&d)
        .into_iter()
        .map(|((person_id, person_id_key, n, c, ab), _)| IntEligibilityStateNormalized {
            person_id,
            person_id_key,
            normalized_state_name: n,
            fips_state_code: c,
            fips_state_abbreviation: ab,
        })
        .collect()
}

pub fn q(db: &'static Db) -> String {
    let el = crate::concepts::eligibility::eligibility(db);
    let il = crate::concepts::input_layer__eligibility::input_layer__eligibility(db, &el);
    let ec = crate::concepts::int_eligibility_casting::int_eligibility_casting(db, &il);
    rows(int_eligibility_state_normalized(db, &ec).iter().map(|v| {
        row(vec![
            V::S(v.person_id),
            V::S(v.person_id_key),
            ostr(v.normalized_state_name),
            ostr(v.fips_state_code),
            ostr(v.fips_state_abbreviation),
        ])
    }))
}
