use super::{Panel, bosses};
use crate::{boss_encounter::BossEncounterPanel, boss_rewards::BossRewardsPanel};
use oteryn_client_runtime::{
    boss_encounter as encounter,
    boss_rewards::{BossRewardsReply, BossRewardsResponse, CommandStatus, boss_rewards as wire},
};
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
fn response(ordinal: u16, quantity: u32) -> BossRewardsResponse {
    BossRewardsResponse {
        status: CommandStatus::Accepted,
        reply: Some(BossRewardsReply::Browse(wire::BrowseReply {
            status: wire::BossRewardStatus::Ok,
            death: [9; 16],
            expires_at_unix_ms: 900 + encounter::BOSS_REWARD_EXPIRY_MS,
            rows: vec![wire::RewardRow {
                ordinal,
                definition_reference: 700,
                quantity,
            }],
            more: false,
        })),
    }
}
#[test]
fn boss_tracker_reuses_server_cooldowns_and_observes_each_drop_once() -> Result {
    let now = std::time::Instant::now();
    let boss = BossEncounterPanel {
        selected: true,
        cooldown_received_at: Some(now),
        snapshot: Some(encounter::Snapshot {
            now: 1000,
            cooldowns: vec![("oteryn:boss.test".into(), 10000)],
            progress: vec![],
            points: 0,
            cooldown_next: None,
            progress_next: None,
            taints: None,
        }),
        rewards: Some(
            encounter::decode_rewards(
                &encounter::encode_reward_chest(
                    1000,
                    &[encounter::BossRewardChestEntry {
                        death: [9; 16],
                        boss_key: "oteryn:boss.test".into(),
                        minted_at_unix_ms: 900,
                        expires_at_unix_ms: 900 + encounter::BOSS_REWARD_EXPIRY_MS,
                        item_count: 1,
                    }],
                )
                .map_err(|_| "encode chest")?,
            )
            .ok_or("chest")?,
        ),
        ..Default::default()
    };
    let mut panel = Panel {
        selected: true,
        ..Default::default()
    };
    bosses::observe(&mut panel, &boss, &response(1, 3));
    bosses::observe(&mut panel, &boss, &response(1, 3));
    bosses::observe(&mut panel, &boss, &response(1, 1));
    assert_eq!(panel.boss_drops.len(), 1);
    assert_eq!(panel.boss_drops[0].quantity, 3);
    assert_eq!(
        panel.boss_drops[0].boss.as_deref(),
        Some("oteryn:boss.test")
    );
    let fresh = bosses::text_at(&panel, &boss, &BossRewardsPanel::default(), now);
    assert!(fresh.contains("9000 ms remaining"));
    assert!(fresh.contains("item 700 × 3"));
    let later = bosses::text_at(
        &panel,
        &boss,
        &BossRewardsPanel::default(),
        now + std::time::Duration::from_secs(20),
    );
    assert!(!later.contains("9000 ms remaining"));
    assert!(later.contains("09090909090909090909090909090909"));
    Ok(())
}
#[test]
fn boss_tracker_rejects_refused_facts_and_bounds_history() -> Result {
    let mut panel = Panel {
        selected: true,
        ..Default::default()
    };
    let boss = BossEncounterPanel::default();
    let mut refused = response(1, 99);
    refused.status = CommandStatus::Rejected;
    bosses::observe(&mut panel, &boss, &refused);
    assert!(panel.boss_drops.is_empty());
    for i in 0..=bosses::MAX_DROPS {
        bosses::observe(&mut panel, &boss, &response(u16::try_from(i + 1)?, 1));
    }
    assert_eq!(panel.boss_drops.len(), bosses::MAX_DROPS);
    assert!(panel.boss_drops_truncated);
    assert_eq!(panel.boss_drops.front().ok_or("front")?.ordinal, 2);
    super::protocol::reset(&mut panel);
    assert!(panel.boss_drops.is_empty());
    Ok(())
}
