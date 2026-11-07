//! I01 replay coverage for the session state used by `tools/dev-client/src/lib.rs`.
//! The consuming code is `container_tree::initial` and `container_tree::apply` in
//! `crates/session/src/container_tree.rs`; the latter stores updates in `Session::container_views`.

use super::*;

fn entry(
    handle: u64,
    definition: u32,
    count: u32,
) -> Result<ItemEntry, Box<dyn std::error::Error>> {
    Ok(ItemEntry {
        handle: ItemHandle::new(handle).ok_or("zero fixture handle")?,
        item_definition_ref: definition.try_into()?,
        count: count.try_into()?,
        sub_type: 0,
    })
}

fn visible(views: &ContainerViews) -> String {
    let mut output = String::new();
    for view in &views.views {
        output.push_str(&format!(
            "view {} {} {} {}\n",
            view.view_id,
            view.container_handle,
            view.parent_handle.map_or(0, ItemHandle::get),
            view.capacity
        ));
        for item in &view.entries {
            output.push_str(&format!(
                "item {} {} {}\n",
                item.handle, item.item_definition_ref, item.count
            ));
        }
    }
    output
}

#[test]
fn e2e_i01_replays_container_capacity_and_stack_quantities()
-> Result<(), Box<dyn std::error::Error>> {
    let (snapshot_payload, update_payload, revisions, expected) =
        if let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
            let directory = std::path::Path::new(&directory);
            (
                std::fs::read(directory.join("i01-snapshot.bin"))?,
                std::fs::read(directory.join("i01-update.bin"))?,
                std::fs::read_to_string(directory.join("i01-revisions.txt"))?,
                std::fs::read_to_string(directory.join("i01-expected.txt"))?,
            )
        } else {
            let nested = entry(12, 2_854, 1)?;
            let stack = entry(13, 3_031, 37)?;
            let views = ContainerViews {
                views: vec![ContainerView {
                    view_id: 1,
                    container_handle: ItemHandle::new(10).ok_or("fixture handle")?,
                    parent_handle: None,
                    capacity: 20,
                    entries: vec![nested, stack],
                    browse: None,
                }],
            };
            (
                encode_container_views(&ContainerViews::default())
                    .map_err(|error| format!("{error:?}"))?,
                encode_container_views(&views).map_err(|error| format!("{error:?}"))?,
                "1 1 2 41".into(),
                visible(&views),
            )
        };
    let numbers = revisions
        .split_whitespace()
        .map(str::parse::<u64>)
        .collect::<Result<Vec<_>, _>>()?;
    let [
        snapshot_revision,
        base_revision,
        new_revision,
        _server_sequence,
    ] = numbers.as_slice()
    else {
        return Err("invalid I01 capture revision metadata".into());
    };
    if snapshot_revision != base_revision || new_revision <= base_revision {
        return Err("inconsistent I01 capture revisions".into());
    }
    let selected = [CAPABILITY_CONTAINER_TREE_V1];
    let mut snapshots = vec![GatedSnapshot {
        domain_id: STATE_DOMAIN_CONTAINER_VIEWS,
        snapshot_type: SNAPSHOT_TYPE_CONTAINER_VIEWS_V1,
        revision: *snapshot_revision,
        payload: snapshot_payload,
    }];
    let mut state = initial(&selected, &snapshots)?.ok_or("I01 snapshot state absent")?;
    assert_eq!(
        state,
        decode_container_views(&snapshots[0].payload).map_err(|error| format!("{error:?}"))?
    );
    snapshots[0].revision = *new_revision;
    snapshots[0].payload = update_payload;
    state = initial(&selected, &snapshots)?.ok_or("I01 update state absent")?;
    assert_eq!(visible(&state), expected);

    let retained = state.clone();
    snapshots[0].payload = vec![0xff];
    assert!(initial(&selected, &snapshots).is_err());
    assert_eq!(state, retained);
    Ok(())
}
