//! Replay the fixed Battle Axe chest interaction from its original inbound wire frames.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::quest_log as journal;
use oteryn_protocol_oteryn::item_view;
use oteryn_protocol_oteryn::world_object::UseDisposition;

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!(
            "W15B_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"
        ));
    }
}

/// `reference:count` of every carried item (backpack, loose entries, equipment), sorted.
fn units(client: &DevClientSession) -> TestResult<String> {
    let inventory = client.inventory().ok_or("inventory snapshot missing")?;
    let mut held: Vec<(u32, u32)> = inventory
        .main_backpack
        .iter()
        .chain(inventory.entries.iter())
        .chain(inventory.equipment.iter().map(|entry| &entry.item))
        .map(|item| (item.item_definition_ref.get(), item.count.get()))
        .collect();
    held.sort_unstable();
    Ok(held
        .iter()
        .map(|(reference, count)| format!("{reference}:{count}"))
        .collect::<Vec<_>>()
        .join(";"))
}

fn list_text(entries: &[journal::QuestListEntry]) -> String {
    entries
        .iter()
        .map(|entry| format!("{}:{}", entry.quest_index, u8::from(entry.completed)))
        .collect::<Vec<_>>()
        .join(";")
}

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
    let snapshot = client
        .session_mut()
        .gated_snapshot(journal::STATE_DOMAIN_QUEST_LOG)
        .ok_or("step=snapshot quest log snapshot absent")?
        .clone();
    let inventory_payload = |client: &DevClientSession| -> TestResult<Vec<u8>> {
        let inventory = client.inventory().ok_or("inventory snapshot missing")?;
        if !client
            .selected_capabilities()
            .contains(&item_view::CAPABILITY_ITEM_EQUIP_DROP_V1)
        {
            return Err("W15b inventory capture requires equipment capability".into());
        }
        item_view::encode_character_inventory_with_equip_drop(inventory, true)
            .map_err(|error| format!("inventory encode: {error:?}").into())
    };
    let before_inventory = client
        .inventory()
        .ok_or("missing pre-claim inventory")?
        .clone();
    let before_custody = custody(&client)?;
    let before_units = units(&client)?;
    let before_payload = inventory_payload(&client)?;
    let before = list(&mut client).await?;
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
    let after_units = units(&client)?;
    let after_payload = inventory_payload(&client)?;
    let expected_reward = provisioning::item_reference(&provisioning::item_index()?, 3266)?.get();
    let awarded = custody(&client)?;
    if !reward_increased(&before_custody, &awarded, expected_reward, 1)
        || after.entries != before.entries
    {
        return Err(format!(
            "step=claim claim={} inventory_changed={} quest_changed={}",
            chest.claim,
            after_units != before_units,
            after.entries != before.entries
        )
        .into());
    }
    let second = use_chest(&mut client, &chest).await?;
    if second != UseDisposition::NothingToUse || units(&client)? != after_units {
        return Err(format!("step=duplicate-use disposition={second:?}").into());
    }
    let duplicate = list(&mut client).await?;
    if duplicate.entries != after.entries || custody(&client)? != awarded {
        return Err("step=duplicate-use changed quest or inventory state".into());
    }
    let expected = format!(
        "claim={}\nsnapshot_revision={}\nbefore_units={before_units}\nafter_units={after_units}\n\
         list_before={}\nlist_before_base={}\nlist_before_new={}\nlist_before_sequence={}\n\
         list_after={}\nlist_after_base={}\nlist_after_new={}\nlist_after_sequence={}\nsecond_use={second:?}\n",
        chest.claim,
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
    capture("w15b-quest-snapshot.bin", &snapshot.payload);
    capture("w15b-quest-before.bin", &before.payload);
    capture("w15b-quest-after.bin", &after.payload);
    capture("w15b-inventory-before.bin", &before_payload);
    capture("w15b-inventory-after.bin", &after_payload);
    capture("w15b-expected.txt", expected.as_bytes());
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

async fn capture_and_replay(clients: &SeamClients<'_>, world: &BundleWorld) -> TestResult<String> {
    use oteryn_dev_client::native_session::{Admission, Session, capture_inbound};
    use std::io::Write as _;
    let root =
        std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("W15b live capture directory missing")?;
    let (expected, bytes) = capture_inbound(journey(clients, world)).await;
    let expected = expected?;
    let path = Path::new(&root).join("w15b-server.wire");
    let mut capture = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;
    capture.write_all(&bytes)?;
    drop(capture);
    let recorded = std::fs::read(&path)?;
    if recorded != bytes || recorded.is_empty() {
        return Err("W15b capture changed between recording and replay".into());
    }
    let stream = tokio::io::join(recorded.as_slice(), tokio::io::sink());
    let mut session = Session::admit(
        stream,
        Admission {
            appearance_items: None,
            schema_revision: 1,
            character_id: crate::foundation::CharacterId::decode(&clients.characters[0])?,
            admission_material: b"recorded-server-replay",
            client_build_id: "oteryn-dev-client/w15b-replay",
            supported_capabilities: oteryn_dev_client::CLIENT_SUPPORTED_CAPABILITIES,
            deadline: Duration::from_secs(5),
        },
    )
    .await?;
    if session.join_snapshot().game_session_id != expected.session_id
        || session.inventory() != Some(&expected.before_inventory)
    {
        return Err("W15b replay admission identity or inventory mismatch".into());
    }
    let before = list_session(&mut session).await?;
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
    let mut fresh = connect_character(clients, 0x89, 0).await?;
    if fresh.join_snapshot().game_session_id == expected.session_id
        || super::full_row_w15b::custody(&fresh)? != expected.awarded
        || list(&mut fresh).await?.entries != expected.after.entries
    {
        return Err("W15b fresh admission did not restore the exact reward and quest list".into());
    }
    let fresh_logout = fresh.session_mut().logout().await?;
    if fresh_logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step=relogin-logout refused: {fresh_logout:?}").into());
    }
    Ok(format!(
        "claim={} replay_sequence={}",
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
