use crate::result::{ResultCell, ResultOrder, ResultSet};

fn text(value: &str) -> ResultCell {
    ResultCell::Text(value.to_owned())
}

#[test]
fn typed_cells_do_not_alias_through_rendering() {
    assert_ne!(ResultCell::Null, text("NULL"));
    assert_ne!(ResultCell::Integer(1), ResultCell::Float(1.0));
    assert_ne!(ResultCell::Float(1.0), ResultCell::decimal(1, 0));
    assert_ne!(text("a|b"), text("a\nb"));
}

#[test]
fn bag_equality_ignores_order_but_preserves_duplicates() {
    let left = ResultSet::from_rows([vec![text("a")], vec![text("b")], vec![text("a")]]);
    let reordered = ResultSet::from_rows([vec![text("b")], vec![text("a")], vec![text("a")]]);
    let missing_duplicate = ResultSet::from_rows([vec![text("a")], vec![text("b")]]);

    assert!(ResultOrder::Bag.equivalent(&["value"], &left, &reordered));
    assert!(!ResultOrder::Bag.equivalent(&["value"], &left, &missing_duplicate));
}

#[test]
fn ordered_equality_allows_only_tie_permutations() {
    let left = ResultSet::from_rows([
        vec![ResultCell::Integer(1), text("a")],
        vec![ResultCell::Integer(1), text("b")],
        vec![ResultCell::Integer(2), text("c")],
    ]);
    let tied_rows_reordered = ResultSet::from_rows([
        vec![ResultCell::Integer(1), text("b")],
        vec![ResultCell::Integer(1), text("a")],
        vec![ResultCell::Integer(2), text("c")],
    ]);
    let key_groups_reordered = ResultSet::from_rows([
        vec![ResultCell::Integer(2), text("c")],
        vec![ResultCell::Integer(1), text("a")],
        vec![ResultCell::Integer(1), text("b")],
    ]);

    let order = ResultOrder::OrderedBy(&["key"]);
    let columns = &["key", "value"];
    assert!(order.equivalent(columns, &left, &tied_rows_reordered));
    assert!(!order.equivalent(columns, &left, &key_groups_reordered));
    assert!(!ResultOrder::OrderedBy(&["missing"]).equivalent(columns, &left, &left));
}

#[test]
fn numbers_are_not_rounded_for_equality() {
    assert_ne!(ResultCell::Float(1.001), ResultCell::Float(1.002));
    assert_eq!(ResultCell::decimal(12_300, 4), ResultCell::decimal(123, 2));
}
