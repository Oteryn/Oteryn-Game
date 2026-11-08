//! W03 replay: the personal reward chest (BOSS_REWARD_OPEN) applied by the production client
//! reducer `apply_result` (`protocol.rs`, chest summary in `protocol/auxiliary.rs`), then the
//! BROWSE refusal and the out-of-reach COLLECT_ALL applied by `boss_rewards::apply_result`
//! (`crates/client-ui/src/features/0224-boss-rewards/protocol.rs`).
use super::*;
use crate::boss_rewards::{
    BossRewardsPanel, BossRewardsReply, BossRewardsResponse, Intent as RewardIntent,
    boss_rewards as rewards_wire,
};
use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};

type Error = Box<dyn std::error::Error>;

struct Captured {
    chest: Vec<u8>,
    browse: Vec<u8>,
    collect: Vec<u8>,
    expected: String,
}

fn captured() -> Result<Captured, Error> {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root);
        return Ok(Captured {
            chest: std::fs::read(root.join("w03-chest.bin"))?,
            browse: std::fs::read(root.join("w03-browse.bin"))?,
            collect: std::fs::read(root.join("w03-collect.bin"))?,
            expected: std::fs::read_to_string(root.join("w03-expected.txt"))?,
        });
    }
    let expires = 1_000 + oteryn_client_runtime::boss_encounter::BOSS_REWARD_EXPIRY_MS;
    let entry = oteryn_client_runtime::boss_encounter::BossRewardChestEntry {
        death: [0x11; 16],
        boss_key: "oteryn:boss.faceless_bane".into(),
        minted_at_unix_ms: 1_000,
        expires_at_unix_ms: expires,
        item_count: 3,
    };
    Ok(Captured {
        chest: oteryn_client_runtime::boss_encounter::encode_reward_chest(2_000, &[entry])
            .map_err(|error| format!("{error:?}"))?,
        browse: rewards_wire::encode_browse_reply(&rewards_wire::BrowseReply {
            status: rewards_wire::BossRewardStatus::Rejected,
            death: [0x57; 16],
            expires_at_unix_ms: 0,
            rows: vec![],
            more: false,
        })
        .map_err(|error| format!("{error:?}"))?,
        collect: rewards_wire::encode_collect_reply(&rewards_wire::CollectReply {
            status: rewards_wire::BossRewardStatus::OutOfReach,
            moved: 0,
            remaining: 0,
        })
        .map_err(|error| format!("{error:?}"))?,
        expected: format!(
            "now=2000\ngroups=1\ngroup0=oteryn:boss.faceless_bane|{}|3|{expires}\nbrowse_status=Rejected\ncollect_status=OutOfReach\ncollect_moved=0\ncollect_remaining=0\n",
            "11".repeat(16)
        ),
    })
}

fn value<'a>(expected: &'a str, key: &str) -> Result<&'a str, Error> {
    expected
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .ok_or_else(|| format!("expected line {key} absent").into())
}

fn result(status: CommandStatus, payload: &[u8]) -> GatedCommandResult {
    GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status,
        payload: payload.to_vec(),
    }
}

#[test]
fn e2e_w03_replays_reward_chest_browse_refusal_and_collect_refusal() -> Result<(), Error> {
    let captured = captured()?;
    let expected = captured.expected.as_str();
    let mut panel = BossEncounterPanel {
        selected: true,
        ..BossEncounterPanel::default()
    };
    let open = request(&mut panel, Intent::Rewards).ok_or("reward open request")?;
    assert_eq!((open.capability, open.kind), (117, 340));
    assert!(open.payload.is_empty());
    assert!(apply_result(
        &mut panel,
        &result(CommandStatus::Accepted, &captured.chest)
    ));
    let groups: usize = value(expected, "groups")?.parse()?;
    let mut text = format!(
        "Reward groups: {groups} | Server time: {}\r\n",
        value(expected, "now")?
    );
    for index in 0..groups {
        let fields: Vec<&str> = value(expected, &format!("group{index}"))?
            .split('|')
            .collect();
        let [boss, death, items, expires] = fields.as_slice() else {
            return Err("group line malformed".into());
        };
        text.push_str(&format!(
            "{boss} | Death: {death} | Items: {items} | Expires: {expires}\n"
        ));
    }
    text.push_str("Use Boss rewards (Ctrl+Shift+1) with a death id to collect.");
    assert_eq!(panel.summary, text);
    assert_eq!(
        panel.rewards.as_ref().map(|chest| chest.entries.len()),
        Some(groups)
    );

    // A rejected OPEN keeps the chest and summary the server last confirmed.
    let (summary, chest) = (panel.summary.clone(), panel.rewards.clone());
    request(&mut panel, Intent::Rewards).ok_or("second open")?;
    assert!(!apply_result(
        &mut panel,
        &result(CommandStatus::Rejected, &[])
    ));
    assert_eq!(panel.summary, summary);
    assert_eq!(panel.rewards, chest);

    // BROWSE of an unknown death is refused by the server's status, not applied as items.
    let mut rewards = BossRewardsPanel {
        selected: true,
        death: [0x57; 16],
        ..BossRewardsPanel::default()
    };
    request_reward(&mut rewards, RewardIntent::Browse)?;
    let browse = rewards_wire::decode_browse_reply(&captured.browse)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(
        format!("{:?}", browse.status),
        value(expected, "browse_status")?
    );
    assert!(!crate::boss_rewards::apply_result(
        &mut rewards,
        &BossRewardsResponse {
            status: CommandStatus::Accepted,
            reply: Some(BossRewardsReply::Browse(browse)),
        }
    ));
    assert!(rewards.feedback.is_some());
    assert!(rewards.browse.as_ref().is_some_and(|b| b.rows.is_empty()));

    // COLLECT_ALL away from a chest moves nothing and names the refusal.
    request_reward(&mut rewards, RewardIntent::CollectAll)?;
    let collect = rewards_wire::decode_collect_reply(&captured.collect)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(
        format!("{:?}", collect.status),
        value(expected, "collect_status")?
    );
    assert_eq!(collect.moved.to_string(), value(expected, "collect_moved")?);
    assert_eq!(
        collect.remaining.to_string(),
        value(expected, "collect_remaining")?
    );
    assert!(!crate::boss_rewards::apply_result(
        &mut rewards,
        &BossRewardsResponse {
            status: CommandStatus::Accepted,
            reply: Some(BossRewardsReply::Collect(collect)),
        }
    ));
    let feedback = match value(expected, "collect_status")? {
        "OutOfReach" => "Move next to a qualified reward chest",
        "Empty" => "No live rewards remain",
        other => return Err(format!("unexpected collect status {other}").into()),
    };
    assert_eq!(rewards.feedback, Some(feedback));
    Ok(())
}

fn request_reward(panel: &mut BossRewardsPanel, intent: RewardIntent) -> Result<(), Error> {
    crate::boss_rewards::request(panel, intent).ok_or("reward request refused")?;
    Ok(())
}
