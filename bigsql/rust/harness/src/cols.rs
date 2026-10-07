use prela::engine::MultiRel;
use prela::loader::{Loader, Set};

pub fn multi_f64<E: 'static>(l: &mut Loader, name: &str) -> Set<E, f64> {
    let r = l.multi_i64::<E>(name);
    let v: &'static [f64] = unsafe { std::slice::from_raw_parts(r.v.as_ptr() as *const f64, r.v.len()) };
    MultiRel::from_csr(r.offsets, v)
}
