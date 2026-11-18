//! Captures the real domain-14 payload that exposes a carried container's capacity and entries.

use super::*;
use oteryn_protocol_oteryn::container_tree as wire;
use std::num::NonZeroU64;

fn opened_backpack(views: &wire::ContainerViews, handle: NonZeroU64) -> TestResult<String> {
    if !views.views.iter().any(|view| view.container_handle == handle && view.browse.is_none()) {
        return Err(format!("I01 Opened reply did not publish a view for backpack {handle}").into());
    }
    Ok(visible(views))
}

fn visible(views: &wire::ContainerViews) -> String {
    let mut output = String::new();
    for view in &views.views {
        output.push_str(&format!(
            "view {} {} {} {}\n",
            view.view_id,
            view.container_handle,
            view.parent_handle.map_or(0, NonZeroU64::get),
            view.capacity
        ));
        for entry in &view.entries {
            output.push_str(&format!(
                "item {} {} {}\n",
                entry.handle, entry.item_definition_ref, entry.count
            ));
        }
    }
    output
}

fn capture(snapshot: &[u8], update: &[u8], revisions: &str, expected: &str) {
    let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    let result = (|| -> TestResult {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()?;
        let directory = Path::new(&directory).canonicalize()?;
        if !directory.starts_with(root) {
            return Err("E2E_ALL_CAPTURE_DIR must be inside the Game worktree".into());
        }
        std::fs::write(directory.join("i01-snapshot.bin"), snapshot)?;
        std::fs::write(directory.join("i01-update.bin"), update)?;
        std::fs::write(directory.join("i01-revisions.txt"), revisions)?;
        std::fs::write(directory.join("i01-expected.txt"), expected)?;
        Ok(())
    })();
    if let Err(error) = result {
        evidence(&format!("I01_CLIENT_CAPTURE_WRITE_ERROR error={error}"));
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_world(clients, 0xc1).await?;
    let backpack = client
        .inventory()
        .and_then(|inventory| inventory.main_backpack)
        .ok_or("I01 client capture starter backpack absent")?;
    let snapshot = client
        .session_mut()
        .gated_snapshot(wire::STATE_DOMAIN_CONTAINER_VIEWS)
        .ok_or("I01 client capture container snapshot absent")?
        .clone();
    let payload = wire::encode_container_view_intent(&wire::ContainerViewIntent::Open {
        handle: backpack.handle,
        replace_view: None,
    })
    .map_err(|error| format!("I01 container intent encode: {error:?}"))?;
    let after = client.last_server_sequence();
    client.take_events();
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_CONTAINER_TREE_V1,
            wire::COMMAND_TYPE_CONTAINER_VIEW_INTENT,
            &payload,
        )
        .await?;
    if reply.status != CommandStatus::Accepted
        || wire::decode_container_view_result(&reply.payload)
            .map_err(|error| format!("I01 container result decode: {error:?}"))?
            != wire::ContainerViewOutcome::Opened
    {
        return Err("I01 starter backpack open refused".into());
    }
    let selected = client.selected_capabilities();
    let browse = selected.contains(&oteryn_protocol_oteryn::browse_field::CAPABILITY_BROWSE_FIELD_V1);
    let house = selected.contains(&wire::CAPABILITY_HOUSE_CONTAINER_CAPACITY_V2);
    let appearances = selected.contains(&oteryn_protocol_oteryn::item_view::CAPABILITY_ITEM_APPEARANCE_V1);
    let encode = move |views: &wire::ContainerViews| {
        wire::encode_container_views_with_appearances(views, browse, house, appearances)
            .map_err(|error| format!("I01 container views encode: {error:?}"))
    };
    let mut baseline = (snapshot.revision, snapshot.payload);
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let (update, expected) = 'views: loop {
        for event in client.take_events() {
            let oteryn_dev_client::SessionEvent::ContainerViews(frame) = event else { continue; };
            if frame.server_sequence > after
                && frame.base_revision == baseline.0
                && let Ok(expected) = opened_backpack(&frame.value, backpack.handle)
            {
                break 'views (frame, expected);
            }
            baseline = (frame.new_revision, encode(&frame.value)?);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("I01 Opened without a fresh view for backpack {}", backpack.handle).into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    };
    capture(
        &baseline.1,
        &encode(&update.value)?,
        &format!(
            "{} {} {} {}",
            baseline.0, update.base_revision, update.new_revision, update.server_sequence
        ),
        &expected,
    );
    evidence(&format!(
        "e2e_all scenario=I01.client status=PASS evidence=domain-14-real-payload;{}",
        expected.replace([' ', '\n'], "_")
    ));
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result = verdict::indexed_timeout(Duration::from_secs(30), verdict::observe(journey(clients))).await;
    if let Err(error) = result.unwrap_or_else(|_| Err("I01 client capture timeout".into())) {
        evidence(&format!(
            "e2e_all scenario=I01.client status=FAIL evidence={}",
            error.to_string().replace([' ', '\n'], "_")
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opened_backpack_requires_its_own_view() -> TestResult {
        let handle = NonZeroU64::new(7).ok_or("backpack handle")?;
        let mut views = wire::ContainerViews::default();
        assert!(opened_backpack(&views, handle).is_err());
        views.views.push(wire::ContainerView {
            view_id: 0,
            container_handle: NonZeroU64::MIN,
            parent_handle: None,
            capacity: 20,
            entries: Vec::new(),
            browse: None,
        });
        assert!(opened_backpack(&views, handle).is_err());
        views.views[0].container_handle = handle;
        assert!(opened_backpack(&views, handle).is_ok());
        Ok(())
    }
}
