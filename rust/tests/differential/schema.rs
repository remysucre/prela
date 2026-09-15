//! Canonical schema metadata and value-constraint overlays used by tests.
//!
//! Benchmark modules own table names, fields, nullability, keys, and references.
//! [`Schema::overlay`] adds a separate list of generation rules
//! without permitting those rules to redefine schema structure.
//!
//! The small structural `entities!` helper materializes Rust table/column
//! metadata from ordinary field declarations. A benchmark's [`TableMeta`] list
//! supplies SQL table and id-column names, and extra value behavior crosses
//! the value-only `constraints!` boundary.

use super::rules::{CellCtx, Domain, ValueRule};
use std::collections::BTreeSet;

/// Synthetic logical column used for an entity's SQL primary-key value.
pub const ID_FIELD: &str = "__id";

/// Stable logical identity of one schema column.
///
/// The `entity` is the Rust type name and differs from the SQL table name; the
/// `field` is used verbatim as the SQL column name. Only table names and the
/// implicit id column are renamed, by [`TableMeta`].
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

    /// Primary key made from ordinary SQL-visible fields rather than `__id`.
    pub(crate) fn explicit_key(&self, entity: &str) -> Option<&'static [ColumnId]> {
        self.primary_key(entity)
            .filter(|columns| !columns.iter().any(|column| column.field == ID_FIELD))
    }

    /// Integer component used to make an explicit generated key unique.
    pub(crate) fn unique_integer_key(&self, entity: &str) -> Option<ColumnId> {
        self.explicit_key(entity)?.iter().copied().find(|column| {
            matches!(
                self.column(*column)
                    .expect("primary key column exists")
                    .kind,
                ScalarKind::I64
            )
        })
    }

    /// Single SQL key column referenced by a typed `Id<Entity>` field.
    /// Legacy schemas without declared key metadata may still expose a mapped
    /// identity column.
    pub(crate) fn reference_key(&self, entity: &str) -> Option<ColumnId> {
        match self.primary_key(entity) {
            Some(key) if key.len() == 1 => Some(key[0]),
            Some(_) => None,
            None => {
                let id = ColumnId::new(
                    self.tables.iter().copied().find(|table| *table == entity)?,
                    ID_FIELD,
                );
                self.sql_column(id).is_some().then_some(id)
            }
        }
    }

    /// Count the typed-id columns, each of which is one single-column
    /// foreign key.
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

    /// Return the SQL name for a logical column: [`TableMeta::id`] for the
    /// synthetic [`ID_FIELD`], and the logical field name itself otherwise.
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

    /// Logical columns of one entity in the stable order used both to build
    /// generated rows and to render `CREATE TABLE`/`INSERT` tuples.
    ///
    /// A SQL-visible identity column comes first. Row identity itself lives on
    /// [`Row`](crate::generate::Row), so entities without such a column do not
    /// need to invent one merely for the Prela adapter.
    pub(crate) fn entity_columns(&self, entity: &'static str) -> Vec<ColumnId> {
        let mut columns = Vec::new();
        if self.sql_column(ColumnId::new(entity, ID_FIELD)).is_some() {
            columns.push(ColumnId::new(entity, ID_FIELD));
        }
        columns.extend(
            self.columns
                .iter()
                .filter(|column| column.entity == entity)
                .map(|column| ColumnId::new(column.entity, column.field)),
        );
        columns
    }

    /// Validate schema shape and every rule before random generation starts.
    pub fn validate(&self) -> Result<(), String> {
        // Duplicate table names would make every name-based lookup ambiguous.
        let tables: BTreeSet<_> = self.tables.iter().copied().collect();
        if tables.len() != self.tables.len() {
            return Err("schema contains duplicate table names".to_owned());
        }
        // Macro-generated schemas satisfy this by construction, but
        // `Schema::canonical` is public so hand-written schemas receive the
        // same protection.
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
                if let Some(key) = self.primary_key(parent)
                    && key.len() != 1
                {
                    return Err(format!(
                        "column {}.{} is one scalar but {parent} has a {}-column primary key",
                        column.entity,
                        column.field,
                        key.len(),
                    ));
                }
                if let Some(target) = self.reference_key(parent)
                    && target.field != ID_FIELD
                    && !matches!(
                        self.require_column(target)?.kind,
                        ScalarKind::I64 | ScalarKind::ForeignKey(_)
                    )
                {
                    return Err(format!(
                        "column {}.{} cannot reference non-integer key {}.{}",
                        column.entity, column.field, target.entity, target.field,
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
/// SQL table and surrogate-key names deliberately cannot be expressed here. A
/// benchmark provides those in its own [`TableMeta`] list.
macro_rules! entities {
    (@entities [$($tables:expr,)*] [$($columns:expr,)*]) => {
        pub static TABLES: &[&str] = &[$($tables,)*];
        pub static COLUMNS: &[$crate::schema::ColumnMeta] = &[$($columns,)*];
    };

    (@entities
      [$($tables:expr,)*] [$($columns:expr,)*]
      pub struct $Ent:ident { $($fields:tt)* } $($rest:tt)*) => {
        #[allow(dead_code)]
        pub struct $Ent { $($fields)* }
        $crate::schema::entities!(@fields
            [$($tables,)* stringify!($Ent),]
            [$($columns,)*]
            $Ent { $($fields)* } $($rest)*);
    };

    (@fields
      [$($tables:expr,)*] [$($columns:expr,)*]
      $Ent:ident { } $($rest:tt)*) => {
        $crate::schema::entities!(@entities
            [$($tables,)*] [$($columns,)*] $($rest)*);
    };

    (@fields
      [$($tables:expr,)*] [$($columns:expr,)*]
      $Ent:ident {
          pub $field:ident: Col<$Owner:ident, Str>, $($fields:tt)*
      } $($rest:tt)*) => {
        $crate::schema::entities!(@fields
            [$($tables,)*]
            [$($columns,)* $crate::schema::ColumnMeta::new(
                stringify!($Ent), stringify!($field),
                $crate::schema::ScalarKind::Str),]
            $Ent { $($fields)* } $($rest)*);
    };

    (@fields
      [$($tables:expr,)*] [$($columns:expr,)*]
      $Ent:ident {
          pub $field:ident: Col<$Owner:ident, i64>, $($fields:tt)*
      } $($rest:tt)*) => {
        $crate::schema::entities!(@fields
            [$($tables,)*]
            [$($columns,)* $crate::schema::ColumnMeta::new(
                stringify!($Ent), stringify!($field),
                $crate::schema::ScalarKind::I64),]
            $Ent { $($fields)* } $($rest)*);
    };

    (@fields
      [$($tables:expr,)*] [$($columns:expr,)*]
      $Ent:ident {
          pub $field:ident: Col<$Owner:ident, Id<$Target:ident>>, $($fields:tt)*
      } $($rest:tt)*) => {
        $crate::schema::entities!(@fields
            [$($tables,)*]
            [$($columns,)* $crate::schema::ColumnMeta::new(
                stringify!($Ent), stringify!($field),
                $crate::schema::ScalarKind::ForeignKey(stringify!($Target))),]
            $Ent { $($fields)* } $($rest)*);
    };

    ($($body:tt)*) => {
        $crate::schema::entities!(@entities [] [] $($body)*);
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
        &$crate::rules::I64 {
            column: $crate::schema::ColumnId::new(stringify!($Ent), stringify!($field)),
            min: $min,
            max: $max,
        } as &dyn $crate::rules::ValueRule
    };
    ($Ent:ident.$field:ident : str => length($min:expr, $max:expr)) => {
        &$crate::rules::Length {
            column: $crate::schema::ColumnId::new(stringify!($Ent), stringify!($field)),
            min: $min,
            max: $max,
        } as &dyn $crate::rules::ValueRule
    };
    ($Ent:ident.$field:ident : str => values([$($value:expr),+ $(,)?])) => {
        &$crate::rules::Values {
            column: $crate::schema::ColumnId::new(stringify!($Ent), stringify!($field)),
            values: &[$($value),+],
        } as &dyn $crate::rules::ValueRule
    };
}

macro_rules! constraints {
    (
        pub static $name:ident for $base:expr;
        $($body:tt)*
    ) => {
        $crate::schema::constraints!(@collect $name, ($base), [] $($body)*);
    };

    (@collect $name:ident, ($base:expr), [$($rules:expr,)*]) => {
        pub static RULES: &[&dyn $crate::rules::ValueRule] = &[
            $($rules,)*
        ];
        pub static $name: $crate::schema::Schema =
            $crate::schema::Schema::overlay(&($base), RULES);
    };

    (@collect $name:ident, ($base:expr), [$($rules:expr,)*]
        $Ent:ident.$field:ident : $kind:tt => $rule:ident($($args:tt)*); $($rest:tt)*) => {
        $crate::schema::constraints!(@collect $name, ($base), [
            $($rules,)*
            $crate::schema::value_rule!(
                $Ent.$field: $kind => $rule($($args)*)
            ),
        ] $($rest)*);
    };

    (@collect $name:ident, ($base:expr), [$($rules:expr,)*]
        $Ent:ident.$left:ident <= $right:ident; $($rest:tt)*) => {
        $crate::schema::constraints!(@collect $name, ($base), [
            $($rules,)*
            &$crate::rules::Order {
                left: $crate::schema::ColumnId::new(
                    stringify!($Ent), stringify!($left)
                ),
                right: $crate::schema::ColumnId::new(
                    stringify!($Ent), stringify!($right)
                ),
            } as &dyn $crate::rules::ValueRule,
        ] $($rest)*);
    };
}

pub(crate) use constraints;
pub(crate) use value_rule;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::{I64, ValueRule};

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
        use crate::rules::ValueRule;
        use crate::schema::{ColumnId, ColumnMeta, ScalarKind, Schema};

        static COLUMNS: &[ColumnMeta] = &[ColumnMeta::new("Thing", "answer", ScalarKind::I64)];
        static BASE: Schema = Schema::canonical(&["Thing"], COLUMNS, &[], &[], &[]);
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
        static BASE: Schema = Schema::canonical(&["Thing"], &[], &[], &[], &[]);
        let schema = Schema::overlay(&BASE, RULES);
        assert!(schema.validate().is_err());
    }

    #[test]
    fn accepts_foreign_keys_to_a_single_explicit_parent_key() {
        static COLUMNS: &[ColumnMeta] = &[
            ColumnMeta::new("Parent", "key", ScalarKind::I64),
            ColumnMeta::new("Child", "parent", ScalarKind::ForeignKey("Parent")),
        ];
        static KEYS: &[PrimaryKey] =
            &[PrimaryKey::new("Parent", &[ColumnId::new("Parent", "key")])];
        let schema = Schema::canonical(&["Parent", "Child"], COLUMNS, &[], &[], KEYS);
        schema.validate().unwrap();
        assert_eq!(
            schema.reference_key("Parent"),
            Some(ColumnId::new("Parent", "key")),
        );
    }

    #[test]
    fn rejects_scalar_foreign_keys_to_composite_parent_keys() {
        static COLUMNS: &[ColumnMeta] = &[
            ColumnMeta::new("Parent", "left", ScalarKind::I64),
            ColumnMeta::new("Parent", "right", ScalarKind::I64),
            ColumnMeta::new("Child", "parent", ScalarKind::ForeignKey("Parent")),
        ];
        static KEYS: &[PrimaryKey] = &[PrimaryKey::new(
            "Parent",
            &[
                ColumnId::new("Parent", "left"),
                ColumnId::new("Parent", "right"),
            ],
        )];
        let schema = Schema::canonical(&["Parent", "Child"], COLUMNS, &[], &[], KEYS);
        assert_eq!(
            schema.validate().unwrap_err(),
            "column Child.parent is one scalar but Parent has a 2-column primary key",
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
