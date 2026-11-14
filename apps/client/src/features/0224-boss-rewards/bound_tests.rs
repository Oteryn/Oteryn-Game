use super::*;
use oteryn_client_runtime::boss_rewards::{BossRewardsCommand, boss_rewards};
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
#[test]
fn boss_reward_chest_owner_rejects_unconfirmed_duplicate_changed_closed_and_shutdown() -> Result {
    let (link, mut commands, outbox) = crate::play::play_channel();
    outbox.bevy_boss_reward_chest_selected();
    let closed = link.boss_reward_chest_bound();
    assert!(link.boss_reward_chest_dispatch_bound(&closed, BossRewardChestAction::Toggle));
    let bound = link.boss_reward_chest_bound();
    let request = |confirmed| BossRewardChestAction::Request {
        intent: Intent::Take,
        death: [9; 16],
        ordinal: 4,
        confirmed,
    };
    assert!(!link.boss_reward_chest_dispatch_bound(&bound, request(false)));
    assert!(link.boss_reward_chest_bound().matches(&bound));
    assert!(commands.try_recv().is_err());
    // A change by the canonical owner invalidates the exact view, even in this session.
    lock(&link.mailbox).boss_rewards.selected_ordinal = 5;
    let changed = link.boss_reward_chest_bound();
    assert!(!link.boss_reward_chest_dispatch_bound(&bound, request(true)));
    assert!(link.boss_reward_chest_bound().matches(&changed));
    assert!(commands.try_recv().is_err());
    assert!(link.boss_reward_chest_dispatch_bound(&changed, request(true)));
    assert!(
        matches!(commands.try_recv()?,crate::combat_input::PlayCommand::BossRewards(BossRewardsCommand::Take(value)) if value==boss_rewards::TakeRequest {death:[9;16],ordinal:4})
    );
    let pending = link.boss_reward_chest_bound();
    assert!(!link.boss_reward_chest_dispatch_bound(&pending, request(true)));
    assert!(!link.boss_reward_chest_dispatch_bound(&changed, request(true)));
    assert!(link.boss_reward_chest_bound().matches(&pending));
    assert!(commands.try_recv().is_err());
    assert!(link.boss_reward_chest_dispatch_bound(&pending, BossRewardChestAction::Close));
    let closed = link.boss_reward_chest_bound();
    assert!(!link.boss_reward_chest_dispatch_bound(&closed, request(true)));
    outbox.bevy_boss_reward_chest_end();
    // Draining the terminal event must not revive this session.
    let mut ended_event = false;
    while let Some(event) = link.poll() {
        ended_event |= matches!(event, crate::play::PlayEvent::Ended(_));
    }
    assert!(ended_event);
    let ended = link.boss_reward_chest_bound();
    assert!(!ended.live);
    assert!(!link.boss_reward_chest_dispatch_bound(&ended, BossRewardChestAction::Toggle));
    assert!(link.boss_reward_chest_bound().matches(&ended));
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn boss_reward_chest_invalid_target_and_dead_transport_leave_owner_unchanged() -> Result {
    let (link, commands, outbox) = crate::play::play_channel();
    outbox.bevy_boss_reward_chest_selected();
    link.boss_reward_chest_dispatch_bound(
        &link.boss_reward_chest_bound(),
        BossRewardChestAction::Toggle,
    );
    let bound = link.boss_reward_chest_bound();
    assert!(!link.boss_reward_chest_dispatch_bound(
        &bound,
        BossRewardChestAction::Request {
            intent: Intent::Browse,
            death: [0; 16],
            ordinal: 0,
            confirmed: false
        }
    ));
    assert!(link.boss_reward_chest_bound().matches(&bound));
    drop(commands);
    assert!(!link.boss_reward_chest_dispatch_bound(
        &bound,
        BossRewardChestAction::Request {
            intent: Intent::Browse,
            death: [9; 16],
            ordinal: 0,
            confirmed: false
        }
    ));
    assert_eq!(link.boss_rewards_panel(), bound.panel);
    Ok(())
}
