//! W15d client replay capture: the player walks to the Way of the Monk guidance shrine and uses
//! it. The raw quest-journal `List` deltas received before and after the use are written byte
//! for byte with the join snapshot; the consumer is the native quest reducer
//! (`crates/client-runtime/src/features/0016-quest-log/state.rs`, `tests/e2e_w15d.rs`). The
//! selected run requires a committed first use, a refused repeat, durable
//! journal state after relog, and replay of the live bytes through that reducer.
use super::*;
use full_row_w15d::{APPEARANCE, SHRINE, approach, assert_guidance_reward, logout_once, object_target, prepare, shrine_revision};
use oteryn_dev_client::client_runtime::quest_log as journal;
use oteryn_dev_client::{AppliedDelta, DevClientSession};
use oteryn_protocol_oteryn::world_object::UseDisposition;

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        std::fs::write(Path::new(&root).join(name), bytes)?;
    }
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

impl Listed {
    fn decoded(&self) -> TestResult<AppliedDelta<journal::QuestLog>> {
        Ok(AppliedDelta {
            server_sequence: self.sequence,
            base_revision: self.base,
            new_revision: self.new,
            value: journal::decode_quest_log(&self.payload)
                .map_err(|error| format!("replay list decode={error:?}"))?,
        })
    }
}

async fn journey(clients: &SeamClients<'_>, world: &BundleWorld) -> TestResult<String> {
    let character =
        character_prep::dedicated_character(clients, "NODE_BOOT_Q_W15D_ACCOUNT").await?;
    let scope = character.clients(clients);
    let mut client = prepare("W15d.client", &scope, 0x88).await?;
    approach(&mut client, world).await?;
    let mut snapshot = client
        .session_mut()
        .gated_snapshot(journal::STATE_DOMAIN_QUEST_LOG)
        .ok_or("step=snapshot quest log snapshot absent")?
        .clone();
    let target = object_target(world, SHRINE, APPEARANCE)?;
    let mut before = list(&mut client).await?;
    assert_guidance_reward(&client, 0)?;
    let used = client
        .session_mut()
        .use_object(&target.placement, target.expected_revision)
        .await?;
    if used.status != CommandStatus::Accepted || used.disposition != UseDisposition::Committed {
        return Err(format!("step=use observed={used:?}").into());
    }
    let mut after = list(&mut client).await?;
    assert_guidance_reward(&client, 1)?;
    if after.entries == before.entries {
        return Err("step=list committed USE produced no quest-log delta".into());
    }
    let observed_revision = client
        .world_object_overlay()
        .iter()
        .find(|entry| entry.placement == target.placement)
        .map(|entry| entry.revision);
    let revision = shrine_revision(target.expected_revision, observed_revision)?;
    let repeated = client
        .session_mut()
        .use_object(&target.placement, revision)
        .await?;
    if repeated.status != CommandStatus::Rejected
        || repeated.disposition != UseDisposition::Rejected
        || list(&mut client).await?.entries != after.entries
    {
        return Err(format!("step=repeat-use observed={repeated:?}").into());
    }
    assert_guidance_reward(&client, 1)?;
    let session = client.join_snapshot().game_session_id;
    logout_once(&mut client).await?;
    let selected = client.selected_capabilities().to_vec();
    drop(client);
    let mut fresh = connect_character(&scope, 0x89, 0).await?;
    if fresh.join_snapshot().game_session_id == session
        || list(&mut fresh).await?.entries != after.entries
    {
        return Err("step=relog missing durable post-USE quest list".into());
    }
    assert_guidance_reward(&fresh, 1)?;
    logout_once(&mut fresh).await?;

    let outcome = "Committed";
    let expected = format!(
        "session={session:?}\nsnapshot_revision={}\nuse={outcome}\nlist_before={}\nbefore_base={}\nbefore_new={}\nbefore_sequence={}\n\
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
    // The live admission owns this directory. Refuse an existing directory so
    // a previous run cannot supply any member of this capture.
    let root = std::env::temp_dir().join(format!("w15d-{}", uuid_text(session.as_bytes())));
    std::fs::create_dir(&root)?;
    std::fs::write(root.join("snapshot.bin"), &snapshot.payload)?;
    std::fs::write(root.join("before.bin"), &before.payload)?;
    std::fs::write(root.join("after.bin"), &after.payload)?;
    std::fs::write(root.join("manifest.txt"), &expected)?;
    if std::fs::read_to_string(root.join("manifest.txt"))? != expected
        || before.sequence >= used.result_server_sequence
        || used.result_server_sequence >= after.sequence
    {
        return Err("W15d capture session or sequence mismatch".into());
    }
    snapshot.payload = std::fs::read(root.join("snapshot.bin"))?;
    before.payload = std::fs::read(root.join("before.bin"))?;
    after.payload = std::fs::read(root.join("after.bin"))?;
    let mut replay = journal::QuestState::from_snapshot(&selected, Some(&snapshot))
        .map_err(|error| format!("replay snapshot={error:?}"))?;
    replay
        .apply(&before.decoded()?)
        .map_err(|error| format!("replay before={error:?}"))?;
    if replay.list != before.entries {
        return Err("replay before list mismatch".into());
    }
    let after_delta = after.decoded()?;
    replay
        .apply(&after_delta)
        .map_err(|error| format!("replay after={error:?}"))?;
    if replay.list != after.entries || replay.revision != Some(after.new) {
        return Err("replay after list or revision mismatch".into());
    }
    let kept = replay.clone();
    if replay.apply(&after_delta) != Err(journal::QuestRefusal::RevisionMismatch) || replay != kept
    {
        return Err("replay stale delta changed quest state".into());
    }
    capture("w15d-snapshot.bin", &snapshot.payload)?;
    capture("w15d-before.bin", &before.payload)?;
    capture("w15d-after.bin", &after.payload)?;
    capture("w15d-expected.txt", expected.as_bytes())?;
    Ok(format!(
        "use={outcome} before={} after={}",
        list_text(&before.entries),
        list_text(&after.entries)
    ))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>, world: &BundleWorld) {
    let started = std::time::Instant::now();
    match verdict::indexed_timeout(
        Duration::from_secs(900),
        verdict::observe(journey(clients, world)),
    )
    .await
    {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=W15d.client status=PASS seconds={:.3} evidence={observed}",
            started.elapsed().as_secs_f64()
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W15d.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=W15d.client status=FAIL evidence=timeout"),
    }
}
