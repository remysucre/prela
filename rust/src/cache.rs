// Cache-format-v2 readers shared by the JOB (data.rs) and TPC-H
// (tpch_data.rs) datasets. The format is specified in src/format.rs and
// produced by the `regen` binary (`cargo run --release --features regen
// --bin regen -- {job|tpch} ...`), which absorbs ALL load-time
// transformation: ids are stored 0-based with `NO_ID` holes baked in,
// dates pre-parsed to yyyymmdd i64, strings laid out as offsets+bytes,
// multi columns as CSR. Loading is mmap + header check + bulk copy/slice.
// (The v1 pair-stream format and its load-time shift/scatter/bucketing —
// inherited from the retired Julia engine, julia-engine branch — are gone;
// a v1 file fails the magic check loudly.)
//
// Strings and CSR payloads are returned as `&'static` borrows — the mmap
// is leaked, so the bytes live for the program. No per-string allocation.

use crate::engine::{MultiRel, VecRel};
use crate::format::*;
use memmap2::Mmap;
use std::fs::File;
use std::path::{Path, PathBuf};

fn cache_dir() -> PathBuf {
    PathBuf::from("../cache")
}

fn mmap_static(path: &Path) -> &'static [u8] {
    let f = File::open(path).unwrap_or_else(|e| {
        panic!("open cache file {path:?}: {e} — run `regen job` / `regen tpch` first")
    });
    let mmap = unsafe { Mmap::map(&f).unwrap() };
    let leaked: &'static Mmap = Box::leak(Box::new(mmap));
    &**leaked
}

/// mmap `<name>.bin`, validate the v2 header against `kind`, return
/// (n, m, full bytes).
fn open(name: &str, kind: u32) -> (usize, usize, &'static [u8]) {
    let path = cache_dir().join(format!("{name}.bin"));
    let bytes = mmap_static(&path);
    let (n, m) = parse_header(bytes, kind, &format!("{path:?}"));
    (n, m, bytes)
}

/// Reinterpret a byte range as a typed slice. Alignment holds by the
/// format's construction (payloads are 4/8-aligned within the page-aligned
/// mmap); debug-asserted anyway.
fn cast_slice<T>(bytes: &'static [u8], off: usize, len: usize) -> &'static [T] {
    let end = off + len * size_of::<T>();
    assert!(end <= bytes.len(), "cache file truncated");
    let ptr = bytes[off..end].as_ptr();
    debug_assert_eq!(ptr as usize % align_of::<T>(), 0);
    unsafe { std::slice::from_raw_parts(ptr as *const T, len) }
}

// ===== dense columns (one value per 0-based id) ==========================

/// Dense i64 column (scalars; dates are pre-parsed yyyymmdd).
pub fn load_i64(name: &str) -> VecRel<i64> {
    let (n, _, bytes) = open(name, KIND_DENSE_I64);
    VecRel { values: cast_slice::<i64>(bytes, HEADER_LEN, n).to_vec() }
}

/// Dense id column (FKs): same payload kind as i64, but the words are
/// 0-based ids with `NO_ID` (= !0) baked into the holes by regen.
pub fn load_ids(name: &str) -> VecRel<usize> {
    let (n, _, bytes) = open(name, KIND_DENSE_I64);
    VecRel { values: cast_slice::<usize>(bytes, HEADER_LEN, n).to_vec() }
}

/// Dense f64 column.
pub fn load_f64(name: &str) -> VecRel<f64> {
    let (n, _, bytes) = open(name, KIND_DENSE_F64);
    VecRel { values: cast_slice::<f64>(bytes, HEADER_LEN, n).to_vec() }
}

/// Dense string column: one pass over the offsets builds the
/// `Vec<&'static str>`; the bytes stay in the leaked mmap. Holes are "".
pub fn load_strs(name: &str) -> VecRel<&'static str> {
    let (n, m, bytes) = open(name, KIND_DENSE_STR);
    let offsets = cast_slice::<u32>(bytes, HEADER_LEN, n + 1);
    let data = cast_slice::<u8>(bytes, HEADER_LEN + (n + 1) * 4, m);
    VecRel { values: strs_from_offsets(offsets, data) }
}

fn strs_from_offsets(offsets: &'static [u32], data: &'static [u8]) -> Vec<&'static str> {
    offsets
        .windows(2)
        .map(|w| {
            let s = &data[w[0] as usize..w[1] as usize];
            // regen wrote these bytes from &str values — valid UTF-8.
            unsafe { std::str::from_utf8_unchecked(s) }
        })
        .collect()
}

// ===== CSR multi columns ================================================

/// CSR with 8-byte word values, read as 0-based ids. Zero-copy: offsets
/// and values are slices into the leaked mmap.
pub fn load_multi_ids(name: &str) -> MultiRel<usize> {
    let (offsets, values) = csr_words::<usize>(name);
    MultiRel::from_csr(offsets, values)
}

/// CSR with 8-byte word values, read as raw i64 scalars.
pub fn load_multi_i64(name: &str) -> MultiRel<i64> {
    let (offsets, values) = csr_words::<i64>(name);
    MultiRel::from_csr(offsets, values)
}

fn csr_words<T>(name: &str) -> (&'static [u32], &'static [T]) {
    let (n, m, bytes) = open(name, KIND_CSR_WORDS);
    let offsets = cast_slice::<u32>(bytes, HEADER_LEN, n + 1);
    let values = cast_slice::<T>(bytes, align8(HEADER_LEN + (n + 1) * 4), m);
    (offsets, values)
}

/// CSR string column: row offsets are zero-copy; the per-string `&str`s
/// are built in one pass and leaked (they index the leaked mmap bytes).
pub fn load_multi_strs(name: &str) -> MultiRel<&'static str> {
    let (n, m, bytes) = open(name, KIND_CSR_STR);
    let row_off = cast_slice::<u32>(bytes, HEADER_LEN, n + 1);
    let str_off_at = HEADER_LEN + (n + 1) * 4;
    let str_off = cast_slice::<u32>(bytes, str_off_at, m + 1);
    let data_at = str_off_at + (m + 1) * 4;
    let data = cast_slice::<u8>(bytes, data_at, bytes.len() - data_at);
    let strs: Vec<&'static str> = strs_from_offsets(str_off, data);
    MultiRel::from_csr(row_off, Vec::leak(strs))
}

// ===== tests — round-trip a tiny file of each kind ======================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{Drive, NO_ID};
    use std::io::Write;

    // The pub loaders resolve names under ../cache; for tests, write into
    // a temp dir and call the internals via a chdir-free path: replicate
    // `open` on an explicit path.
    fn write_file(name: &str, head: [u8; HEADER_LEN], payload: &[u8]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("prela_cache_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        let mut f = File::create(&path).unwrap();
        f.write_all(&head).unwrap();
        f.write_all(payload).unwrap();
        path
    }

    fn open_at(path: &Path, kind: u32) -> (usize, usize, &'static [u8]) {
        let bytes = mmap_static(path);
        let (n, m) = parse_header(bytes, kind, &format!("{path:?}"));
        (n, m, bytes)
    }

    #[test]
    fn dense_words_round_trip() {
        let vals: [u64; 3] = [7, NO_ID as u64, 0];
        let mut payload = Vec::new();
        for v in vals {
            payload.extend_from_slice(&v.to_le_bytes());
        }
        let p = write_file("ids.bin", header(KIND_DENSE_I64, 3, 0), &payload);
        let (n, _, bytes) = open_at(&p, KIND_DENSE_I64);
        let ids = cast_slice::<usize>(bytes, HEADER_LEN, n);
        assert_eq!(ids, &[7, NO_ID, 0]);
    }

    #[test]
    fn dense_str_round_trip() {
        // 3 slots: "ab", "" (hole), "c"
        let mut payload = Vec::new();
        for off in [0u32, 2, 2, 3] {
            payload.extend_from_slice(&off.to_le_bytes());
        }
        payload.extend_from_slice(b"abc");
        let p = write_file("strs.bin", header(KIND_DENSE_STR, 3, 3), &payload);
        let (n, m, bytes) = open_at(&p, KIND_DENSE_STR);
        let offsets = cast_slice::<u32>(bytes, HEADER_LEN, n + 1);
        let data = cast_slice::<u8>(bytes, HEADER_LEN + (n + 1) * 4, m);
        assert_eq!(strs_from_offsets(offsets, data), vec!["ab", "", "c"]);
    }

    #[test]
    fn csr_words_round_trip() {
        // rows: 0 → [5, 6], 1 → [], 2 → [9]
        let mut payload = Vec::new();
        for off in [0u32, 2, 2, 3] {
            payload.extend_from_slice(&off.to_le_bytes());
        }
        payload.resize(align8(HEADER_LEN + 4 * 4) - HEADER_LEN, 0); // pad
        for v in [5u64, 6, 9] {
            payload.extend_from_slice(&v.to_le_bytes());
        }
        let p = write_file("csr.bin", header(KIND_CSR_WORDS, 3, 3), &payload);
        let (n, m, bytes) = open_at(&p, KIND_CSR_WORDS);
        let offsets = cast_slice::<u32>(bytes, HEADER_LEN, n + 1);
        let values = cast_slice::<usize>(bytes, align8(HEADER_LEN + (n + 1) * 4), m);
        let rel = MultiRel::from_csr(offsets, values);
        let mut got = Vec::new();
        rel.drive(|k, v| got.push((k, v)));
        assert_eq!(got, vec![(0, 5), (0, 6), (2, 9)]);
    }

    #[test]
    #[should_panic(expected = "stale v1 cache")]
    fn v1_file_fails_loudly() {
        // a v1 file starts with its u64 pair count — no magic
        let mut payload = Vec::new();
        payload.extend_from_slice(&3u64.to_le_bytes());
        payload.extend_from_slice(&[0u8; 48]);
        let p = write_file("v1.bin", [0u8; HEADER_LEN], &[]);
        std::fs::write(&p, &payload).unwrap();
        open_at(&p, KIND_DENSE_I64);
    }

    #[test]
    #[should_panic(expected = "cache/loader mismatch")]
    fn kind_mismatch_fails_loudly() {
        let p = write_file("kind.bin", header(KIND_DENSE_F64, 0, 0), &[]);
        open_at(&p, KIND_DENSE_I64);
    }
}
