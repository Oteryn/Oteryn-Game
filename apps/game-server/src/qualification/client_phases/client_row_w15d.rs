//! W15d client replay capture: the player walks to the Way of the Monk guidance shrine and uses
//! it. The raw quest-journal `List` deltas received before and after the use are written byte
//! for byte with the join snapshot; the consumer is the native quest reducer
//! (`crates/client-runtime/src/features/0016-quest-log/state.rs`, `tests/e2e_w15d.rs`). The
//! shrine callback is one-way: the earlier full row already ran it for this Character, so the
//! use may be refused (`Rejected`, list unchanged) or commit (list changed); the observed outcome
//! is recorded and the list is asserted against it.
use super::*;
use crate::map::view::MapFacts;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::quest_log as journal;
use oteryn_protocol_oteryn::world_object::UseDisposition;

const SHRINE: TilePos = TilePos {
    x: 32454,
    y: 32292,
    floor: -7,
};
const APPEARANCE: u16 = 50242;

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&root).join(name), bytes)?;

    Ok(())
}

struct Listed {
    base: u64,
    new: u64,
    sequence: u64,
    payload: Vec<u8>,
    entries: Vec<journal::QuestListEntry>,
}

fn list_text(entries: &[journal::QuestListEntry]) -> String {
    entries
        .iter()
        .map(|entry| format!("{}:{}", entry.quest_index, u8::from(entry.completed)))
        .collect::<Vec<_>>()
        .join(";")
}

async fn list(client: &mut DevClientSession) -> TestResult<Listed> {
    let deadline = tokio::time::Instant::now() + Duration::from_millis(1050);
    while tokio::time::Instant::now() < deadline {
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    client.take_events();
    let payload = journal::encode_quest_log_query(&journal::QuestLogQuery::List)
        .map_err(|error| format!("list encode: {error:?}"))?;
    let reply = journal::send(client.session_mut(), &payload).await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("quest list reply={reply:?}").into());
    }
    let delta = client
        .session_mut()
        .read_gated_delta(journal::STATE_DOMAIN_QUEST_LOG, 1)
        .await?;
    let entries = match journal::decode_quest_log(&delta.payload)
        .map_err(|error| format!("quest list decode={error:?}"))?
        .view
    {
        Some(journal::QuestLogView::List(entries)) => entries,
        other => return Err(format!("quest list view={other:?}").into()),
    };
    Ok(Listed {
        base: delta.base_revision,
        new: delta.new_revision,
        sequence: delta.server_sequence,
        payload: delta.payload,
        entries,
    })
}

fn shrine_placement(world: &BundleWorld) -> TestResult<(Vec<u8>, u64)> {
    let tile = world
        .base()
        .tile(SHRINE.x, SHRINE.y, SHRINE.floor)
        .ok_or("authored guidance shrine tile absent")?;
    let ordinal = tile
        .ids()
        .iter()
        .zip(tile.depths())
        .filter(|(_, depth)| **depth == 0)
        .enumerate()
        .find_map(|(ordinal, (id, _))| {
            let ordinal = u8::try_from(ordinal).ok()?;
            world
                .facts()
                .base_entry(SHRINE, ordinal, *id)
                .filter(|facts| facts.appearance_id == APPEARANCE)
                .map(|_| ordinal)
        })
        .ok_or("authored guidance shrine appearance absent")?;
    let key = crate::map::view::placement_key(SHRINE, ordinal).ok_or("shrine placement key")?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok((placement, world.facts().object_revision(key)))
}

async fn journey(clients: &SeamClients<'_>, world: &BundleWorld) -> TestResult<String> {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(180);
    let mut client = connect_world(clients, 0x88).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
    let snapshot = client
        .session_mut()
        .gated_snapshot(journal::STATE_DOMAIN_QUEST_LOG)
        .ok_or("step=snapshot quest log snapshot absent")?
        .clone();
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        at.floor == SHRINE.floor
            && at.x.abs_diff(SHRINE.x).max(at.y.abs_diff(SHRINE.y)) <= 1
            && at != SHRINE
    })
    .ok_or("step=walk no player route to the guidance shrine")?;
    walk(&mut client, world, &path).await?;
    let (placement, revision) = shrine_placement(world)?;
    let before = list(&mut client).await?;
    let used = client.session_mut().use_object(&placement, revision).await?;
    let outcome = match (used.status, used.disposition) {
        (CommandStatus::Accepted, UseDisposition::Committed) => "Committed",
        (CommandStatus::Rejected, UseDisposition::Rejected) => "Rejected",
        other => return Err(format!("step=use observed={other:?}").into()),
    };
    let after = list(&mut client).await?;
    let changed = after.entries != before.entries;
    if changed != (outcome == "Committed") {
        return Err(format!("step=list use={outcome} list_changed={changed}").into());
    }
    let expected = format!(
        "snapshot_revision={}\nuse={outcome}\nlist_before={}\nbefore_base={}\nbefore_new={}\nbefore_sequence={}\n\
         list_after={}\nafter_base={}\nafter_new={}\nafter_sequence={}\n",
        snapshot.revision,
        list_text(&before.entries),
        before.base,
        before.new,
        before.sequence,
        list_text(&after.entries),
        after.base,
        after.new,
        after.sequence,
    );
    capture("w15d-snapshot.bin", &snapshot.payload)?;
    capture("w15d-before.bin", &before.payload)?;
    capture("w15d-after.bin", &after.payload)?;
    capture("w15d-expected.txt", expected.as_bytes())?;

    Ok(format!("use={outcome} before={} after={}", list_text(&before.entries), list_text(&after.entries)))

    }).await.unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>, world: &BundleWorld) {
    match row_complete(journey(clients, world)).await {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=W15d.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W15d.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=W15d.client status=FAIL evidence=timeout"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quest_list_text_keeps_completion_and_identity() {
        assert_eq!(list_text(&[]), "");
        let entry = journal::QuestListEntry {
            quest_index: u32::MAX,
            completed: true,
        };
        assert_eq!(list_text(&[entry]), "4294967295:1");
    }
}

#[cfg(test)]
mod world_helper_tests {
    use super::*;
    #[test]
    fn absent_guidance_shrine_has_no_invented_placement() -> TestResult {
        let world = row_helper_world()?;
        assert!(shrine_placement(&world).is_err());
        Ok(())
    }
}
