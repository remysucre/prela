//! Benchmark-neutral DuckDB execution for differential tests.

use super::generate::Database;
use super::result::{ResultCell, ResultOrder, ResultSet};
use super::schema::Schema;
use super::sql::to_sql;
use duckdb::Connection;
use duckdb::types::ValueRef;

/// End-to-end Join Order Benchmark execution.
#[path = "runner/job.rs"]
pub mod job;

/// Execute one SQL query against a generated fixture.
pub(crate) fn run_sql(
    schema: &Schema,
    database: &Database,
    name: &str,
    sql: &str,
    order: ResultOrder,
) -> Result<(Vec<String>, ResultSet), String> {
    let connection = Connection::open_in_memory()
        .map_err(|error| format!("open DuckDB for Q{name}: {error}"))?;
    let fixture = to_sql(schema, database);
    connection
        .execute_batch(&fixture)
        .map_err(|error| format!("load fixture for Q{name}: {error}\n{fixture}"))?;
    let mut statement = connection
        .prepare(sql)
        .map_err(|error| format!("prepare Q{name}: {error}\n{sql}"))?;
    let mut cursor = statement
        .query([])
        .map_err(|error| format!("execute Q{name}: {error}"))?;

    // DuckDB exposes result-column metadata only after statement execution.
    let columns = cursor
        .as_ref()
        .expect("query cursor has a statement")
        .column_names();
    for (index, column) in columns.iter().enumerate() {
        if columns[..index].contains(column) {
            return Err(format!(
                "Q{name} returns duplicate column name {column:?}; add a distinct SQL alias"
            ));
        }
    }
    if let ResultOrder::OrderedBy(order_columns) = order {
        for column in order_columns {
            if !columns.iter().any(|actual| actual == column) {
                return Err(format!(
                    "Q{name} orders by {column:?}, but DuckDB returned columns {columns:?}"
                ));
            }
        }
    }

    let column_count = columns.len();
    let mut output = Vec::new();
    while let Some(row) = cursor
        .next()
        .map_err(|error| format!("read Q{name} result: {error}"))?
    {
        let mut cells = Vec::with_capacity(column_count);
        for index in 0..column_count {
            cells.push(result_cell(row.get_ref(index).map_err(|error| {
                format!("read Q{name} column {index}: {error}")
            })?)?);
        }
        output.push(cells);
    }
    Ok((columns, ResultSet::from_rows(output)))
}

fn result_cell(value: ValueRef<'_>) -> Result<ResultCell, String> {
    let cell = match value {
        ValueRef::Null => ResultCell::Null,
        ValueRef::Boolean(value) => ResultCell::Boolean(value),
        ValueRef::TinyInt(value) => ResultCell::Integer(i128::from(value)),
        ValueRef::SmallInt(value) => ResultCell::Integer(i128::from(value)),
        ValueRef::Int(value) => ResultCell::Integer(i128::from(value)),
        ValueRef::BigInt(value) => ResultCell::Integer(i128::from(value)),
        ValueRef::HugeInt(value) => ResultCell::Integer(value),
        ValueRef::UTinyInt(value) => ResultCell::Integer(i128::from(value)),
        ValueRef::USmallInt(value) => ResultCell::Integer(i128::from(value)),
        ValueRef::UInt(value) => ResultCell::Integer(i128::from(value)),
        ValueRef::UBigInt(value) => ResultCell::Integer(i128::from(value)),
        ValueRef::Float(value) => ResultCell::Float(f64::from(value)),
        ValueRef::Double(value) => ResultCell::Float(value),
        ValueRef::Decimal(value) => ResultCell::decimal(value.mantissa(), value.scale()),
        ValueRef::Text(bytes) => ResultCell::Text(
            std::str::from_utf8(bytes)
                .map_err(|error| format!("DuckDB returned non-UTF-8 text: {error}"))?
                .to_owned(),
        ),
        ValueRef::Date32(days) => ResultCell::Date(date_from_days(days)),
        other => return Err(format!("unsupported DuckDB result value {other:?}")),
    };
    Ok(cell)
}

/// Convert days since 1970-01-01 to Prela's packed `yyyymmdd` date.
fn date_from_days(days: i32) -> i64 {
    let z = i64::from(days) + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    year * 10_000 + month * 100 + day
}

#[cfg(test)]
mod tests {
    use super::date_from_days;

    #[test]
    fn date_conversion_matches_boundaries() {
        assert_eq!(date_from_days(0), 19_700_101);
        assert_eq!(date_from_days(10_957), 20_000_101);
        assert_eq!(date_from_days(-1), 19_691_231);
    }
}
