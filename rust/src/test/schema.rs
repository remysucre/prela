//! Canonical schema metadata and value-constraint overlays used by tests.
//!
//! Benchmark modules own table names, fields, nullability, keys, and references.
//! [`Schema::overlay`] adds a separate list of generation rules
//! without permitting those rules to redefine schema structure.
//!
//! The small structural `entities!` helper materializes Rust table/column
//! metadata from ordinary field declarations. Physical SQL names live in the
//! benchmark's separate adapter registry, and extra value behavior crosses the
//! value-only `constraints!` boundary.

use super::rules::{CellCtx, Domain, ValueRule};
use std::collections::BTreeSet;

/// Synthetic logical column used for an entity's SQL primary-key value.
pub const ID_FIELD: &str = "__id";

/// Stable logical identity of one schema column.
///
/// These are Rust/logical names (`Lineitem.shipdate`), not physical SQL names
/// (`l_shipdate`). Database adapters own that translation.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ColumnId {
    pub entity: &'static str,
    pub field: &'static str,
}

impl ColumnId {
    pub const fn new(entity: &'static str, field: &'static str) -> Self {
        Self { entity, field }
    }
}

/// Scalar information the generator can recover from a `Col<Entity, T>` type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScalarKind {
    /// Prela's borrowed string scalar (`Str`).
    Str,
    /// Signed integer scalar.
    I64,
    /// Typed entity id.  The target entity name determines its value domain.
    ForeignKey(&'static str),
}

/// Canonical primary key for one table.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PrimaryKey {
    pub entity: &'static str,
    pub columns: &'static [ColumnId],
}

impl PrimaryKey {
    pub const fn new(entity: &'static str, columns: &'static [ColumnId]) -> Self {
        Self { entity, columns }
    }
}

/// Runtime metadata for one field in a canonical Rust schema.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ColumnMeta {
    /// Owning entity name.
    pub entity: &'static str,
    /// Rust field name.
    pub field: &'static str,
    /// Logical scalar kind inferred by the macro from the `Col` type.
    pub kind: ScalarKind,
}

impl ColumnMeta {
    /// Const constructor used in macro-generated static slices.
    pub const fn new(entity: &'static str, field: &'static str, kind: ScalarKind) -> Self {
        Self {
            entity,
            field,
            kind,
        }
    }
}

/// Exact SQL names belonging to one logical entity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TableMeta {
    /// Rust/logical entity name.
    pub entity: &'static str,
    /// SQL base-table name.
    pub sql: &'static str,
    /// SQL name of the implicit `Id<E>` column, when the entity has one.
    pub id: Option<&'static str>,
}

impl TableMeta {
    /// Const constructor used by a benchmark's canonical SQL adapter.
    pub const fn new(entity: &'static str, sql: &'static str, id: Option<&'static str>) -> Self {
        Self { entity, sql, id }
    }
}

/// Complete logical description consumed by database generation.
///
/// All data is static because it is emitted from schema declarations.  Rules
/// are trait objects so user-defined rule types can coexist with built-ins.
#[derive(Debug)]
pub struct Schema {
    /// Entity names in dependency-friendly generation order.
    pub tables: &'static [&'static str],
    /// All declared fields, in entity and declaration order.
    pub columns: &'static [ColumnMeta],
    /// Exact SQL table names and implicit-id names declared by the adapter.
    pub sql: &'static [TableMeta],
    /// Value constraints supplied by an overlay.
    pub rules: &'static [&'static dyn ValueRule],
    /// Structurally required non-key columns.
    pub required: &'static [ColumnId],
    /// Canonical primary keys, including implicit `__id` keys.
    pub primary_keys: &'static [PrimaryKey],
}

impl Schema {
    /// Const constructor used by the macro's generated `SCHEMA` static.
    pub const fn new(
        tables: &'static [&'static str],
        columns: &'static [ColumnMeta],
        rules: &'static [&'static dyn ValueRule],
    ) -> Self {
        Self {
            tables,
            columns,
            sql: &[],
            rules,
            required: &[],
            primary_keys: &[],
        }
    }

    /// Construct a canonical benchmark schema. Value-generation constraints
    /// deliberately live elsewhere and can be applied with [`Schema::overlay`].
    pub const fn canonical(
        tables: &'static [&'static str],
        columns: &'static [ColumnMeta],
        sql: &'static [TableMeta],
        required: &'static [ColumnId],
        primary_keys: &'static [PrimaryKey],
    ) -> Self {
        Self {
            tables,
            columns,
            sql,
            rules: &[],
            required,
            primary_keys,
        }
    }

    /// Add value-domain constraints without redeclaring or overriding schema
    /// structure. This is the boundary used by the value-only constraints DSL.
    pub const fn overlay(base: &'static Self, rules: &'static [&'static dyn ValueRule]) -> Self {
        Self {
            tables: base.tables,
            columns: base.columns,
            sql: base.sql,
            rules,
            required: base.required,
            primary_keys: base.primary_keys,
        }
    }

    /// Canonical key for an entity, if one is declared.
    pub fn primary_key(&self, entity: &str) -> Option<&'static [ColumnId]> {
        self.primary_keys
            .iter()
            .find(|key| key.entity == entity)
            .map(|key| key.columns)
    }

    /// Iterate canonical foreign keys, expanding typed single-column ids.
    pub fn foreign_key_count(&self) -> usize {
        self.columns
            .iter()
            .filter(|column| matches!(column.kind, ScalarKind::ForeignKey(_)))
            .count()
    }

    /// Find metadata for a stable logical column id.
    pub fn column(&self, id: ColumnId) -> Option<&ColumnMeta> {
        self.columns
            .iter()
            .find(|column| column.entity == id.entity && column.field == id.field)
    }

    /// Return the declared SQL base-table name for a logical entity.
    pub fn sql_table(&self, entity: &'static str) -> Option<&'static str> {
        self.sql
            .iter()
            .find(|table| table.entity == entity)
            .map(|table| table.sql)
    }

    /// Return the declared SQL name for a logical column.
    pub fn sql_column(&self, id: ColumnId) -> Option<&'static str> {
        if id.field == ID_FIELD {
            return self
                .sql
                .iter()
                .find(|table| table.entity == id.entity)
                .and_then(|table| table.id);
        }
        self.column(id).map(|column| column.field)
    }

    /// Resolve a column's complete present-value domain.
    ///
    /// Resolution begins from [`ScalarKind`]'s defaults, then passes the same
    /// [`CellCtx`] through all rules.  Unrelated rules are cheap no-ops.
    pub fn domain(&self, column: ColumnId) -> Domain {
        let mut ctx = CellCtx::new(self, column);
        for rule in self.rules {
            rule.cell(&mut ctx);
        }
        ctx.domain
    }

    /// Whether generation may produce `NULL` for this column.
    ///
    /// Ordinary columns are nullable unless marked required. Declared key
    /// columns are always non-null.
    pub fn is_nullable(&self, column: ColumnId) -> bool {
        !self.is_structural(column) && !self.required.contains(&column)
    }

    /// Whether a column participates in a declared key and therefore must not
    /// be selected as the generator's forced NULL target.
    ///
    pub fn is_structural(&self, column: ColumnId) -> bool {
        if column.field == ID_FIELD && self.sql_column(column).is_some() {
            return true;
        }
        if self
            .primary_keys
            .iter()
            .any(|key| key.columns.contains(&column))
        {
            return true;
        }
        false
    }

    /// Whether an entity declares a key made from ordinary fields.
    ///
    /// Entities without such a declaration use their implicit Prela `Id<E>` as
    /// the logical `__id` column.  This lets adapters discover key shape from
    /// schema structure instead of maintaining a database-specific table list.
    pub(crate) fn has_key(&self, entity: &'static str) -> bool {
        if let Some(columns) = self.primary_key(entity) {
            return !columns.iter().any(|column| column.field == ID_FIELD);
        }
        false
    }

    /// Validate schema shape and every rule before random generation starts.
    pub fn validate(&self) -> Result<(), String> {
        // Duplicate table names would make every name-based lookup ambiguous.
        let tables: BTreeSet<_> = self.tables.iter().copied().collect();
        if tables.len() != self.tables.len() {
            return Err("schema contains duplicate table names".to_owned());
        }
        // Macro-generated schemas satisfy this by construction, but `Schema::new`
        // is public so hand-written/custom schemas receive the same protection.
        let mut column_ids = BTreeSet::new();
        for column in self.columns {
            if !tables.contains(column.entity) {
                return Err(format!(
                    "column {}.{} belongs to an unknown table",
                    column.entity, column.field
                ));
            }
            if !column_ids.insert(ColumnId::new(column.entity, column.field)) {
                return Err(format!(
                    "duplicate logical column {}.{}",
                    column.entity, column.field
                ));
            }
            if let ScalarKind::ForeignKey(parent) = column.kind {
                if !tables.contains(parent) {
                    return Err(format!(
                        "column {}.{} references unknown table {parent}",
                        column.entity, column.field
                    ));
                }
                let source_position = self
                    .tables
                    .iter()
                    .position(|entity| *entity == column.entity)
                    .expect("validated source table");
                let parent_position = self
                    .tables
                    .iter()
                    .position(|entity| *entity == parent)
                    .expect("validated parent table");
                if parent_position >= source_position {
                    return Err(format!(
                        "column {}.{} references {parent}, which must be declared first",
                        column.entity, column.field
                    ));
                }
                if self.has_key(parent) {
                    return Err(format!(
                        "column {}.{} references {parent}'s implicit id, but {parent} has an explicit key",
                        column.entity, column.field
                    ));
                }
            }
        }
        let mut seen = BTreeSet::new();
        for &column in self.required {
            self.require_column(column)?;
            if !seen.insert(column) {
                return Err(format!(
                    "duplicate required column {}.{}",
                    column.entity, column.field
                ));
            }
        }
        let mut keyed_tables = BTreeSet::new();
        for key in self.primary_keys {
            if key.columns.is_empty() {
                return Err(format!("primary key for {} is empty", key.entity));
            }
            if !tables.contains(key.entity) {
                return Err(format!(
                    "primary key belongs to unknown table {}",
                    key.entity
                ));
            }
            if !keyed_tables.insert(key.entity) {
                return Err(format!("duplicate primary key for {}", key.entity));
            }
            let mut fields = BTreeSet::new();
            for &column in key.columns {
                if column.entity != key.entity || !fields.insert(column.field) {
                    return Err(format!("invalid primary key for {}", key.entity));
                }
                if column.field == ID_FIELD {
                    if key.columns.len() != 1 || self.sql_column(column).is_none() {
                        return Err(format!("invalid implicit primary key for {}", key.entity));
                    }
                } else {
                    self.require_column(column)?;
                }
            }
            let implicit = key.columns.iter().any(|column| column.field == ID_FIELD);
            let all_foreign = key.columns.iter().all(|column| {
                self.column(*column)
                    .is_some_and(|column| matches!(column.kind, ScalarKind::ForeignKey(_)))
            });
            let has_integer_component = key
                .columns
                .iter()
                .filter(|column| column.field != ID_FIELD)
                .any(|column| matches!(self.domain(*column), Domain::I64 { .. }));
            if !implicit && !all_foreign && !has_integer_component {
                return Err(format!(
                    "primary key for {} cannot be generated uniquely",
                    key.entity
                ));
            }
        }
        // Explicit SQL names must cover each logical table once and be unique
        // within the namespace where SQL resolves them.
        if !self.sql.is_empty() {
            if self.sql.len() != self.tables.len() {
                return Err("SQL metadata must name every table exactly once".to_owned());
            }
            let mut entities = BTreeSet::new();
            let mut sql_tables = BTreeSet::new();
            for table in self.sql {
                if !tables.contains(table.entity) || !entities.insert(table.entity) {
                    return Err(format!("invalid SQL table metadata for {}", table.entity));
                }
                if !sql_tables.insert(table.sql.to_ascii_lowercase()) {
                    return Err(format!("duplicate SQL table name {}", table.sql));
                }
            }
            for &entity in self.tables {
                let mut names = BTreeSet::new();
                if let Some(id) = self
                    .sql
                    .iter()
                    .find(|table| table.entity == entity)
                    .and_then(|table| table.id)
                {
                    names.insert(id.to_ascii_lowercase());
                }
                for column in self.columns.iter().filter(|column| column.entity == entity) {
                    if !names.insert(column.field.to_ascii_lowercase()) {
                        return Err(format!(
                            "duplicate SQL column name {} in {}",
                            column.field, entity
                        ));
                    }
                }
            }
        }
        // Overlay-specific checks include value types and ranges.
        for rule in self.rules {
            rule.validate(self)?;
        }
        Ok(())
    }

    /// Lookup variant used by value-rule validation, with a useful error.
    pub(crate) fn require_column(&self, id: ColumnId) -> Result<&ColumnMeta, String> {
        self.column(id)
            .ok_or_else(|| format!("rule names unknown column {}.{}", id.entity, id.field))
    }
}

/// Materialize ordinary Rust entity declarations as logical metadata.
///
/// Physical table and surrogate-key names deliberately cannot be expressed
/// here. A benchmark provides those in its canonical SQL adapter registry.
macro_rules! entities {
    (@entities [$($tables:expr,)*] [$($columns:expr,)*]) => {
        pub static TABLES: &[&str] = &[$($tables,)*];
        pub static COLUMNS: &[$crate::test::schema::ColumnMeta] = &[$($columns,)*];
    };

    (@entities
      [$($tables:expr,)*] [$($columns:expr,)*]
      pub struct $Ent:ident { $($fields:tt)* } $($rest:tt)*) => {
        #[allow(dead_code)]
        pub struct $Ent { $($fields)* }
        $crate::test::schema::entities!(@fields
            [$($tables,)* stringify!($Ent),]
            [$($columns,)*]
            $Ent { $($fields)* } $($rest)*);
    };

    (@fields
      [$($tables:expr,)*] [$($columns:expr,)*]
      $Ent:ident { } $($rest:tt)*) => {
        $crate::test::schema::entities!(@entities
            [$($tables,)*] [$($columns,)*] $($rest)*);
    };

    (@fields
      [$($tables:expr,)*] [$($columns:expr,)*]
      $Ent:ident {
          pub $field:ident: Col<$Owner:ident, Str>, $($fields:tt)*
      } $($rest:tt)*) => {
        $crate::test::schema::entities!(@fields
            [$($tables,)*]
            [$($columns,)* $crate::test::schema::ColumnMeta::new(
                stringify!($Ent), stringify!($field),
                $crate::test::schema::ScalarKind::Str),]
            $Ent { $($fields)* } $($rest)*);
    };

    (@fields
      [$($tables:expr,)*] [$($columns:expr,)*]
      $Ent:ident {
          pub $field:ident: Col<$Owner:ident, i64>, $($fields:tt)*
      } $($rest:tt)*) => {
        $crate::test::schema::entities!(@fields
            [$($tables,)*]
            [$($columns,)* $crate::test::schema::ColumnMeta::new(
                stringify!($Ent), stringify!($field),
                $crate::test::schema::ScalarKind::I64),]
            $Ent { $($fields)* } $($rest)*);
    };

    (@fields
      [$($tables:expr,)*] [$($columns:expr,)*]
      $Ent:ident {
          pub $field:ident: Col<$Owner:ident, Id<$Target:ident>>, $($fields:tt)*
      } $($rest:tt)*) => {
        $crate::test::schema::entities!(@fields
            [$($tables,)*]
            [$($columns,)* $crate::test::schema::ColumnMeta::new(
                stringify!($Ent), stringify!($field),
                $crate::test::schema::ScalarKind::ForeignKey(stringify!($Target))),]
            $Ent { $($fields)* } $($rest)*);
    };

    ($($body:tt)*) => {
        $crate::test::schema::entities!(@entities [] [] $($body)*);
    };
}

pub(crate) use entities;

/// Declare value-generation constraints over a canonical Rust schema.
///
/// This deliberately has no syntax for SQL names, types, nullability, keys, or
/// references: those belong exclusively to the canonical schema. The explicit
/// scalar token is checked by the existing rule validators. A whitelist macro
/// below makes structural rules such as `not_null`, `key`, and `reference`
/// impossible to spell in this DSL.
macro_rules! value_rule {
    ($Ent:ident.$field:ident : i64 => range($min:expr, $max:expr)) => {
        &$crate::test::rules::I64 {
            column: $crate::test::schema::ColumnId::new(stringify!($Ent), stringify!($field)),
            min: $min,
            max: $max,
        } as &dyn $crate::test::rules::ValueRule
    };
    ($Ent:ident.$field:ident : str => length($min:expr, $max:expr)) => {
        &$crate::test::rules::Length {
            column: $crate::test::schema::ColumnId::new(stringify!($Ent), stringify!($field)),
            min: $min,
            max: $max,
        } as &dyn $crate::test::rules::ValueRule
    };
    ($Ent:ident.$field:ident : str => values([$($value:expr),+ $(,)?])) => {
        &$crate::test::rules::Values {
            column: $crate::test::schema::ColumnId::new(stringify!($Ent), stringify!($field)),
            values: &[$($value),+],
        } as &dyn $crate::test::rules::ValueRule
    };
}

macro_rules! constraints {
    (
        pub static $name:ident for $base:expr;
        $($body:tt)*
    ) => {
        $crate::test::schema::constraints!(@collect $name, ($base), [] $($body)*);
    };

    (@collect $name:ident, ($base:expr), [$($rules:expr,)*]) => {
        pub static RULES: &[&dyn $crate::test::rules::ValueRule] = &[
            $($rules,)*
        ];
        pub static $name: $crate::test::schema::Schema =
            $crate::test::schema::Schema::overlay(&($base), RULES);
    };

    (@collect $name:ident, ($base:expr), [$($rules:expr,)*]
        $Ent:ident.$field:ident : $kind:tt => $rule:ident($($args:tt)*); $($rest:tt)*) => {
        $crate::test::schema::constraints!(@collect $name, ($base), [
            $($rules,)*
            $crate::test::schema::value_rule!(
                $Ent.$field: $kind => $rule($($args)*)
            ),
        ] $($rest)*);
    };

    (@collect $name:ident, ($base:expr), [$($rules:expr,)*]
        $Ent:ident.$left:ident <= $right:ident; $($rest:tt)*) => {
        $crate::test::schema::constraints!(@collect $name, ($base), [
            $($rules,)*
            &$crate::test::rules::Order {
                left: $crate::test::schema::ColumnId::new(
                    stringify!($Ent), stringify!($left)
                ),
                right: $crate::test::schema::ColumnId::new(
                    stringify!($Ent), stringify!($right)
                ),
            } as &dyn $crate::test::rules::ValueRule,
        ] $($rest)*);
    };
}

pub(crate) use constraints;
pub(crate) use value_rule;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::rules::{I64, ValueRule};

    /// Example of extending the rule system without modifying its built-ins.
    #[derive(Debug)]
    struct FortyTwo(ColumnId);

    impl ValueRule for FortyTwo {
        fn cell(&self, ctx: &mut CellCtx) {
            if ctx.column == self.0 {
                ctx.domain = Domain::I64 { min: 42, max: 42 };
            }
        }
    }

    // The typed overlay accepts an ordinary user-defined value refinement.
    mod fixture {
        use super::FortyTwo;
        use crate::test::rules::ValueRule;
        use crate::test::schema::{ColumnId, ColumnMeta, ScalarKind, Schema};

        static COLUMNS: &[ColumnMeta] = &[ColumnMeta::new("Thing", "answer", ScalarKind::I64)];
        static BASE: Schema = Schema::new(&["Thing"], COLUMNS, &[]);
        static ANSWER: FortyTwo = FortyTwo(ColumnId::new("Thing", "answer"));
        static RULES: &[&dyn ValueRule] = &[&ANSWER];
        pub static SCHEMA: Schema = Schema::overlay(&BASE, RULES);
    }

    #[test]
    /// Hand-built schemas are validated just like macro-generated schemas.
    fn rejects_rules_for_unknown_columns() {
        static RANGE: I64 = I64 {
            column: ColumnId::new("Thing", "missing"),
            min: 0,
            max: 1,
        };
        static RULES: &[&dyn ValueRule] = &[&RANGE];
        let schema = Schema::new(&["Thing"], &[], RULES);
        assert!(schema.validate().is_err());
    }

    #[test]
    fn rejects_foreign_keys_without_an_implicit_parent_id() {
        static COLUMNS: &[ColumnMeta] = &[
            ColumnMeta::new("Parent", "key", ScalarKind::I64),
            ColumnMeta::new("Child", "parent", ScalarKind::ForeignKey("Parent")),
        ];
        static KEYS: &[PrimaryKey] =
            &[PrimaryKey::new("Parent", &[ColumnId::new("Parent", "key")])];
        let schema = Schema::canonical(&["Parent", "Child"], COLUMNS, &[], &[], KEYS);
        assert_eq!(
            schema.validate().unwrap_err(),
            "column Child.parent references Parent's implicit id, but Parent has an explicit key"
        );
    }

    #[test]
    /// The custom rule participates in the normal cell-domain pipeline.
    fn custom_rules_can_extend_cell_generation() {
        assert_eq!(
            fixture::SCHEMA.domain(ColumnId::new("Thing", "answer")),
            Domain::I64 { min: 42, max: 42 }
        );
    }
}
