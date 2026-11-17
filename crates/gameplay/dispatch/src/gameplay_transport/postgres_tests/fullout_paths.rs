//! FULLOUT production ingress proofs: consume/transform and Task Board shop.
use super::TestResult;
use super::fixture::{Harness, debug};
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
use std::num::NonZeroU32;

#[test]
fn fullout_voucher_last_addon_and_replay() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::item_cosmetics::run_fullout_voucher(false)
}

#[test]
fn fullout_mixed_quest_then_voucher_and_replay() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::item_cosmetics::run_fullout_voucher(true)
}

#[test]
fn fullout_task_board_last_addon_and_replay() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(
        "fullout-task-board",
        super::task_board::run_fullout,
    )
}

pub(super) async fn voucher(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    mut admitted: AdmittedSession,
    reference: impl Fn(&str) -> Option<NonZeroU32>,
    mixed: bool,
) -> TestResult {
    use crate::durability::item_mint::TypedDefinitionRef;
    use crate::durability::item_transfer::{ItemWeightLookup, ItemWeightRefused};
    use crate::durability::quest_state::{
        QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
    };
    use crate::durability::reward_claim_mint::QuestRewardAdmission;
    use crate::foundation::{CommandId, CommandRef};
    use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
    use oteryn_gameplay_quests::quest::reward_callbacks::RewardCallback;
    use oteryn_gameplay_quests::quest::{
        QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack,
        QuestTransition,
    };
    use oteryn_protocol_oteryn::world_object::{
        UseDisposition, UseIntent, UseTarget, encode_item_use_intent,
    };
    struct NoItems<'a>(&'a NativeGameplayState);
    impl ItemWeightLookup for NoItems<'_> {
        fn generation_digest(&self) -> [u8; 32] {
            self.0.source_digest()
        }
        fn weight_of(&self, _: &TypedDefinitionRef) -> Result<u32, ItemWeightRefused> {
            Err(ItemWeightRefused::Unknown)
        }
    }
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let weights = NoItems(native);
    let key = "oteryn:quest-reward/fullout/rascoohan";
    let quest = "oteryn:quest.fullout_rascoohan";
    let track = "oteryn:quest-progress/fullout/rascoohan";
    let callback: RewardCallback = serde_json::from_value(serde_json::json!({
        "key":key,"quest":quest,"npc":"oteryn:npc.lynda","keywords":["addon"],"topic":0,"next_topic":0,
        "storage":[{"track":track,"minimum":0,"maximum":0,"set":1}],"take":[],"give":null,
        "outfit":{"outfit":"oteryn:outfit.rascoohan","addons":u8::from(mixed)},
        "achievement":null,"reply":["Your outfit."],"sources":["TibiaWiki Honorary Rascoohan page, spoiler field (fetched 2026-11-17)"]
    }))?;
    let catalogue = std::sync::Arc::new(
        QuestStateCatalogue::new(
            "content-1",
            vec![QuestTrack {
                key: track.into(),
                quest: quest.into(),
                initial: 0,
                min: 0,
                max: 1,
            }],
            vec![QuestTransition {
                key: key.into(),
                quest: quest.into(),
                effects: vec![QuestEffect {
                    track: track.into(),
                    from: QuestComparison::Any,
                    effect: QuestEffectKind::Set(1),
                }],
                completes: false,
                experience: None,
                world_change: None,
            }],
        )
        .map_err(debug)?,
    );
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let result = owner
        .root
        .commit_fixture_quest_reward(
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: key.into(),
                cause: QuestCause::Command(CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(9700).map_err(debug)?,
                )),
            },
            catalogue,
            QuestRewardAdmission {
                native,
                weights: &weights,
                fee_payment: None,
                actor_conditions: None,
                carried_use: None,
                creature_use: None,
                peer_use: None,
            },
            callback,
        )
        .await
        .map_err(debug)?;
    assert!(matches!(result, QuestTransitionOutcome::Committed(_)));
    slot.observe_endgame_progression();
    drop(slot);
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    assert_eq!(achievement_count(h, "honorary_rascoohan").await?, 0);
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::new(1).ok_or("handle")?),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(debug)?;
    let items: &[u32] = if mixed { &[35695] } else { &[35595, 35695] };
    let mut last = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let source =
            reference(&format!("oteryn:item.tibia.i{item}")).ok_or("voucher definition")?;
        let (end, result, actual) = super::rust_lottery::exchange(
            owner,
            admitted,
            1 + index as u64,
            &payload,
            Some(source),
            None,
        )
        .await?;
        admitted = end;
        assert_eq!(result, UseDisposition::Committed);
        assert_eq!(
            achievement_count(h, "honorary_rascoohan").await?,
            i64::from(index + 1 == items.len())
        );
        last = actual;
    }
    let (_, result, _) =
        super::rust_lottery::exchange(owner, admitted, items.len() as u64, &last, None, None)
            .await?;
    assert_eq!(result, UseDisposition::Committed);
    assert_eq!(achievement_count(h, "honorary_rascoohan").await?, 1);
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_account_achievement_grant_requests WHERE achievement_key='oteryn:achievement/honorary_rascoohan'").fetch_one(&h.pool).await?,1);
    let source: String = sqlx::query_scalar("SELECT source_kind FROM game_account_achievement_grant_requests WHERE achievement_key='oteryn:achievement/honorary_rascoohan'")
        .fetch_one(&h.pool).await?;
    assert_eq!(
        source, "full_outfit",
        "the shared appearance evaluator owns the award, including voucher completion"
    );
    eprintln!("FULLOUT_PATH_COMPLETED voucher mixed={mixed}");
    Ok(())
}

pub(super) async fn achievement_count(h: &Harness, slug: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM game_account_achievements WHERE achievement_key=$1",
    )
    .bind(format!("oteryn:achievement/{slug}"))
    .fetch_one(&h.pool)
    .await?)
}
