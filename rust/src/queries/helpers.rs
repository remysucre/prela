// Terminal continuation: drive a query, fold the lexicographic minimum of
// each output column independently, and render `a || b || …` (or "(empty)"
// when no row survived) — the JOB benchmark's MIN(...) projection.

use crate::data::Data;
use crate::engine::*;

/// An output-row shape: scalar columns and nested `Prod` tuples thereof.
pub trait Row: Copy {
    /// Column-wise minimum of two rows.
    fn col_min(self, other: Self) -> Self;
    /// Append each column, formatted, to `cols`.
    fn push_cols(self, cols: &mut Vec<String>);
}

impl Row for &'static str {
    fn col_min(self, other: Self) -> Self { if self <= other { self } else { other } }
    fn push_cols(self, cols: &mut Vec<String>) { cols.push(self.to_string()); }
}

impl Row for i64 {
    fn col_min(self, other: Self) -> Self { self.min(other) }
    fn push_cols(self, cols: &mut Vec<String>) { cols.push(self.to_string()); }
}

impl<A: Row, B: Row> Row for (A, B) {
    fn col_min(self, other: Self) -> Self {
        (self.0.col_min(other.0), self.1.col_min(other.1))
    }
    fn push_cols(self, cols: &mut Vec<String>) {
        self.0.push_cols(cols);
        self.1.push_cols(cols);
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

// ===== shared sub-queries (Julia `let` bindings used by several queries) =

/// Companies named *Film*/*Warner*, non-Polish production companies without
/// a note — the `co` binding of queries 21a-c and 27a-c.
pub fn film_or_warner_co<'d>(d: &'d Data) -> impl Rel<R = i64, D = i64> + Drive + Probe + 'd {
    (&d.movie_company).in_s(
        (&d.company_country).ne("[pl]").k()
            .and(
                (&d.company_name).rx(r"Film").k()
                    .or((&d.company_name).rx(r"Warner").k())
            )
            .and(
                (&d.company_type).o(&d.companytype_kind).eq("production companies").k()
                    .minus((&d.company_note).k())
            )
    )
}

/// Movie links whose link type matches "follow" — the `lk` binding of
/// queries 21a-c and 27a-c.
pub fn follow_link<'d>(d: &'d Data) -> impl Rel<R = i64, D = i64> + Drive + Probe + 'd {
    (&d.movie_link).in_s(
        (&d.movielink_type).o(&d.linktype_link).rx(r"follow").k()
    )
}
