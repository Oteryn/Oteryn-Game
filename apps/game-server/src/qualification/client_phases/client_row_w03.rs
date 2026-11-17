//! W03 client replay capture: the real BOSS_REWARD_OPEN chest reply, the BROWSE refusal for an
//! unknown death and the COLLECT_ALL reply away from a reward chest. The consumers are
//! `apply_result` in `crates/client-ui/src/features/0117-boss-encounter/protocol.rs` (chest) and
//! `crates/client-ui/src/features/0224-boss-rewards/protocol.rs` (browse and collect).
use super::*;
use oteryn_dev_client::client_runtime::{boss_encounter, boss_rewards};
use oteryn_protocol_oteryn::boss_rewards::{BrowseRequest, CollectAllRequest};

const UNKNOWN_DEATH: [u8; 16] = [0x57; 16];

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!(
            "W03_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"
        ));
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn reward_refusals(
    browse_rejected: bool,
    collect_out_of_reach: bool,
    moved: u16,
    remaining: u32,
) -> bool {
    browse_rejected && collect_out_of_reach && moved == 0 && remaining == 0
}

#[test]
fn boss_rewards_require_authoritative_refusals() {
    assert!(!reward_refusals(false, true, 0, 0));
    assert!(!reward_refusals(true, true, 1, 0));
    assert!(reward_refusals(true, true, 0, 0));
}

fn reward_chests(world: &BundleWorld) -> TestResult<Vec<TilePos>> {
    use crate::map::view::MapFacts as _;
    use oteryn_protocol_oteryn::world_map::MapDefinition;
    let chest = MapDefinition::Item(provisioning::item_reference(
        &provisioning::item_index()?,
        19_250,
    )?);
    let facts = world.facts().pinned();
    Ok(world
        .base()
        .tiles()
        .filter_map(|(floor, x, y, tile)| {
            let at = TilePos { floor, x, y };
            tile.ids()
                .iter()
                .zip(tile.depths())
                .filter(|(_, depth)| **depth == 0)
                .enumerate()
                .any(|(ordinal, (id, _))| {
                    u8::try_from(ordinal)
                        .ok()
                        .and_then(|ordinal| facts.base_entry(at, ordinal, *id))
                        .is_some_and(|entry| entry.definition == chest)
                })
                .then_some(at)
        })
        .collect())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let chests = reward_chests(world)?;
    let adjacent = |at: TilePos| {
        chests.iter().any(|chest| {
            at.floor == chest.floor && at.x.abs_diff(chest.x).max(at.y.abs_diff(chest.y)) <= 1
        })
    };
    let mut client = connect_world(clients, 0x06).await?;
    let near = route(
        world,
        tile(client.world_spatial().actor_position)?,
        adjacent,
    )
    .ok_or("no route to an authored boss reward chest")?;
    walk(&mut client, world, &near).await?;
    let open = client
        .session_mut()
        .gated_command(
            boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1,
            boss_encounter::COMMAND_TYPE_BOSS_REWARD_OPEN,
            &boss_encounter::encode_reward_chest_query(None)
                .map_err(|error| format!("reward OPEN encode: {error:?}"))?,
        )
        .await?;
    if open.status != CommandStatus::Accepted {
        return Err(format!("reward OPEN status={:?}", open.status).into());
    }
    let chest = boss_encounter::decode_rewards(&open.payload).ok_or("chest reply malformed")?;
    let browse = client
        .session_mut()
        .gated_command(
            boss_rewards::boss_rewards::CAPABILITY_BOSS_REWARDS_V1,
            boss_rewards::boss_rewards::COMMAND_TYPE_BOSS_REWARD_BROWSE,
            &boss_rewards::boss_rewards::encode_browse_request(&BrowseRequest {
                death: UNKNOWN_DEATH,
                after_ordinal: 0,
            })
            .map_err(|error| format!("browse encode: {error:?}"))?,
        )
        .await?;
    let browsed =
        boss_rewards::boss_rewards::decode_browse_reply(&browse.payload).map_err(|error| {
            format!(
                "browse reply malformed: {error:?} status={:?}",
                browse.status
            )
        })?;
    let away = route(world, tile(client.world_spatial().actor_position)?, |at| {
        !adjacent(at)
    })
    .ok_or("no route out of boss reward chest range")?;
    walk(&mut client, world, &away).await?;
    let collect = client
        .session_mut()
        .gated_command(
            boss_rewards::boss_rewards::CAPABILITY_BOSS_REWARDS_V1,
            boss_rewards::boss_rewards::COMMAND_TYPE_BOSS_REWARD_COLLECT_ALL,
            &boss_rewards::boss_rewards::encode_collect_all_request(&CollectAllRequest {
                death: None,
            })
            .map_err(|error| format!("collect encode: {error:?}"))?,
        )
        .await?;
    let collected =
        boss_rewards::boss_rewards::decode_collect_reply(&collect.payload).map_err(|error| {
            format!(
                "collect reply malformed: {error:?} status={:?}",
                collect.status
            )
        })?;
    if browse.status != CommandStatus::Accepted || collect.status != CommandStatus::Accepted {
        return Err(format!("browse={:?} collect={:?}", browse.status, collect.status).into());
    }
    use oteryn_protocol_oteryn::boss_rewards::BossRewardStatus;
    if !reward_refusals(
        browsed.status == BossRewardStatus::Rejected
            && browsed.death == UNKNOWN_DEATH
            && browsed.rows.is_empty()
            && !browsed.more,
        collected.status == BossRewardStatus::OutOfReach,
        collected.moved,
        collected.remaining,
    ) {
        return Err(format!(
            "unexpected reward outcomes: browse={browsed:?}; collect={collected:?}"
        )
        .into());
    }
    capture("w03-chest.bin", &open.payload);
    capture("w03-browse.bin", &browse.payload);
    capture("w03-collect.bin", &collect.payload);
    let mut expected = format!("now={}\ngroups={}\n", chest.now, chest.entries.len());
    for (index, group) in chest.entries.iter().enumerate() {
        expected.push_str(&format!(
            "group{index}={}|{}|{}|{}\n",
            group.boss_key,
            hex(&group.death),
            group.item_count,
            group.expires_at_unix_ms
        ));
    }
    expected.push_str(&format!(
        "browse_status={:?}\ncollect_status={:?}\ncollect_moved={}\ncollect_remaining={}\n",
        browsed.status, collected.status, collected.moved, collected.remaining
    ));
    capture("w03-expected.txt", expected.as_bytes());
    Ok(expected.replace('\n', ";"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=W03.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W03.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=W03.client status=FAIL evidence=timeout"),
    }
}
