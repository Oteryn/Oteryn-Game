//! Collection counts every live group without an unbounded Item-table aggregate.
use super::*;

pub(super) async fn all_groups(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    fence: CurrentCharacterItemFence,
) -> TestResult {
    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native Item facts")?;
    let index = native.item_index().ok_or("Item index")?;
    let reference = index
        .definition_ref("Item", "oteryn:item.tibia.i3031", "definition-r1")
        .ok_or("gold Item reference")?;
    let definition = crate::gameplay_transport::item_look::typed_definition(index, reference)
        .ok_or("gold definition")?;
    let minted: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
            .fetch_one(&h.pool)
            .await?;
    let contribution = RewardParticipation {
        damage_dealt: 1,
        damage_received: 0,
        healing_others: 0,
    };
    // One item remains from the real collection above; sixty-four further owner-issued
    // groups put the exact `remaining` result across the keyset page boundary.
    for ordinal in 0..MAX_PAGE_ROWS {
        let mut death = id(191);
        death[15] = u8::try_from(ordinal)?;
        assert!(
            h.root
                .settle_boss_reward_chests(
                    owner.character,
                    owner.holder,
                    fence.runtime_scope,
                    fence.scope_ownership_generation,
                    BossRewardDeath {
                        death,
                        boss_key: "oteryn:boss.paged_fixture".into(),
                        minted_at_unix_ms: u64::try_from(minted)?,
                        contributors: vec![BossRewardContributor {
                            character_id: fence.character_id,
                            session_id: fence.game_session_id,
                            lease_generation: fence.character_lease_generation,
                            contribution
                        }],
                        chests: vec![BossRewardChestPlan {
                            character_id: fence.character_id,
                            session_id: fence.game_session_id,
                            lease_generation: fence.character_lease_generation,
                            contribution,
                            items: vec![BossRewardItem {
                                definition: definition.clone(),
                                quantity: 1
                            }]
                        }],
                    }
                )
                .await?
        );
    }
    // Move back to the real reward chest after the previous moved-away replay check.
    movement::walk_corridor(owner, *admitted, [2, 1, 0]).await?;
    // Discovery must cross the same 64-group boundary without requiring the
    // player to consume older rewards first. The legacy first page is unchanged.
    use oteryn_protocol_oteryn::boss_encounter::{
        COMMAND_TYPE_BOSS_REWARD_OPEN, encode_reward_chest_query,
    };
    let mut cursor = id(191);
    cursor[15] = u8::try_from(MAX_PAGE_ROWS - 2)?;
    let mut last = id(191);
    last[15] = u8::try_from(MAX_PAGE_ROWS - 1)?;
    let command = admitted.continuity.next_command_id;
    let (status, first) =
        exchange(owner, admitted, command, COMMAND_TYPE_BOSS_REWARD_OPEN, &[]).await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert!(first.windows(16).any(|bytes| bytes == cursor));
    assert!(!first.windows(16).any(|bytes| bytes == last));
    let query = encode_reward_chest_query(Some(cursor)).map_err(debug)?;
    let command = admitted.continuity.next_command_id;
    let (status, next) = exchange(
        owner,
        admitted,
        command,
        COMMAND_TYPE_BOSS_REWARD_OPEN,
        &query,
    )
    .await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert!(next.windows(16).any(|bytes| bytes == last));
    for ordinal in 0..MAX_PAGE_ROWS - 1 {
        let mut earlier = id(191);
        earlier[15] = u8::try_from(ordinal)?;
        assert!(!next.windows(16).any(|bytes| bytes == earlier));
    }
    let mut legacy = *admitted;
    legacy.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 117])
            .ok_or("legacy boss selection")?;
    assert!(
        matches!(
            owner
                .boss_cooldowns_command(legacy, 1, COMMAND_TYPE_BOSS_REWARD_OPEN, &query)
                .await,
            GuildReply::Rejected
        ),
        "continuation requires the offered and selected extension"
    );
    let unknown = encode_reward_chest_query(Some(id(222))).map_err(debug)?;
    assert!(
        matches!(
            owner
                .boss_cooldowns_command(*admitted, 1, COMMAND_TYPE_BOSS_REWARD_OPEN, &unknown)
                .await,
            GuildReply::Rejected
        ),
        "unknown cursor is not an arbitrary sort key"
    );
    let end = encode_reward_chest_query(Some(last)).map_err(debug)?;
    let command = admitted.continuity.next_command_id;
    let (status, empty) = exchange(
        owner,
        admitted,
        command,
        COMMAND_TYPE_BOSS_REWARD_OPEN,
        &end,
    )
    .await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert!(
        empty.len() < 18,
        "only the existing now field remains after the final group"
    );
    eprintln!("BOSS_REWARDS_GROUP_PAGES POSTGRES_CASE_COMPLETED");
    let all = encode_collect_all_request(&CollectAllRequest { death: None }).map_err(debug)?;
    let command = admitted.continuity.next_command_id;
    let (_, bytes) = exchange(
        owner,
        admitted,
        command,
        COMMAND_TYPE_BOSS_REWARD_COLLECT_ALL,
        &all,
    )
    .await?;
    assert_eq!(
        decode_collect_reply(&bytes).map_err(debug)?,
        CollectReply {
            status: BossRewardStatus::NoRoom,
            moved: 0,
            remaining: u32::try_from(MAX_PAGE_ROWS + 1)?,
        }
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_boss_reward_item_collections")
            .fetch_one(&h.pool)
            .await?,
        3,
        "capacity refusal preserves all unclaimed groups"
    );
    Ok(())
}
