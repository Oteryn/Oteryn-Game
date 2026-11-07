use super::*;

#[test]
fn boss_reward_transport_rejects_foreign_command_and_malformed_selection() {
    assert!(decode(0, &[]).is_none());
    assert!(decode(wire::COMMAND_TYPE_BOSS_REWARD_BROWSE, &[]).is_none());
    assert!(decode(wire::COMMAND_TYPE_BOSS_REWARD_TAKE, &[1; 18]).is_none());
    assert!(decode(wire::COMMAND_TYPE_BOSS_REWARD_COLLECT_ALL, &[0; 17]).is_none());
}

#[test]
fn boss_reward_terminal_statuses_keep_partial_delivery_and_refusal_distinct() {
    assert_eq!(
        status(BossRewardCollectionStatus::Collected),
        wire::BossRewardStatus::Ok
    );
    assert_eq!(
        status(BossRewardCollectionStatus::NoRoom),
        wire::BossRewardStatus::NoRoom
    );
    assert_eq!(
        status(BossRewardCollectionStatus::Expired),
        wire::BossRewardStatus::Expired
    );
    assert_eq!(
        status(BossRewardCollectionStatus::ContentUnavailable),
        wire::BossRewardStatus::ContentUnavailable
    );
}
