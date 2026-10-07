use crate::concepts::int_eligibility_casting::IntEligibilityCasting;
use crate::schema::Db;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct IntEligibilityDatesNormalized {
    pub person_id: Str,
    pub person_id_key: Str,
    pub normalized_birth_date: Date,
    pub normalized_death_date: Option<Date>,
    pub normalized_enrollment_start_date: Date,
    pub normalized_enrollment_end_date: Option<Date>,
}

type Row = (Str, Str, Date, Option<Date>, Date, Option<Date>);

pub fn int_eligibility_dates_normalized(_db: &'static Db, ec: &[IntEligibilityCasting]) -> Vec<IntEligibilityDatesNormalized> {
    let src = rel(ec.to_vec());
    let d = (&src)
        .map(|e: IntEligibilityCasting| -> Row {
            (e.person_id, e.person_id_key, e.birth_date, e.death_date, e.enrollment_start_date, e.enrollment_end_date)
        })
        .inv()
        .map(|_| ())
        .fold((), |a, _| a);
    drain(&d)
        .into_iter()
        .map(|((person_id, person_id_key, b, de, s, en), _)| IntEligibilityDatesNormalized {
            person_id,
            person_id_key,
            normalized_birth_date: b,
            normalized_death_date: de,
            normalized_enrollment_start_date: s,
            normalized_enrollment_end_date: en,
        })
        .collect()
}

pub fn q(db: &'static Db) -> String {
    let el = crate::concepts::eligibility::eligibility(db);
    let il = crate::concepts::input_layer__eligibility::input_layer__eligibility(db, &el);
    let ec = crate::concepts::int_eligibility_casting::int_eligibility_casting(db, &il);
    rows(int_eligibility_dates_normalized(db, &ec).iter().map(|v| {
        row(vec![
            V::S(v.person_id),
            V::S(v.person_id_key),
            V::D(v.normalized_birth_date),
            odate(v.normalized_death_date),
            V::D(v.normalized_enrollment_start_date),
            odate(v.normalized_enrollment_end_date),
        ])
    }))
}
