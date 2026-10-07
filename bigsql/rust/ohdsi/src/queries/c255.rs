use crate::schema::*;
use harness::prelude::*;
use std::cmp::Reverse;

#[derive(Clone, Copy)]
pub struct Ev {
    pub person_id: i64,
    pub event_id: i64,
    pub start: Date,
    pub end: Date,
}

#[derive(Clone, Copy)]
pub struct Pe {
    pub person_id: i64,
    pub event_id: i64,
    pub start: Date,
    pub end: Date,
    pub op_start: Date,
    pub op_end: Date,
}

#[derive(Clone, Copy)]
pub struct C255 {
    pub subject_id: i64,
    pub start: Date,
    pub end: Date,
}

type Cs = HashIdx<i64, i64>;
type Ops = HashIdx<i64, Id<ObservationPeriod>>;
type Cnt = Fold<(i64, i64), i64>;
type Keys = HashIdx<(i64, i64), i64>;

fn kp(x: Pe) -> (i64, i64) {
    (x.person_id, x.event_id)
}

fn ke(x: Ev) -> (i64, i64) {
    (x.person_id, x.event_id)
}

fn concepts(db: &'static Db, ids: &[i64], desc: &[i64]) -> Fold<i64, ()> {
    let c = &db.concept;
    let ca = &db.concept_ancestor;
    let valid = (&c.id).minus(&c.invalid_reason);
    (&c.concept_id)
        .is_in(ids.iter().copied())
        .inv()
        .map(|_| ())
        .union(
            (&ca.id)
                .with((&ca.ancestor_concept_id).is_in(desc.iter().copied()))
                .select(&ca.descendant_concept)
                .with(valid)
                .select(&c.concept_id)
                .inv()
                .map(|_| ()),
        )
        .fold((), |a, _| a)
}

fn direct(db: &'static Db, ids: &[i64]) -> Fold<i64, ()> {
    (&db.concept.concept_id).is_in(ids.iter().copied()).inv().map(|_| ()).fold((), |a, _| a)
}

fn codesets(db: &'static Db) -> Cs {
    let s0 = concepts(db, &[378419], &[378419]);
    let l2 = [
        37312036, 37312035, 4041685, 37312031, 37312030, 35608576, 4092747, 4182210, 37311665, 4043378, 45765480,
        45765477, 4059191, 37311890, 37312577,
    ];
    let s2 = concepts(db, &l2, &l2);
    let e2 = direct(db, &[37116464, 37017549, 4244346, 377788, 372610, 37017247]);
    let l3 = [715997, 757627, 701322, 733523, 836654];
    let s3 = concepts(db, &l3, &l3);
    let l4 = [4169175, 40491929, 40490379, 4167593, 4013636, 4125350, 4019823];
    let s4 = concepts(db, &l4, &l4);
    let l5 = [4304008, 440424, 132342, 4173136, 4024716];
    let s5 = concepts(db, &l5, &l5);
    let l6 = [443605, 44782710, 380701];
    let s6 = concepts(db, &l6, &l6);
    let s8 = concepts(
        db,
        &[372924, 375557, 376713, 443454, 441874, 439847, 43530727, 42538062, 4148906, 432923],
        &[376713, 443454, 439847],
    );
    let e8 = concepts(db, &[379778], &[379778]);
    let s9 = concepts(db, &[262, 9201], &[262, 9201]);
    (&s0)
        .map(|_| 0i64)
        .union((&s2).minus(&e2).map(|_| 2i64))
        .union((&s3).map(|_| 3i64))
        .union((&s4).map(|_| 4i64))
        .union((&s5).map(|_| 5i64))
        .union((&s6).map(|_| 6i64))
        .union((&s8).minus(&e8).map(|_| 8i64))
        .union((&s9).map(|_| 9i64))
        .collect()
}

fn evs<Q: Drive<R = Ev>>(q: Q) -> Vec<Ev> {
    drain(q).into_iter().map(|(_, e)| e).collect()
}

fn cond(db: &'static Db, cs: &Cs, k: i64) -> Vec<Ev> {
    let co = &db.condition_occurrence;
    evs((&co.condition_concept_id)
        .select(cs)
        .filt(move |s: i64| s == k)
        .and(
            (&co.person_id)
                .and(&co.condition_occurrence_id)
                .and(&co.condition_start_date)
                .and((&co.condition_end_date).opt()),
        )
        .map(|(_, (((person_id, event_id), start), end)): (i64, (((i64, i64), Date), Option<Date>))| Ev {
            person_id,
            event_id,
            start,
            end: end.unwrap_or(start + DAY_US),
        }))
}

fn drug(db: &'static Db, cs: &Cs, k: i64) -> Vec<Ev> {
    let de = &db.drug_exposure;
    evs((&de.drug_concept_id)
        .select(cs)
        .filt(move |s: i64| s == k)
        .and(
            (&de.person_id)
                .and(&de.drug_exposure_id)
                .and(&de.drug_exposure_start_date)
                .and(&de.drug_exposure_end_date),
        )
        .map(|(_, (((person_id, event_id), start), end)): (i64, (((i64, i64), Date), Date))| Ev {
            person_id,
            event_id,
            start,
            end,
        }))
}

fn proc(db: &'static Db, cs: &Cs, k: i64) -> Vec<Ev> {
    let po = &db.procedure_occurrence;
    evs((&po.procedure_concept_id)
        .select(cs)
        .filt(move |s: i64| s == k)
        .and((&po.person_id).and(&po.procedure_occurrence_id).and(&po.procedure_date))
        .map(|(_, ((person_id, event_id), start)): (i64, ((i64, i64), Date))| Ev {
            person_id,
            event_id,
            start,
            end: start + DAY_US,
        }))
}

fn meas(db: &'static Db, cs: &Cs, k: i64) -> Vec<Ev> {
    let m = &db.measurement;
    evs((&m.measurement_concept_id)
        .select(cs)
        .filt(move |s: i64| s == k)
        .and((&m.person_id).and(&m.measurement_id).and(&m.measurement_date))
        .map(|(_, ((person_id, event_id), start)): (i64, ((i64, i64), Date))| Ev {
            person_id,
            event_id,
            start,
            end: start + DAY_US,
        }))
}

fn obs(db: &'static Db, cs: &Cs, k: i64) -> Vec<Ev> {
    let o = &db.observation;
    evs((&o.observation_concept_id)
        .select(cs)
        .filt(move |s: i64| s == k)
        .and((&o.person_id).and(&o.observation_id).and(&o.observation_date))
        .map(|(_, ((person_id, event_id), start)): (i64, ((i64, i64), Date))| Ev {
            person_id,
            event_id,
            start,
            end: start + DAY_US,
        }))
}

fn visit(db: &'static Db, cs: &Cs, k: i64) -> Vec<Ev> {
    let vo = &db.visit_occurrence;
    evs((&vo.visit_concept_id)
        .select(cs)
        .filt(move |s: i64| s == k)
        .and((&vo.person_id).and(&vo.visit_occurrence_id).and(&vo.visit_start_date).and(&vo.visit_end_date))
        .map(|(_, (((person_id, event_id), start), end)): (i64, (((i64, i64), Date), Date))| Ev {
            person_id,
            event_id,
            start,
            end,
        }))
}

fn first(v: &[Ev]) -> Vec<Ev> {
    let r = rel(v.to_vec());
    let w = (&r).map(|e: Ev| e.person_id).inv().select(&r).window(
        row_number,
        |e: Ev| (e.start, e.event_id),
        |a: &(Date, i64), b: &(Date, i64)| a.cmp(b),
    );
    drain((&w).filt(|(_, rn): (Ev, i64)| rn == 1)).into_iter().map(|(_, (e, _))| e).collect()
}

fn with_op(db: &'static Db, ops: &Ops, v: &[Ev]) -> Vec<Pe> {
    let op = &db.observation_period;
    let r = rel(v.to_vec());
    let q = (&r)
        .and(
            (&r).map(|e: Ev| e.person_id)
                .select(ops)
                .select((&op.observation_period_start_date).and(&op.observation_period_end_date)),
        )
        .filt(|(e, (s, en)): (Ev, (Date, Date))| s <= e.start && en >= e.start)
        .map(|(e, (op_start, op_end)): (Ev, (Date, Date))| Pe {
            person_id: e.person_id,
            event_id: e.event_id,
            start: e.start,
            end: e.end,
            op_start,
            op_end,
        });
    drain(&q).into_iter().map(|(_, p)| p).collect()
}

fn crit(p: &VecRel<usize, Pe>, a: &[Ev], ok: impl Fn(Pe, Ev) -> bool) -> Cnt {
    let ar = rel(a.to_vec());
    let by: HashIdx<i64, usize> = (&ar).map(|e: Ev| e.person_id).inv().collect();
    p.and(p.map(|x: Pe| x.person_id).select(&by).select(&ar))
        .filt(move |(x, e): (Pe, Ev)| e.start >= x.op_start && e.start <= x.op_end && ok(x, e))
        .map(|(x, _): (Pe, Ev)| kp(x))
        .inv()
        .fold(0i64, |c, _| c + 1)
}

fn group_count(p: &VecRel<usize, Pe>, cq: &Keys) -> Cnt {
    p.map(kp).inv().select(p.map(kp).select(cq)).fold(0i64, |c, _| c + 1)
}

fn pick(f: &[Ev], g: &Cnt, ok: impl Fn(i64) -> bool) -> Vec<Ev> {
    let r = rel(f.to_vec());
    evs((&r).and((&r).map(ke).select(g.filt(ok))).map(|(e, _): (Ev, i64)| e))
}

fn absent(qr: &VecRel<usize, Pe>, n: &Cnt) -> Keys {
    let g = qr.map(kp).inv().select(qr.map(kp).select(n).opt()).fold(0i64, |a, c: Option<i64>| a + c.unwrap_or(0));
    (&g).filt(|c: i64| c <= 0).map(|_| 0i64).collect()
}

fn incl(qr: &VecRel<usize, Pe>, cq: &Keys) -> Vec<(i64, i64)> {
    let g = group_count(qr, cq);
    drain(qr.map(kp).with((&g).filt(|n: i64| n == 1))).into_iter().map(|(_, k)| k).collect()
}

pub fn c255(db: &'static Db) -> Vec<C255> {
    let cs = codesets(db);
    let ops: Ops = (&db.observation_period.person_id).inv().collect();

    let pc0 = || rel(with_op(db, &ops, &first(&cond(db, &cs, 0))));
    let pc2 = || rel(with_op(db, &ops, &first(&cond(db, &cs, 2))));
    let pd3 = || rel(with_op(db, &ops, &first(&drug(db, &cs, 3))));

    let f0 = crit(&pc0(), &drug(db, &cs, 3), |x, e| e.start >= x.start && e.start <= x.op_end);
    let f1 = crit(&pc0(), &cond(db, &cs, 0), |x, e| e.start >= x.start + DAY_US && e.start <= x.op_end);
    let f2 = crit(&pc0(), &proc(db, &cs, 4), |x, e| e.start >= x.op_start && e.start <= x.start);
    let f3 = crit(&pc0(), &meas(db, &cs, 4), |x, e| e.start >= x.op_start && e.start <= x.start);
    let f4 = crit(&pc0(), &obs(db, &cs, 4), |x, e| e.start >= x.op_start && e.start <= x.start);
    let f5 = crit(&pc0(), &cond(db, &cs, 5), |x, e| e.start >= x.op_start && e.start <= x.op_end);
    let f6 = crit(&pc0(), &visit(db, &cs, 9), |x, e| {
        e.start >= x.op_start && e.start <= x.start && e.end >= x.start && e.end <= x.op_end
    });
    let cqa: Keys = (&f0)
        .filt(|c: i64| c >= 1)
        .map(|_| 0i64)
        .union((&f1).filt(|c: i64| c >= 1).map(|_| 1i64))
        .union((&f2).filt(|c: i64| c >= 1).map(|_| 2i64))
        .union((&f3).filt(|c: i64| c >= 1).map(|_| 3i64))
        .union((&f4).filt(|c: i64| c >= 1).map(|_| 4i64))
        .union((&f5).filt(|c: i64| c >= 1).map(|_| 5i64))
        .union((&f6).filt(|c: i64| c >= 1).map(|_| 6i64))
        .collect();
    let ba = rel(pick(&first(&cond(db, &cs, 0)), &group_count(&pc0(), &cqa), |n| n > 0));

    let fb = crit(&pc2(), &drug(db, &cs, 3), |x, e| e.start >= x.start && e.start <= x.op_end);
    let cqb: Keys = (&fb).filt(|c: i64| c >= 2).map(|_| 0i64).collect();
    let bb = rel(pick(&first(&cond(db, &cs, 2)), &group_count(&pc2(), &cqb), |n| n == 1));

    let fc = crit(&pd3(), &cond(db, &cs, 0), |x, e| e.start >= x.start && e.start <= x.op_end);
    let cqc: Keys = (&fc).filt(|c: i64| c >= 1).map(|_| 0i64).collect();
    let bc = rel(pick(&first(&drug(db, &cs, 3)), &group_count(&pd3(), &cqc), |n| n == 1));

    let prim = evs((&ba).union(&bb).union(&bc));
    let pop = rel(with_op(db, &ops, &prim));
    let pw = (&pop)
        .filt(|x: Pe| x.op_start + 365 * DAY_US <= x.start && x.start <= x.op_end)
        .map(|x: Pe| x.person_id)
        .inv()
        .select(&pop)
        .window(row_number, |x: Pe| (x.start, x.event_id), |a: &(Date, i64), b: &(Date, i64)| a.cmp(b));
    let pe1 = rel(
        drain((&pw).filt(|(_, rn): (Pe, i64)| rn == 1))
            .into_iter()
            .map(|(_, (x, rn))| Pe { event_id: rn, ..x })
            .collect::<Vec<Pe>>(),
    );
    let qw = (&pe1).map(|x: Pe| x.person_id).inv().select(&pe1).window(
        row_number,
        |x: Pe| x.start,
        |a: &Date, b: &Date| a.cmp(b),
    );
    let qr = rel(drain(&qw).into_iter().map(|(_, (x, _))| x).collect::<Vec<Pe>>());

    let c6 = cond(db, &cs, 6);
    let n0 = crit(&qr, &c6, |x, e| e.start >= x.start + DAY_US && e.start <= x.op_end);
    let i0 = rel(incl(&qr, &absent(&qr, &n0)));
    let c8 = cond(db, &cs, 8);
    let n1 = crit(&qr, &c8, |x, e| e.start >= x.start - 730 * DAY_US && e.start <= x.start);
    let i1 = rel(incl(&qr, &absent(&qr, &n1)));
    let pid: HashIdx<i64, Id<Person>> = (&db.person.person_id).inv().collect();
    let age = (&qr)
        .and((&qr).map(|x: Pe| x.person_id).select(&pid).select(&db.person.year_of_birth))
        .filt(|(x, yob): (Pe, i64)| year(x.start) - yob >= 18)
        .map(|(x, _): (Pe, i64)| kp(x))
        .inv()
        .fold((), |a, _| a);
    let cq2: Keys = (&age).map(|_| 0i64).collect();
    let i2 = rel(incl(&qr, &cq2));

    let ie = rel(drain(
        (&i0).map(|k: (i64, i64)| (k, 0i64))
            .union((&i1).map(|k: (i64, i64)| (k, 1i64)))
            .union((&i2).map(|k: (i64, i64)| (k, 2i64))),
    )
    .into_iter()
    .map(|(_, r)| r)
    .collect::<Vec<((i64, i64), i64)>>());
    let ie_by: Keys =
        (&ie).map(|(k, _): ((i64, i64), i64)| k).inv().select(&ie).map(|(_, r): ((i64, i64), i64)| r).collect();
    let mg = (&qr)
        .map(|x: Pe| (x.event_id, x.person_id, (x.start, x.end), (x.op_start, x.op_end)))
        .inv()
        .select((&qr).map(kp).select(&ie_by).opt())
        .fold(0i64, |m, r: Option<i64>| m + r.map_or(0, |r| 1i64 << r));
    let matched = rel(
        drain((&mg).filt(|m: i64| m == 7))
            .into_iter()
            .map(|((event_id, person_id, (start, end), (op_start, op_end)), _)| Pe {
                person_id,
                event_id,
                start,
                end,
                op_start,
                op_end,
            })
            .collect::<Vec<Pe>>(),
    );
    let iw = (&matched).map(|x: Pe| x.person_id).inv().select(&matched).window(
        row_number,
        |x: Pe| x.start,
        |a: &Date, b: &Date| a.cmp(b),
    );
    let ir = rel(
        drain((&iw).filt(|(_, rn): (Pe, i64)| rn == 1))
            .into_iter()
            .map(|(_, (x, _))| x)
            .collect::<Vec<Pe>>(),
    );

    let ce_by: HashIdx<(i64, i64), usize> = (&ir).map(kp).inv().collect();
    let j = (&ir)
        .and((&ir).map(kp).select(&ce_by).select(&ir).map(|c: Pe| c.op_end))
        .filt(|(i, e): (Pe, Date)| e >= i.start);
    let jr = rel(drain(&j).into_iter().map(|(_, p)| p).collect::<Vec<(Pe, Date)>>());
    let fw = (&jr).map(|(i, _): (Pe, Date)| kp(i)).inv().select(&jr).window(
        row_number,
        |(_, e): (Pe, Date)| e,
        |a: &Date, b: &Date| a.cmp(b),
    );
    let cr = rel(
        drain((&fw).filt(|(_, rn): ((Pe, Date), i64)| rn == 1))
            .into_iter()
            .map(|(_, ((i, e), _))| (i.person_id, i.start, e))
            .collect::<Vec<(i64, Date, Date)>>(),
    );

    let w1 = (&cr).map(|(p, _, _): (i64, Date, Date)| p).inv().select(&cr).window(
        |g: &[(Date, (i64, Date, Date))], out: &mut Vec<i64>| {
            let mut mx: Option<Date> = None;
            for &(s, (_, _, e)) in g {
                out.push(if mx.is_some_and(|m| m >= s) { 0 } else { 1 });
                mx = Some(mx.map_or(e, |m| m.max(e)));
            }
        },
        |(_, s, _): (i64, Date, Date)| s,
        |a: &Date, b: &Date| a.cmp(b),
    );
    let st = rel(drain(&w1).into_iter().map(|(_, r)| r).collect::<Vec<((i64, Date, Date), i64)>>());
    let w2 = (&st).map(|((p, _, _), _): ((i64, Date, Date), i64)| p).inv().select(&st).window(
        |g: &[((Date, Reverse<i64>), ((i64, Date, Date), i64))], out: &mut Vec<i64>| {
            let mut s = 0;
            for &(_, (_, is)) in g {
                s += is;
                out.push(s);
            }
        },
        |((_, s, _), is): ((i64, Date, Date), i64)| (s, Reverse(is)),
        |a: &(Date, Reverse<i64>), b: &(Date, Reverse<i64>)| a.cmp(b),
    );
    let gr = rel(drain(&w2).into_iter().map(|(_, r)| r).collect::<Vec<(((i64, Date, Date), i64), i64)>>());
    let fc = (&gr)
        .map(|(((p, _, _), _), gi): (((i64, Date, Date), i64), i64)| (p, gi))
        .inv()
        .select((&gr).map(|(((_, s, e), _), _): (((i64, Date, Date), i64), i64)| (s, e)))
        .fold((i64::MAX, i64::MIN), |(lo, hi), (s, e): (Date, Date)| (lo.min(s), hi.max(e)));
    drain(&fc).into_iter().map(|((subject_id, _), (start, end))| C255 { subject_id, start, end }).collect()
}

pub fn q(db: &'static Db) -> String {
    rows(c255(db).into_iter().map(|c| row(vec![V::I(1), V::I(c.subject_id), V::D(c.start), V::D(c.end)])))
}
