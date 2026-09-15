// Terminal continuation: drive a query, fold the lexicographic minimum of
// each output column independently, and render `a || b || …` (or "(empty)"
// when no row survived) — the JOB benchmark's MIN(...) projection.

use crate::engine::*;

/// One typed scalar returned by the feature-gated query-result hook.
#[cfg(feature = "test")]
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Result {
    Null,
    Integer(i64),
    Text(String),
}

/// An output-row shape: scalar columns and nested `Prod` tuples thereof.
pub trait Row: Copy {
    /// Number of scalar columns, so an empty result can be padded to the
    /// right arity without the caller restating it.
    const WIDTH: usize;
    /// Column-wise minimum of two rows.
    fn col_min(self, other: Self) -> Self;
    /// Append each column, formatted, to `cols`.
    fn push_cols(self, cols: &mut Vec<String>);
    /// Append each column without losing its scalar type.
    #[cfg(feature = "test")]
    fn push_result_cells(self, cells: &mut Vec<Result>);
}

impl Row for &'static str {
    const WIDTH: usize = 1;
    fn col_min(self, other: Self) -> Self { if self <= other { self } else { other } }
    fn push_cols(self, cols: &mut Vec<String>) { cols.push(self.to_string()); }
    #[cfg(feature = "test")]
    fn push_result_cells(self, cells: &mut Vec<Result>) {
        cells.push(Result::Text(self.to_owned()));
    }
}

impl Row for i64 {
    const WIDTH: usize = 1;
    fn col_min(self, other: Self) -> Self { self.min(other) }
    fn push_cols(self, cols: &mut Vec<String>) { cols.push(self.to_string()); }
    #[cfg(feature = "test")]
    fn push_result_cells(self, cells: &mut Vec<Result>) {
        cells.push(Result::Integer(self));
    }
}

impl<A: Row, B: Row> Row for (A, B) {
    const WIDTH: usize = A::WIDTH + B::WIDTH;
    fn col_min(self, other: Self) -> Self {
        (self.0.col_min(other.0), self.1.col_min(other.1))
    }
    fn push_cols(self, cols: &mut Vec<String>) {
        self.0.push_cols(cols);
        self.1.push_cols(cols);
    }
    #[cfg(feature = "test")]
    fn push_result_cells(self, cells: &mut Vec<Result>) {
        self.0.push_result_cells(cells);
        self.1.push_result_cells(cells);
    }
}

/// Drive `q`, accumulate per-column minima, render `min0 || min1 || …`.
pub fn min_row<Q: Drive>(q: Q) -> String where Q::R: Row {
    let mut m: Option<Q::R> = None;
    q.drive(|_, v| m = Some(match m { Some(acc) => acc.col_min(v), None => v }));
    match m {
        None => "(empty)".into(),
        Some(row) => {
            let mut cols = Vec::new();
            row.push_cols(&mut cols);
            cols.join(" || ")
        }
    }
}

/// Drive `q` and accumulate per-column minima as typed cells, padding an
/// empty result to [`Row::WIDTH`] nulls the way SQL's `MIN` over no rows does.
#[cfg(feature = "test")]
pub fn min_result<Q: Drive>(q: Q) -> Vec<Result>
where
    Q::R: Row,
{
    let mut minimum: Option<Q::R> = None;
    q.drive(|_, value| {
        minimum = Some(match minimum {
            Some(old) => old.col_min(value),
            None => value,
        })
    });
    match minimum {
        None => vec![Result::Null; Q::R::WIDTH],
        Some(row) => {
            let mut values = Vec::with_capacity(Q::R::WIDTH);
            row.push_result_cells(&mut values);
            debug_assert_eq!(values.len(), Q::R::WIDTH);
            values
        }
    }
}

/// Consume the same registered plans as benchmark text or typed test results.
pub(crate) trait Output {
    type Value: 'static;
    fn finish<Q: Drive>(name: &str, query: Q) -> Self::Value where Q::R: Row;
}

pub(crate) struct TextOutput;

impl Output for TextOutput {
    type Value = String;
    fn finish<Q: Drive>(_: &str, query: Q) -> String where Q::R: Row {
        min_row(query)
    }
}

#[cfg(feature = "test")]
pub(crate) struct TypedOutput;

#[cfg(feature = "test")]
impl Output for TypedOutput {
    type Value = Vec<Result>;
    fn finish<Q: Drive>(name: &str, query: Q) -> Vec<Result> where Q::R: Row {
        let mut cells = min_result(query);
        // The benchmark groups some projections differently from JOB SQL.
        // Restore SQL column order without changing the production plans.
        match name {
            "6a" | "6b" | "6c" | "6d" | "6e" | "6f" | "9b" | "9c" | "32b" => {
                cells.swap(1, 2);
            }
            "13b" | "13c" => cells.rotate_left(1),
            "33a" | "33b" | "33c" => {
                cells = [0, 3, 1, 4, 2, 5].map(|index| cells[index].clone()).to_vec();
            }
            _ => {}
        }
        // SQL projects the same name twice; the benchmark reports it once.
        if matches!(name, "17a" | "17b" | "17c") {
            debug_assert_eq!(cells.len(), 1);
            cells.push(cells[0].clone());
        }
        cells
    }
}
