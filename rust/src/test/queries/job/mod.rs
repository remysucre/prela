//! All 113 JOB SQL cases paired with the production Prela queries.

pub(crate) mod schema;

use crate::test::result::ResultOrder;
use std::path::PathBuf;

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

macro_rules! query {
    ($module:ident, $name:literal, $directory:literal) => {
        mod $module {
            use super::{Query, ResultOrder};

            pub(super) const QUERY: Query = Query {
                name: $name,
                sql: include_str!(concat!($directory, "/query.sql")),
                order: ResultOrder::Bag,
                directory: concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/src/test/queries/job/",
                    $directory,
                ),
            };
        }
    };
}

query!(q01a, "1a", "q01a");
query!(q01b, "1b", "q01b");
query!(q01c, "1c", "q01c");
query!(q01d, "1d", "q01d");
query!(q02a, "2a", "q02a");
query!(q02b, "2b", "q02b");
query!(q02c, "2c", "q02c");
query!(q02d, "2d", "q02d");
query!(q03a, "3a", "q03a");
query!(q03b, "3b", "q03b");
query!(q03c, "3c", "q03c");
query!(q04a, "4a", "q04a");
query!(q04b, "4b", "q04b");
query!(q04c, "4c", "q04c");
query!(q05a, "5a", "q05a");
query!(q05b, "5b", "q05b");
query!(q05c, "5c", "q05c");
query!(q06a, "6a", "q06a");
query!(q06b, "6b", "q06b");
query!(q06c, "6c", "q06c");
query!(q06d, "6d", "q06d");
query!(q06e, "6e", "q06e");
query!(q06f, "6f", "q06f");
query!(q07a, "7a", "q07a");
query!(q07b, "7b", "q07b");
query!(q07c, "7c", "q07c");
query!(q08a, "8a", "q08a");
query!(q08b, "8b", "q08b");
query!(q08c, "8c", "q08c");
query!(q08d, "8d", "q08d");
query!(q09a, "9a", "q09a");
query!(q09b, "9b", "q09b");
query!(q09c, "9c", "q09c");
query!(q09d, "9d", "q09d");
query!(q10a, "10a", "q10a");
query!(q10b, "10b", "q10b");
query!(q10c, "10c", "q10c");
query!(q11a, "11a", "q11a");
query!(q11b, "11b", "q11b");
query!(q11c, "11c", "q11c");
query!(q11d, "11d", "q11d");
query!(q12a, "12a", "q12a");
query!(q12b, "12b", "q12b");
query!(q12c, "12c", "q12c");
query!(q13a, "13a", "q13a");
query!(q13b, "13b", "q13b");
query!(q13c, "13c", "q13c");
query!(q13d, "13d", "q13d");
query!(q14a, "14a", "q14a");
query!(q14b, "14b", "q14b");
query!(q14c, "14c", "q14c");
query!(q15a, "15a", "q15a");
query!(q15b, "15b", "q15b");
query!(q15c, "15c", "q15c");
query!(q15d, "15d", "q15d");
query!(q16a, "16a", "q16a");
query!(q16b, "16b", "q16b");
query!(q16c, "16c", "q16c");
query!(q16d, "16d", "q16d");
query!(q17a, "17a", "q17a");
query!(q17b, "17b", "q17b");
query!(q17c, "17c", "q17c");
query!(q17d, "17d", "q17d");
query!(q17e, "17e", "q17e");
query!(q17f, "17f", "q17f");
query!(q18a, "18a", "q18a");
query!(q18b, "18b", "q18b");
query!(q18c, "18c", "q18c");
query!(q19a, "19a", "q19a");
query!(q19b, "19b", "q19b");
query!(q19c, "19c", "q19c");
query!(q19d, "19d", "q19d");
query!(q20a, "20a", "q20a");
query!(q20b, "20b", "q20b");
query!(q20c, "20c", "q20c");
query!(q21a, "21a", "q21a");
query!(q21b, "21b", "q21b");
query!(q21c, "21c", "q21c");
query!(q22a, "22a", "q22a");
query!(q22b, "22b", "q22b");
query!(q22c, "22c", "q22c");
query!(q22d, "22d", "q22d");
query!(q23a, "23a", "q23a");
query!(q23b, "23b", "q23b");
query!(q23c, "23c", "q23c");
query!(q24a, "24a", "q24a");
query!(q24b, "24b", "q24b");
query!(q25a, "25a", "q25a");
query!(q25b, "25b", "q25b");
query!(q25c, "25c", "q25c");
query!(q26a, "26a", "q26a");
query!(q26b, "26b", "q26b");
query!(q26c, "26c", "q26c");
query!(q27a, "27a", "q27a");
query!(q27b, "27b", "q27b");
query!(q27c, "27c", "q27c");
query!(q28a, "28a", "q28a");
query!(q28b, "28b", "q28b");
query!(q28c, "28c", "q28c");
query!(q29a, "29a", "q29a");
query!(q29b, "29b", "q29b");
query!(q29c, "29c", "q29c");
query!(q30a, "30a", "q30a");
query!(q30b, "30b", "q30b");
query!(q30c, "30c", "q30c");
query!(q31a, "31a", "q31a");
query!(q31b, "31b", "q31b");
query!(q31c, "31c", "q31c");
query!(q32a, "32a", "q32a");
query!(q32b, "32b", "q32b");
query!(q33a, "33a", "q33a");
query!(q33b, "33b", "q33b");
query!(q33c, "33c", "q33c");

const QUERIES: [Query; 113] = [
    q01a::QUERY,
    q01b::QUERY,
    q01c::QUERY,
    q01d::QUERY,
    q02a::QUERY,
    q02b::QUERY,
    q02c::QUERY,
    q02d::QUERY,
    q03a::QUERY,
    q03b::QUERY,
    q03c::QUERY,
    q04a::QUERY,
    q04b::QUERY,
    q04c::QUERY,
    q05a::QUERY,
    q05b::QUERY,
    q05c::QUERY,
    q06a::QUERY,
    q06b::QUERY,
    q06c::QUERY,
    q06d::QUERY,
    q06e::QUERY,
    q06f::QUERY,
    q07a::QUERY,
    q07b::QUERY,
    q07c::QUERY,
    q08a::QUERY,
    q08b::QUERY,
    q08c::QUERY,
    q08d::QUERY,
    q09a::QUERY,
    q09b::QUERY,
    q09c::QUERY,
    q09d::QUERY,
    q10a::QUERY,
    q10b::QUERY,
    q10c::QUERY,
    q11a::QUERY,
    q11b::QUERY,
    q11c::QUERY,
    q11d::QUERY,
    q12a::QUERY,
    q12b::QUERY,
    q12c::QUERY,
    q13a::QUERY,
    q13b::QUERY,
    q13c::QUERY,
    q13d::QUERY,
    q14a::QUERY,
    q14b::QUERY,
    q14c::QUERY,
    q15a::QUERY,
    q15b::QUERY,
    q15c::QUERY,
    q15d::QUERY,
    q16a::QUERY,
    q16b::QUERY,
    q16c::QUERY,
    q16d::QUERY,
    q17a::QUERY,
    q17b::QUERY,
    q17c::QUERY,
    q17d::QUERY,
    q17e::QUERY,
    q17f::QUERY,
    q18a::QUERY,
    q18b::QUERY,
    q18c::QUERY,
    q19a::QUERY,
    q19b::QUERY,
    q19c::QUERY,
    q19d::QUERY,
    q20a::QUERY,
    q20b::QUERY,
    q20c::QUERY,
    q21a::QUERY,
    q21b::QUERY,
    q21c::QUERY,
    q22a::QUERY,
    q22b::QUERY,
    q22c::QUERY,
    q22d::QUERY,
    q23a::QUERY,
    q23b::QUERY,
    q23c::QUERY,
    q24a::QUERY,
    q24b::QUERY,
    q25a::QUERY,
    q25b::QUERY,
    q25c::QUERY,
    q26a::QUERY,
    q26b::QUERY,
    q26c::QUERY,
    q27a::QUERY,
    q27b::QUERY,
    q27c::QUERY,
    q28a::QUERY,
    q28b::QUERY,
    q28c::QUERY,
    q29a::QUERY,
    q29b::QUERY,
    q29c::QUERY,
    q30a::QUERY,
    q30b::QUERY,
    q30c::QUERY,
    q31a::QUERY,
    q31b::QUERY,
    q31c::QUERY,
    q32a::QUERY,
    q32b::QUERY,
    q33a::QUERY,
    q33b::QUERY,
    q33c::QUERY,
];

pub fn all() -> [Query; 113] {
    QUERIES
}

pub fn get(name: &str) -> Option<Query> {
    QUERIES.into_iter().find(|query| query.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_contains_every_job_sql_query() {
        let mut copied = all().map(|query| query.name).to_vec();
        let mut production = crate::job_queries::all_queries()
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
