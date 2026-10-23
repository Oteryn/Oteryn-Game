use super::*;
use std::path::Path;

pub(super) fn runtime_fragments() -> Result<Vec<Value>, Box<dyn Error>> {
    let directory =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../protocol-oteryn/capabilities");
    let mut entries: Vec<Value> = Vec::new();
    for item in std::fs::read_dir(directory)? {
        let item = item?;
        if item.path().extension().is_some_and(|ext| ext == "json") {
            entries.push(serde_json::from_str(&std::fs::read_to_string(
                item.path(),
            )?)?);
        }
    }
    entries.sort_by_key(|entry| entry["id"].as_u64());
    Ok(entries)
}

#[test]
fn all_capability_offer_sets_and_gates_match_fragments() -> Result<(), Box<dyn Error>> {
    let entries = runtime_fragments()?;
    for (set, offered) in [
        ("production", PRODUCTION_OFFERED_CAPABILITIES),
        ("item_view", ITEM_VIEW_OFFERED_CAPABILITIES),
        ("bundle_world", BUNDLE_WORLD_OFFERED_CAPABILITIES),
    ] {
        let expected: Vec<_> = entries
            .iter()
            .filter(|entry| entry["offers"][set] == true)
            .map(|entry| {
                Ok((
                    u32::try_from(entry["id"].as_u64().ok_or("id")?)?,
                    ids(&entry["requires"])?,
                ))
            })
            .collect::<Result<_, Box<dyn Error>>>()?;
        assert_eq!(
            offered
                .iter()
                .map(|offer| (offer.id, offer.requires.to_vec()))
                .collect::<Vec<_>>(),
            expected,
            "{set}"
        );
        assert!(offered.len() <= SELECTED_CAPACITY);
    }
    for entry in entries.iter().filter(|entry| !entry["domains"].is_null()) {
        let capability = u32::try_from(entry["id"].as_u64().ok_or("id")?)?;
        let selected = selection(&[capability]);
        for command in ids(&entry["commands"])? {
            assert!(
                !SelectedCapabilities::NONE.command_selected(command),
                "{command}"
            );
            assert!(selected.command_selected(command), "{command}");
        }
        for domain in ids(&entry["domains"])? {
            assert!(
                !SelectedCapabilities::NONE.domain_selected(domain),
                "{domain}"
            );
            assert!(selected.domain_selected(domain), "{domain}");
        }
    }
    Ok(())
}
