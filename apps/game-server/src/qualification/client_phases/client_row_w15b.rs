//! W15b client replay capture: a Character that has not claimed it yet walks to a served quest
//! reward chest and uses it twice. The client views captured are the quest journal `List` deltas
//! (raw bytes, before and after the claim), the join-time quest snapshot, and the character
//! inventory before and after (session-owned views re-encoded with the production encoder, the
//! C09/I11 practice; the wire inventory delta is not retained by `Session`). The expected file
//! holds the server-observed values; `crates/session/src/tests/e2e_w15b.rs` replays them.
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

fn inventory_quantities(client: &DevClientSession) -> TestResult<BTreeMap<u32, u64>> {
    let inventory = client.inventory().ok_or("inventory snapshot missing")?;
    let mut counts = BTreeMap::new();
    for item in inventory
        .main_backpack
        .iter()
        .chain(&inventory.entries)
        .chain(inventory.equipment.iter().map(|entry| &entry.item))
    {
        *counts.entry(item.item_definition_ref.get()).or_default() += u64::from(item.count.get());
    }
    Ok(counts)
}

fn list_text(entries: &[journal::QuestListEntry]) -> String {
    entries
        .iter()
        .map(|entry| format!("{}:{}", entry.quest_index, u8::from(entry.completed)))
        .collect::<Vec<_>>()
        .join(";")
}

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

#[derive(Clone)]
struct Chest {
    at: TilePos,
    placement: Vec<u8>,
    claim: String,
    quest_index: u32,
    rewards: Vec<(u32, u32)>,
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
    use oteryn_gameplay_quests::quest::{
        loader::load_embedded_quest_state, log::loader::load_embedded_quest_log,
    };
    let quest_state = load_embedded_quest_state("qualification")
        .map_err(|error| format!("quest state load: {error:?}"))?;
    let log = load_embedded_quest_log(&quest_state)?;
    let journal = log
        .content
        .catalogues()
        .next()
        .ok_or("quest log catalogue absent")?;
    let mut chests = Vec::new();
    for claim in report.served {
        for placement in claim.placements {
            let Some(transition) = placement.placement.quest_transition.as_deref() else {
                continue;
            };
            let quest = &quest_state
                .catalogue()
                .transition(transition)
                .ok_or_else(|| format!("unknown chest transition {transition}"))?
                .quest;
            let Some(quest_index) = journal
                .quests()
                .iter()
                .position(|entry| &entry.key == quest)
            else {
                continue;
            };
            let mut rewards = Vec::new();
            for reward in &placement.placement.items {
                let reference = items
                    .definition_ref(
                        "Item",
                        reward.item.key().as_str(),
                        reward.item.revision().as_str(),
                    )
                    .ok_or("authored chest reward reference missing")?;
                rewards.push((reference.get(), reward.count));
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
                quest_index: u32::try_from(quest_index)?,
                rewards,
            });
        }
    }
    chests.sort_by_key(|chest| (chest.at, chest.claim.clone()));
    Ok(chests)
}

fn adjacent(a: TilePos, b: TilePos) -> bool {
    a.floor == b.floor && a.x.abs_diff(b.x) <= 1 && a.y.abs_diff(b.y) <= 1
}

async fn use_chest(client: &mut DevClientSession, chest: &Chest) -> TestResult<UseDisposition> {
    Ok(client
        .session_mut()
        .use_object(&chest.placement, 0)
        .await?
        .disposition)
}

fn expected_chest_effects(
    inventory_changed: bool,
    journal_changed: bool,
    exact_reward: bool,
    right_quest: bool,
) -> bool {
    inventory_changed && journal_changed && exact_reward && right_quest
}

#[test]
fn chest_requires_the_authored_reward_and_quest() {
    assert!(!expected_chest_effects(true, true, false, true));
    assert!(!expected_chest_effects(true, true, true, false));
    assert!(expected_chest_effects(true, true, true, true));
}

async fn journey(clients: &SeamClients<'_>, world: &BundleWorld) -> TestResult<String> {
    let mut client = connect_character(clients, 0x87, 0).await?;
    let snapshot = client
        .session_mut()
        .gated_snapshot(journal::STATE_DOMAIN_QUEST_LOG)
        .ok_or("step=snapshot quest log snapshot absent")?
        .clone();
    let inventory_payload = |client: &DevClientSession| -> TestResult<Vec<u8>> {
        let inventory = client.inventory().ok_or("inventory snapshot missing")?;
        item_view::encode_character_inventory(inventory)
            .map_err(|error| format!("inventory encode: {error:?}").into())
    };
    let before_units = units(&client)?;
    let mut expected_inventory = inventory_quantities(&client)?;
    let before_payload = inventory_payload(&client)?;
    let before = list(&mut client).await?;
    // The earlier full row claimed the first chest on another Character; this Character claims
    // the first chest whose use commits, and a chest that reports NothingToUse is skipped.
    let mut claimed = None;
    for chest in quest_chests(world)?.into_iter().take(12) {
        let from = tile(client.world_spatial().actor_position)?;
        let Some(path) = route(world, from, |at| adjacent(at, chest.at)) else {
            continue;
        };
        walk(&mut client, world, &path).await?;
        match use_chest(&mut client, &chest).await? {
            UseDisposition::Committed => {
                claimed = Some(chest);
                break;
            }
            UseDisposition::NothingToUse => continue,
            other => {
                return Err(format!("step=use claim={} disposition={other:?}", chest.claim).into());
            }
        }
    }
    let chest = claimed.ok_or("step=use no reachable unclaimed quest reward chest committed")?;
    let after_units = units(&client)?;
    let after_payload = inventory_payload(&client)?;
    let after = list(&mut client).await?;
    for (reference, count) in &chest.rewards {
        *expected_inventory.entry(*reference).or_default() += u64::from(*count);
    }
    let journal_entry = |rows: &[journal::QuestListEntry]| {
        rows.iter()
            .find(|entry| entry.quest_index == chest.quest_index)
            .copied()
    };
    let after_quest = journal_entry(&after.entries);
    let unrelated_quests = |rows: &[journal::QuestListEntry]| {
        rows.iter()
            .copied()
            .filter(|row| row.quest_index != chest.quest_index)
            .collect::<Vec<_>>()
    };
    let right_quest = after_quest.is_some()
        && after_quest != journal_entry(&before.entries)
        && unrelated_quests(&before.entries) == unrelated_quests(&after.entries);
    if !expected_chest_effects(
        after_units != before_units,
        after.entries != before.entries,
        inventory_quantities(&client)? == expected_inventory,
        right_quest,
    ) {
        return Err(format!(
            "step=claim claim={} inventory_changed={} quest_changed={}",
            chest.claim,
            after_units != before_units,
            after.entries != before.entries
        )
        .into());
    }
    let awarded = client
        .inventory()
        .cloned()
        .ok_or("awarded inventory missing")?;
    let second = use_chest(&mut client, &chest).await?;
    if second != UseDisposition::NothingToUse
        || units(&client)? != after_units
        || list(&mut client).await?.entries != after.entries
        || client.inventory() != Some(&awarded)
    {
        return Err(format!("step=duplicate-use disposition={second:?}").into());
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
    let _ = client.session_mut().logout().await;
    Ok(format!(
        "claim={} before={before_units} after={after_units}",
        chest.claim
    ))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>, world: &BundleWorld) {
    match verdict::indexed_timeout(
        Duration::from_secs(180),
        verdict::observe(journey(clients, world)),
    )
    .await
    {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=W15b.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W15b.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=W15b.client status=FAIL evidence=timeout"),
    }
}
