// Terminal continuation: drive a query, fold the lexicographic minimum of
// each output column independently, and render `a || b || …` (or "(empty)"
// when no row survived) — the JOB benchmark's MIN(...) projection.
// Plus the typed shared sub-queries used by several queries.

use crate::engine::*;
use crate::job_schema::*;

/// One typed scalar returned by the feature-gated query-result hook.
#[cfg(feature = "test")]
#[derive(Clone, Debug, Eq, PartialEq)]
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

// ===== shared sub-queries (used by several queries) =====================

// ===== keyword patterns, resolved to ids once ===========================
/// Companies named *Film*/*Warner*, non-Polish production companies without
/// a note — the `co` binding of queries 21a-c and 27a-c.
pub fn film_or_warner_co(db: &'static Job) -> impl Query<R = Id<Company>, D = Id<Movie>> + Drive + Probe {
    let Movie { company, .. } = &db.movie;
    let Company { name: company_name, country, note: company_note, ty: company_ty, .. } = &db.company;
    company.with(country.ne("[pl]")
            .and(company_name.rx(r"Film|Warner"))
            .and(company_ty.eq("production companies").minus(company_note)))
}

/// The link-type label ("followed by", …) of each movie's "follow"-typed
/// links — the `lk` binding of queries 21a-c and 27a-c. String-valued: the
/// hop to the link type's label is explicit here, so output products use
/// the result directly.
pub fn follow_link(db: &'static Job) -> impl Query<D = Id<Movie>, R = &'static str> + Drive + Probe {
    let Movie { link, .. } = &db.movie;
    let MovieLink { ty: movielink_ty, .. } = &db.movie_link;
    link.select(movielink_ty.rx(r"follow"))
}
