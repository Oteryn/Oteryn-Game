//! Production Oramond selectors and daily NPC admission share the real quest/MINT writer.
use super::engitem_reward_text::{Weights, command, equip_backpack};
use crate::bestiary_postgres_harness as f;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{ItemWeightLookup, ItemWeightRefused};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::durability::reward_claim_mint::QuestRewardAdmission;
use oteryn_gameplay_quests::quest::{reward_callbacks, world_callbacks};
use std::sync::Arc;
struct PremiumGate;
impl crate::durability::quest_state::grizzly::GrizzlyPremiumGate for PremiumGate {
    async fn task_permitted(
        &self,
        _: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        _: [u8; 16],
    ) -> Result<bool, crate::durability::DurabilityError> {
        Ok(true)
    }
}
struct PremiumWeights<'a>(Weights<'a>);
impl ItemWeightLookup for PremiumWeights<'_> {
    fn generation_digest(&self) -> [u8; 32] {
        self.0.generation_digest()
    }
    fn weight_of(&self, definition: &TypedDefinitionRef) -> Result<u32, ItemWeightRefused> {
        self.0.weight_of(definition)
    }
    fn wheel_premium_current(&self, _: [u8; 16]) -> bool {
        true
    }
}
#[test]
fn engitem_oramond_native_roots_persist_exactly_five_daily_grants() -> f::TestResult {
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
                    let h = f::Harness::create(admin, "engitem_oramond", true).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "ENGITEM_PG_EXECUTED oramond_native_roots duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "Oramond PG thread failed")?
        .map_err(Into::into)
}
async fn exercise(h: &f::Harness) -> f::TestResult {
    let callbacks: Vec<_> = world_callbacks::load()?
        .into_iter()
        .filter(|c| c.quest == "oteryn:quest.oramond_quest")
        .collect();
    assert_eq!(callbacks.len(), 5);
    let rewards = reward_callbacks::load()?;
    let start = rewards
        .iter()
        .find(|c| c.key == "oteryn:quest-reward/oramond/native-roots-start")
        .ok_or("start")?;
    let restart = rewards
        .iter()
        .find(|c| c.key == "oteryn:quest-reward/oramond/native-roots-restart")
        .ok_or("restart")?;
    let lowered = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(f::debug)?;
    let catalogue = Arc::new(lowered.catalogue().clone());
    let (controller, pin) =
        oteryn_gameplay_content::content::native_gameplay::test_support::activated_with_loot_tables(
            f::scope()?.world_id(),
            None,
        );
    let native = controller
        .active()
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    assert_eq!(
        native.source_digest(),
        pin.identity().server_artifact_digest()
    );
    // Standard control-fixture grant; the real activation writer still enforces it.
    sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(f::id(f::WORLD).as_slice()).bind(f::id(f::CHANNEL).as_slice()).execute(&h.pool).await?;
    assert!(
        h.root
            .record_content_activation(
                &crate::durability::content_activation::ContentActivationRequest {
                    world_id: f::scope()?.world_id(),
                    channel_id: crate::foundation::ChannelId::decode(&f::id(f::CHANNEL))
                        .map_err(f::debug)?,
                    activation_sequence: 1,
                    previous_sequence: None,
                    server_artifact_digest: native.source_digest(),
                    client_artifact_digest: pin.identity().client_artifact_digest(),
                    frame_binding_digest: pin.frame_binding().digest(),
                }
            )
            .await
            .map_err(f::debug)?
    );
    let weights = PremiumWeights(Weights(native));
    let admission = QuestRewardAdmission {
        native,
        weights: &weights,
        fee_payment: None,
        actor_conditions: None,
        carried_use: None,
        peer_use: None,
        creature_use: None,
    };
    let context = crate::durability::quest_world_callback::QuestWorldContext {
        map_revision: "map-1".into(),
        content_revision: format!(
            "sha256:{}",
            native
                .source_digest()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        ),
        placement_context: pin.frame_binding().digest().to_vec(),
        reset_epoch: 0,
        actor_placement: f::id(17),
        movement_revision: 1,
    };
    let seal = h.recovery.seal_current().map_err(f::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(f::debug)?;
    let lane = crate::durability::spell_owner_commit::SpellLanePermit::of_fresh_scope(f::scope()?)
        .await
        .map_err(f::debug)?;

    let sequencer =
        crate::durability::character_revision_sequencer::CharacterRevisionSequencer::new();
    let world = |revision, id, callback: &world_callbacks::Callback| -> f::TestResult<_> {
        Ok(h.root.commit_fixture_world_reward(
            &authority,
            &h.node,
            f::fence(revision)?,
            QuestTransitionRequest {
                transition_key: callback.key.clone(),
                cause: QuestCause::Use(command(id)?),
            },
            catalogue.clone(),
            &lane,
            context.clone(),
            admission,
            callback.clone(),
        ))
    };
    assert!(
        matches!(
            world(1, 9700, &callbacks[0])?.await.map_err(f::debug)?,
            QuestTransitionOutcome::Refused(_)
        ),
        "start required"
    );
    assert!(matches!(
        sequencer
            .acquire(f::fence(1)?.character_id)
            .await
            .commit_quest_reward(
                &h.root,
                &authority,
                &h.node,
                f::fence(1)?,
                QuestTransitionRequest {
                    transition_key: start.key.clone(),
                    cause: QuestCause::Command(command(9701)?)
                },
                catalogue.clone(),
                admission,
                &PremiumGate
            )
            .await
            .map_err(f::debug)?,
        QuestTransitionOutcome::Committed(_)
    ));
    assert!(
        matches!(
            world(2, 9702, &callbacks[0])?.await.map_err(f::debug)?,
            QuestTransitionOutcome::Refused(_)
        ),
        "no backpack"
    );
    equip_backpack(h, &authority, native, &weights.0).await?;
    for (n, callback) in callbacks.iter().enumerate() {
        let revision = 2 + n as u64;
        let id = 9710 + n as u64;
        assert!(matches!(
            world(revision, id, callback)?.await.map_err(f::debug)?,
            QuestTransitionOutcome::Committed(_)
        ));
        assert!(matches!(
            world(revision, id, callback)?.await.map_err(f::debug)?,
            QuestTransitionOutcome::AlreadyCommitted(_)
        ));
        assert!(
            matches!(
                world(revision + 1, id + 100, callback)?
                    .await
                    .map_err(f::debug)?,
                QuestTransitionOutcome::Refused(_)
            ),
            "same plant fresh command"
        );
    }
    assert!(
        matches!(
            sequencer
                .acquire(f::fence(7)?.character_id)
                .await
                .commit_quest_reward(
                    &h.root,
                    &authority,
                    &h.node,
                    f::fence(7)?,
                    QuestTransitionRequest {
                        transition_key: restart.key.clone(),
                        cause: QuestCause::Command(command(9800)?)
                    },
                    catalogue.clone(),
                    admission,
                    &PremiumGate
                )
                .await
                .map_err(f::debug)?,
            QuestTransitionOutcome::Refused(_)
        ),
        "daily clock prevents reopening"
    );
    let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_reward_claim_mint_receipts m JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key='oteryn:item.tibia.i21291'").fetch_one(&h.pool).await?;
    assert_eq!(count, 5);
    let atomic:i64=sqlx::query_scalar("SELECT count(*) FROM game_reward_claim_mint_receipts m JOIN game_character_quest_receipts q ON q.character_id=m.character_id AND q.cause_id=m.game_session_id AND q.cause_ordinal=m.command_id AND q.created_xact_id=m.created_xact_id WHERE q.quest_key='oteryn:quest.oramond_quest'").fetch_one(&h.pool).await?;
    assert_eq!(atomic, 5);
    let harvests:i64=sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key='oteryn:quest-progress/quest/u10_50/oramond_quest/to_take_roots/count'").fetch_one(&h.pool).await?;
    assert_eq!(
        harvests, 5,
        "existing Chavis hand-in sees the real harvests"
    );
    let report = rewards
        .iter()
        .find(|c| c.key == "oteryn:quest-reward/oramond/native-roots-report")
        .ok_or("root report")?;
    let reported = sequencer
        .acquire(f::fence(7)?.character_id)
        .await
        .commit_quest_reward(
            &h.root,
            &authority,
            &h.node,
            f::fence(7)?,
            QuestTransitionRequest {
                transition_key: report.key.clone(),
                cause: QuestCause::Command(command(9801)?),
            },
            catalogue.clone(),
            admission,
            &PremiumGate,
        )
        .await
        .map_err(f::debug)?;
    assert!(
        matches!(reported, QuestTransitionOutcome::Committed(_)),
        "{reported:?}"
    );
    let premature = sequencer
        .acquire(f::fence(8)?.character_id)
        .await
        .commit_quest_reward(
            &h.root,
            &authority,
            &h.node,
            f::fence(8)?,
            QuestTransitionRequest {
                transition_key: restart.key.clone(),
                cause: QuestCause::Command(command(9802)?),
            },
            catalogue.clone(),
            admission,
            &PremiumGate,
        )
        .await
        .map_err(f::debug)?;
    assert!(
        matches!(premature, QuestTransitionOutcome::Refused(_)),
        "day must elapse even after hand-in: {premature:?}"
    );
    let remaining:i64=sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i21291' AND lifecycle=1").fetch_one(&h.pool).await?;
    assert_eq!(remaining, 0, "exactly five harvested roots handed in once");

    Ok(())
}
