//! The Cobra rejects an unpoisoned greeting over the production NPC talk caller.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::foundation::{
    ChannelContentPin, MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements,
};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;

#[test]
fn tombs_cobra_unpoisoned_greeting_refuses_without_opening_conversation() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Cobra greeting PostgreSQL thread panicked")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED Cobra greeting duration={:?}",
        started.elapsed()
    );
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "tombs_cobra", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) = activated_with_loot_tables(world, None);
        let active = controller.active().ok_or("active content")?;
        let deps = Dependencies::with_content(
            ChannelContentPin::from_activation(
                world,
                pin.activation_sequence(),
                pin.identity().server_artifact_digest(),
                pin.identity().client_artifact_digest(),
                pin.frame_binding().digest(),
                pin.qualified_room().map_revision_digest(),
                (2, 0, 7),
            ),
            pin.qualified_room().clone(),
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        deps.initialize(&owner).await;
        let catalogue = NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"),
            npc_catalogue_preproduction_limits(),
        )
        .map_err(debug)?;
        let model = catalogue.service_model_for_project().map_err(debug)?;
        owner
            .states
            .chat
            .install_npc_talk(Arc::new(NpcTalkBook::from_model(&model)));
        let session = deps.admitted.game_session_id;
        owner
            .states
            .chat
            .admit(session, "Fixture Hero", owner.owner_now());
        owner.states.chat.set_npc_service(session);
        let npc = {
            let mut runtime = deps.runtime.lock().await;
            runtime
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            let at = runtime
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position();
            let placed = runtime
                .place_npcs(
                    &NpcPlacementTable {
                        npcs: vec![NpcPlacements {
                            key: "oteryn:npc.cobra".into(),
                            placements: vec![NpcPlacement {
                                x: u16::try_from(at.x + 1)?,
                                y: u16::try_from(at.y)?,
                                floor: -i8::try_from(at.floor)?,
                                direction: MovementFacing::South,
                            }],
                        }],
                    },
                    |_| true,
                    true,
                )
                .map_err(debug)?;
            EntityRef {
                identity: placed.actors[0].placement_identity(),
                generation: placed.actors[0].actor_local_generation(),
            }
        };
        let before = deps
            .runtime
            .lock()
            .await
            .read_actor_position(deps.actor)
            .map_err(debug)?;
        let reply = super::npc_paid_travel::talk(&owner, &deps, npc, 1, "hi").await?;
        let conversation = reply.conversation.ok_or("Cobra refusal")?;
        assert_eq!(conversation.open, None);
        assert_eq!(
            conversation.lines,
            ["The cobra only admits those marked by poison."]
        );
        assert_eq!(owner.states.chat.engaged(session), None);
        assert_eq!(
            deps.runtime
                .lock()
                .await
                .read_actor_position(deps.actor)
                .map_err(debug)?,
            before
        );
        Ok(())
    })
}

#[test]
fn tombs_cobra_poisoned_greeting_commits_progress_and_arrival_once() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("qengine-rewards-ancient-tombs-cobra", || {
        super::quest_world_callbacks::scenario(
            "qengine-rewards-ancient-tombs-cobra",
            oteryn_gameplay_quests::quest::world_callbacks::Position {
                x: 33396,
                y: 32836,
                floor: -14,
            },
            false,
        )
    })
}

pub(super) async fn exercise_poisoned(
    owner: &mut super::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &super::AdmittedSession,
    h: &Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    use crate::domain::condition::DotElement;
    use oteryn_gameplay_foundation::foundation::condition::{
        ApplicationFacts, ConditionDefinition, DamageSchedule, DamageSegment,
    };
    let [cobra] = super::classic_quests::install(owner, admitted, ["oteryn:npc.cobra"]).await?;
    let actor = admitted.runtime_actor.ok_or("Cobra actor")?;
    let session = admitted.game_session_id;
    let poison = ConditionDefinition::new_damage_schedule(
        "tombs.cobra.poison",
        1,
        DotElement::Poison,
        DamageSchedule::Fixed {
            delayed: true,
            segments: vec![DamageSegment {
                count: 100,
                interval_ms: 1000,
                amount: 1,
            }],
        },
    )
    .ok_or("poison condition")?;
    {
        let runtime = owner.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        let root = oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes([91; 32]);
        let facts = ApplicationFacts {
            now: owner.owner_now().get(),
            base_speed: 220,
            mana_shield_capacity: 0,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: true,
            decision_root: &root,
            occurrence: oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes([92; 16]),
        };
        let next = states
            .get(&runtime, actor, session)
            .ok_or("Cobra vitals")?
            .stage_owned_test_conditions(&[poison], &facts)
            .ok_or("stage Cobra poison")?;
        assert!(states.commit(&runtime, actor, session, next));
    }
    let command = 5600;
    super::classic_quests::talk(owner, admitted, cobra, command, "hi").await?;
    let placed = owner.runtime.lock().await.read_actor_position(actor).map_err(debug)?;
    let position = placed.position();
    assert_eq!((position.x, position.y, position.floor), (33396, 32836, 14));
    assert_eq!(owner.states.chat.engaged(session), None);
    assert_eq!(
        super::classic_quests::progress(
            owner, admitted, authority, "oteryn:quest-track/ancient-tombs/thalas-treasure",
        ).await?,
        2
    );
    super::classic_quests::talk(owner, admitted, cobra, command, "hi").await?;
    assert_eq!(owner.runtime.lock().await.read_actor_position(actor).map_err(debug)?, placed);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM game_quest_npc_teleport_receipts WHERE quest_changed",
        ).fetch_one(&h.pool).await?,
        1
    );
    Ok(())
}
