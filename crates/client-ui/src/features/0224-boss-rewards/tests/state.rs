use super::*;
const DEATH: [u8; 16] = [7; 16];

#[test]
fn requests_encode_every_server_command_with_the_contract_layout() -> Result<(), String> {
    let mut panel = BossRewardsPanel {
        selected: true,
        death: DEATH,
        after_ordinal: 9,
        selected_ordinal: 3,
        ..Default::default()
    };
    let cases = [
        (Intent::Browse, 428),
        (Intent::Take, 429),
        (Intent::CollectGroup, 430),
        (Intent::CollectAll, 430),
    ];
    for (intent, kind) in cases {
        let command = request(&mut panel, intent).ok_or("valid fixture")?;
        let (actual_kind, bytes) = match command {
            BossRewardsCommand::Browse(value) => (
                boss_rewards::COMMAND_TYPE_BOSS_REWARD_BROWSE,
                boss_rewards::encode_browse_request(&value)
                    .map_err(|error| format!("encode: {error:?}"))?,
            ),
            BossRewardsCommand::Take(value) => (
                boss_rewards::COMMAND_TYPE_BOSS_REWARD_TAKE,
                boss_rewards::encode_take_request(&value)
                    .map_err(|error| format!("encode: {error:?}"))?,
            ),
            BossRewardsCommand::CollectAll(value) => (
                boss_rewards::COMMAND_TYPE_BOSS_REWARD_COLLECT_ALL,
                boss_rewards::encode_collect_all_request(&value)
                    .map_err(|error| format!("encode: {error:?}"))?,
            ),
        };
        assert_eq!(actual_kind, kind);
        assert_eq!(bytes[0], 1);
        assert_eq!(bytes.len(), if kind == 430 { 17 } else { 19 });
        panel.pending = false;
    }
    Ok(())
}

#[test]
fn server_encoded_browse_and_collection_replies_reduce_into_panel_state() -> Result<(), String> {
    let encoded = boss_rewards::encode_browse_reply(&boss_rewards::BrowseReply {
        status: boss_rewards::BossRewardStatus::Ok,
        death: DEATH,
        expires_at_unix_ms: 42,
        rows: vec![boss_rewards::RewardRow {
            ordinal: 3,
            definition_reference: 901,
            quantity: 2,
        }],
        more: true,
    })
    .map_err(|error| format!("server codec: {error:?}"))?;
    let reply = boss_rewards::decode_browse_reply(&encoded)
        .map_err(|error| format!("client codec: {error:?}"))?;
    let mut panel = BossRewardsPanel {
        selected: true,
        pending: true,
        ..Default::default()
    };
    assert!(apply_result(
        &mut panel,
        &BossRewardsResponse {
            status: CommandStatus::Accepted,
            reply: Some(BossRewardsReply::Browse(reply))
        }
    ));
    assert_eq!(
        panel.browse.as_ref().ok_or("page")?.rows[0].definition_reference,
        901
    );
    assert!(panel_text(&panel).contains("item 901 x2"));
    let encoded = boss_rewards::encode_collect_reply(&boss_rewards::CollectReply {
        status: boss_rewards::BossRewardStatus::NoRoom,
        moved: 0,
        remaining: 2,
    })
    .map_err(|error| format!("server codec: {error:?}"))?;
    let reply = boss_rewards::decode_collect_reply(&encoded)
        .map_err(|error| format!("client codec: {error:?}"))?;
    assert!(!apply_result(
        &mut panel,
        &BossRewardsResponse {
            status: CommandStatus::Accepted,
            reply: Some(BossRewardsReply::Collect(reply))
        }
    ));
    assert!(
        panel
            .feedback
            .is_some_and(|value| value.contains("backpack"))
    );
    Ok(())
}
