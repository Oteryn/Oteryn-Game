#[test]
fn holdings_labels_use_server_names_and_retain_unknown_keys() {
    use oteryn_client_runtime::cyclopedia_character::items::Row;
    let mut items = crate::cyclopedia_character::ItemSummary::default();
    let row = Row {
        definition: "oteryn:item.tibia.i3031".into(),
        quantities: [0, 0, 7, 0],
    };
    assert_eq!(super::display_name(&items, &row), row.definition);
    items
        .names
        .insert(row.definition.clone(), "gold coin".into());
    assert_eq!(super::display_name(&items, &row), "gold coin");
}
