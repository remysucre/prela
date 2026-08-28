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

// ===== keyword patterns, resolved to ids once ===========================
// `keyword.rx(…)` elides `Id<Keyword>` to its `text` field, so it runs the
// regex ONCE PER movie-keyword pair — ~4.5M times when the drive is the
// whole movie universe. The predicate only ever looks at the keyword, so
// resolve it up front instead: one pass over the 134k-row text column
// (~0.1 ms) into a bitset over the keyword id space, after which the
// per-pair test is a bit lookup. Spelled `keyword.with(kw_rx(db, r"sequel"))`.
//
// `inv` is what puts the ids in value position — `Bitset::over` sets a bit
// per emitted VALUE, and the text column is keyed BY id — and in drive
// position it is a callback-argument swap, not a materialization.
//
// This only pays when the keyword test really does run over the whole
// universe. Where a more selective conjunct can lead — `link`, say, which
// only 6.4k movies have — plain `keyword.eq(…)` on the few survivors beats
// the bitset, whose up-front scan then dominates. Order the conjuncts first;
// reach for this second.

/// Keywords whose text matches `re`.
pub fn kw_rx(db: &'static Job, re: &str) -> Bitset<Id<Keyword>> {
    let Keyword { text: keyword_text, .. } = &db.keyword;
    Bitset::over(&db.keyword, keyword_text.rx(re).inv())
}

/// Companies named *Film*/*Warner*, non-Polish production companies without
/// a note — the `co` binding of queries 21a-c and 27a-c.
pub fn film_or_warner_co(db: &'static Job) -> impl Query<R = Id<Company>, D = Id<Movie>> + Drive + Probe {
    let Movie { company, .. } = &db.movie;
    let Company { name: company_name, country, note: company_note, ty: company_ty, .. } = &db.company;
    let CompanyType { text: companytype_text, .. } = &db.company_type;
    company.with(country.ne("[pl]")
            .and(company_name.rx(r"Film|Warner"))
            .and(company_ty.select(companytype_text).eq("production companies").minus(company_note)))
}

/// The link-type label ("followed by", …) of each movie's "follow"-typed
/// links — the `lk` binding of queries 21a-c and 27a-c. String-valued like
/// Julia's `link → (MovieLink.type ~ r"follow")`, which composes through to
/// the link type's label; here that hop is the explicit
/// `.select(linktype_text)`, so output products use the result directly.
pub fn follow_link(db: &'static Job) -> impl Query<D = Id<Movie>, R = &'static str> + Drive + Probe {
    let Movie { link, .. } = &db.movie;
    let LinkType { text: linktype_text, .. } = &db.link_type;
    let MovieLink { ty: movielink_ty, .. } = &db.movie_link;
    link.select(movielink_ty.select(linktype_text).rx(r"follow"))
}
