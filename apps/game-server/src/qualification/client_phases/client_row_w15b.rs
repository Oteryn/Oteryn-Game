//! Replay the fixed Battle Axe chest interaction from its original inbound wire frames.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::quest_log as journal;
use oteryn_protocol_oteryn::world_object::UseDisposition;

#[derive(PartialEq, Eq)]
struct Listed {
    base: u64,
    new: u64,
    sequence: u64,
    payload: Vec<u8>,
    entries: Vec<journal::QuestListEntry>,
}

async fn list(client: &mut DevClientSession) -> TestResult<Listed> {
    let deadline = tokio::time::Instant::now() + Duration::from_millis(1050);
    while tokio::time::Instant::now() < deadline {
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    client.take_events();
    list_session(client.session_mut()).await
}

async fn list_session<S: oteryn_dev_client::native_session::SessionStream>(
    session: &mut oteryn_dev_client::native_session::Session<S>,
) -> TestResult<Listed> {
    let payload = journal::encode_quest_log_query(&journal::QuestLogQuery::List)
        .map_err(|error| format!("list encode: {error:?}"))?;
    let reply = journal::send(session, &payload).await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("quest list reply={reply:?}").into());
    }
    let delta = session
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

// This isolated character starts with no displayed quests. Global's displayed
// Quest Log omits the Battle Axe Quest, including after its chest is claimed.
pub(super) fn check_baseline(entries: &[journal::QuestListEntry]) -> TestResult {
    if !entries.is_empty() {
        return Err(
            format!("W15b expected an empty fixture Quest Log, observed {entries:?}").into(),
        );
    }
    Ok(())
}

fn check_relog_refusal(disposition: UseDisposition, unchanged: bool) -> TestResult {
    if disposition != UseDisposition::NothingToUse || !unchanged {
        return Err(format!(
            "W15b relogged chest use disposition={disposition:?} unchanged={unchanged}"
        )
        .into());
    }
    Ok(())
}

#[derive(Clone)]
struct Chest {
    at: TilePos,
    placement: Vec<u8>,
    claim: String,
}

fn quest_chests(world: &BundleWorld) -> TestResult<Vec<Chest>> {
    let items = provisioning::item_index()?;
    let report = crate::content::world_reward_claims::bind_embedded_reward_claims(
        world.base().as_ref(),
        &|item| {
            item.key()
                .as_str()
                .strip_prefix("oteryn:item.tibia.i")
                .and_then(|id| id.parse().ok())
                .is_some_and(|id| provisioning::item_reference(&items, id).is_ok())
        },
    )?;
    let mut chests = Vec::new();
    for claim in report.served {
        for placement in claim.placements {
            if placement.placement.quest_transition.is_none() {
                continue;
            }
            let mut key = world.base().digest().to_vec();
            key.extend_from_slice(&placement.bundle_placement_key.to_be_bytes());
            chests.push(Chest {
                at: TilePos {
                    x: placement.cell.x,
                    y: placement.cell.y,
                    floor: placement.cell.floor,
                },
                placement: key,
                claim: claim.key.as_str().to_owned(),
            });
        }
    }
    chests.sort_by_key(|chest| (chest.at, chest.claim.clone()));
    Ok(chests)
}

async fn use_chest(client: &mut DevClientSession, chest: &Chest) -> TestResult<UseDisposition> {
    Ok(client
        .session_mut()
        .use_object(&chest.placement, 0)
        .await?
        .disposition)
}

struct ReplayExpected {
    session_id: crate::foundation::GameSessionId,
    sequence: u64,
    placement: Vec<u8>,
    before_inventory: oteryn_protocol_oteryn::item_view::CharacterInventory,
    after_inventory: oteryn_protocol_oteryn::item_view::CharacterInventory,
    awarded: super::full_row_w15b::Custody,
    before: Listed,
    after: Listed,
    duplicate: Listed,
}

async fn journey(clients: &SeamClients<'_>, world: &BundleWorld) -> TestResult<ReplayExpected> {
    use super::full_row_w15b::{
        BATTLE_AXE_CHEST, BATTLE_AXE_CLAIM, connect_at_battle_axe, custody, reward_increased,
    };
    let mut client = connect_at_battle_axe(clients, 0x87, 0, "W15b.client").await?;
    let before_inventory = client
        .inventory()
        .ok_or("missing pre-claim inventory")?
        .clone();
    let before_custody = custody(&client)?;
    let before = list(&mut client).await?;
    check_baseline(&before.entries)?;
    let chest = quest_chests(world)?
        .into_iter()
        .find(|chest| chest.at == BATTLE_AXE_CHEST && chest.claim == BATTLE_AXE_CLAIM)
        .ok_or("step=use exact Battle Axe chest is not served")?;
    let first = use_chest(&mut client, &chest).await?;
    if first != UseDisposition::Committed {
        return Err(format!("step=use claim={} disposition={first:?}", chest.claim).into());
    }
    let after = list(&mut client).await?;
    let after_inventory = client
        .inventory()
        .ok_or("missing post-claim inventory")?
        .clone();
    let expected_reward = provisioning::item_reference(&provisioning::item_index()?, 3266)?.get();
    let awarded = custody(&client)?;
    if !reward_increased(&before_custody, &awarded, expected_reward, 1)
        || after.entries != before.entries
    {
        return Err(format!(
            "step=claim claim={} inventory_changed={} quest_changed={}",
            chest.claim,
            awarded != before_custody,
            after.entries != before.entries
        )
        .into());
    }
    let second = use_chest(&mut client, &chest).await?;
    if second != UseDisposition::NothingToUse || custody(&client)? != awarded {
        return Err(format!("step=duplicate-use disposition={second:?}").into());
    }
    let duplicate = list(&mut client).await?;
    if duplicate.entries != after.entries || custody(&client)? != awarded {
        return Err("step=duplicate-use changed quest or inventory state".into());
    }
    let logout = client.session_mut().logout().await?;
    if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step=logout refused: {logout:?}").into());
    }
    Ok(ReplayExpected {
        session_id: client.join_snapshot().game_session_id,
        sequence: client.last_server_sequence(),
        placement: chest.placement,
        before_inventory,
        after_inventory,
        awarded,
        before,
        after,
        duplicate,
    })
}

fn persist_capture(root: &Path, name: &str, bytes: &[u8]) -> TestResult<Vec<u8>> {
    use std::io::Write as _;
    let path = root.join(name);
    let mut capture = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;
    capture.write_all(bytes)?;
    drop(capture);
    let recorded = std::fs::read(path)?;
    if recorded != bytes || recorded.is_empty() {
        return Err("W15b capture changed between recording and replay".into());
    }
    Ok(recorded)
}

async fn capture_and_replay(clients: &SeamClients<'_>, world: &BundleWorld) -> TestResult<String> {
    use oteryn_dev_client::native_session::{Admission, Session, capture_inbound};
    let root =
        std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("W15b live capture directory missing")?;
    let (expected, bytes) = capture_inbound(journey(clients, world)).await;
    let expected = expected?;
    let recorded = persist_capture(Path::new(&root), "w15b-server.wire", &bytes)?;
    let replay_admission = || -> TestResult<Admission<'static>> {
        Ok(Admission {
            appearance_items: None,
            schema_revision: 1,
            character_id: crate::foundation::CharacterId::decode(&clients.characters[0])?,
            admission_material: b"recorded-server-replay",
            client_build_id: "oteryn-dev-client/w15b-replay",
            supported_capabilities: oteryn_dev_client::CLIENT_SUPPORTED_CAPABILITIES,
            deadline: Duration::from_secs(5),
        })
    };
    let stream = tokio::io::join(recorded.as_slice(), tokio::io::sink());
    let mut session = Session::admit(stream, replay_admission()?).await?;
    if session.join_snapshot().game_session_id != expected.session_id
        || session.inventory() != Some(&expected.before_inventory)
    {
        return Err("W15b replay admission identity or inventory mismatch".into());
    }
    let before = list_session(&mut session).await?;
    check_baseline(&before.entries)?;
    let first = session.use_object(&expected.placement, 0).await?;
    let after = list_session(&mut session).await?;
    let second = session.use_object(&expected.placement, 0).await?;
    let duplicate = list_session(&mut session).await?;
    let logout = session.logout().await?;
    if first.disposition != UseDisposition::Committed
        || second.disposition != UseDisposition::NothingToUse
        || before != expected.before
        || after != expected.after
        || duplicate != expected.duplicate
        || session.inventory() != Some(&expected.after_inventory)
        || logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
        || session.last_server_sequence() != expected.sequence
    {
        return Err(
            "W15b replay diverged from live inventory, journal, refusal or sequence".into(),
        );
    }
    let (relog, relog_bytes) = capture_inbound(async {
        let mut fresh = connect_character(clients, 0x89, 0).await?;
        if fresh.join_snapshot().game_session_id == expected.session_id
            || super::full_row_w15b::custody(&fresh)? != expected.awarded
        {
            return Err("W15b fresh admission did not restore the exact reward".into());
        }
        let inventory = fresh.inventory().ok_or("missing relog inventory")?.clone();
        let before = list(&mut fresh).await?;
        if before.entries != expected.after.entries {
            return Err("W15b fresh admission changed the Quest Log".into());
        }
        let reply = fresh
            .session_mut()
            .use_object(&expected.placement, 0)
            .await?;
        let after = list(&mut fresh).await?;
        check_relog_refusal(
            reply.disposition,
            fresh.inventory() == Some(&inventory) && after.entries == before.entries,
        )?;
        let logout = fresh.session_mut().logout().await?;
        if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
            return Err(format!("step=relogin-logout refused: {logout:?}").into());
        }
        TestResult::Ok((
            fresh.join_snapshot().game_session_id,
            fresh.last_server_sequence(),
            inventory,
            before,
            after,
        ))
    })
    .await;
    let (relog_id, relog_sequence, inventory, relog_before, relog_after) = relog?;
    let relog_wire = persist_capture(Path::new(&root), "w15b-relog.wire", &relog_bytes)?;
    let mut session = Session::admit(
        tokio::io::join(relog_wire.as_slice(), tokio::io::sink()),
        replay_admission()?,
    )
    .await?;
    if session.join_snapshot().game_session_id != relog_id
        || session.inventory() != Some(&inventory)
    {
        return Err("W15b relog replay admission identity or inventory mismatch".into());
    }
    let before = list_session(&mut session).await?;
    let reply = session.use_object(&expected.placement, 0).await?;
    let after = list_session(&mut session).await?;
    check_relog_refusal(
        reply.disposition,
        session.inventory() == Some(&inventory) && before == relog_before && after == relog_after,
    )?;
    let logout = session.logout().await?;
    if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
        || session.last_server_sequence() != relog_sequence
    {
        return Err("W15b relog replay logout or terminal sequence mismatch".into());
    }
    Ok(format!(
        "claim={} replay_sequence={} relog_replay_sequence={relog_sequence}",
        super::full_row_w15b::BATTLE_AXE_CLAIM,
        expected.sequence
    ))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>, world: &BundleWorld) {
    let started = std::time::Instant::now();
    match verdict::indexed_timeout(
        Duration::from_secs(180),
        verdict::observe(capture_and_replay(clients, world)),
    )
    .await
    {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=W15b.client status=PASS seconds={:.3} evidence={observed}",
            started.elapsed().as_secs_f64()
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W15b.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=W15b.client status=FAIL evidence=timeout"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_polluted_login_baseline_even_if_the_list_never_changes() {
        assert!(check_baseline(&[]).is_ok());
        let polluted = [journal::QuestListEntry {
            quest_index: 1,
            completed: true,
        }];
        assert!(check_baseline(&polluted).is_err());
    }

    #[test]
    fn rejects_a_reset_claim_even_if_the_original_axe_survived() {
        assert!(check_relog_refusal(UseDisposition::Committed, true).is_err());
        assert!(check_relog_refusal(UseDisposition::NothingToUse, false).is_err());
        assert!(check_relog_refusal(UseDisposition::NothingToUse, true).is_ok());
    }
}
