//! Secret Service recruitment through the production NPC command and QuestState writer.
use super::fixture::{Harness, debug, id, run};
use super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session,
};
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::spell_premium_coordinator::{
    SpellPremiumCoordinator, purchase_fixture,
};
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

const QUEST: &str = "oteryn:quest.secret_service_quest";
const FACTION: &str = "oteryn:quest-progress/secret-service/faction";
const RECRUITERS: [&str; 3] = [
    "oteryn:npc.chester_kahs",
    "oteryn:npc.uncle",
    "oteryn:npc.emma",
];

#[test]
fn recruitment_requires_level_and_premium_and_locks_the_faction() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("secret-service-recruitment", || {
        for (level, premium, faction) in [
            (49, true, 0),
            (50, false, 0),
            (50, true, 1),
            (50, true, 2),
            (50, true, 3),
        ] {
            let started = std::time::Instant::now();
            let result = recruitment(level, premium, faction);
            eprintln!(
                "Secret Service recruitment level={level} premium={premium} faction={faction} duration={:?}",
                started.elapsed()
            );
            result?;
        }
        Ok(())
    })
}

fn recruitment(level: u32, premium: bool, faction: usize) -> TestResult {
    run(async |admin| {
        let h =
            Harness::create_with_admission_seed(admin, "qcx_secret_service", true, false).await?;
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        let updated = sqlx::query("UPDATE game_character_progression_state SET level=$1")
            .bind(i64::from(level))
            .execute(&mut *tx)
            .await?;
        assert_eq!(updated.rows_affected(), 1);
        tx.commit().await?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
            .bind(id(41).as_slice())
            .execute(&h.pool)
            .await?;
        super::reset();
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        deps.runtime
            .lock()
            .await
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: 6,
                    y: 1,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        let (consumer, clock, activation, _) = purchase_fixture::current(id(40));
        let entitlement = SpellPremiumCoordinator::new(
            consumer,
            Some(clock),
            Some(activation),
            Some(h.root.clone()),
        );
        let (controller, _) = super::death_ml_loss_composed::production_generation_with_pin(
            deps.admitted.world_id,
            "content-1",
        )?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = controller.active();
        owner.quest_catalogue = Some(std::sync::Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        ));
        let session = deps.admitted.game_session_id;
        if premium {
            owner.premium_coordinator = Some(&entitlement);
            owner.admit_premium(id(40), session);
        }
        let mut admitted = deps.admitted;
        admitted.first_entry = super::FirstEntryOutcome::Positioned;
        let npcs = super::questl2_fixture::install_at(
            &owner,
            &admitted,
            RECRUITERS,
            [5, 6, 7].map(|x| Position { x, y: 2, floor: -7 }),
        )
        .await?;
        {
            let runtime = deps.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            let initialized = states.initialize(
                &runtime,
                deps.actor,
                session,
                crate::spell::cast::CharacterCastFacts {
                    level,
                    ..crate::gameplay_transport::actor_spell::tests::FACTS
                },
                (3, 7_000_000),
                owner.owner_now(),
            );
            assert!(initialized.is_some());
        }
        owner.refresh_quest_session(session, true).await;
        let order = match faction {
            2 => [1, 2, 0],
            3 => [2, 0, 1],
            _ => [0, 1, 2],
        };
        for (turn, index) in order.into_iter().enumerate() {
            let npc = npcs[index];
            let command = 100 + 4 * u64::try_from(turn)?;
            talk(&mut owner, &admitted, npc, command, "hi").await?;
            talk(&mut owner, &admitted, npc, command + 1, "join").await?;
            let confirmation = talk(&mut owner, &admitted, npc, command + 2, "yes").await?;
            if faction > 0 && turn == 0 {
                assert_eq!(confirmation, NpcIntentDisposition::Ok);
            }
            let fence = owner
                .current_quest_fence(session)
                .await
                .map_err(|()| "fence")?
                .ok_or("fence")?;
            let copy = h
                .root
                .read_character_quest_state(&authority, fence.character_id)
                .await
                .map_err(debug)?;
            assert_eq!(
                *copy.tracks().get(FACTION).unwrap_or(&0),
                i64::try_from(faction)?,
                "recruiter {} changed the faction incorrectly",
                RECRUITERS[index],
            );
            talk(&mut owner, &admitted, npc, command + 3, "bye").await?;
        }
        let receipts: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1",
        )
        .bind(QUEST)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(receipts, i64::from(faction > 0));
        let achievement: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/secret_agent'",
        ).fetch_one(&h.pool).await?;
        // Crystal uncle.lua:63 and emma.lua:63 grant it during recruitment.
        assert_eq!(achievement, i64::from(matches!(faction, 2 | 3)));
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn talk(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    npc: EntityRef,
    command_id: u64,
    text: &str,
) -> TestResult<NpcIntentDisposition> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let result = owner
        .npc_talk(
            admitted.runtime_actor.ok_or("actor")?,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id,
                item_fence: Some(fence.item_fence()),
            },
            NpcTalkIntent {
                npc_actor: npc,
                text: text.into(),
            },
        )
        .await;
    assert!(!result.unknown, "{command_id}: {text}");
    Ok(result.disposition)
}
