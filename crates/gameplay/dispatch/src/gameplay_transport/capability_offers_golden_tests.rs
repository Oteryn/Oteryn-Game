use super::*;

#[test]
fn atelier_v2_negotiates_only_bundle_clients_with_its_wheel_parent() {
    use oteryn_protocol_oteryn::atelier::v2 as atelier;
    use oteryn_protocol_oteryn::wheel::CAPABILITY_WHEEL_ALLOCATION_V1;

    let supported = [
        CAPABILITY_WHEEL_ALLOCATION_V1,
        atelier::CAPABILITY_GEM_ATELIER_V2,
    ];
    let commands = [
        atelier::COMMAND_TYPE_ATELIER_V2_QUERY,
        atelier::COMMAND_TYPE_ATELIER_V2_SET_LOCK,
        atelier::COMMAND_TYPE_ATELIER_V2_MUTATE,
        atelier::COMMAND_TYPE_ATELIER_V2_REPLACE_VESSEL,
        atelier::COMMAND_TYPE_ATELIER_V2_RESHAPE,
    ];
    let selected = SelectedCapabilities::select(BUNDLE_WORLD_OFFERED_CAPABILITIES, &supported)
        .expect("bundle Atelier v2 selection");
    assert_eq!(selected.as_slice(), supported);
    for command in commands {
        assert!(selected.command_selected(command), "command {command}");
    }
    for supported in [&supported[..1], &supported[1..]] {
        let selected = SelectedCapabilities::select(BUNDLE_WORLD_OFFERED_CAPABILITIES, supported)
            .expect("legacy or missing-parent selection");
        assert!(!selected.contains(atelier::CAPABILITY_GEM_ATELIER_V2));
        for command in commands {
            assert!(!selected.command_selected(command), "command {command}");
        }
    }
    for offered in [
        PRODUCTION_OFFERED_CAPABILITIES,
        ITEM_VIEW_OFFERED_CAPABILITIES,
    ] {
        assert!(
            !offered
                .iter()
                .any(|entry| entry.id == atelier::CAPABILITY_GEM_ATELIER_V2)
        );
        let selected =
            SelectedCapabilities::select(offered, &supported).expect("non-bundle selection");
        for command in commands {
            assert!(!selected.command_selected(command), "command {command}");
        }
    }
    let all: Vec<_> = BUNDLE_WORLD_OFFERED_CAPABILITIES
        .iter()
        .map(|entry| entry.id)
        .collect();
    let selected = SelectedCapabilities::select(BUNDLE_WORLD_OFFERED_CAPABILITIES, &all)
        .expect("complete bundle selection fits existing capacity");
    assert_eq!(selected.as_slice(), all);
}

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
