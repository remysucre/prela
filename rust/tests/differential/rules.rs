//! Value constraints for generated database schemas.
//!
//! Canonical Rust metadata owns names, types, nullability, keys, and references.
//! The separate DSL stores ordinary [`ValueRule`] trait objects which can only
//! narrow cell domains or relate values within a row.
//!
//! Generation exposes two value scopes:
//!
//! - [`ValueRule::cell`] narrows the domain for one scalar;
//! - [`ValueRule::row`] refines it using values already drawn in its row.

use super::generate::Cell;
use super::schema::{ColumnId, ScalarKind, Schema};
use std::collections::BTreeMap;
use std::fmt::Debug;

// Conservative defaults used when a field has no annotation.  They produce
// natural-looking values while keeping Hegel's search and shrinking space
// manageable.  An inline rule replaces the relevant bounds.
const NATURAL_MAX: i64 = 10_000;
const TEXT_MAX: usize = 32;

/// Fully resolved set of present values from which a cell may be generated.
///
/// NULL is intentionally absent from this enum.  Nullability comes from the
/// database generation, independently of the domain of non-NULL values.
#[derive(Clone, Debug, PartialEq)]
pub enum Domain {
    /// Inclusive integer range.
    I64 {
        /// Inclusive lower bound.
        min: i64,
        /// Inclusive upper bound.
        max: i64,
    },
    /// Text length range, optionally narrowed to an allowed vocabulary.
    Text {
        /// Minimum character count.
        min: usize,
        /// Maximum character count.
        max: usize,
        /// Optional finite vocabulary; `None` means arbitrary bounded text.
        values: Option<&'static [&'static str]>,
    },
    /// Foreign-key id referring to a row of `entity`.
    Reference {
        /// Logical name of the referenced entity.
        entity: &'static str,
    },
}

impl Domain {
    /// Select the unconstrained default domain implied by a declared type.
    fn new(kind: ScalarKind) -> Self {
        match kind {
            ScalarKind::I64 => Self::I64 {
                min: 0,
                max: NATURAL_MAX,
            },
            ScalarKind::Str => Self::Text {
                min: 0,
                max: TEXT_MAX,
                values: None,
            },
            ScalarKind::ForeignKey(entity) => Self::Reference { entity },
        }
    }
}

/// Mutable context passed through every cell-phase rule for one column.
///
/// Rules form a small refinement pipeline: `domain` begins with the default for
/// the column type, then matching rules replace or enrich it in declaration
/// order.  For example, `length` installs bounds and `values` preserves those
/// bounds while adding a vocabulary.
#[derive(Debug)]
pub struct CellCtx {
    /// Column currently being resolved.
    pub column: ColumnId,
    /// Current domain after all previously visited cell rules.
    pub domain: Domain,
}

impl CellCtx {
    /// Begin domain resolution from the field's declared scalar kind.
    pub(crate) fn new(schema: &Schema, column: ColumnId) -> Self {
        let kind = schema.column(column).expect("known schema column").kind;
        let domain = Domain::new(kind);
        Self { column, domain }
    }
}

/// Mutable context for rules relating one cell to values already drawn in its
/// row.
///
/// Row rules do not rewrite generated values.  They add inclusive bounds to
/// the current cell before Hegel draws it.  This makes relationships dependent
/// generators: for `left <= right`, whichever column appears second is drawn
/// from the part of its domain allowed by the first.
#[derive(Debug)]
pub struct RowCtx<'a> {
    /// Entity owning the row.  A global rule list uses this to ignore other
    /// tables cheaply.
    pub entity: &'static str,
    /// Column about to be generated.
    pub column: ColumnId,
    /// Cells generated earlier in this row.
    pub cells: &'a BTreeMap<ColumnId, Cell>,
    bounds: RowBounds,
}

impl<'a> RowCtx<'a> {
    /// Start resolving dependent bounds for one cell.
    pub(crate) fn new(
        entity: &'static str,
        column: ColumnId,
        cells: &'a BTreeMap<ColumnId, Cell>,
    ) -> Self {
        Self {
            entity,
            column,
            cells,
            bounds: RowBounds::default(),
        }
    }

    /// Require the current present value to compare at least `value`.
    ///
    /// A NULL dependency adds no bound because SQL comparisons involving NULL
    /// are UNKNOWN rather than constraint violations.
    pub fn at_least(&mut self, value: &Cell) {
        self.bounds.raise_lower(value);
    }

    /// Require the current present value to compare at most `value`.
    pub fn at_most(&mut self, value: &Cell) {
        self.bounds.tighten_upper(value);
    }

    pub(crate) fn finish(self) -> RowBounds {
        self.bounds
    }
}

/// Inclusive dynamic bounds accumulated by row rules for one cell.
#[derive(Debug, Default)]
pub(crate) struct RowBounds {
    pub lower: Option<Cell>,
    pub upper: Option<Cell>,
}

impl RowBounds {
    fn raise_lower(&mut self, value: &Cell) {
        if matches!(value, Cell::Null) {
            return;
        }
        if self
            .lower
            .as_ref()
            .is_none_or(|lower| compare_cells(lower, value).is_lt())
        {
            self.lower = Some(value.clone());
        }
    }

    fn tighten_upper(&mut self, value: &Cell) {
        if matches!(value, Cell::Null) {
            return;
        }
        if self
            .upper
            .as_ref()
            .is_none_or(|upper| compare_cells(upper, value).is_gt())
        {
            self.upper = Some(value.clone());
        }
    }

    /// Check values fixed by structural constraints against row constraints.
    pub(crate) fn contains(&self, value: &Cell) -> bool {
        if matches!(value, Cell::Null) {
            return true;
        }
        self.lower
            .as_ref()
            .is_none_or(|lower| !compare_cells(value, lower).is_lt())
            && self
                .upper
                .as_ref()
                .is_none_or(|upper| !compare_cells(value, upper).is_gt())
    }
}

/// Compare two non-NULL cells belonging to the same resolved domain.
fn compare_cells(left: &Cell, right: &Cell) -> std::cmp::Ordering {
    match (left, right) {
        (Cell::I64(left), Cell::I64(right)) => left.cmp(right),
        (Cell::Text(left), Cell::Text(right)) => left.cmp(right),
        _ => panic!("row constraint compared incompatible generated values"),
    }
}

/// A generation-only refinement of canonical column values.
///
/// This trait has no nullability, table, or database callback, so a value
/// overlay cannot redefine keys, foreign keys, or requiredness.
pub trait ValueRule: Debug + Sync {
    fn validate(&self, _schema: &Schema) -> Result<(), String> {
        Ok(())
    }

    fn cell(&self, _ctx: &mut CellCtx) {}

    fn row(&self, _ctx: &mut RowCtx<'_>) {}
}

/// Inclusive integer range rule for an `i64` field.
#[derive(Debug)]
pub struct I64 {
    /// Field being constrained.
    pub column: ColumnId,
    /// Inclusive lower bound.
    pub min: i64,
    /// Inclusive upper bound.
    pub max: i64,
}

impl ValueRule for I64 {
    fn validate(&self, schema: &Schema) -> Result<(), String> {
        require_kind(schema, self.column, ScalarKind::I64, "an integer")?;
        require_range(self.column, self.min <= self.max)
    }

    fn cell(&self, ctx: &mut CellCtx) {
        // Only the matching column is refined; every rule is stored in one
        // schema-wide list and therefore visits unrelated columns too.
        if ctx.column == self.column {
            ctx.domain = Domain::I64 {
                min: self.min,
                max: self.max,
            };
        }
    }
}

/// Inclusive character-count bounds for a text field.
#[derive(Debug)]
pub struct Length {
    /// Field being constrained.
    pub column: ColumnId,
    /// Minimum number of Unicode scalar values.
    pub min: usize,
    /// Maximum number of Unicode scalar values.
    pub max: usize,
}

impl ValueRule for Length {
    fn validate(&self, schema: &Schema) -> Result<(), String> {
        require_kind(schema, self.column, ScalarKind::Str, "text")?;
        require_range(self.column, self.min <= self.max)
    }

    fn cell(&self, ctx: &mut CellCtx) {
        if ctx.column == self.column {
            // Preserve a vocabulary installed by a preceding/following Values
            // rule.  Length and Values are independent refinements of Text.
            let values = match ctx.domain {
                Domain::Text { values, .. } => values,
                _ => None,
            };
            ctx.domain = Domain::Text {
                min: self.min,
                max: self.max,
                values,
            };
        }
    }
}

/// Finite allowed vocabulary for a text field.
#[derive(Debug)]
pub struct Values {
    /// Field being constrained.
    pub column: ColumnId,
    /// Nonempty set of values Hegel may sample.
    pub values: &'static [&'static str],
}

impl ValueRule for Values {
    fn validate(&self, schema: &Schema) -> Result<(), String> {
        require_kind(schema, self.column, ScalarKind::Str, "text")?;
        if self.values.is_empty() {
            return Err(format!(
                "{}.{} has no allowed values",
                self.column.entity, self.column.field
            ));
        }
        // Validate against the fully resolved domain so annotation order does
        // not let a vocabulary escape the field's length bounds.
        let Domain::Text { min, max, .. } = schema.domain(self.column) else {
            unreachable!()
        };
        if self
            .values
            .iter()
            .any(|value| !(min..=max).contains(&value.chars().count()))
        {
            return Err(format!(
                "{}.{} has an allowed value outside its length limits",
                self.column.entity, self.column.field
            ));
        }
        Ok(())
    }

    fn cell(&self, ctx: &mut CellCtx) {
        if ctx.column == self.column {
            // Preserve length bounds already installed by Length.
            let (min, max) = match ctx.domain {
                Domain::Text { min, max, .. } => (min, max),
                _ => (0, TEXT_MAX),
            };
            ctx.domain = Domain::Text {
                min,
                max,
                values: Some(self.values),
            };
        }
    }
}

/// In-row ordering rule declared by the value-constraint DSL.
#[derive(Debug)]
pub struct Order {
    /// Column that must compare less than or equal to `right`.
    pub left: ColumnId,
    /// Column that must compare greater than or equal to `left`.
    pub right: ColumnId,
}

impl ValueRule for Order {
    fn validate(&self, schema: &Schema) -> Result<(), String> {
        schema.require_column(self.left)?;
        schema.require_column(self.right)?;
        if self.left.entity != self.right.entity {
            return Err("ordered columns must belong to the same table".to_owned());
        }
        // Requiring identical resolved domains catches nonsensical comparisons
        // and guarantees that either side can provide a bound for the other.
        if schema.domain(self.left) != schema.domain(self.right) {
            return Err("ordered columns must use the same generation domain".to_owned());
        }
        if matches!(schema.domain(self.left), Domain::Reference { .. }) {
            return Err("foreign-key columns cannot use a row ordering rule".to_owned());
        }
        if matches!(schema.domain(self.left), Domain::Text { values: None, .. }) {
            return Err("ordered text columns require a finite values rule".to_owned());
        }
        Ok(())
    }

    fn row(&self, ctx: &mut RowCtx<'_>) {
        if self.left.entity != ctx.entity {
            return;
        }
        if ctx.column == self.right {
            if let Some(left) = ctx.cells.get(&self.left) {
                ctx.at_least(left);
            }
        } else if ctx.column == self.left {
            // This symmetric case makes the rule independent of declaration
            // order: if `right` was declared first, draw `left <= right`.
            if let Some(right) = ctx.cells.get(&self.right) {
                ctx.at_most(right);
            }
        }
    }
}

/// Check that an annotation is attached to the scalar type it expects.
fn require_kind(
    schema: &Schema,
    column: ColumnId,
    kind: ScalarKind,
    description: &str,
) -> Result<(), String> {
    let meta = schema.require_column(column)?;
    if meta.kind == kind {
        Ok(())
    } else {
        Err(format!(
            "{}.{} is not {description}",
            column.entity, column.field
        ))
    }
}

/// Common validation/error formatting for bounded domains.
fn require_range(column: ColumnId, valid: bool) -> Result<(), String> {
    if valid {
        Ok(())
    } else {
        Err(format!(
            "{}.{} has an empty or invalid range",
            column.entity, column.field
        ))
    }
}
