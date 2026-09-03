//! Generic, schema-directed generation of tiny logical databases.
//!
//! This module knows nothing about benchmark names, SQL syntax, or Prela's physical
//! relation types.  Its inputs are a logical
//! [`Schema`](crate::schema::Schema). Its output is a neutral
//! [`Database`] which an adapter can later encode for either engine.
//!
//! Generation is deliberately small (one to three rows per table).  Small
//! counterexamples are easier for a property-testing library to shrink and
//! easier for a person to understand when SQL and Prela disagree.

use super::rules::{Domain, RowBounds, RowCtx};
use super::schema::{ColumnId, ID_FIELD, ScalarKind, Schema};
use std::collections::BTreeMap;

/// Characters used for unconstrained text.  Besides ordinary alphanumerics it
/// includes spaces, SQL wildcard characters, and a quote, all of which exercise
/// useful escaping and predicate edge cases.
const TEXT_ALPHABET: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 _-%'";

/// Probability of drawing a nonempty string when both empty and nonempty text
/// satisfy the field's length rule.  Empty text remains a first-class boundary
/// value, but it no longer dominates ordinary generated fixtures.
const NONEMPTY_TEXT_PROBABILITY: f64 = 0.9;

/// Probability that an ordinary nullable cell is absent. The deliberately
/// high rate makes NULL-sensitive predicates and joins common rather than
/// occasional in differential fixtures.
const NULL_PROBABILITY: f64 = 0.5;

/// One scalar value in the engine-independent logical database.
///
/// `Null` is a logical absence, not a physical sentinel such as `0` or `""`.
/// The SQL adapter writes it as `NULL`; the eventual Prela adapter must omit the
/// corresponding `(row_id, value)` fact.
#[derive(Clone, Debug, PartialEq)]
pub enum Cell {
    /// No value is present for this row and column.
    Null,
    /// An ordinary integer value.
    I64(i64),
    /// Ordinary text.  The empty string is a real value, distinct from `Null`.
    Text(String),
    /// A typed reference to another generated row.
    Reference(RowId),
}

/// Stable identity of a row inside one generated database.
///
/// Adapters decide how this identity is represented physically. Prela uses the
/// row index as an `Id<E>`; SQL replaces it with the referenced row's declared
/// primary-key value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RowId {
    pub entity: &'static str,
    pub index: usize,
}

/// A logical row keyed by stable `(entity, field)` identifiers.
///
/// A map is more verbose than a positional vector, but it prevents adapters
/// and rules from silently disagreeing about column order.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    /// Adapter-independent identity of this row within its table.
    pub id: RowId,
    /// Every ordinary declared column. The synthetic `__id` is derived from
    /// `id` by adapters and is never stored as a scalar cell.
    pub cells: BTreeMap<ColumnId, Cell>,
}

/// One generated entity/table.
#[derive(Clone, Debug, PartialEq)]
pub struct Table {
    /// Logical Rust entity name, for example `"Lineitem"`.
    pub entity: &'static str,
    /// The generated rows, generally no more than three.
    pub rows: Vec<Row>,
}

/// Complete neutral fixture shared by the SQL and Prela adapters.
#[derive(Clone, Debug, PartialEq)]
pub struct Database {
    /// Tables appear in schema order.  Reference targets must precede sources,
    /// so every generated row reference names an existing parent table.
    pub tables: Vec<Table>,
}

/// Build a Hegel generator for small databases covering the complete schema.
///
/// There are four conceptual phases:
///
/// 1. choose table sizes;
/// 2. let database/table rules adjust sizes and plan references/keys;
/// 3. draw cells, forcing key/reference values where randomness alone would
///    violate a constraint;
/// 4. let row rules refine later cell domains from earlier cell values.
///
/// At least one nullable column is selected as a mandatory NULL target. Other
/// nullable columns independently become NULL occasionally. This guarantees
/// that a nullable schema exercises absence without making most rows useless
/// to multi-column predicates.
pub fn generator(schema: &'static Schema) -> impl hegel::Generator<Database> + Send + Sync {
    use hegel::generators as gs;

    // Validation is deterministic, so do it once while constructing the
    // generator rather than once per generated test case.
    schema.validate().expect("invalid generation schema");

    // Primary-key columns cannot be NULL. Row identity is stored separately,
    // so it is not part of this ordinary-column list.
    let nullable_columns: Vec<_> = schema
        .columns
        .iter()
        .map(|column| ColumnId::new(column.entity, column.field))
        .filter(|column| column.field != ID_FIELD && schema.is_nullable(*column))
        .collect();

    hegel::compose!(|tc| {
        // Pick one column whose first row is guaranteed to be NULL.
        // Hegel records this draw, so it remains part of shrinking/replay.
        let target = if nullable_columns.is_empty() {
            None
        } else {
            Some(tc.draw(gs::sampled_from(nullable_columns.clone())))
        };

        // Start with tiny independent sizes for every table, then cap tables
        // whose explicit key has a smaller finite unique-value space.
        let mut row_counts = BTreeMap::new();
        for &entity in schema.tables {
            let row_count = tc.draw(gs::integers::<usize>().min_value(1).max_value(3));
            row_counts.insert(entity, row_count);
        }
        for &entity in schema.tables {
            let capacity = explicit_key_capacity(schema, entity, &row_counts);
            row_counts.insert(entity, row_counts[entity].min(capacity));
        }
        // Tables are generated in declaration order.  This makes an earlier
        // referenced table available when a later source copies its key.
        let mut tables: Vec<Table> = Vec::with_capacity(schema.tables.len());
        for &entity in schema.tables {
            let row_count = row_counts[entity];
            let columns = schema.entity_columns(entity);
            let mut rows = Vec::with_capacity(row_count);
            for row_index in 0..row_count {
                let mut cells = BTreeMap::new();

                for &column in columns.iter().filter(|column| column.field != ID_FIELD) {
                    // Row annotations are interpreted generically. For an
                    // ordering rule, the value already in `cells` becomes a
                    // dynamic bound on this draw; no benchmark field names occur in
                    // the generator and no completed row is repaired later.
                    let bounds = {
                        let mut ctx = RowCtx::new(entity, column, &cells);
                        for rule in schema.rules {
                            rule.row(&mut ctx);
                        }
                        ctx.finish()
                    };
                    // Precedence is significant:
                    // 1. explicit key discriminators are unique;
                    // 2. all-reference composite keys enumerate unique tuples;
                    // 3. the mandatory NULL target wins for ordinary fields;
                    // 4. remaining fields are drawn from their domain.
                    let cell = if schema.unique_integer_key(entity) == Some(column) {
                        unique_integer_key_cell(schema, column, row_index)
                    } else if let Some(index) =
                        key_reference_index(schema, entity, column, row_index, &row_counts)
                    {
                        let ScalarKind::ForeignKey(parent) =
                            schema.column(column).expect("known key column").kind
                        else {
                            unreachable!()
                        };
                        Cell::Reference(RowId {
                            entity: parent,
                            index,
                        })
                    } else if row_index == 0 && target == Some(column) {
                        Cell::Null
                    } else {
                        draw_cell(
                            tc,
                            schema,
                            column,
                            schema.is_nullable(column),
                            &row_counts,
                            &bounds,
                        )
                    };
                    assert!(
                        bounds.contains(&cell),
                        "fixed value for {}.{} violates a row constraint",
                        column.entity,
                        column.field
                    );
                    cells.insert(column, cell);
                }
                rows.push(Row {
                    id: RowId {
                        entity,
                        index: row_index,
                    },
                    cells,
                });
            }
            tables.push(Table { entity, rows });
        }
        Database { tables }
    })
}

/// Draw one optional or present scalar from the column's rule-derived domain.
///
/// `nullable` comes from the schema's declared policy. It does not mean the
/// physical Prela column stores an `Option`; absence is represented only when
/// adapting this logical cell.
fn draw_cell(
    tc: &hegel::TestCase,
    schema: &Schema,
    column: ColumnId,
    nullable: bool,
    row_counts: &BTreeMap<&'static str, usize>,
    bounds: &RowBounds,
) -> Cell {
    use hegel::generators as gs;

    // Nullable columns independently have a 50% chance of absence, in
    // addition to the one mandatory target chosen by `generator`.
    if nullable && tc.draw(gs::weighted_booleans(NULL_PROBABILITY)) {
        return Cell::Null;
    }
    match schema.domain(column) {
        Domain::Text { min, max, values } => match values {
            // Enumerated business vocabularies (for example order status) are
            // sampled directly; otherwise generate arbitrary bounded text.
            Some(values) => {
                let values: Vec<_> = values
                    .iter()
                    .copied()
                    .filter(|value| bounds.contains(&Cell::Text((*value).to_owned())))
                    .collect();
                assert!(!values.is_empty(), "row constraint has no valid text value");
                Cell::Text(tc.draw(gs::sampled_from(values)).to_owned())
            }
            None => {
                assert!(
                    bounds.lower.is_none() && bounds.upper.is_none(),
                    "ordered free-form text requires a finite values rule"
                );
                // Hegel naturally favors and shrinks toward the shortest valid
                // text.  When `min == 0`, that made most ordinary fixtures full
                // of empty strings.  Treat emptiness as an explicit 10% edge
                // case and otherwise raise this individual draw's lower bound
                // to one.  A field constrained to exactly length zero still
                // always produces the empty string.
                let min = if min == 0 && max > 0 {
                    if !tc.draw(gs::weighted_booleans(NONEMPTY_TEXT_PROBABILITY)) {
                        return Cell::Text(String::new());
                    }
                    1
                } else {
                    min
                };
                Cell::Text(
                    tc.draw(
                        gs::text()
                            .alphabet(TEXT_ALPHABET)
                            .min_size(min)
                            .max_size(max),
                    ),
                )
            }
        },
        Domain::I64 { mut min, mut max } => {
            if let Some(Cell::I64(lower)) = &bounds.lower {
                min = min.max(*lower);
            }
            if let Some(Cell::I64(upper)) = &bounds.upper {
                max = max.min(*upper);
            }
            assert!(min <= max, "row constraint has an empty integer domain");
            Cell::I64(tc.draw(gs::integers::<i64>().min_value(min).max_value(max)))
        }
        Domain::Reference { entity } => {
            let parent_rows = row_counts.get(entity).copied().unwrap_or(0);
            let upper = parent_rows
                .checked_sub(1)
                .expect("referenced table has a row");
            Cell::Reference(RowId {
                entity,
                index: tc.draw(gs::integers::<usize>().min_value(0).max_value(upper)),
            })
        }
    }
}

/// Maximum number of rows whose declared explicit key can make unique.
fn explicit_key_capacity(
    schema: &Schema,
    entity: &'static str,
    row_counts: &BTreeMap<&'static str, usize>,
) -> usize {
    let Some(key) = schema.explicit_key(entity) else {
        return usize::MAX;
    };
    if let Some(column) = schema.unique_integer_key(entity) {
        let Domain::I64 { min, max } = schema.domain(column) else {
            unreachable!()
        };
        return usize::try_from(i128::from(max) - i128::from(min) + 1).unwrap_or(usize::MAX);
    }
    key.iter()
        .map(
            |column| match schema.column(*column).expect("validated key column").kind {
                ScalarKind::ForeignKey(parent) => row_counts[parent],
                _ => unreachable!("validated explicit key has a uniqueness strategy"),
            },
        )
        .product()
}

/// Give one integer component a deterministic distinct value in every row.
fn unique_integer_key_cell(schema: &Schema, column: ColumnId, row_index: usize) -> Cell {
    let Domain::I64 { min, .. } = schema.domain(column) else {
        unreachable!()
    };
    let offset = i64::try_from(row_index).expect("tiny row index fits i64");
    Cell::I64(
        min.checked_add(offset)
            .expect("validated key capacity fits i64"),
    )
}

/// Enumerate the Cartesian product for an all-reference composite key.
fn key_reference_index(
    schema: &Schema,
    entity: &'static str,
    column: ColumnId,
    row_index: usize,
    row_counts: &BTreeMap<&'static str, usize>,
) -> Option<usize> {
    let key = schema.explicit_key(entity)?;
    if schema.unique_integer_key(entity).is_some()
        || !key.iter().all(|column| {
            matches!(
                schema.column(*column).expect("validated key column").kind,
                ScalarKind::ForeignKey(_)
            )
        })
    {
        return None;
    }
    let position = key.iter().position(|candidate| *candidate == column)?;
    let stride = key[..position]
        .iter()
        .map(
            |column| match schema.column(*column).expect("validated key column").kind {
                ScalarKind::ForeignKey(parent) => row_counts[parent],
                _ => unreachable!(),
            },
        )
        .product::<usize>();
    let ScalarKind::ForeignKey(parent) = schema.column(column).expect("validated key column").kind
    else {
        unreachable!()
    };
    Some((row_index / stride) % row_counts[parent])
}

#[cfg(test)]
mod tests {
    use super::*;
    use hegel::TestCase;

    // A deliberately small schema proves the generator is generic and that
    // foreign-key domains are inferred from `Col<Child, Id<Parent>>` metadata.
    mod fixture {
        use crate::schema::{ColumnMeta, ScalarKind, Schema, constraints};

        static COLUMNS: &[ColumnMeta] = &[
            ColumnMeta::new("Parent", "name", ScalarKind::Str),
            ColumnMeta::new("Child", "parent", ScalarKind::ForeignKey("Parent")),
            ColumnMeta::new("Child", "count", ScalarKind::I64),
        ];
        static BASE: Schema = Schema::canonical(&["Parent", "Child"], COLUMNS, &[], &[], &[]);

        constraints! {
            pub static SCHEMA for BASE;
            Parent.name: str => length(1, 8);
            Child.count: i64 => range(1, 20);
        }
    }

    // A second small schema proves that row annotations—not
    // field names in the generator—create dependent Hegel draws.
    mod ordered_fixture {
        use crate::schema::{ColumnMeta, ScalarKind, Schema, constraints};

        static COLUMNS: &[ColumnMeta] = &[
            ColumnMeta::new("Interval", "lower", ScalarKind::I64),
            ColumnMeta::new("Interval", "upper", ScalarKind::I64),
        ];
        static BASE: Schema = Schema::canonical(&["Interval"], COLUMNS, &[], &[], &[]);

        constraints! {
            pub static SCHEMA for BASE;
            Interval.lower: i64 => range(-100, 100);
            Interval.upper: i64 => range(-100, 100);
            Interval.lower <= upper;
        }
    }

    mod explicit_key_fixture {
        use crate::schema::{ColumnId, ColumnMeta, PrimaryKey, ScalarKind, Schema, constraints};

        static COLUMNS: &[ColumnMeta] = &[
            ColumnMeta::new("Parent", "key", ScalarKind::I64),
            ColumnMeta::new("Child", "parent", ScalarKind::ForeignKey("Parent")),
        ];
        static REQUIRED: &[ColumnId] = &[ColumnId::new("Child", "parent")];
        static KEYS: &[PrimaryKey] =
            &[PrimaryKey::new("Parent", &[ColumnId::new("Parent", "key")])];
        static BASE: Schema = Schema::canonical(&["Parent", "Child"], COLUMNS, &[], REQUIRED, KEYS);

        constraints! {
            pub static SCHEMA for BASE;
            Parent.key: i64 => range(40, 42);
        }
    }

    mod reference_key_fixture {
        use crate::schema::{ColumnId, ColumnMeta, PrimaryKey, ScalarKind, Schema};

        static COLUMNS: &[ColumnMeta] = &[
            ColumnMeta::new("Link", "left", ScalarKind::ForeignKey("Left")),
            ColumnMeta::new("Link", "right", ScalarKind::ForeignKey("Right")),
        ];
        static KEYS: &[PrimaryKey] = &[PrimaryKey::new(
            "Link",
            &[
                ColumnId::new("Link", "left"),
                ColumnId::new("Link", "right"),
            ],
        )];
        pub static SCHEMA: Schema =
            Schema::canonical(&["Left", "Right", "Link"], COLUMNS, &[], &[], KEYS);
    }

    /// Generated scalar columns exercise NULL while present foreign keys remain
    /// valid references into the generated parent table.
    #[hegel::test(test_cases = 10)]
    fn generator_is_schema_directed(tc: TestCase) {
        let database = tc.draw(generator(&fixture::SCHEMA));
        let parent_column = ColumnId::new("Child", "parent");
        let parents = database
            .tables
            .iter()
            .find(|table| table.entity == "Parent")
            .unwrap();
        let children = database
            .tables
            .iter()
            .find(|table| table.entity == "Child")
            .unwrap();

        assert!(!parents.rows.is_empty());
        assert!(database.tables.iter().any(|table| {
            table
                .rows
                .iter()
                .any(|row| row.cells.values().any(|cell| matches!(cell, Cell::Null)))
        }));
        assert!(
            children
                .rows
                .iter()
                .all(|row| match row.cells[&parent_column] {
                    Cell::Reference(parent) => {
                        parent.entity == "Parent" && parent.index < parents.rows.len()
                    }
                    Cell::Null => true,
                    _ => false,
                })
        );
    }

    #[hegel::test(test_cases = 25)]
    fn explicit_keys_are_unique_and_references_name_rows(tc: TestCase) {
        let database = tc.draw(generator(&explicit_key_fixture::SCHEMA));
        let parents = database
            .tables
            .iter()
            .find(|table| table.entity == "Parent")
            .unwrap();
        let children = database
            .tables
            .iter()
            .find(|table| table.entity == "Child")
            .unwrap();
        let key = ColumnId::new("Parent", "key");
        let values = parents
            .rows
            .iter()
            .map(|row| match row.cells[&key] {
                Cell::I64(value) => value,
                ref other => panic!("integer key generated {other:?}"),
            })
            .collect::<std::collections::BTreeSet<_>>();

        assert_eq!(values.len(), parents.rows.len());
        assert!(children.rows.iter().all(|row| {
            matches!(
                row.cells[&ColumnId::new("Child", "parent")],
                Cell::Reference(RowId { entity: "Parent", index })
                    if index < parents.rows.len()
            )
        }));
    }

    #[hegel::test(test_cases = 25)]
    fn all_reference_composite_keys_are_unique(tc: TestCase) {
        let database = tc.draw(generator(&reference_key_fixture::SCHEMA));
        let links = database
            .tables
            .iter()
            .find(|table| table.entity == "Link")
            .unwrap();
        let left = ColumnId::new("Link", "left");
        let right = ColumnId::new("Link", "right");
        let keys = links
            .rows
            .iter()
            .map(|row| match (&row.cells[&left], &row.cells[&right]) {
                (Cell::Reference(left), Cell::Reference(right)) => (left.index, right.index),
                other => panic!("reference key generated {other:?}"),
            })
            .collect::<std::collections::BTreeSet<_>>();

        assert_eq!(keys.len(), links.rows.len());
    }

    /// The annotation turns the first Hegel result into the second draw's
    /// lower bound. No post-generation sorting or database-specific hook is
    /// involved.
    #[hegel::test(test_cases = 25)]
    fn row_order_annotations_drive_dependent_draws(tc: TestCase) {
        let lower = ColumnId::new("Interval", "lower");
        let upper = ColumnId::new("Interval", "upper");
        let counts = BTreeMap::from([("Interval", 1)]);
        let unbounded = RowBounds::default();
        let lower_cell = draw_cell(
            &tc,
            &ordered_fixture::SCHEMA,
            lower,
            false,
            &counts,
            &unbounded,
        );
        let cells = BTreeMap::from([(lower, lower_cell.clone())]);
        let bounds = {
            let mut ctx = RowCtx::new("Interval", upper, &cells);
            for rule in ordered_fixture::SCHEMA.rules {
                rule.row(&mut ctx);
            }
            ctx.finish()
        };
        let upper_cell = draw_cell(
            &tc,
            &ordered_fixture::SCHEMA,
            upper,
            false,
            &counts,
            &bounds,
        );

        let (Cell::I64(lower), Cell::I64(upper)) = (lower_cell, upper_cell) else {
            panic!("integer domains must generate integer cells")
        };
        assert!(lower <= upper);
    }
}
