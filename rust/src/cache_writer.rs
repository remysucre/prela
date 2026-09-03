//! Construction and serialization of Prela cache columns.
//!
//! Both benchmark regeneration and in-memory differential fixtures use these
//! buffers. Dense scatter, CSR construction, string layout, hole values, and
//! the on-disk encoding therefore have one implementation.

use crate::format::{HEADER_LEN, KIND_CSR_STR, KIND_CSR_WORDS, KIND_DENSE_STR, align8, header};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

#[derive(Clone, Default)]
pub struct WordColumn {
    pairs: Vec<(usize, u64)>,
}

impl WordColumn {
    pub fn push(&mut self, key: usize, value: u64) {
        self.pairs.push((key, value));
    }

    pub fn n_from_keys(&self) -> usize {
        self.pairs
            .iter()
            .map(|&(key, _)| key + 1)
            .max()
            .unwrap_or(0)
    }

    pub fn n_from_values(&self) -> usize {
        self.pairs
            .iter()
            .map(|&(_, value)| value as usize + 1)
            .max()
            .unwrap_or(0)
    }

    pub fn dense_words(self, n: usize, fill: u64) -> Vec<u64> {
        let mut values = vec![fill; n];
        for (key, value) in self.pairs {
            values[key] = value;
        }
        values
    }

    pub fn csr_words(self, n: usize) -> (Vec<u32>, Vec<u64>) {
        csr(n, self.pairs)
    }

    pub fn dense(self, n: usize, kind: u32, fill: u64) -> CacheColumn {
        CacheColumn::DenseWords {
            kind,
            values: self.dense_words(n, fill),
        }
    }

    pub fn multi(self, n: usize) -> CacheColumn {
        let (offsets, values) = self.csr_words(n);
        CacheColumn::CsrWords { offsets, values }
    }

    pub fn write_dense(self, path: &Path, n: usize, kind: u32, fill: u64) {
        self.dense(n, kind, fill).write(path);
    }
}

#[derive(Clone)]
pub struct StringColumn {
    keys: Vec<usize>,
    offsets: Vec<u32>,
    bytes: Vec<u8>,
}

impl Default for StringColumn {
    fn default() -> Self {
        Self {
            keys: Vec::new(),
            offsets: vec![0],
            bytes: Vec::new(),
        }
    }
}

impl StringColumn {
    pub fn push(&mut self, key: usize, value: &str) {
        self.keys.push(key);
        self.bytes.extend_from_slice(value.as_bytes());
        self.offsets
            .push(u32::try_from(self.bytes.len()).expect("string column > 4 GB"));
    }

    pub fn n_from_keys(&self) -> usize {
        self.keys.iter().map(|key| key + 1).max().unwrap_or(0)
    }

    fn string_at<'a>(offsets: &[u32], bytes: &'a [u8], index: usize) -> &'a str {
        let value = &bytes[offsets[index] as usize..offsets[index + 1] as usize];
        // Values entered through `push` were valid UTF-8 strings.
        unsafe { std::str::from_utf8_unchecked(value) }
    }

    pub fn dense(self, n: usize) -> CacheColumn {
        let mut chosen = vec![u32::MAX; n];
        for (index, key) in self.keys.iter().copied().enumerate() {
            chosen[key] = u32::try_from(index).expect("string column > u32::MAX values");
        }
        let mut offsets = Vec::with_capacity(n + 1);
        let mut bytes = Vec::new();
        offsets.push(0);
        for index in chosen {
            if index != u32::MAX {
                bytes.extend_from_slice(
                    Self::string_at(&self.offsets, &self.bytes, index as usize).as_bytes(),
                );
            }
            offsets.push(u32::try_from(bytes.len()).expect("string column > 4 GB"));
        }
        CacheColumn::DenseStrings { offsets, bytes }
    }

    pub fn multi(self, n: usize) -> CacheColumn {
        let pairs = self
            .keys
            .iter()
            .copied()
            .enumerate()
            .map(|(index, key)| (key, index))
            .collect();
        let (row_offsets, order) = csr(n, pairs);
        let mut string_offsets = Vec::with_capacity(order.len() + 1);
        let mut bytes = Vec::new();
        string_offsets.push(0);
        for index in order {
            bytes.extend_from_slice(Self::string_at(&self.offsets, &self.bytes, index).as_bytes());
            string_offsets.push(u32::try_from(bytes.len()).expect("string column > 4 GB"));
        }
        CacheColumn::CsrStrings {
            row_offsets,
            string_offsets,
            bytes,
        }
    }

    pub fn write_dense(self, path: &Path, n: usize) {
        self.dense(n).write(path);
    }
}

fn csr<R: Copy + Default>(n: usize, pairs: Vec<(usize, R)>) -> (Vec<u32>, Vec<R>) {
    let mut offsets = vec![0_u32; n + 1];
    let mut len = 0;
    for &(key, _) in &pairs {
        if key < n {
            offsets[key + 1] = offsets[key + 1]
                .checked_add(1)
                .expect("relation > u32::MAX pairs");
            len += 1;
        }
    }
    for index in 1..=n {
        offsets[index] = offsets[index]
            .checked_add(offsets[index - 1])
            .expect("relation > u32::MAX pairs");
    }
    let mut next = offsets.clone();
    let mut values = vec![R::default(); len];
    for (key, value) in pairs {
        if key < n {
            let slot = &mut next[key];
            values[*slot as usize] = value;
            *slot += 1;
        }
    }
    (offsets, values)
}

pub enum CacheColumn {
    DenseWords {
        kind: u32,
        values: Vec<u64>,
    },
    DenseStrings {
        offsets: Vec<u32>,
        bytes: Vec<u8>,
    },
    CsrWords {
        offsets: Vec<u32>,
        values: Vec<u64>,
    },
    CsrStrings {
        row_offsets: Vec<u32>,
        string_offsets: Vec<u32>,
        bytes: Vec<u8>,
    },
}

impl CacheColumn {
    pub fn n_dom(&self) -> usize {
        match self {
            Self::DenseWords { values, .. } => values.len(),
            Self::DenseStrings { offsets, .. } => offsets.len() - 1,
            Self::CsrWords { offsets, .. } => offsets.len() - 1,
            Self::CsrStrings { row_offsets, .. } => row_offsets.len() - 1,
        }
    }

    pub fn write(self, path: &Path) {
        match self {
            Self::DenseWords { kind, values } => {
                let mut file = out_file(path, kind, values.len(), 0);
                write_words(&mut file, &values);
            }
            Self::DenseStrings { offsets, bytes } => {
                let mut file = out_file(path, KIND_DENSE_STR, offsets.len() - 1, bytes.len());
                write_u32s(&mut file, &offsets);
                file.write_all(&bytes).unwrap();
            }
            Self::CsrWords { offsets, values } => {
                let n = offsets.len() - 1;
                let mut file = out_file(path, KIND_CSR_WORDS, n, values.len());
                write_u32s(&mut file, &offsets);
                pad_after_offsets(&mut file, n);
                write_words(&mut file, &values);
            }
            Self::CsrStrings {
                row_offsets,
                string_offsets,
                bytes,
            } => {
                let mut file = out_file(
                    path,
                    KIND_CSR_STR,
                    row_offsets.len() - 1,
                    string_offsets.len() - 1,
                );
                write_u32s(&mut file, &row_offsets);
                write_u32s(&mut file, &string_offsets);
                file.write_all(&bytes).unwrap();
            }
        }
    }

    fn into_memory(self) -> MemoryColumn {
        match self {
            Self::DenseWords { values, .. } => MemoryColumn::DenseWords(values),
            Self::DenseStrings { offsets, bytes } => {
                MemoryColumn::DenseStrings(strings(offsets, bytes))
            }
            Self::CsrWords { offsets, values } => MemoryColumn::CsrWords {
                offsets: Box::leak(offsets.into_boxed_slice()),
                values: Box::leak(values.into_boxed_slice()),
            },
            Self::CsrStrings {
                row_offsets,
                string_offsets,
                bytes,
            } => MemoryColumn::CsrStrings {
                offsets: Box::leak(row_offsets.into_boxed_slice()),
                values: Box::leak(strings(string_offsets, bytes).into_boxed_slice()),
            },
        }
    }
}

fn strings(offsets: Vec<u32>, bytes: Vec<u8>) -> Vec<&'static str> {
    let bytes = Box::leak(bytes.into_boxed_slice());
    offsets
        .windows(2)
        .map(|window| {
            let value = &bytes[window[0] as usize..window[1] as usize];
            unsafe { std::str::from_utf8_unchecked(value) }
        })
        .collect()
}

pub enum MemoryColumn {
    DenseWords(Vec<u64>),
    DenseStrings(Vec<&'static str>),
    CsrWords {
        offsets: &'static [u32],
        values: &'static [u64],
    },
    CsrStrings {
        offsets: &'static [u32],
        values: &'static [&'static str],
    },
}

impl MemoryColumn {
    pub fn n_dom(&self) -> usize {
        match self {
            Self::DenseWords(values) => values.len(),
            Self::DenseStrings(values) => values.len(),
            Self::CsrWords { offsets, .. } | Self::CsrStrings { offsets, .. } => offsets.len() - 1,
        }
    }
}

#[derive(Default)]
pub struct CacheColumns {
    columns: Vec<(String, CacheColumn)>,
}

impl CacheColumns {
    pub fn push(&mut self, name: &str, column: CacheColumn) {
        assert!(
            !self.columns.iter().any(|(existing, _)| existing == name),
            "duplicate cache column {name}"
        );
        self.columns.push((name.to_owned(), column));
    }

    pub fn write_all(self, directory: &Path) -> Vec<String> {
        std::fs::create_dir_all(directory).unwrap();
        let mut written = Vec::with_capacity(self.columns.len());
        for (name, column) in self.columns {
            column.write(&directory.join(format!("{name}.bin")));
            written.push(name);
        }
        written
    }

    pub fn into_memory(self) -> BTreeMap<String, MemoryColumn> {
        self.columns
            .into_iter()
            .map(|(name, column)| (name, column.into_memory()))
            .collect()
    }
}

fn out_file(path: &Path, kind: u32, n: usize, m: usize) -> BufWriter<File> {
    let mut file = BufWriter::new(File::create(path).unwrap());
    file.write_all(&header(kind, n as u64, m as u64)).unwrap();
    file
}

fn write_words(file: &mut BufWriter<File>, words: &[u64]) {
    let bytes = unsafe {
        std::slice::from_raw_parts(words.as_ptr().cast::<u8>(), std::mem::size_of_val(words))
    };
    file.write_all(bytes).unwrap();
}

fn write_u32s(file: &mut BufWriter<File>, values: &[u32]) {
    let bytes = unsafe {
        std::slice::from_raw_parts(values.as_ptr().cast::<u8>(), std::mem::size_of_val(values))
    };
    file.write_all(bytes).unwrap();
}

fn pad_after_offsets(file: &mut BufWriter<File>, n: usize) {
    let end = HEADER_LEN + (n + 1) * size_of::<u32>();
    file.write_all(&vec![0_u8; align8(end) - end]).unwrap();
}
