// Cache format v2 — the shared spec between the writer (`regen`, which
// includes this file via `#[path]`) and the reader (src/cache.rs). The
// cache stores FINAL physical layouts: 0-based ids with `NO_ID` holes
// baked in, dates pre-parsed to yyyymmdd i64, strings as offsets+bytes,
// multi-valued columns as CSR. Loading is mmap + header check + bulk
// copy/slice — no per-pair work. (Format v1 — the Julia-era 1-based
// (i64, i64) pair streams — is gone; see the julia-engine branch for the
// historic implementation.)
//
// Every `<Entity>_<field>.bin` file is:
//
//   [ 0..8)   magic   b"prela2\0\0"
//   [ 8..12)  u32     kind (below)
//   [12..16)  u32     reserved, 0
//   [16..24)  u64     n — number of key slots (the universe size)
//   [24..32)  u64     m — kind-specific second count (see below)
//   [32..)    payload
//
// All integers little-endian; the payload starts 8-byte aligned (mmaps are
// page-aligned). Kinds:
//
//   0  DENSE_I64   payload = [n × i64]                              m = 0
//                  one value per id; also id/FK columns (stored as the
//                  0-based id words with NO_ID = !0u64 in the holes) and
//                  dates (pre-parsed yyyymmdd)
//   1  DENSE_F64   payload = [n × f64]                              m = 0
//   2  DENSE_STR   payload = [(n+1) × u32 offsets][m bytes]         m = total bytes
//                  string i = bytes[off[i]..off[i+1]]; holes are empty
//   3  CSR_WORDS   payload = [(n+1) × u32 offsets][pad to 8][m × 8-byte words]
//                  m = total values; row i = values[off[i]..off[i+1]]
//                  (words are 0-based ids or raw i64s — caller's type)
//   4  CSR_STR     payload = [(n+1) × u32 row-offsets]
//                            [(m+1) × u32 byte-offsets][bytes]      m = total strings
//                  row i = strings off[i]..off[i+1]; string j =
//                  bytes[boff[j]..boff[j+1]]
//
// A v1 file starts with its u64 pair count, which can never collide with
// the magic — stale caches fail loudly at the magic check.

pub const MAGIC: [u8; 8] = *b"prela2\0\0";
pub const HEADER_LEN: usize = 32;

pub const KIND_DENSE_I64: u32 = 0;
pub const KIND_DENSE_F64: u32 = 1;
pub const KIND_DENSE_STR: u32 = 2;
pub const KIND_CSR_WORDS: u32 = 3;
pub const KIND_CSR_STR: u32 = 4;

/// Hole word for id/FK dense columns: the on-disk image of `engine::NO_ID`.
#[allow(dead_code)] // writer-side only (regen); the reader copies words as-is
pub const NO_ID_WORD: u64 = u64::MAX;

/// Offset of the payload section for a given kind, given the header's
/// n — i.e. where the variable part after any leading offset array starts.
#[inline]
pub fn align8(x: usize) -> usize {
    (x + 7) & !7
}

#[allow(dead_code)] // writer side (regen binary only)
pub fn header(kind: u32, n: u64, m: u64) -> [u8; HEADER_LEN] {
    let mut h = [0u8; HEADER_LEN];
    h[0..8].copy_from_slice(&MAGIC);
    h[8..12].copy_from_slice(&kind.to_le_bytes());
    // [12..16) reserved = 0
    h[16..24].copy_from_slice(&n.to_le_bytes());
    h[24..32].copy_from_slice(&m.to_le_bytes());
    h
}

/// Parse + validate a v2 header. Loud failure on anything unexpected: an
/// old 1-based v1 cache silently read as v2 would be an off-by-one
/// disaster, so a bad magic aborts with the regen instruction.
#[allow(dead_code)] // reader side (prela binary only)
pub fn parse_header(bytes: &[u8], expect_kind: u32, what: &str) -> (usize, usize) {
    if bytes.len() < HEADER_LEN || bytes[0..8] != MAGIC {
        panic!(
            "{what}: not a cache-format-v2 file (stale v1 cache?) — \
             rerun `regen job` / `regen tpch`"
        );
    }
    let kind = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
    if kind != expect_kind {
        panic!(
            "{what}: cache kind {kind} but the loader expects kind {expect_kind} — \
             cache/loader mismatch, rerun `regen job` / `regen tpch`"
        );
    }
    let n = u64::from_le_bytes(bytes[16..24].try_into().unwrap()) as usize;
    let m = u64::from_le_bytes(bytes[24..32].try_into().unwrap()) as usize;
    (n, m)
}

// The format (and its zero-copy readers) assumes little-endian 64-bit —
// id words are read straight off the mmap as `usize`.
const _: () = assert!(cfg!(target_endian = "little") && size_of::<usize>() == 8);
