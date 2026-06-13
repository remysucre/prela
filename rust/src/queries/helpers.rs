// Terminal continuation: drive a query, fold the lexicographic minimum of
// each output column independently, and render `a || b || …` (or "(empty)"
// when no row survived) — the JOB benchmark's MIN(...) projection.
// Plus the typed shared sub-queries (Julia `let` bindings used by several
// queries).

use crate::engine::*;
use crate::job_schema::*;

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
pub fn film_or_warner_co() -> impl Query<R = Id<Company>, D = Id<Movie>> + Drive + Probe {
    company.when(country.ne("[pl]")
                   .and(Company::name.rx(r"Film").or(Company::name.rx(r"Warner")))
                   .and(Company::ty.text().eq("production companies").minus(Company::note)))
}

/// The link-type label ("followed by", …) of each movie's "follow"-typed
/// links — the `lk` binding of queries 21a-c and 27a-c. String-valued like
/// Julia's `link → (MovieLink.type ~ r"follow")` (whose primary elision
/// composes through to `LinkType.link`), so output products use it directly.
pub fn follow_link() -> impl Query<D = Id<Movie>, R = &'static str> + Drive + Probe {
    link.get(MovieLink::ty.text().rx(r"follow"))
}
