use super::fixture::{Harness, id};
use super::underwater_drowning_fixtures::{QuestStepCase, quest_step_scenario, step};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::{ExactActorRef, GameSessionId};
use oteryn_gameplay_quests::quest::world_callbacks::Callback;
use oteryn_protocol_oteryn::world_spatial::StepDirection;

const BOSS: &str = "oteryn:boss.essence_of_malice";

pub(super) fn essence_staging_portal_returns_character_until_boss_cooldown_expires_scenario()
-> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            quest_step_scenario(QuestStepCase::BossCooldown).map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Essence cooldown PostgreSQL fixture panicked")?
        .map_err(Into::into)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    mover: (AdmittedSession, ExactActorRef, GameSessionId),
    h: &Harness,
) -> TestResult {
    // Crystal 201189f8 movements_boss_timer.lua:13-15,82-99, actions_bosses_levers.lua:278,
    // and config.lua.dist:658. The staging step reads the cooldown written at fight admission.
    let callback: Callback = serde_json::from_value(serde_json::json!({
        "key":"oteryn:quest-callback/essence-cooldown-fixture",
        "quest":"oteryn:quest.cults_of_tibia_quest",
        "trigger":"step_in",
        "selector":{"position":{"x":2,"y":0,"floor":-7},"ordinal":0},
        "reversible":false, "passages":[], "storage":[], "offerings":[], "return_on_failure":true,
        "teleport":{"x":4,"y":0,"floor":-7},
        "effects":{"boss_cooldown":{"boss":BOSS,"name":"Essence of Malice"},"teleport_effect":11},
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/cults_of_tibia/movements_boss_timer.lua:13-99"]
    }))?;
    assert!(
        owner
            .states
            .quest_callbacks
            .fixture
            .set(vec![callback])
            .is_ok()
    );
    sqlx::query(
        "INSERT INTO game_character_boss_cooldowns(character_id,boss_key,available_at,admission,reset_epoch)
         VALUES(encode($1,'hex')::uuid,$2,floor(extract(epoch FROM clock_timestamp())*1000)::bigint+72000000,
                encode($3,'hex')::uuid,0)",
    )
    .bind(id(41).as_slice())
    .bind(BOSS)
    .bind(id(91).as_slice())
    .execute(&h.pool)
    .await?;
    step(owner, mover, StepDirection::East, 1).await?;
    sqlx::query(
        "UPDATE game_character_boss_cooldowns
         SET available_at=floor(extract(epoch FROM clock_timestamp())*1000)::bigint-1
         WHERE character_id=encode($1,'hex')::uuid AND boss_key=$2",
    )
    .bind(id(41).as_slice())
    .bind(BOSS)
    .execute(&h.pool)
    .await?;
    step(owner, mover, StepDirection::East, 4).await?;
    Ok(())
}
