use crate::concepts::member_month__month_spine::MemberMonthMonthSpine;
use crate::concepts::normalized__eligibility::NormalizedEligibility;
use crate::schema::Db;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct MemberMonth {
    pub member_month_id: Str,
    pub person_id: Str,
    pub member_id: Str,
    pub year_month: Str,
    pub payer: Str,
    pub plan: Str,
    pub tuva_last_run: Ts,
    pub data_source: Str,
}

fn leak(s: String) -> Str {
    Box::leak(s.into_boxed_str())
}

pub fn md5_hex(input: &[u8]) -> String {
    const S: [u32; 64] = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14,
        20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6,
        10, 15, 21,
    ];
    let k: Vec<u32> = (0..64).map(|i| ((i as f64 + 1.0).sin().abs() * 4294967296.0) as u32).collect();
    let mut msg = input.to_vec();
    let bits = (input.len() as u64).wrapping_mul(8);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bits.to_le_bytes());
    let (mut a0, mut b0, mut c0, mut d0) = (0x67452301u32, 0xefcdab89u32, 0x98badcfeu32, 0x10325476u32);
    for chunk in msg.chunks(64) {
        let m: Vec<u32> = chunk.chunks(4).map(|w| u32::from_le_bytes([w[0], w[1], w[2], w[3]])).collect();
        let (mut a, mut b, mut c, mut d) = (a0, b0, c0, d0);
        for i in 0..64 {
            let (f, g) = match i / 16 {
                0 => ((b & c) | (!b & d), i),
                1 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                2 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };
            let f2 = f.wrapping_add(a).wrapping_add(k[i]).wrapping_add(m[g]);
            a = d;
            d = c;
            c = b;
            b = b.wrapping_add(f2.rotate_left(S[i]));
        }
        a0 = a0.wrapping_add(a);
        b0 = b0.wrapping_add(b);
        c0 = c0.wrapping_add(c);
        d0 = d0.wrapping_add(d);
    }
    [a0, b0, c0, d0].iter().flat_map(|x| x.to_le_bytes()).map(|b| format!("{b:02x}")).collect()
}

type Stg = (((Str, Str), (Str, Str)), ((Date, Date), (Ts, Str)));
type Key = (Str, Str, i64, Str, Str, Ts, Str);

pub fn member_month(_db: &'static Db, ne: &[NormalizedEligibility], spine: &[MemberMonthMonthSpine]) -> Vec<MemberMonth> {
    let er = rel(ne.to_vec());
    let sr = rel(spine.to_vec());
    let a = (&er)
        .filt(|e: NormalizedEligibility| e.enrollment_start_date.is_some())
        .map(|e: NormalizedEligibility| -> Stg {
            let last = trunc_day(e.tuva_last_run);
            let eff = match e.enrollment_end_date {
                Some(x) if x <= last => x,
                _ => last,
            };
            (((e.person_id, e.member_id), (e.payer, e.plan)), ((e.enrollment_start_date.unwrap(), eff), (e.tuva_last_run, e.data_source)))
        });
    let j = (&a)
        .cross(&sr)
        .filt(|((_, ((st, eff), _)), b): (Stg, MemberMonthMonthSpine)| {
            st <= b.last_day_of_month && eff >= b.first_day_of_month && st <= eff
        })
        .map(|((((p, m), (py, pl)), (_, (t, ds))), b): (Stg, MemberMonthMonthSpine)| -> Key {
            (p, m, b.year_month_int, py, pl, t, ds)
        })
        .inv()
        .map(|_| ())
        .fold((), |a, _| a);
    drain(&j)
        .into_iter()
        .map(|((person_id, member_id, ym, payer, plan, tuva_last_run, data_source), _)| {
            let year_month = format!("{ym:06}");
            let id = md5_hex(format!("{person_id}-{member_id}-{year_month}-{payer}-{plan}-{data_source}").as_bytes());
            MemberMonth {
                member_month_id: leak(id),
                person_id,
                member_id,
                year_month: leak(year_month),
                payer,
                plan,
                tuva_last_run,
                data_source,
            }
        })
        .collect()
}

pub fn q(db: &'static Db) -> String {
    let el = crate::concepts::eligibility::eligibility(db);
    let il = crate::concepts::input_layer__eligibility::input_layer__eligibility(db, &el);
    let ec = crate::concepts::int_eligibility_casting::int_eligibility_casting(db, &il);
    let dn = crate::concepts::int_eligibility_dates_normalized::int_eligibility_dates_normalized(db, &ec);
    let sn = crate::concepts::int_eligibility_state_normalized::int_eligibility_state_normalized(db, &ec);
    let ne = crate::concepts::normalized__eligibility::normalized__eligibility(db, &ec, &dn, &sn);
    let sp = crate::concepts::member_month__month_spine::member_month__month_spine(db);
    rows(member_month(db, &ne, &sp).iter().map(|v| {
        row(vec![
            V::S(v.member_month_id),
            V::S(v.person_id),
            V::S(v.member_id),
            V::S(v.year_month),
            V::S(v.payer),
            V::S(v.plan),
            V::T(v.tuva_last_run),
            V::S(v.data_source),
        ])
    }))
}
