//! Nullable relations backed by one generated differential-test fixture.
//!
//! Query implementations request only the typed relations they use. A present
//! cell contributes one fact and `NULL` contributes no fact. The relation
//! buffers belong to one adapted database, so every generated case is reclaimed
//! normally.

use super::generate::{Cell, Database};
use super::schema::ColumnId;
use crate::engine::{Dense, Drive, Id, Member, MultiRel, Probe, Query, Universe, VecRel};
use std::marker::PhantomData;

/// A partial, single-valued Prela column.
pub type Col<E, T> = OwnedMultiRel<T, Id<E>>;

/// Owned CSR relation used by differential fixtures.
pub struct OwnedMultiRel<R: Copy, D: Dense> {
    _domain: PhantomData<D>,
    offsets: Vec<u32>,
    values: Vec<R>,
}

impl<R: Copy, D: Dense> OwnedMultiRel<R, D> {
    fn new(offsets: Vec<u32>, values: Vec<R>) -> Self {
        assert!(
            !offsets.is_empty(),
            "CSR offsets must contain an initial zero"
        );
        assert_eq!(
            offsets.last().copied().unwrap_or_default() as usize,
            values.len(),
            "CSR offsets must end at the value count"
        );
        Self {
            _domain: PhantomData,
            offsets,
            values,
        }
    }

    pub fn n_keys(&self) -> usize {
        self.offsets.len() - 1
    }

    fn row(&self, key: usize) -> &[R] {
        if key < self.n_keys() {
            &self.values[self.offsets[key] as usize..self.offsets[key + 1] as usize]
        } else {
            &[]
        }
    }

    pub fn into_dense(self, name: &str) -> Result<VecRel<R, D>, String> {
        if self.offsets.windows(2).any(|row| row[1] - row[0] != 1) {
            return Err(format!("{name} is not a required single-valued column"));
        }
        Ok(VecRel::new(self.values))
    }
}

impl<R: Copy + 'static, D: Dense> OwnedMultiRel<R, D> {
    pub fn into_multi(self) -> MultiRel<R, D> {
        MultiRel::from_csr(
            Box::leak(self.offsets.into_boxed_slice()),
            Box::leak(self.values.into_boxed_slice()),
        )
    }
}

impl<R: Copy, D: Dense> Query for OwnedMultiRel<R, D> {
    type D = D;
    type R = R;
}

impl<R: Copy, D: Dense> Drive for OwnedMultiRel<R, D> {
    fn drive<K: FnMut(D, R)>(&self, mut emit: K) {
        for (index, window) in self.offsets.windows(2).enumerate() {
            for &value in &self.values[window[0] as usize..window[1] as usize] {
                emit(D::from_idx(index), value);
            }
        }
    }
}

impl<R: Copy, D: Dense> Member for OwnedMultiRel<R, D> {
    fn member(&self, key: D) -> bool {
        !self.row(key.idx()).is_empty()
    }
}

impl<R: Copy, D: Dense> Probe for OwnedMultiRel<R, D> {
    fn probe<K: FnMut(R)>(&self, key: D, mut emit: K) {
        for &value in self.row(key.idx()) {
            emit(value);
        }
    }

    fn probe_any<K: FnMut(R) -> bool>(&self, key: D, emit: K) -> bool {
        self.row(key.idx()).iter().copied().any(emit)
    }
}

/// On-demand typed access to a neutral generated database.
///
/// The entity type parameter keeps table identities distinct at compile time
/// without duplicating the schema as Rust adapter structs.
pub struct Relations<'data> {
    database: &'data Database,
}

impl<'data> Relations<'data> {
    pub fn new(database: &'data Database) -> Self {
        Self { database }
    }

    pub fn table<E: 'static>(&self, entity: &'static str) -> Result<Universe<Id<E>>, String> {
        let rows = self
            .database
            .tables
            .iter()
            .find(|table| table.entity == entity)
            .ok_or_else(|| format!("generated database has no {entity} table"))?
            .rows
            .len();
        Ok(Universe::new(rows))
    }

    pub fn static_text<E: 'static>(
        &self,
        entity: &'static str,
        field: &'static str,
    ) -> Result<Col<E, &'static str>, String> {
        // Production JOB columns borrow mmap-backed strings for the life of
        // the process. Tiny generated fixtures need the same representation.
        column(self.database, entity, field, |cell| match cell {
            Cell::Text(value) => Ok(&*Box::leak(value.clone().into_boxed_str())),
            other => Err(format!("{entity}.{field} expected text, got {other:?}")),
        })
    }

    pub fn integer<E: 'static>(
        &self,
        entity: &'static str,
        field: &'static str,
    ) -> Result<Col<E, i64>, String> {
        column(self.database, entity, field, |cell| match cell {
            Cell::I64(value) => Ok(*value),
            other => Err(format!("{entity}.{field} expected integer, got {other:?}")),
        })
    }

    pub fn foreign<E: 'static, T: 'static>(
        &self,
        entity: &'static str,
        field: &'static str,
    ) -> Result<Col<E, Id<T>>, String> {
        column(self.database, entity, field, |cell| match cell {
            Cell::I64(value) if *value > 0 => Ok(Id::new((*value - 1) as usize)),
            other => Err(format!(
                "{entity}.{field} expected positive foreign key, got {other:?}"
            )),
        })
    }
}

fn column<'data, T: Copy, E: 'static>(
    database: &'data Database,
    entity: &'static str,
    field: &'static str,
    mut convert: impl FnMut(&'data Cell) -> Result<T, String>,
) -> Result<Col<E, T>, String> {
    let table = database
        .tables
        .iter()
        .find(|table| table.entity == entity)
        .ok_or_else(|| format!("generated database has no {entity} table"))?;
    let id = ColumnId::new(entity, field);
    let mut offsets = Vec::with_capacity(table.rows.len() + 1);
    let mut values = Vec::new();
    offsets.push(0_u32);
    for row in &table.rows {
        let cell = row
            .cells
            .get(&id)
            .ok_or_else(|| format!("missing {entity}.{field}"))?;
        if !matches!(cell, Cell::Null) {
            values.push(convert(cell)?);
        }
        offsets.push(u32::try_from(values.len()).expect("tiny fixture fits u32"));
    }
    Ok(OwnedMultiRel::new(offsets, values))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::generate::{Row, Table};
    use std::collections::BTreeMap;

    enum Thing {}

    #[test]
    fn null_is_an_absent_fact_without_removing_the_row() {
        let value = ColumnId::new("Thing", "value");
        let database = Database {
            tables: vec![Table {
                entity: "Thing",
                rows: vec![
                    Row {
                        cells: BTreeMap::from([(value, Cell::Null)]),
                    },
                    Row {
                        cells: BTreeMap::from([(value, Cell::I64(7))]),
                    },
                ],
            }],
        };

        let values = Relations::new(&database)
            .integer::<Thing>("Thing", "value")
            .unwrap();
        let mut facts = Vec::new();
        values.drive(|key, value| facts.push((key.idx(), value)));

        assert_eq!(values.n_keys(), 2);
        assert_eq!(facts, vec![(1, 7)]);
    }
}
