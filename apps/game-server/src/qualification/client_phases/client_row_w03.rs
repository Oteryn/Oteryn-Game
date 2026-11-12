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
        evidence(&format!("W03_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"));
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_world(clients, 0x06).await?;
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
    let browsed = boss_rewards::boss_rewards::decode_browse_reply(&browse.payload)
        .map_err(|error| format!("browse reply malformed: {error:?} status={:?}", browse.status))?;
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
    let collected = boss_rewards::boss_rewards::decode_collect_reply(&collect.payload)
        .map_err(|error| format!("collect reply malformed: {error:?} status={:?}", collect.status))?;
    if browse.status != CommandStatus::Accepted || collect.status != CommandStatus::Accepted {
        return Err(format!("browse={:?} collect={:?}", browse.status, collect.status).into());
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
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients))).await {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=W03.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W03.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=W03.client status=FAIL evidence=timeout"),
    }
}
