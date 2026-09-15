//! Typed query results and explicit equality policies for differential tests.
//!
//! Rendering belongs only to failure diagnostics. Equality operates on cells,
//! so SQL `NULL`, text, numbers, and dates cannot alias through formatting.

use std::fmt;

/// One typed cell returned by either query implementation.
#[derive(Clone, Debug)]
pub enum ResultCell {
    Null,
    Boolean(bool),
    Integer(i128),
    /// Binary floating-point value. Values are not rounded for equality.
    Float(f64),
    /// Exact fixed-point decimal, normalized to remove insignificant trailing
    /// zeroes from the mantissa.
    Decimal {
        mantissa: i128,
        scale: u32,
    },
    Text(String),
    /// Packed Gregorian date in `yyyymmdd` form.
    Date(i64),
}

impl PartialEq for ResultCell {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Null, Self::Null) => true,
            (Self::Boolean(left), Self::Boolean(right)) => left == right,
            (Self::Integer(left), Self::Integer(right)) => left == right,
            (Self::Float(left), Self::Float(right)) => {
                left == right || (left.is_nan() && right.is_nan())
            }
            (
                Self::Decimal {
                    mantissa: left_mantissa,
                    scale: left_scale,
                },
                Self::Decimal {
                    mantissa: right_mantissa,
                    scale: right_scale,
                },
            ) => left_mantissa == right_mantissa && left_scale == right_scale,
            (Self::Text(left), Self::Text(right)) => left == right,
            (Self::Date(left), Self::Date(right)) => left == right,
            _ => false,
        }
    }
}

impl Eq for ResultCell {}

impl ResultCell {
    pub fn decimal(mut mantissa: i128, mut scale: u32) -> Self {
        while scale != 0 && mantissa % 10 == 0 {
            mantissa /= 10;
            scale -= 1;
        }
        Self::Decimal { mantissa, scale }
    }
}

pub type ResultRow = Vec<ResultCell>;

/// Ordered rows returned by one query implementation.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ResultSet {
    pub rows: Vec<ResultRow>,
}

impl ResultSet {
    pub fn from_rows(rows: impl IntoIterator<Item = ResultRow>) -> Self {
        Self {
            rows: rows.into_iter().collect(),
        }
    }
}

/// How row order contributes to equality for one query.
///
/// Every JOB query is a `MIN(...)` projection and so returns a single row,
/// which makes [`Self::Bag`] the only policy the JOB suite selects. Ordering is
/// implemented here because a suite with `ORDER BY` queries needs it, not
/// because a current caller does.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResultOrder {
    /// Compare rows as a multiset, preserving duplicate counts.
    Bag,
    /// Compare the sequence of order-key groups. Rows within an equal-key group
    /// form a bag because SQL leaves their relative order unspecified.
    OrderedBy(&'static [&'static str]),
}

impl ResultOrder {
    pub fn equivalent<C: AsRef<str>>(
        self,
        result_columns: &[C],
        left: &ResultSet,
        right: &ResultSet,
    ) -> bool {
        match self {
            Self::Bag => same_bag(&left.rows, &right.rows),
            Self::OrderedBy(names) => {
                let Some(columns) = names
                    .iter()
                    .map(|name| {
                        result_columns
                            .iter()
                            .position(|column| column.as_ref() == *name)
                    })
                    .collect::<Option<Vec<_>>>()
                else {
                    return false;
                };
                same_ordered_groups(&left.rows, &right.rows, &columns)
            }
        }
    }
}

fn same_bag(left: &[ResultRow], right: &[ResultRow]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut matched = vec![false; right.len()];
    for row in left {
        let Some(index) = right
            .iter()
            .enumerate()
            .position(|(index, candidate)| !matched[index] && candidate == row)
        else {
            return false;
        };
        matched[index] = true;
    }
    true
}

fn same_ordered_groups(left: &[ResultRow], right: &[ResultRow], columns: &[usize]) -> bool {
    let (mut left_start, mut right_start) = (0, 0);
    while left_start < left.len() && right_start < right.len() {
        if !same_key(&left[left_start], &right[right_start], columns) {
            return false;
        }
        let left_end = group_end(left, left_start, columns);
        let right_end = group_end(right, right_start, columns);
        if !same_bag(&left[left_start..left_end], &right[right_start..right_end]) {
            return false;
        }
        left_start = left_end;
        right_start = right_end;
    }
    left_start == left.len() && right_start == right.len()
}

fn group_end(rows: &[ResultRow], start: usize, columns: &[usize]) -> usize {
    (start + 1..rows.len())
        .find(|&index| !same_key(&rows[start], &rows[index], columns))
        .unwrap_or(rows.len())
}

fn same_key(left: &ResultRow, right: &ResultRow, columns: &[usize]) -> bool {
    columns.iter().all(|&column| {
        left.get(column)
            .zip(right.get(column))
            .is_some_and(|(l, r)| l == r)
    })
}

impl fmt::Display for ResultSet {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.rows.is_empty() {
            return formatter.write_str("<no rows>");
        }
        for (index, row) in self.rows.iter().enumerate() {
            if index != 0 {
                formatter.write_str("\n")?;
            }
            write!(formatter, "{row:?}")?;
        }
        Ok(())
    }
}
