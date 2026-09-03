// Terminal continuation: drive a query, fold the lexicographic minimum of
// each output column independently, and render `a || b || …` (or "(empty)"
// when no row survived) — the JOB benchmark's MIN(...) projection.
// Plus the typed shared sub-queries used by several queries.

use crate::engine::*;
use crate::job_schema::*;

/// An output-row shape: scalar columns and nested `Prod` tuples thereof.
pub trait Row: Copy {
    /// Column-wise minimum of two rows.
    fn col_min(self, other: Self) -> Self;
    /// Append each column, formatted, to `cols`.
    fn push_cols(self, cols: &mut Vec<String>);
    /// Append each column without losing its scalar type.
    #[cfg(all(test, feature = "test"))]
    fn push_result_cells(self, cells: &mut Vec<crate::test::result::ResultCell>);
}

impl Row for &'static str {
    fn col_min(self, other: Self) -> Self { if self <= other { self } else { other } }
    fn push_cols(self, cols: &mut Vec<String>) { cols.push(self.to_string()); }
    #[cfg(all(test, feature = "test"))]
    fn push_result_cells(self, cells: &mut Vec<crate::test::result::ResultCell>) {
        cells.push(crate::test::result::ResultCell::Text(self.to_owned()));
    }
}

impl Row for i64 {
    fn col_min(self, other: Self) -> Self { self.min(other) }
    fn push_cols(self, cols: &mut Vec<String>) { cols.push(self.to_string()); }
    #[cfg(all(test, feature = "test"))]
    fn push_result_cells(self, cells: &mut Vec<crate::test::result::ResultCell>) {
        cells.push(crate::test::result::ResultCell::Integer(i128::from(self)));
    }
}

impl<A: Row, B: Row> Row for (A, B) {
    fn col_min(self, other: Self) -> Self {
        (self.0.col_min(other.0), self.1.col_min(other.1))
    }
    fn push_cols(self, cols: &mut Vec<String>) {
        self.0.push_cols(cols);
        self.1.push_cols(cols);
    }
    #[cfg(all(test, feature = "test"))]
    fn push_result_cells(self, cells: &mut Vec<crate::test::result::ResultCell>) {
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

#[cfg(all(test, feature = "test"))]
pub fn min_result<Q: Drive>(q: Q, width: usize) -> crate::test::result::ResultSet
where
    Q::R: Row,
{
    use crate::test::result::{ResultCell, ResultSet};

    let mut minimum: Option<Q::R> = None;
    q.drive(|_, value| {
        minimum = Some(match minimum {
            Some(old) => old.col_min(value),
            None => value,
        })
    });
    let row = match minimum {
        None => vec![ResultCell::Null; width],
        Some(row) => {
            let mut values = Vec::with_capacity(width);
            row.push_result_cells(&mut values);
            assert_eq!(values.len(), width);
            values
        }
    };
    ResultSet::from_rows([row])
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
