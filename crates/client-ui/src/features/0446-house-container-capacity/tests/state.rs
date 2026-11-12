use crate::{container_tree::ContainerPanel, house_container_capacity::project};
use oteryn_client_runtime::container_tree::{
    ContainerView, ContainerViewIntent, ContainerViews, ItemEntry,
    decode_container_views_with_house_capacity, encode_container_views_with_house_capacity,
};
#[test]
fn negotiated_twenty_fifth_entry_is_visible_and_selectable()
-> Result<(), Box<dyn std::error::Error>> {
    let mut entries = Vec::new();
    for id in 2_u64..=26 {
        entries.push(ItemEntry {
            handle: id.try_into()?,
            item_definition_ref: 2854_u32.try_into()?,
            count: 1_u32.try_into()?,
            sub_type: 0,
        });
    }
    let views = ContainerViews {
        views: vec![ContainerView {
            view_id: 0,
            container_handle: 1_u64.try_into()?,
            parent_handle: None,
            capacity: 25,
            entries,
            browse: None,
        }],
    };
    let bytes = encode_container_views_with_house_capacity(&views, false, true)
        .map_err(|error| format!("{error:?}"))?;
    assert!(decode_container_views_with_house_capacity(&bytes, false, false).is_err());
    let decoded = decode_container_views_with_house_capacity(&bytes, false, true)
        .map_err(|error| format!("{error:?}"))?;
    let mut legacy = ContainerPanel::default();
    assert!(!project(&mut legacy, Some(decoded.clone())));
    assert!(legacy.views.is_none());
    let mut panel = ContainerPanel::default();
    panel.house_capacity_v2 = true;
    assert!(project(&mut panel, Some(decoded)));
    assert_eq!(panel.rows().len(), 25);
    assert!(
        crate::house_container_capacity::view::panel_text(&panel)
            .contains("House container — 25 / 25 items")
    );
    let last = 26_u64.try_into()?;
    assert!(panel.select(last));
    assert!(
        matches!(panel.open_selected(),Some(ContainerViewIntent::Open{handle,..}) if handle==last)
    );
    Ok(())
}

#[test]
fn negotiated_extension_keeps_ordinary_backpack_label_generic()
-> Result<(), Box<dyn std::error::Error>> {
    let mut panel = ContainerPanel::default();
    panel.house_capacity_v2 = true;
    assert!(project(
        &mut panel,
        Some(ContainerViews {
            views: vec![ContainerView {
                view_id: 0,
                container_handle: 7_u64.try_into()?,
                parent_handle: None,
                capacity: 20,
                entries: vec![],
                browse: None,
            }]
        })
    ));
    let text = crate::house_container_capacity::view::panel_text(&panel);
    assert!(text.contains("Container 7"));
    assert!(!text.contains("House container"));
    Ok(())
}
