use crate::schema::*;
use harness::prelude::*;

const CODESETS: [(i64, &[i64]); 18] = [
    (0, &[201254, 435216, 40484648]),
    (2, &[4297650, 4334806, 46273442, 2107572, 2107561, 4058299, 4107913, 2108721, 4102493, 80809, 2107560, 2107558, 2107559, 4083556, 4035611]),
    (3, &[75614, 140168]),
    (4, &[81931]),
    (5, &[374919]),
    (6, &[46273369, 4145240, 257628]),
    (7, &[443394]),
    (8, &[4232076]),
    (9, &[254443]),
    (10, &[135215]),
    (11, &[76685, 432893]),
    (12, &[4137275]),
    (13, &[432295]),
    (14, &[194992]),
    (15, &[46272236, 4331739, 441928, 255304, 4105026, 46270482, 134442, 4063582]),
    (16, &[438688, 4262578, 45772123, 4105005]),
    (17, &[46273477, 81893, 46273478, 46274082, 46269952, 4116142]),
    (18, &[201606, 46269889, 46269999]),
];

const BRANCHES: [(bool, i64); 21] = [
    (false, 0), (false, 2), (true, 2), (false, 3), (false, 4), (false, 5), (false, 6), (false, 7), (false, 8),
    (false, 9), (false, 10), (false, 11), (false, 12), (false, 13), (false, 14), (false, 15), (false, 16),
    (false, 17), (true, 17), (false, 18), (true, 18),
];

#[derive(Clone, Copy)]
pub struct C29 {
    pub cohort_definition_id: i64,
    pub subject_id: i64,
    pub cohort_start_date: Date,
    pub cohort_end_date: Date,
}

type Cs = (i64, i64);
type Ev = (i64, i64, Date, Date, Option<i64>);
type Pe = (Ev, (Date, Date));
type Qe = (i64, i64, Date, Date, Date, Date);
type Cr = (i64, Date, Date);

pub fn codesets(db: &'static Db) -> Vec<Cs> {
    let c = &db.concept;
    let ca = &db.concept_ancestor;
    let mut all: Vec<Cs> = Vec::new();
    for (k, ids) in CODESETS {
        let direct = (&c.id).with((&c.concept_id).is_in(ids.iter().copied())).select(&c.concept_id);
        let desc = (&ca.id)
            .with((&ca.ancestor_concept_id).is_in(ids.iter().copied()))
            .select(&ca.descendant_concept)
            .select((&c.concept_id).minus(&c.invalid_reason));
        let u = (&direct).inv().map(|_| ()).union((&desc).inv().map(|_| ())).fold((), |a, _| a);
        all.extend(drain(&u).into_iter().map(|(cid, _)| (k, cid)));
    }
    all
}

pub fn c29(db: &'static Db) -> Vec<C29> {
    let co = &db.condition_occurrence;
    let ob = &db.observation;
    let op = &db.observation_period;

    let cs = rel(codesets(db));
    let mut ev: Vec<Ev> = Vec::new();
    for (obs, k) in BRANCHES {
        let by: HashIdx<i64, usize> =
            (&cs).filt(move |(c, _): Cs| c == k).map(|(_, c): Cs| c).inv().collect();
        if obs {
            let b = (&ob.person_id)
                .and(&ob.observation_id)
                .and(&ob.observation_date)
                .and(&ob.visit_occurrence_id)
                .and((&ob.observation_concept_id).select(&by))
                .map(|((((p, e), s), v), _): ((((i64, i64), Date), i64), usize)| -> Ev { (p, e, s, s + DAY_US, Some(v)) });
            ev.extend(drain(&b).into_iter().map(|(_, e)| e));
        } else {
            let b = (&co.person_id)
                .and(&co.condition_occurrence_id)
                .and(&co.condition_start_date)
                .and((&co.condition_end_date).opt())
                .and((&co.visit_occurrence_id).opt())
                .and((&co.condition_concept_id).select(&by))
                .map(|(((((p, e), s), en), v), _): (((((i64, i64), Date), Option<Date>), Option<i64>), usize)| -> Ev {
                    (p, e, s, en.unwrap_or(s + DAY_US), v)
                });
            ev.extend(drain(&b).into_iter().map(|(_, e)| e));
        }
    }
    let er = rel(ev);

    let op_by: HashIdx<i64, _> = (&op.person_id).inv().collect();
    let pe = (&er)
        .and(
            (&er)
                .map(|e: Ev| e.0)
                .select(&op_by)
                .select((&op.observation_period_start_date).and(&op.observation_period_end_date)),
        )
        .filt(|((_, _, s, _, _), (a, b)): Pe| s >= a && s <= b && a <= s && s <= b);
    let pr = rel(drain(&pe).into_iter().map(|(_, x)| x).collect::<Vec<Pe>>());
    let pw = (&pr).map(|(e, _): Pe| e.0).inv().select(&pr).window(
        row_number,
        |((_, id, s, _, _), _): Pe| (s, id),
        |a: &(Date, i64), b: &(Date, i64)| a.cmp(b),
    );
    let qe: Vec<Qe> = drain((&pw).filt(|(_, rn): (Pe, i64)| rn == 1))
        .into_iter()
        .map(|(_, (((p, _, s, en, _), (a, b)), rn))| (rn, p, s, en, a, b))
        .collect();
    let qer = rel(qe);
    let qw = (&qer).map(|q: Qe| q.1).inv().select(&qer).window(row_number, |q: Qe| q.2, |a: &Date, b: &Date| a.cmp(b));
    let qr = rel(drain(&qw).into_iter().map(|(_, (q, _))| q).collect::<Vec<Qe>>());

    let inc_ev = rel(Vec::<(i64, i64, i64)>::new());
    let inc_by: HashIdx<(i64, i64), i64> = (&inc_ev)
        .map(|(_, p, e): (i64, i64, i64)| (p, e))
        .inv()
        .select((&inc_ev).map(|(r, _, _): (i64, i64, i64)| r))
        .collect();
    let mg = (&qr)
        .inv()
        .group_by(&qr)
        .select((&qr).map(|(e, p, ..): Qe| (p, e)).select((&inc_by).opt()))
        .fold(0i64, |m, r: Option<i64>| m + r.map_or(0, |r| 1i64 << r));
    let mr = rel(drain(&mg).into_iter().map(|(q, _)| q).collect::<Vec<Qe>>());
    let iw = (&mr).map(|q: Qe| q.1).inv().select(&mr).window(row_number, |q: Qe| q.2, |a: &Date, b: &Date| a.cmp(b));
    let ie = rel(drain((&iw).filt(|(_, rn): (Qe, i64)| rn == 1)).into_iter().map(|(_, (q, _))| q).collect::<Vec<Qe>>());

    let ce_by: HashIdx<(i64, i64), usize> = (&ie).map(|q: Qe| (q.0, q.1)).inv().collect();
    let fe = (&ie)
        .and((&ie).map(|q: Qe| (q.0, q.1)).select(&ce_by).select(&ie).map(|q: Qe| q.5))
        .filt(|(i, end): (Qe, Date)| end >= i.2);
    let fr = rel(drain(&fe).into_iter().map(|(_, x)| x).collect::<Vec<(Qe, Date)>>());
    let fw = (&fr).map(|(i, _): (Qe, Date)| (i.1, i.0)).inv().select(&fr).window(
        row_number,
        |(_, end): (Qe, Date)| end,
        |a: &Date, b: &Date| a.cmp(b),
    );
    let cr = rel(
        drain((&fw).filt(|(_, rn): ((Qe, Date), i64)| rn == 1))
            .into_iter()
            .map(|(_, ((i, end), _))| -> Cr { (i.1, i.2, end) })
            .collect::<Vec<Cr>>(),
    );

    let st = (&cr).map(|r: Cr| r.0).inv().select(&cr).window(
        |g: &[(Date, Cr)], out: &mut Vec<i64>| {
            let mut mx: Option<Date> = None;
            for &(s, (_, _, e)) in g {
                out.push(if mx.is_some_and(|m| m >= s) { 0 } else { 1 });
                mx = Some(mx.map_or(e, |m| m.max(e)));
            }
        },
        |r: Cr| r.1,
        |a: &Date, b: &Date| a.cmp(b),
    );
    let sr = rel(drain(&st).into_iter().map(|(_, x)| x).collect::<Vec<(Cr, i64)>>());
    let gr = (&sr).map(|(r, _): (Cr, i64)| r.0).inv().select(&sr).window(
        |g: &[((Date, i64), (Cr, i64))], out: &mut Vec<i64>| {
            let mut acc = 0;
            for &(_, (_, is)) in g {
                acc += is;
                out.push(acc);
            }
        },
        |((_, s, _), is): (Cr, i64)| (s, -is),
        |a: &(Date, i64), b: &(Date, i64)| a.cmp(b),
    );
    let grr = rel(drain(&gr).into_iter().map(|(_, ((r, _), g))| (r, g)).collect::<Vec<(Cr, i64)>>());
    let fc = (&grr)
        .inv()
        .group_by((&grr).map(|((p, _, _), g): (Cr, i64)| (p, g)))
        .select(&grr)
        .fold((i64::MAX, i64::MIN), |(lo, hi), ((_, s, e), _): (Cr, i64)| (lo.min(s), hi.max(e)));
    drain(&fc)
        .into_iter()
        .map(|((p, _), (s, e))| C29 { cohort_definition_id: 1, subject_id: p, cohort_start_date: s, cohort_end_date: e })
        .collect()
}

pub fn q(db: &'static Db) -> String {
    rows(c29(db).into_iter().map(|c| {
        row(vec![V::I(c.cohort_definition_id), V::I(c.subject_id), V::D(c.cohort_start_date), V::D(c.cohort_end_date)])
    }))
}
