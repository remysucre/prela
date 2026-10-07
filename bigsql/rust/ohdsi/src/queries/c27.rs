use crate::schema::*;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct C27 {
    pub person_id: i64,
    pub start_date: Date,
    pub end_date: Date,
}

type Ev = (i64, i64, Date, Date);
type EvOp = (Ev, (Date, Date));
type Qe = (i64, i64, Date, Date, Date, Date);
type Cr = (i64, Date, Date);

const CS0: [i64; 3] = [317009, 4235703, 4279553];
const CS2: [i64; 2] = [255573, 258780];
const CS3: [i64; 1] = [21603292];
const CS4: [i64; 119] = [42483138, 36812530, 40727741, 1356123, 40142665, 46234463, 1356111, 1356108, 40142703, 21603292, 41143395, 36421291, 40745353, 36787954, 21158944, 36894458, 21090035, 36883710, 35130061, 40142784, 42479684, 35133500, 783228, 40142910, 36787269, 1356143, 35135829, 1356140, 44120754, 41205832, 40924271, 40986621, 40830666, 41205663, 40861866, 43274335, 43263324, 43621601, 1356154, 1356147, 40143105, 41080205, 41174344, 41111516, 1356173, 1356180, 40144087, 43134418, 40727834, 36811735, 40861768, 40727839, 35150375, 44081619, 1356217, 1356215, 1356101, 42800913, 40143326, 36894464, 44817882, 40143337, 42941603, 35146684, 35160199, 44029688, 42481922, 36813480, 36812414, 1356244, 40143708, 21150787, 44082127, 41206083, 1356138, 1356136, 40152662, 40156382, 41048760, 40754973, 21089505, 40144020, 36882733, 42482744, 40144024, 37592046, 43532281, 43291091, 41267401, 40144035, 35147990, 35149212, 40144037, 40223712, 42629522, 42480849, 1356191, 1356187, 44107471, 43145868, 43259954, 1356196, 35145836, 42479568, 1356211, 44055847, 44081467, 1356120, 40182262, 40746100, 21174574, 45775117, 41205378, 40152687, 40167702, 35158799, 783089, 1356189, 35152712];

fn branch(db: &'static Db, k: i64, ids: &[i64]) -> Vec<(i64, i64)> {
    let c = &db.concept;
    let ca = &db.concept_ancestor;
    let direct = (&c.id).with((&c.concept_id).is_in(ids.iter().copied())).select(&c.concept_id);
    let desc = (&ca.id)
        .with((&ca.ancestor_concept_id).is_in(ids.iter().copied()))
        .select(&ca.descendant_concept)
        .select((&c.concept_id).minus(&c.invalid_reason));
    let u = direct.inv().map(|_| ()).union(desc.inv().map(|_| ())).fold((), |a, _| a);
    drain(&u).into_iter().map(|(cid, _)| (k, cid)).collect()
}

fn codesets(db: &'static Db) -> Vec<(i64, i64)> {
    let (b0, b2, b3, b4) =
        (rel(branch(db, 0, &CS0)), rel(branch(db, 2, &CS2)), rel(branch(db, 3, &CS3)), rel(branch(db, 4, &CS4)));
    let u = (&b0).union(&b2).union(&b3).union(&b4);
    drain(&u).into_iter().map(|(_, r)| r).collect()
}

fn cs_by(cs: &VecRel<usize, (i64, i64)>) -> HashIdx<i64, i64> {
    cs.map(|(_, c): (i64, i64)| c).inv().select(cs.map(|(k, _): (i64, i64)| k)).collect()
}

fn drugs(db: &'static Db, cs: &HashIdx<i64, i64>, k: i64) -> Vec<Ev> {
    let de = &db.drug_exposure;
    let r = (&de.id)
        .with((&de.drug_concept_id).select(cs).filt(move |x| x == k))
        .select(
            (&de.person_id)
                .and(&de.drug_exposure_id)
                .and(&de.drug_exposure_start_date)
                .and(&de.drug_exposure_end_date),
        );
    drain(&r).into_iter().map(|(_, (((p, e), s), en))| (p, e, s, en)).collect()
}

fn drugs_aged(db: &'static Db, cs: &HashIdx<i64, i64>) -> Vec<Ev> {
    let de = &db.drug_exposure;
    let r = (&de.id)
        .with((&de.drug_concept_id).select(cs).filt(|x| x == 4))
        .select(
            (&de.person_id)
                .and(&de.drug_exposure_id)
                .and(&de.drug_exposure_start_date)
                .and(&de.drug_exposure_end_date)
                .and((&de.person).select(&db.person.year_of_birth)),
        )
        .filt(|((((_, _), s), _), y): ((((i64, i64), Date), Date), i64)| year(s) - y < 55);
    drain(&r).into_iter().map(|(_, ((((p, e), s), en), _))| (p, e, s, en)).collect()
}

fn in_op(db: &'static Db, op_by: &HashIdx<i64, Id<ObservationPeriod>>, evs: Vec<Ev>) -> Vec<EvOp> {
    let op = &db.observation_period;
    let r = rel(evs);
    let j = (&r)
        .and(
            (&r).map(|e: Ev| e.0)
                .select(op_by)
                .select((&op.observation_period_start_date).and(&op.observation_period_end_date)),
        )
        .filt(|(e, (s, en)): EvOp| s <= e.2 && en >= e.2);
    drain(&j).into_iter().map(|(_, x)| x).collect()
}

fn no_prior(qer: &VecRel<usize, Qe>, evs: Vec<Ev>) -> Vec<(i64, i64)> {
    let ar = rel(evs);
    let a_by: HashIdx<i64, usize> = (&ar).map(|e: Ev| e.0).inv().collect();
    let cc = (qer)
        .and(qer.map(|q: Qe| q.0).select(&a_by).select(&ar))
        .filt(|(p, a): (Qe, Ev)| a.2 >= p.4 && a.2 <= p.5 && a.2 >= p.4 && a.2 <= p.2)
        .map(|(p, _): (Qe, Ev)| (p.0, p.1));
    let n = cc.inv().fold(0i64, |a, _| a + 1);
    let k = qer.map(|q: Qe| (q.0, q.1));
    let g = (&k).inv().select((&k).select(&n).opt()).fold(0i64, |s, x: Option<i64>| s + x.unwrap_or(0));
    drain((&g).filt(|c: i64| c == 0)).into_iter().map(|(x, _)| x).collect()
}

pub fn c27(db: &'static Db) -> Vec<C27> {
    let op = &db.observation_period;
    let co = &db.condition_occurrence;
    let ob = &db.observation;
    let csr = rel(codesets(db));
    let cs = cs_by(&csr);
    let op_by: HashIdx<i64, Id<ObservationPeriod>> = (&op.person_id).inv().collect();

    let pe = rel(drugs_aged(db, &cs));
    let qr = rel(in_op(db, &op_by, drugs_aged(db, &cs)));
    let pr = rel(in_op(db, &op_by, drugs_aged(db, &cs)));
    let ar = rel(drugs(db, &cs, 4));
    let a_by: HashIdx<i64, usize> = (&ar).map(|e: Ev| e.0).inv().collect();
    let cc = (&pr)
        .and((&pr).map(|(e, _): EvOp| e.0).select(&a_by).select(&ar))
        .filt(|((p, _), a): (EvOp, Ev)| a.2 >= p.2 - 365 * DAY_US && a.2 <= p.2 - 180 * DAY_US)
        .map(|((p, _), _): (EvOp, Ev)| (p.0, p.1));
    let cq = cc.inv().fold(0i64, |n, _| n + 1);
    let cqf: HashIdx<(i64, i64), i64> = (&cq).filt(|n| n >= 1).collect();
    let g = (&qr).map(|(e, _): EvOp| (e.0, e.1)).inv().and(&cqf).fold(0i64, |n, _| n + 1);
    let ac: HashIdx<(i64, i64), i64> = (&g).filt(|n| n == 1).collect();
    let prim_drug = (&pe).and((&pe).map(|e: Ev| (e.0, e.1)).select(&ac)).map(|(e, _): (Ev, i64)| e);

    let cond = (&co.id)
        .with((&co.condition_concept_id).select(&cs).filt(|x| x == 0))
        .select(
            (&co.person_id)
                .and(&co.condition_occurrence_id)
                .and(&co.condition_start_date)
                .and((&co.condition_end_date).opt()),
        )
        .map(|(((p, e), s), en): (((i64, i64), Date), Option<Date>)| -> Ev { (p, e, s, en.unwrap_or(s + DAY_US)) });
    let obs = (&ob.id)
        .with((&ob.observation_concept_id).select(&cs).filt(|x| x == 0))
        .select((&ob.person_id).and(&ob.observation_id).and(&ob.observation_date))
        .map(|((p, e), s): ((i64, i64), Date)| -> Ev { (p, e, s, s + DAY_US) });
    let u = prim_drug.inv().map(|_| ()).union(cond.inv().map(|_| ())).union(obs.inv().map(|_| ()));
    let evs: Vec<Ev> = drain(&u).into_iter().map(|(e, _)| e).collect();
    let er = rel(in_op(db, &op_by, evs));
    let w = (&er).map(|(e, _): EvOp| e.0).inv().select(&er).window(
        row_number,
        |(e, _): EvOp| (e.2, e.1),
        |a: &(Date, i64), b: &(Date, i64)| a.cmp(b),
    );
    let prim: Vec<Qe> = drain((&w).filt(|(_, n): (EvOp, i64)| n == 1))
        .into_iter()
        .map(|(_, ((e, (os, oe)), n))| (e.0, n, e.2, e.3, os, oe))
        .collect();
    let pr2 = rel(prim);
    let w2 = (&pr2).map(|q: Qe| q.0).inv().select(&pr2).window(row_number, |q: Qe| q.2, |a: &Date, b: &Date| a.cmp(b));
    let qe: Vec<Qe> = drain(&w2).into_iter().map(|(_, (q, _))| q).collect();
    let qer = rel(qe);

    let c2 = (&co.id)
        .with((&co.condition_concept_id).select(&cs).filt(|x| x == 2))
        .select(
            (&co.person_id)
                .and(&co.condition_occurrence_id)
                .and(&co.condition_start_date)
                .and((&co.condition_end_date).opt()),
        )
        .map(|(((p, e), s), en): (((i64, i64), Date), Option<Date>)| -> Ev { (p, e, s, en.unwrap_or(s + DAY_US)) });
    let c2v: Vec<Ev> = drain(&c2).into_iter().map(|(_, e)| e).collect();
    let i0 = rel(no_prior(&qer, c2v));
    let i1 = rel(no_prior(&qer, drugs(db, &cs, 3)));
    let iu = (&i0).map(|k: (i64, i64)| (0i64, k)).union((&i1).map(|k: (i64, i64)| (1i64, k)));
    let iur = rel(drain(iu).into_iter().map(|(_, x)| x).collect::<Vec<(i64, (i64, i64))>>());
    let iu_by: HashIdx<(i64, i64), usize> = (&iur).map(|(_, k): (i64, (i64, i64))| k).inv().collect();
    let g2 = (&qer).map(|q: Qe| (q.0, q.1)).inv().and(&iu_by).fold(0i64, |n, _| n + 1);
    let g2f: HashIdx<(i64, i64), i64> = (&g2).filt(|n| n == 2).collect();
    let inc: HashIdx<(i64, i64), i64> = (&qer).map(|q: Qe| (q.0, q.1)).inv().and(&g2f).map(|_| 0i64).collect();

    let mask = (&qer).map(|q: Qe| (q.0, q.1)).select(&inc).fold(0i64, |a, r: i64| a + (1i64 << r));
    let mg = (&qer).and((&mask).opt()).filt(|(_, m): (Qe, Option<i64>)| m.unwrap_or(0) == 1);
    let mgv: Vec<Qe> = drain(&mg).into_iter().map(|(_, (q, _))| q).collect();
    let mgr = rel(mgv);
    let w3 = (&mgr).map(|q: Qe| q.0).inv().select(&mgr).window(row_number, |q: Qe| q.2, |a: &Date, b: &Date| a.cmp(b));
    let incl: Vec<Qe> = drain((&w3).filt(|(_, n): (Qe, i64)| n == 1)).into_iter().map(|(_, (q, _))| q).collect();
    let ir = rel(incl);

    let se = (&ir).map(|q: Qe| (q.1, q.0, if q.3 > q.5 { q.5 } else { q.3 }));
    let ser = rel(drain(&se).into_iter().map(|(_, x)| x).collect::<Vec<(i64, i64, Date)>>());
    let se_by: HashIdx<(i64, i64), usize> = (&ser).map(|(e, p, _): (i64, i64, Date)| (e, p)).inv().collect();
    let f = (&ir)
        .and((&ir).map(|q: Qe| (q.1, q.0)).select(&se_by).select(&ser))
        .filt(|(q, (_, _, en)): (Qe, (i64, i64, Date))| en >= q.2);
    let fr = rel(drain(&f).into_iter().map(|(_, (q, (_, _, en)))| (q.0, q.1, q.2, en)).collect::<Vec<Ev>>());
    let w4 = (&fr).map(|x: Ev| (x.0, x.1)).inv().select(&fr).window(row_number, |x: Ev| x.3, |a: &Date, b: &Date| a.cmp(b));
    let crv: Vec<Cr> = drain((&w4).filt(|(_, n): (Ev, i64)| n == 1))
        .into_iter()
        .map(|(_, (x, _))| (x.0, x.2, x.3))
        .collect();

    let crr = rel(crv);
    let st = (&crr).map(|c: Cr| c.0).inv().select(&crr).window(
        |g: &[(Date, Cr)], out: &mut Vec<i64>| {
            let mut mx: Option<Date> = None;
            for &(_, (_, s, e)) in g {
                out.push(if mx.is_some_and(|m| m >= s) { 0 } else { 1 });
                mx = Some(mx.map_or(e, |m| m.max(e)));
            }
        },
        |c: Cr| c.1,
        |a: &Date, b: &Date| a.cmp(b),
    );
    let stv = rel(drain(&st).into_iter().map(|(_, x)| x).collect::<Vec<(Cr, i64)>>());
    let gr = (&stv).map(|(c, _): (Cr, i64)| c.0).inv().select(&stv).window(
        |g: &[((Date, i64), (Cr, i64))], out: &mut Vec<i64>| {
            let mut s = 0;
            for &(_, (_, is)) in g {
                s += is;
                out.push(s);
            }
        },
        |(c, is): (Cr, i64)| (c.1, -is),
        |a: &(Date, i64), b: &(Date, i64)| a.cmp(b),
    );
    let gv = rel(drain(&gr).into_iter().map(|(_, ((c, _), gi))| ((c.0, gi), (c.1, c.2))).collect::<Vec<((i64, i64), (Date, Date))>>());
    let fc = (&gv)
        .map(|(k, _): ((i64, i64), (Date, Date))| k)
        .inv()
        .select((&gv).map(|(_, v): ((i64, i64), (Date, Date))| v))
        .fold(None, |a: Option<(Date, Date)>, (s, e): (Date, Date)| {
            Some(a.map_or((s, e), |(ms, me): (Date, Date)| (ms.min(s), me.max(e))))
        });
    drain(&fc)
        .into_iter()
        .map(|((person_id, _), m)| {
            let (start_date, end_date) = m.unwrap();
            C27 { person_id, start_date, end_date }
        })
        .collect()
}

pub fn q(db: &'static Db) -> String {
    rows(c27(db).into_iter().map(|v| row(vec![V::I(1), V::I(v.person_id), V::D(v.start_date), V::D(v.end_date)])))
}
