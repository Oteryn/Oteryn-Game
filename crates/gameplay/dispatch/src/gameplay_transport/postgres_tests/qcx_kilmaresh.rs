//! Kilmaresh Quest/Spoiler (curid 88780, oldid 1199254), The Boards that Mean the World.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::quest_state::{QuestTransitionOutcome, npc_program::NpcProgramRequest};
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_quests::quest::{loader, npc_rules::NpcRuleTrigger};
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
        let (controller, _) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(
                deps.admitted.world_id,
                None,
            );
        let native = controller
            .active()
            .and_then(|active| active.native_gameplay())
            .ok_or("native items")?;
        let catalogue = Arc::new(
            loader::load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        );
        let fence = owner
            .current_quest_fence(deps.admitted.game_session_id)
            .await
            .map_err(|()| "quest fence")?
            .ok_or("active quest session")?;
        let permit = owner.spell_lane_permit().await.ok_or("NPC item lane")?;

        // Create durable hand-in states at the claim boundary.
        for (command, lyre, tortoise) in [(100, 3, 3), (101, 4, 2)] {
            for (track, value) in [(FAFNAR, 301), (LYRE, lyre), (TORTOISE, tortoise)] {
                sqlx::query(
                    "INSERT INTO game_character_quest_tracks
                     (character_id, track_key, quest_key, value, committed_character_revision)
                     VALUES (encode($1,'hex')::uuid, $2, $3, $4, 2)
                     ON CONFLICT (character_id, track_key) DO UPDATE SET value=excluded.value",
                )
                .bind(id(41).as_slice())
                .bind(track)
                .bind(QUEST)
                .bind(i64::from(value))
                .execute(&h.pool)
                .await?;
            }
            let result = owner
                .root
                .commit_character_npc_program(
                    owner.character,
                    owner.holder,
                    &permit,
                    fence,
                    NpcProgramRequest {
                        command: CommandRef::new(
                            fence.game_session_id,
                            CommandId::new(command).map_err(debug)?,
                        ),
                        npc: "oteryn:npc.alyxo".into(),
                        trigger: NpcRuleTrigger::Say,
                        message: "yes".into(),
                        topic: 13,
                        transition_key: "oteryn:quest-npc/m41-kilmaresh/alyxo/sculptor-reward"
                            .into(),
                    },
                    catalogue.clone(),
                    &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native),
                    &owner.premium_coordinator,
                )
                .await
                .map_err(debug)?;
            assert!(
                matches!(result.transition, QuestTransitionOutcome::Refused(_)),
                "Alyxo granted a reward with lyre={lyre} and tortoise={tortoise}: {result:?}"
            );
        }
        Ok(())
    })
}
