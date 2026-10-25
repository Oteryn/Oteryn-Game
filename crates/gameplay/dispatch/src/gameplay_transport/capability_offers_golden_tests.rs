use super::*;

#[test]
fn caps_split_1_golden_offered_selection() -> Result<(), Box<dyn Error>> {
    let everything: Vec<u32> = registry_capabilities()?
        .iter()
        .map(|entry| Ok(u32::try_from(entry["id"].as_u64().ok_or("id")?)?))
        .collect::<Result<_, Box<dyn Error>>>()?;
    for (set, offered) in [
        ("production", PRODUCTION_OFFERED_CAPABILITIES),
        ("item_view", ITEM_VIEW_OFFERED_CAPABILITIES),
        ("bundle_world", BUNDLE_WORLD_OFFERED_CAPABILITIES),
    ] {
        let expected = golden_offer_ids(set)?;
        assert_eq!(
            SelectedCapabilities::select(offered, &everything)
                .as_ref()
                .map(SelectedCapabilities::as_slice),
            Some(&expected[..]),
            "{set}"
        );
    }
    assert_eq!(
        SelectedCapabilities::select(ITEM_VIEW_OFFERED_CAPABILITIES, &[4, 13])
            .as_ref()
            .map(SelectedCapabilities::as_slice),
        Some(&[13][..])
    );
    Ok(())
}
