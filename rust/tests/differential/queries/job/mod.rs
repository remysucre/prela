//! All 113 JOB SQL cases paired with the production Prela queries.

pub(crate) mod schema;

use crate::result::ResultOrder;
use std::path::PathBuf;

/// One SQL case: the text to run, how to compare its rows, and where to
/// deposit a reproduction when it disagrees with Prela.
#[derive(Clone, Copy, Debug)]
pub struct Query {
    pub name: &'static str,
    pub sql: &'static str,
    pub order: ResultOrder,
    directory: &'static str,
}

impl Query {
    pub fn mismatch_path(self) -> PathBuf {
        PathBuf::from(self.directory).join("mismatch.sql")
    }
}

/// Build the registry from one line per query: the directory holding its
/// `query.sql`, and the JOB name `prela::job_queries::differential` answers
/// to. The directory name is not derivable from the JOB name (it zero-pads
/// the template number), so both appear.
macro_rules! job_sql_queries {
    ($($directory:ident => $name:literal;)*) => {
        const QUERIES: &[Query] = &[
            $(Query {
                name: $name,
                sql: include_str!(concat!(stringify!($directory), "/query.sql")),
                order: ResultOrder::Bag,
                directory: concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/tests/differential/queries/job/",
                    stringify!($directory),
                ),
            },)*
        ];
    };
}

job_sql_queries! {
    q01a => "1a";
    q01b => "1b";
    q01c => "1c";
    q01d => "1d";
    q02a => "2a";
    q02b => "2b";
    q02c => "2c";
    q02d => "2d";
    q03a => "3a";
    q03b => "3b";
    q03c => "3c";
    q04a => "4a";
    q04b => "4b";
    q04c => "4c";
    q05a => "5a";
    q05b => "5b";
    q05c => "5c";
    q06a => "6a";
    q06b => "6b";
    q06c => "6c";
    q06d => "6d";
    q06e => "6e";
    q06f => "6f";
    q07a => "7a";
    q07b => "7b";
    q07c => "7c";
    q08a => "8a";
    q08b => "8b";
    q08c => "8c";
    q08d => "8d";
    q09a => "9a";
    q09b => "9b";
    q09c => "9c";
    q09d => "9d";
    q10a => "10a";
    q10b => "10b";
    q10c => "10c";
    q11a => "11a";
    q11b => "11b";
    q11c => "11c";
    q11d => "11d";
    q12a => "12a";
    q12b => "12b";
    q12c => "12c";
    q13a => "13a";
    q13b => "13b";
    q13c => "13c";
    q13d => "13d";
    q14a => "14a";
    q14b => "14b";
    q14c => "14c";
    q15a => "15a";
    q15b => "15b";
    q15c => "15c";
    q15d => "15d";
    q16a => "16a";
    q16b => "16b";
    q16c => "16c";
    q16d => "16d";
    q17a => "17a";
    q17b => "17b";
    q17c => "17c";
    q17d => "17d";
    q17e => "17e";
    q17f => "17f";
    q18a => "18a";
    q18b => "18b";
    q18c => "18c";
    q19a => "19a";
    q19b => "19b";
    q19c => "19c";
    q19d => "19d";
    q20a => "20a";
    q20b => "20b";
    q20c => "20c";
    q21a => "21a";
    q21b => "21b";
    q21c => "21c";
    q22a => "22a";
    q22b => "22b";
    q22c => "22c";
    q22d => "22d";
    q23a => "23a";
    q23b => "23b";
    q23c => "23c";
    q24a => "24a";
    q24b => "24b";
    q25a => "25a";
    q25b => "25b";
    q25c => "25c";
    q26a => "26a";
    q26b => "26b";
    q26c => "26c";
    q27a => "27a";
    q27b => "27b";
    q27c => "27c";
    q28a => "28a";
    q28b => "28b";
    q28c => "28c";
    q29a => "29a";
    q29b => "29b";
    q29c => "29c";
    q30a => "30a";
    q30b => "30b";
    q30c => "30c";
    q31a => "31a";
    q31b => "31b";
    q31c => "31c";
    q32a => "32a";
    q32b => "32b";
    q33a => "33a";
    q33b => "33b";
    q33c => "33c";
}

pub fn all() -> &'static [Query] {
    QUERIES
}

pub fn get(name: &str) -> Option<Query> {
    QUERIES.iter().copied().find(|query| query.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_contains_every_job_sql_query() {
        let mut copied = all().iter().map(|query| query.name).collect::<Vec<_>>();
        let mut production = prela::job_queries::all_queries()
            .into_iter()
            .map(|entry| entry.0)
            .filter(|name| *name != "6a/method")
            .collect::<Vec<_>>();
        copied.sort_unstable();
        production.sort_unstable();
        assert_eq!(copied, production);
        assert_eq!(copied.len(), 113);
        assert!(all().iter().all(|query| query.order == ResultOrder::Bag));
    }
}
