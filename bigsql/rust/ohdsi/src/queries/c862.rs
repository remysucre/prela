use crate::schema::*;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct C862 {
    pub cohort_definition_id: i64,
    pub subject_id: i64,
    pub cohort_start_date: Date,
    pub cohort_end_date: Date,
}

#[derive(Clone, Copy)]
struct Ev {
    person_id: i64,
    event_id: i64,
    start: Date,
    end: Date,
    visit: Option<i64>,
}

#[derive(Clone, Copy)]
struct Eop {
    e: Ev,
    op_start: Date,
    op_end: Date,
}

type K = (i64, i64);

fn key(x: Eop) -> K {
    (x.e.person_id, x.e.event_id)
}

fn concept_set(db: &'static Db, ids: &[i64], desc: &[i64]) -> Fold<i64, ()> {
    let c = &db.concept;
    let ca = &db.concept_ancestor;
    let direct = (&c.concept_id).is_in(ids.to_vec()).inv().map(|_| ());
    let d = (&ca.id)
        .with((&ca.ancestor_concept_id).is_in(desc.to_vec()))
        .select(&ca.descendant_concept)
        .select((&c.concept_id).minus(&c.invalid_reason))
        .inv()
        .map(|_| ());
    if desc.is_empty() {
        direct.fold((), |a, _| a)
    } else {
        direct.union(d).fold((), |a, _| a)
    }
}

fn codeset(db: &'static Db, id: i64, inc: (&[i64], &[i64]), exc: Option<(&[i64], &[i64])>) -> HashIdx<i64, i64> {
    let i = concept_set(db, inc.0, inc.1);
    match exc {
        None => (&i).map(move |_| id).collect(),
        Some((e0, e1)) => {
            let e = concept_set(db, e0, e1);
            (&i).minus(&e).map(move |_| id).collect()
        }
    }
}

fn codesets(db: &'static Db) -> Vec<(i64, i64)> {
    let [p0, p2, p3, p6, p8, p10, p11, p12, p13] = [
        codeset(db, 0, (&[378419], &[378419]), None),
        codeset(db, 2, (&[4182210], &[4182210]), Some((&[378419, 4182210, 43530666], &[]))),
        codeset(
            db,
            3,
            (&[715997, 757627, 701322, 733523, 836654], &[715997, 757627, 701322, 733523, 836654]),
            None,
        ),
        codeset(db, 6, (&[443605, 44782710, 380701, 4043378], &[443605, 44782710, 380701, 4043378]), None),
        codeset(
            db,
            8,
            (
                &[372924, 375557, 376713, 443454, 441874, 439847, 43530727, 42538062, 4148906, 432923],
                &[376713, 443454, 439847],
            ),
            Some((&[379778], &[379778])),
        ),
        codeset(db, 10, (&[4140090], &[4140090]), None),
        codeset(
            db,
            11,
            (
                &[789578, 720810, 713823, 715710, 786426, 837027, 782211],
                &[789578, 720810, 713823, 715710, 786426, 837027, 782211],
            ),
            None,
        ),
        codeset(db, 12, (&[43530666, 4182210], &[]), None),
        codeset(db, 13, (&[4182210], &[4182210]), Some((&[378419, 4182210], &[378419]))),
    ];
    let u = (&p0).union(&p2).union(&p3).union(&p6).union(&p8).union(&p10).union(&p11).union(&p12).union(&p13);
    drain(u).into_iter().map(|(c, id)| (id, c)).collect()
}

fn cs(csr: &VecRel<usize, (i64, i64)>, k: i64) -> MatSet<i64> {
    csr.filt(move |(c, _): (i64, i64)| c == k).map(|(_, x): (i64, i64)| x).collect()
}

fn conds(db: &'static Db, s: &MatSet<i64>) -> VecRel<usize, Ev> {
    let co = &db.condition_occurrence;
    let q = (&co.condition_concept_id).with(s).and(
        (&co.person_id)
            .and(&co.condition_occurrence_id)
            .and(&co.condition_start_date)
            .and((&co.condition_end_date).opt())
            .and((&co.visit_occurrence_id).opt()),
    );
    rel(drain(&q)
        .into_iter()
        .map(|(_, (_, ((((person_id, event_id), start), end), visit)))| Ev {
            person_id,
            event_id,
            start,
            end: end.unwrap_or(start + DAY_US),
            visit,
        })
        .collect())
}

fn drugs(db: &'static Db, s: &MatSet<i64>) -> VecRel<usize, Ev> {
    let de = &db.drug_exposure;
    let q = (&de.drug_concept_id).with(s).and(
        (&de.person_id)
            .and(&de.drug_exposure_id)
            .and(&de.drug_exposure_start_date)
            .and(&de.drug_exposure_end_date)
            .and((&de.visit_occurrence_id).opt()),
    );
    rel(drain(&q)
        .into_iter()
        .map(|(_, (_, ((((person_id, event_id), start), end), visit)))| Ev { person_id, event_id, start, end, visit })
        .collect())
}

fn with_op(db: &'static Db, er: &VecRel<usize, Ev>) -> VecRel<usize, Eop> {
    let op = &db.observation_period;
    let op_by: HashIdx<i64, Id<ObservationPeriod>> = (&op.person_id).inv().collect();
    let q = er
        .and(
            er.map(|e: Ev| e.person_id)
                .select(&op_by)
                .select((&op.observation_period_start_date).and(&op.observation_period_end_date)),
        )
        .filt(|(e, (s, t)): (Ev, (Date, Date))| s <= e.start && t >= e.start);
    rel(drain(&q).into_iter().map(|(_, (e, (op_start, op_end)))| Eop { e, op_start, op_end }).collect())
}

fn corr(p: &VecRel<usize, Eop>, a: &VecRel<usize, Ev>, lo: impl Fn(Eop) -> Date, hi: impl Fn(Eop) -> Date) -> Fold<K, i64> {
    let a_by: HashIdx<i64, usize> = a.map(|e: Ev| e.person_id).inv().collect();
    p.and(p.map(|x: Eop| x.e.person_id).select(&a_by).select(a))
        .filt(|(x, y): (Eop, Ev)| y.start >= x.op_start && y.start <= x.op_end && y.start >= lo(x) && y.start <= hi(x))
        .map(|(x, _): (Eop, Ev)| key(x))
        .inv()
        .fold(0i64, |c, _| c + 1)
}

type Csr = VecRel<usize, (i64, i64)>;

fn d3op(db: &'static Db, csr: &Csr) -> VecRel<usize, Eop> {
    with_op(db, &drugs(db, &cs(csr, 3)))
}

fn at_least(db: &'static Db, csr: &Csr, a: &VecRel<usize, Ev>, lo: impl Fn(Eop) -> Date, hi: impl Fn(Eop) -> Date, idx: i64) -> Fold<K, i64> {
    let pp = d3op(db, csr);
    let c = corr(&pp, a, lo, hi);
    c.filt(|n: i64| n >= 1).fold(idx, |a, _| a)
}

fn none(db: &'static Db, csr: &Csr, a: &VecRel<usize, Ev>, lo: impl Fn(Eop) -> Date, hi: impl Fn(Eop) -> Date, idx: i64) -> Fold<K, i64> {
    let p = d3op(db, csr);
    let pp = d3op(db, csr);
    let cc = corr(&pp, a, lo, hi);
    let n = (&p).map(key).inv().select((&p).map(key).select(&cc).opt()).fold(0i64, |s, x: Option<i64>| s + x.unwrap_or(0));
    (&n).filt(|c: i64| c == 0).fold(idx, |a, _| a)
}

fn group(db: &'static Db, csr: &Csr, cq: &HashIdx<K, i64>, keep: impl Fn(i64) -> bool) -> Fold<K, i64> {
    let e = d3op(db, csr);
    let ek: HashIdx<K, usize> = (&e).map(key).inv().collect();
    let g = (&ek).and(cq).fold(0i64, |c, _| c + 1);
    (&g).filt(keep).fold(0i64, |a, _| a)
}

pub fn c862(db: &'static Db) -> Vec<C862> {
    let csr = rel(codesets(db));

    let k0 = at_least(db, &csr, &conds(db, &cs(&csr, 0)), |x| x.e.start + DAY_US, |x| x.e.start + 365 * DAY_US, 0);

    let k10 = none(db, &csr, &conds(db, &cs(&csr, 13)), |x| x.op_start, |x| x.e.start + 7 * DAY_US, 0);
    let k11 = none(db, &csr, &conds(db, &cs(&csr, 6)), |x| x.op_start, |x| x.e.start + 7 * DAY_US, 1);
    let k12 = none(db, &csr, &conds(db, &cs(&csr, 10)), |x| x.e.start - 365 * DAY_US, |x| x.e.start + 7 * DAY_US, 2);
    let k13 = none(db, &csr, &conds(db, &cs(&csr, 8)), |x| x.e.start - 365 * DAY_US, |x| x.e.start + 7 * DAY_US, 3);
    let k14 = none(db, &csr, &drugs(db, &cs(&csr, 11)), |x| x.e.start - 365 * DAY_US, |x| x.e.start + 7 * DAY_US, 4);
    let cq1: HashIdx<K, i64> = (&k10).union(&k11).union(&k12).union(&k13).union(&k14).collect();
    let g1 = group(db, &csr, &cq1, |c| c == 5);

    let k20 = at_least(db, &csr, &conds(db, &cs(&csr, 12)), |x| x.e.start - 365 * DAY_US, |x| x.e.start + 7 * DAY_US, 0);
    let k21 = none(db, &csr, &conds(db, &cs(&csr, 2)), |x| x.op_start, |x| x.e.start + 7 * DAY_US, 1);
    let k22 = none(db, &csr, &conds(db, &cs(&csr, 6)), |x| x.op_start, |x| x.e.start + 7 * DAY_US, 2);
    let k23 = none(db, &csr, &conds(db, &cs(&csr, 8)), |x| x.op_start, |x| x.e.start, 3);
    let k24 = none(db, &csr, &conds(db, &cs(&csr, 10)), |x| x.op_start, |x| x.e.start + 7 * DAY_US, 4);
    let k25 = none(db, &csr, &drugs(db, &cs(&csr, 11)), |x| x.e.start - 365 * DAY_US, |x| x.e.start + 7 * DAY_US, 5);
    let cq2: HashIdx<K, i64> = (&k20).union(&k21).union(&k22).union(&k23).union(&k24).union(&k25).collect();
    let g2 = group(db, &csr, &cq2, |c| c == 6);

    let cq: HashIdx<K, i64> =
        (&k0).map(|_| 0i64).union((&g1).map(|_| 1i64)).union((&g2).map(|_| 2i64)).collect();
    let acr = group(db, &csr, &cq, |c| c > 0);

    let c0 = conds(db, &cs(&csr, 0));
    let d3 = drugs(db, &cs(&csr, 3));
    let pe = (&d3)
        .and((&d3).map(|e: Ev| (e.person_id, e.event_id)).select(&acr))
        .map(|(e, _): (Ev, i64)| e);
    let prim = rel(drain((&c0).union(pe)).into_iter().map(|(_, e)| e).collect::<Vec<Ev>>());
    let po0 = with_op(db, &prim);
    let po = rel(
        drain((&po0).filt(|x: Eop| x.op_start <= x.e.start && x.e.start <= x.op_end))
            .into_iter()
            .map(|(_, x)| x)
            .collect::<Vec<Eop>>(),
    );
    let w1 = (&po).map(|x: Eop| x.e.person_id).inv().select(&po).window(
        row_number,
        |x: Eop| (x.e.start, x.e.event_id),
        |a: &(Date, i64), b: &(Date, i64)| a.cmp(b),
    );
    let pe1: Vec<Eop> = drain((&w1).filt(|(_, rn): (Eop, i64)| rn == 1))
        .into_iter()
        .map(|(_, (x, rn))| Eop { e: Ev { event_id: rn, ..x.e }, ..x })
        .collect();
    let pe1 = rel(pe1);
    let w2 = (&pe1).map(|x: Eop| x.e.person_id).inv().select(&pe1).window(
        row_number,
        |x: Eop| x.e.start,
        |a: &Date, b: &Date| a.cmp(b),
    );
    let qe = rel(drain(&w2).into_iter().map(|(_, (x, _))| x).collect::<Vec<Eop>>());

    let incr = rel(Vec::<(i64, i64, i64)>::new());
    let inc: HashIdx<K, i64> =
        (&incr).map(|(_, p, e): (i64, i64, i64)| (p, e)).inv().select(&incr).map(|(r, _, _): (i64, i64, i64)| r).collect();
    let mg = (&qe)
        .and((&qe).map(key).select((&inc).opt()))
        .fold((None::<Eop>, 0i64), |(_, m), (x, r): (Eop, Option<i64>)| (Some(x), m + r.map(|i| 1i64 << i).unwrap_or(0)));
    let mgv = rel(drain(&mg).into_iter().map(|(_, (x, _))| x.unwrap()).collect::<Vec<Eop>>());
    let w3 = (&mgv).map(|x: Eop| x.e.person_id).inv().select(&mgv).window(
        row_number,
        |x: Eop| x.e.start,
        |a: &Date, b: &Date| a.cmp(b),
    );
    let ie = rel(drain((&w3).filt(|(_, rn): (Eop, i64)| rn == 1)).into_iter().map(|(_, (x, _))| x).collect::<Vec<Eop>>());

    let ce_by: HashIdx<K, usize> = (&ie).map(key).inv().collect();
    let f = (&ie)
        .and((&ie).map(key).select(&ce_by).select(&ie).map(|c: Eop| c.op_end))
        .filt(|(i, end): (Eop, Date)| end >= i.e.start)
        .map(|(i, end): (Eop, Date)| (i, end));
    let fr = rel(drain(&f).into_iter().map(|(_, x)| x).collect::<Vec<(Eop, Date)>>());
    let w4 = (&fr).map(|(i, _): (Eop, Date)| key(i)).inv().select(&fr).window(
        row_number,
        |(_, end): (Eop, Date)| end,
        |a: &Date, b: &Date| a.cmp(b),
    );
    let cr = rel(
        drain((&w4).filt(|(_, rn): ((Eop, Date), i64)| rn == 1))
            .into_iter()
            .map(|(_, ((i, end), _))| (i.e.person_id, i.e.start, end))
            .collect::<Vec<(i64, Date, Date)>>(),
    );

    let w5 = (&cr).map(|(p, _, _): (i64, Date, Date)| p).inv().select(&cr).window(
        |s: &[(Date, (i64, Date, Date))], out: &mut Vec<i64>| {
            let mut mx: Option<Date> = None;
            for &(st, (_, _, e)) in s {
                out.push(if mx.is_some_and(|m| m >= st) { 0 } else { 1 });
                mx = Some(mx.map_or(e, |m| m.max(e)));
            }
        },
        |(_, s, _): (i64, Date, Date)| s,
        |a: &Date, b: &Date| a.cmp(b),
    );
    let st = rel(drain(&w5).into_iter().map(|(_, x)| x).collect::<Vec<((i64, Date, Date), i64)>>());
    let w6 = (&st).map(|((p, _, _), _): ((i64, Date, Date), i64)| p).inv().select(&st).window(
        |s: &[((Date, i64), ((i64, Date, Date), i64))], out: &mut Vec<i64>| {
            let mut acc = 0;
            for &(_, (_, is_start)) in s {
                acc += is_start;
                out.push(acc);
            }
        },
        |((_, s, _), is_start): ((i64, Date, Date), i64)| (s, -is_start),
        |a: &(Date, i64), b: &(Date, i64)| a.cmp(b),
    );
    let gr = rel(drain(&w6).into_iter().map(|(_, (((p, s, e), _), g))| ((p, g), (s, e))).collect::<Vec<((i64, i64), (Date, Date))>>());
    let fc = (&gr)
        .map(|(k, _): ((i64, i64), (Date, Date))| k)
        .inv()
        .select(&gr)
        .fold((i64::MAX, i64::MIN), |(lo, hi), (_, (s, e)): ((i64, i64), (Date, Date))| (lo.min(s), hi.max(e)));
    drain(&fc)
        .into_iter()
        .map(|((p, _), (s, e))| C862 { cohort_definition_id: 1, subject_id: p, cohort_start_date: s, cohort_end_date: e })
        .collect()
}

pub fn q(db: &'static Db) -> String {
    rows(c862(db).into_iter().map(|v| {
        row(vec![V::I(v.cohort_definition_id), V::I(v.subject_id), V::D(v.cohort_start_date), V::D(v.cohort_end_date)])
    }))
}
