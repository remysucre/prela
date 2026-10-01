// Shared machinery for the SQLStorm ports: the generated schema, the
// canonical output format that prela and DuckDB are compared in, epoch
// microsecond time helpers, and the batch runner.
//
// A batch crate is a `main` that hands `run` its entries:
//
//     fn main() { harness::run(&[("10117", q10117), ("10235", q10235)]) }
//
// and each entry is `fn(&'static So) -> String` producing rows in the
// canonical format (see `fmt`).

pub mod fmt;
pub mod kit;
pub mod run;
pub mod schema;
pub mod time;
pub mod views;

pub use run::{Entry, run};
pub use schema::So;

// So a query file only needs `use harness::prelude::*;`.
pub mod prelude {
    pub use crate::fmt::{V, odate, ofloat, oint, ostr, ots, row, rows};
    pub use crate::schema::*;
    pub use crate::views::*;
    pub use crate::kit::*;
    pub use crate::time::*;
    pub use prela::engine::*;
    pub use prela::loader::{Col, Key, Set, Str};
}
