// Pull-protocol twins of src/queries/helpers.rs — the same Row shapes and
// shared sub-queries, with the terminal continuation consuming via
// `Iterator::fold` (and a raw-`next()` variant for the `pull-next` suite).

use crate::data::Data;
use crate::engine::*;
use crate::pull::{Iterate, ProbeIter};

pub use crate::queries::helpers::Row;

/// Pull twin of `min_row` — `q.iter().fold(...)` accumulates the
/// per-column minima via internal iteration.
pub fn min_row_pull<Q: Iterate>(q: Q) -> String where Q::R: Row {
    let m = q.iter().fold(None, |m: Option<Q::R>, (_, v)| {
        Some(match m { Some(acc) => acc.col_min(v), None => v })
    });
    render_min(m)
}

/// `min_row_pull` with raw-`next()` consumption (a `for` loop) — the
/// consumption-style axis. Used only by the `pull-next` suite.
pub fn min_row_pull_next<Q: Iterate>(q: Q) -> String where Q::R: Row {
    let mut m: Option<Q::R> = None;
    for (_, v) in q.iter() {
        m = Some(match m { Some(acc) => acc.col_min(v), None => v });
    }
    render_min(m)
}

fn render_min<R: Row>(m: Option<R>) -> String {
    match m {
        None => "(empty)".into(),
        Some(row) => {
            let mut cols = Vec::new();
            row.push_cols(&mut cols);
            cols.join(" || ")
        }
    }
}

// ===== shared sub-queries (same plans as the push helpers) ==============

/// Companies named *Film*/*Warner*, non-Polish production companies without
/// a note — the `co` binding of queries 21a-c and 27a-c.
pub fn film_or_warner_co<'d>(d: &'d Data) -> impl Query<R = usize, D = usize> + Drive + Probe + Iterate + ProbeIter + 'd {
    (&d.movie_company).in_s(
        (&d.company_country).ne("[pl]")
            .and(
                (&d.company_name).rx(r"Film")
                    .or((&d.company_name).rx(r"Warner"))
            )
            .and(
                (&d.company_type).o(&d.companytype_kind).eq("production companies")
                    .minus(&d.company_note)
            )
    )
}

/// Movie links whose link type matches "follow" — the `lk` binding of
/// queries 21a-c and 27a-c.
pub fn follow_link<'d>(d: &'d Data) -> impl Query<R = usize, D = usize> + Drive + Probe + Iterate + ProbeIter + 'd {
    (&d.movie_link).in_s(
        (&d.movielink_type).o(&d.linktype_link).rx(r"follow")
    )
}
