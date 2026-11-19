//! Kilmaresh Quest/Spoiler (curid 88780, oldid 1199254), The Boards that Mean the World.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::quest_state::{
    QuestCause, QuestTransitionOutcome, QuestTransitionRequest, npc_program::NpcProgramRequest,
};
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestTransition, loader,
    npc_rules::NpcRuleTrigger,
};
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.kilmaresh_quest";
const LYRE: &str = "oteryn:quest-progress/crystalserver/kilmaresh/thirteen/lyre";
const TORTOISE: &str = "oteryn:quest-progress/crystalserver/kilmaresh/thirteen/presente";
const FAFNAR: &str = "oteryn:quest-progress/crystalserver/kilmaresh/thirteen/fafnar";

#[test]
fn alyxo_requires_both_item_hand_ins_before_granting_sunray_emblem() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let start = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| alyxo().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Kilmaresh PostgreSQL case panicked")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED alyxo_reward_gate duration={:?}",
        start.elapsed()
    );
    Ok(())
}

fn alyxo() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "qcx_kilmaresh", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        let owner = deps.owner(&h, &authority);
        let (controller, activation) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(
                deps.admitted.world_id,
                None,
            );
        let native = controller
            .active()
            .and_then(|active| active.native_gameplay())
            .ok_or("native items")?;
        let mut catalogue = loader::load_embedded_quest_state("content-1")
            .map_err(debug)?
            .catalogue()
            .clone();
        let cases = [
            (
                "oteryn:quest-transition/qcx54/no-lyre",
                &[(FAFNAR, 301), (LYRE, 3), (TORTOISE, 3)][..],
                false,
            ),
            (
                "oteryn:quest-transition/qcx54/no-tortoise",
                &[(LYRE, 4), (TORTOISE, 2)][..],
                false,
            ),
            (
                "oteryn:quest-transition/qcx54/ready",
                &[(TORTOISE, 3)][..],
                true,
            ),
        ];
        for (key, tracks, _) in cases {
            catalogue = catalogue
                .with_fixture_transition(QuestTransition {
                    key: key.into(),
                    quest: QUEST.into(),
                    effects: tracks
                        .iter()
                        .map(|&(track, value)| QuestEffect {
                            track: track.into(),
                            from: QuestComparison::Any,
                            effect: QuestEffectKind::Set(value),
                        })
                        .collect(),
                    completes: false,
                    experience: None,
                    world_change: None,
                })
                .map_err(debug)?;
        }
        let catalogue = Arc::new(catalogue);
        let fence = owner
            .current_quest_fence(deps.admitted.game_session_id)
            .await
            .map_err(|()| "quest fence")?
            .ok_or("active quest session")?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
            .bind(id(41).as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        let generation = controller.active().ok_or("generation")?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice())
            .bind(activation.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        super::item_look_fixtures::seed_main_backpack(&h, &[]).await?;
        let permit = owner.spell_lane_permit().await.ok_or("NPC item lane")?;

        // Each boundary state is written with the same receipt checks as live progress.
        for (index, (key, _, complete)) in cases.into_iter().enumerate() {
            let command = |id| -> TestResult<_> {
                Ok(CommandRef::new(
                    fence.game_session_id,
                    CommandId::new(id).map_err(debug)?,
                ))
            };
            let id = 100 + u64::try_from(index)? * 2;
            let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
            let seeded = slot
                .commit_quest_transition(
                    owner.root,
                    owner.character,
                    owner.holder,
                    fence,
                    QuestTransitionRequest {
                        transition_key: key.into(),
                        cause: QuestCause::Command(command(id)?),
                    },
                    Arc::clone(&catalogue),
                )
                .await
                .map_err(debug)?;
            assert!(
                matches!(seeded, QuestTransitionOutcome::Committed(_)),
                "{seeded:?}"
            );
            let result = slot
                .commit_npc_program(
                    owner.root,
                    owner.character,
                    owner.holder,
                    &permit,
                    fence,
                    NpcProgramRequest {
                        command: command(id + 1)?,
                        npc: "oteryn:npc.alyxo".into(),
                        trigger: NpcRuleTrigger::Say,
                        message: "yes".into(),
                        topic: 13,
                        transition_key: "oteryn:quest-npc/m41-kilmaresh/alyxo/sculptor-reward"
                            .into(),
                    },
                    catalogue.clone(),
                    &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native)
                        .with_wheel(owner.premium_coordinator),
                    &owner.premium_coordinator,
                )
                .await
                .map_err(debug)?;
            assert_eq!(
                matches!(result.transition, QuestTransitionOutcome::Committed(_)),
                complete,
                "Unexpected Alyxo claim result after {key}: {result:?}"
            );
            let quantity: i64 = sqlx::query_scalar(
                "SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances
                 WHERE definition_production_key='oteryn:item.tibia.i31574' AND lifecycle=1",
            )
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(quantity, i64::from(complete));
        }
        Ok(())
    })
}
