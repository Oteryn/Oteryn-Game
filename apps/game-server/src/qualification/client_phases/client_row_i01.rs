//! Captures the real domain-14 payload that exposes a carried container's capacity and entries.

use super::*;
use oteryn_protocol_oteryn::container_tree as wire;
use std::num::NonZeroU64;

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

fn capture(snapshot: &[u8], update: &[u8], revisions: &str, expected: &str) -> TestResult {
    let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
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
    result?;

    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    let mut client = connect_world(clients, 0xc1).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
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
        let update = client
            .session_mut()
            .read_gated_delta(
                wire::STATE_DOMAIN_CONTAINER_VIEWS,
                wire::DELTA_TYPE_CONTAINER_VIEWS_V1,
            )
            .await?;
        let views = wire::decode_container_views(&update.payload)
            .map_err(|error| format!("I01 container update decode: {error:?}"))?;
        let expected = visible(&views);
        capture(
            &snapshot.payload,
            &update.payload,
            &format!(
                "{} {} {} {}",
                snapshot.revision,
                update.base_revision,
                update.new_revision,
                update.server_sequence
            ),
            &expected,
        )?;
        evidence(&format!(
            "e2e_all scenario=I01.client status=PASS evidence=domain-14-real-payload;{}",
            expected.replace([' ', '\n'], "_")
        ));
        Ok(())
    })
    .await
    .unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result = row_complete(journey(clients)).await;
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
    fn no_container_views_produce_no_visible_rows() {
        assert_eq!(visible(&wire::ContainerViews { views: vec![] }), "");
    }
}
