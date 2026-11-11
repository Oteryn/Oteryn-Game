//! Literal source text composes the existing Quest and backpack MINT transaction.
use crate::bestiary_postgres_harness as f;
use crate::durability::item_mint::{
    GroundPlacement, ItemMintCause, ItemMintOutcome, ItemMintRequest, TypedDefinitionRef,
};
use crate::durability::item_transfer::{
    ItemTransferDestination, ItemTransferOutcome, ItemTransferRequest, ItemWeightLookup,
    ItemWeightRefused,
};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::durability::reward_claim_mint::QuestRewardAdmission;
use crate::foundation::{
    ChannelId, CommandId, CommandRef, GameSessionId, ScopeOwnershipGeneration, WorldId,
};
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_items::item_ref_admission::generation_item_facts;
use oteryn_gameplay_quests::quest::reward_callbacks::{self, RewardCallback};
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, QuestTrack};
use std::sync::Arc;

#[test]
fn engitem_literal_reward_text_is_atomic_refused_and_replayed() -> f::TestResult {
    let Some(admin) = f::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            f::runtime()
                .map_err(f::debug)?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let h = f::Harness::create(admin, "engitem_reward_text", true).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "ENGITEM_PG_EXECUTED literal_reward_text duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "reward text PG thread failed")?
        .map_err(Into::into)
}

pub(super) struct Weights<'a>(pub(super) &'a NativeGameplayState);
impl ItemWeightLookup for Weights<'_> {
    fn generation_digest(&self) -> [u8; 32] {
        self.0.source_digest()
    }
    fn weight_of(&self, definition: &TypedDefinitionRef) -> Result<u32, ItemWeightRefused> {
        generation_item_facts(self.0, definition)
            .map_err(|_| ItemWeightRefused::DefinitionMismatch)?
            .weight
            .ok_or(ItemWeightRefused::Unknown)
    }
}
pub(super) fn definition(item: u32) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: "Item".into(),
        production_key: format!("oteryn:item.tibia.i{item}"),
        revision_ref: "definition-r1".into(),
    }
}
pub(super) fn command(n: u64) -> f::TestResult<CommandRef> {
    Ok(CommandRef::new(
        GameSessionId::decode(&f::id(f::SESSION)).map_err(f::debug)?,
        CommandId::new(n).map_err(f::debug)?,
    ))
}
async fn exercise(h: &f::Harness) -> f::TestResult {
    let body = "A source-authored letter.\nKeep the literal café text.\n";
    let callback: RewardCallback = serde_json::from_value(serde_json::json!({
        "key":"oteryn:quest-reward/engitem-readable", "quest":"oteryn:quest.engitem_readable",
        "npc":"oteryn:npc.fixture", "keywords":["yes"], "topic":0,"next_topic":0,
        "storage":[{"track":"oteryn:quest-progress/engitem-readable","minimum":0,"maximum":0,"set":1}],
        "take":[],"give":{"item":"oteryn:item.tibia.i2814","count":1,"contents":[],"text":body},
        "outfit":null,"achievement":null,"reply":["Granted."],"sources":["engine fixture"]
    }))?;
    let catalogue = Arc::new(
        QuestStateCatalogue::new(
            "content-1",
            vec![QuestTrack {
                key: callback.storage[0].track.clone(),
                quest: callback.quest.clone(),
                initial: 0,
                min: 0,
                max: 1,
            }],
            vec![reward_callbacks::primary_transition(&callback).map_err(f::debug)?],
        )
        .map_err(f::debug)?,
    );
    let request = QuestTransitionRequest {
        transition_key: callback.key.clone(),
        cause: QuestCause::Command(command(9400)?),
    };
    let (controller, _) =
        oteryn_gameplay_content::content::native_gameplay::test_support::activated_with_loot_tables(
            f::scope()?.world_id(),
            None,
        );
    let native = controller
        .active()
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let weights = Weights(native);
    let admission = QuestRewardAdmission {
        native,
        weights: &weights,
        fee_payment: None,
        actor_conditions: None,
        carried_use: None,
        creature_use: None,
        peer_use: None,
    };
    let seal = h.recovery.seal_current().map_err(f::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(f::debug)?;
    let quest_fence = f::fence(1)?;
    let apply = |callback: RewardCallback, catalogue: Arc<QuestStateCatalogue>| {
        h.root.commit_fixture_quest_reward(
            &authority,
            &h.node,
            quest_fence,
            request.clone(),
            catalogue,
            admission,
            callback,
        )
    };
    assert!(matches!(
        apply(callback.clone(), catalogue.clone())
            .await
            .map_err(f::debug)?,
        QuestTransitionOutcome::Refused(_)
    ));
    let absent:(i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_item_texts),(SELECT count(*) FROM game_character_quest_receipts)").fetch_one(&h.pool).await?;
    assert_eq!(
        absent,
        (0, 0),
        "no backpack: no partial text or quest progress"
    );
    equip_backpack(h, &authority, native, &weights).await?;
    assert!(matches!(
        apply(callback.clone(), catalogue.clone())
            .await
            .map_err(f::debug)?,
        QuestTransitionOutcome::Committed(_)
    ));
    let read = || {
        sqlx::query_as::<_,(String,i64,i64)>("SELECT t.text,(SELECT count(*) FROM game_reward_claim_mint_receipts),(SELECT count(*) FROM game_character_quest_receipts) FROM game_item_texts t").fetch_one(&h.pool)
    };
    assert_eq!(read().await?, (body.into(), 1, 1));
    let atomic: bool = sqlx::query_scalar("SELECT m.root_text=t.text AND (s.plan #>> '{give,text}')=t.text AND t.writer_character_id IS NULL AND t.text_revision=1 AND t.xmin=m.xmin AND m.xmin=s.xmin AND s.xmin=q.xmin FROM game_item_texts t JOIN game_reward_claim_mint_receipts m USING(item_instance_id) JOIN game_quest_reward_sources s ON s.mint_transaction_id=m.transaction_id JOIN game_character_quest_receipts q ON (q.character_id,q.cause_id,q.cause_ordinal,q.transition_key)=(s.character_id,s.cause_id,s.cause_ordinal,s.transition_key)")
        .fetch_one(&h.pool).await?;
    assert!(
        atomic,
        "exact text, source, MINT and progress share one transaction"
    );
    assert!(matches!(
        apply(callback, catalogue).await.map_err(f::debug)?,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    assert_eq!(
        read().await?,
        (body.into(), 1, 1),
        "exact replay never remints or rewrites text"
    );
    Ok(())
}

// Shared engine-only native item fixture; no quest data or direct custody SQL.
pub(super) async fn equip_backpack(
    h: &f::Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    native: &NativeGameplayState,
    weights: &Weights<'_>,
) -> f::TestResult {
    let mut minted = h
        .root
        .freeze_item_mint(
            authority,
            &h.node,
            ItemMintRequest {
                cause: ItemMintCause::for_test(
                    WorldId::decode(&f::id(f::WORLD)).map_err(f::debug)?,
                    ChannelId::decode(&f::id(f::CHANNEL)).map_err(f::debug)?,
                    ScopeOwnershipGeneration::new(1).map_err(f::debug)?,
                    1,
                    1,
                    TypedDefinitionRef {
                        family: "LootTable".into(),
                        production_key: "fixture:engitem".into(),
                        revision_ref: "definition-r1".into(),
                    },
                    "fixture:engitem".into(),
                    0,
                ),
                item: definition(2854),
                quantity: 1,
                ground: GroundPlacement {
                    spatial_position: vec![0, 0, 0, 1, 0, 0, 0, 2, 0, 3],
                    corpse_ref: f::id(3).to_vec(),
                    map_revision: "map-1".into(),
                    content_revision: "content-1".into(),
                    native_room_placement_context: f::id(6).to_vec(),
                },
                content_revision: "content-1".into(),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
            },
        )
        .await
        .map_err(f::debug)?;
    let permit = minted.fresh_death_lane_permit().await;
    let ItemMintOutcome::Committed(bag) = h
        .root
        .commit_item_mint(&permit, authority, &h.node, &mut minted)
        .await
        .map_err(f::debug)?
    else {
        return Err("bag mint".into());
    };
    let fence = f::item_fence()?;
    let mut transfer = h
        .root
        .freeze_item_transfer(
            authority,
            &h.node,
            fence,
            ItemTransferRequest {
                weight_generation_digest: native.source_digest(),
                command: command(9300)?,
                source_item_instance_id: bag.item_instance_id,
                destination: ItemTransferDestination::ContainerSlot,
                item: generation_item_facts(native, &definition(2854)).map_err(f::debug)?,
                backpack: None,
                content_revision: "content-1".into(),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
            },
            weights,
        )
        .await
        .map_err(f::debug)?;
    let lane =
        crate::durability::spell_owner_commit::SpellLanePermit::of_fresh_scope(fence.runtime_scope)
            .await
            .map_err(f::debug)?;
    assert!(matches!(
        h.root
            .commit_item_transfer(&lane, authority, &h.node, fence, &mut transfer, weights)
            .await
            .map_err(f::debug)?,
        ItemTransferOutcome::Committed(_)
    ));
    Ok(())
}
