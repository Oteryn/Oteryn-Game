//! W03 client replay capture: the real BOSS_REWARD_OPEN chest reply, the BROWSE refusal for an
//! unknown death and the COLLECT_ALL reply away from a reward chest. The consumers are
//! `apply_result` in `crates/client-ui/src/features/0117-boss-encounter/protocol.rs` (chest) and
//! `crates/client-ui/src/features/0224-boss-rewards/protocol.rs` (browse and collect).
use super::*;
use oteryn_dev_client::client_runtime::{boss_encounter, boss_rewards};
use oteryn_protocol_oteryn::boss_rewards::{BrowseRequest, CollectAllRequest};

const UNKNOWN_DEATH: [u8; 16] = [0x57; 16];

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    use std::io::Write as _;

    let root = required("E2E_ALL_CAPTURE_DIR")?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(Path::new(&root).join(name))?;
    file.write_all(bytes)?;
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn same_inventory_contents(
    before: &oteryn_protocol_oteryn::item_view::CharacterInventory,
    after: &oteryn_protocol_oteryn::item_view::CharacterInventory,
) -> bool {
    use oteryn_protocol_oteryn::item_view::ItemEntry;

    // Handles address a session view. Compare the item facts that survive admission.
    let item = |entry: &ItemEntry| {
        (
            entry.item_definition_ref,
            entry.count,
            entry.sub_type,
            entry.appearance_id,
        )
    };
    before.main_backpack.as_ref().map(item) == after.main_backpack.as_ref().map(item)
        && before
            .entries
            .iter()
            .map(item)
            .eq(after.entries.iter().map(item))
        && before
            .equipment
            .iter()
            .map(|entry| (entry.slot, item(&entry.item)))
            .eq(after
                .equipment
                .iter()
                .map(|entry| (entry.slot, item(&entry.item))))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_world(clients, 0x06).await?;
    let session = client.join_snapshot().game_session_id;
    let inventory = client
        .inventory()
        .cloned()
        .ok_or("initial inventory missing")?;
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
    if browsed.status != BossRewardStatus::OutOfReach
        || browsed.death != UNKNOWN_DEATH
        || browsed.expires_at_unix_ms != 0
        || !browsed.rows.is_empty()
        || browsed.more
        || collected.status != BossRewardStatus::OutOfReach
        || collected.moved != 0
        || collected.remaining != 0
        || client.inventory() != Some(&inventory)
        || chest
            .entries
            .iter()
            .any(|group| group.death == UNKNOWN_DEATH)
    {
        return Err(
            format!("unexpected refusal or inventory change: {browsed:?} {collected:?}").into(),
        );
    }

    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("W03 refusal verification logout was refused".into());
    }
    let mut fresh = connect_world(clients, 0x07).await?;
    let reread = fresh
        .session_mut()
        .gated_command(
            boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1,
            boss_encounter::COMMAND_TYPE_BOSS_REWARD_OPEN,
            &[],
        )
        .await?;
    let after =
        boss_encounter::decode_rewards(&reread.payload).ok_or("relog reward chest malformed")?;
    if fresh.join_snapshot().game_session_id == session
        || reread.status != CommandStatus::Accepted
        || after.entries != chest.entries
        || !fresh
            .inventory()
            .is_some_and(|after| same_inventory_contents(&inventory, after))
    {
        return Err("W03 refusal state changed after relog".into());
    }
    if fresh.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("W03 relog cleanup logout was refused".into());
    }

    capture("w03-chest.bin", &open.payload)?;
    capture("w03-browse.bin", &browse.payload)?;
    capture("w03-collect.bin", &collect.payload)?;
    let mut expected = format!(
        "session={}\nopen={}|{}\nbrowse={}|{}\ncollect={}|{}\nnow={}\ngroups={}\n",
        hex(session.as_bytes()),
        open.command_id,
        open.server_sequence,
        browse.command_id,
        browse.server_sequence,
        collect.command_id,
        collect.server_sequence,
        chest.now,
        chest.entries.len(),
    );
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
    capture("w03-expected.txt", expected.as_bytes())?;
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
