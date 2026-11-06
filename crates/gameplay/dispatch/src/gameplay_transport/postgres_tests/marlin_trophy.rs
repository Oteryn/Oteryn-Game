//! Pemaret's once-only carried-fish exchange through production NPC dispatch.
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

const TRACK: &str = "oteryn:quest-progress/quest/u7_8/marlin_trophy";

#[test]
fn marlin_trophy_carried_fish_exchange_is_atomic_once_and_replays() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("marlin-trophy-exchange", || {
        super::quest_world_callbacks::scenario(
            "marlin-trophy-exchange",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })?;
    println!(
        "POSTGRES_CASE_COMPLETED marlin_trophy_carried_fish_exchange_is_atomic_once_and_replays duration={:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let [pemaret] = super::classic_quests::install(owner, admitted, ["oteryn:npc.pemaret"]).await?;
    super::classic_quests::talk(owner, admitted, pemaret, 2900, "hi").await?;
    let missing = super::classic_quests::talk(owner, admitted, pemaret, 2901, "marlin").await?;
    assert!(
        !missing.iter().any(|line| line.contains("May I have it")),
        "no offer without a carried fish"
    );
    let before: (i64, i64) = sqlx::query_as("SELECT total_experience,(SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i902') FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    for n in [400, 401] {
        super::quest_world_callbacks::input_item(
            owner,
            fence.item_fence(),
            &Offering {
                position: Position {
                    x: 33293,
                    y: 31677,
                    floor: -15,
                },
                item: "oteryn:item.tibia.i901".into(),
                count: 1,
            },
            n,
            true,
        )
        .await?;
    }
    let offer = super::classic_quests::talk(owner, admitted, pemaret, 2902, "marlin").await?;
    assert!(offer.iter().any(|line| line.contains("May I have it")));
    super::classic_quests::talk(owner, admitted, pemaret, 2903, "no").await?;
    let fish: i64 = sqlx::query_scalar("SELECT sum(quantity)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i901'")
        .fetch_one(&h.pool).await?;
    assert_eq!(fish, 2);
    super::classic_quests::talk(owner, admitted, pemaret, 2904, "marlin").await?;
    refuse_after_values(owner, admitted, h, authority, pemaret, before.1).await?;
    super::classic_quests::talk(owner, admitted, pemaret, 2906, "hi").await?;
    super::classic_quests::talk(owner, admitted, pemaret, 2907, "marlin").await?;
    let delivered = super::classic_quests::talk(owner, admitted, pemaret, 2908, "yes").await?;
    assert!(
        delivered
            .iter()
            .any(|line| line.contains("hope you like it"))
    );
    assert_eq!(
        super::classic_quests::progress(owner, admitted, authority, TRACK).await?,
        1
    );
    let after: (i64, i64, i64) = sqlx::query_as("SELECT (SELECT total_experience FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid),(SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i902'),(SELECT sum(quantity)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i901')")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(after, (before.0, before.1 + 1, 1));
    owner.admit_quest_session(admitted).await;
    let replay = owner
        .root
        .reconcile_npc_program(
            authority,
            fence.character_id,
            crate::durability::quest_state::npc_program::NpcProgramRequest {
                command: crate::foundation::CommandRef::new(
                    admitted.game_session_id,
                    crate::foundation::CommandId::new(2908).map_err(debug)?,
                ),
                npc: "oteryn:npc.pemaret".into(),
                trigger: oteryn_gameplay_quests::quest::npc_rules::NpcRuleTrigger::Say,
                message: "yes".into(),
                topic: 1,
                transition_key: "oteryn:quest-npc/marlin_trophy_quest/pemaret/deliver".into(),
            },
        )
        .await
        .map_err(debug)?
        .ok_or("exchange receipt")?;
    assert_eq!(replay.reply, delivered);
    assert!(matches!(
        replay.transition,
        crate::durability::quest_state::QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    super::classic_quests::talk(owner, admitted, pemaret, 2909, "marlin").await?;
    super::classic_quests::talk(owner, admitted, pemaret, 2910, "yes").await?;
    let final_counts: (i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i902'),(SELECT sum(quantity)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i901')")
        .fetch_one(&h.pool).await?;
    assert_eq!(final_counts, (before.1 + 1, 1));
    Ok(())
}

async fn refuse_after_values(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    pemaret: EntityRef,
    trophies_before: i64,
) -> TestResult {
    sqlx::query("CREATE SEQUENCE fixture_marlin_failure_seen")
        .execute(&h.pool)
        .await?;
    sqlx::query(
        "GRANT USAGE, SELECT ON SEQUENCE fixture_marlin_failure_seen TO oteryn_game_runtime",
    )
    .execute(&h.pool)
    .await?;
    sqlx::query("CREATE FUNCTION fixture_marlin_fail() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.track_key='oteryn:quest-progress/quest/u7_8/marlin_trophy' THEN PERFORM nextval('fixture_marlin_failure_seen'); RAISE EXCEPTION 'injected final Marlin track failure'; END IF; RETURN NEW; END $$").execute(&h.pool).await?;
    sqlx::query("CREATE TRIGGER fixture_marlin_fail BEFORE INSERT OR UPDATE ON game_character_quest_tracks FOR EACH ROW EXECUTE FUNCTION fixture_marlin_fail()").execute(&h.pool).await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let command = UseCommand {
        game_session_id: admitted.game_session_id,
        command_id: 2905,
        item_fence: Some(fence.item_fence()),
    };
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let failed = owner
        .npc_talk(
            admitted.runtime_actor.ok_or("actor")?,
            command,
            NpcTalkIntent {
                npc_actor: pemaret,
                text: "yes".into(),
            },
        )
        .await;
    assert_eq!(failed.disposition, NpcIntentDisposition::Rejected);
    assert!(
        sqlx::query_scalar::<_, bool>("SELECT is_called FROM fixture_marlin_failure_seen")
            .fetch_one(&h.pool)
            .await?,
        "failure must follow fish burn and trophy mint"
    );
    let rolled_back: (i64, i64, i64) = sqlx::query_as("SELECT (SELECT sum(quantity)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i901'),(SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i902'),(SELECT count(*) FROM game_character_quest_receipts WHERE quest_key='oteryn:quest.marlin_trophy_quest')")
        .fetch_one(&h.pool).await?;
    assert_eq!(
        rolled_back,
        (2, trophies_before, 0),
        "failed exchange must restore fish, trophy and quest receipt"
    );
    let state = owner
        .root
        .read_character_quest_state(authority, fence.character_id)
        .await
        .map_err(debug)?;
    assert!(
        !state.tracks().contains_key(TRACK),
        "rollback must leave the source track absent"
    );
    sqlx::query("DROP TRIGGER fixture_marlin_fail ON game_character_quest_tracks")
        .execute(&h.pool)
        .await?;
    sqlx::query("DROP FUNCTION fixture_marlin_fail()")
        .execute(&h.pool)
        .await?;
    sqlx::query("DROP SEQUENCE fixture_marlin_failure_seen")
        .execute(&h.pool)
        .await?;
    let recovery = owner
        .resume_npc_bank(admitted.runtime_actor.ok_or("actor")?, command)
        .await
        .ok_or("retained failed exchange")?;
    assert!(!recovery.unknown);
    assert_eq!(recovery.disposition, NpcIntentDisposition::Rejected);
    Ok(())
}
