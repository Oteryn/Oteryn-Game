//! Pema healing through the live NPC conversation owner.
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
fn pema_healing_keyword_in_conversation_refills_injured_player() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run("pema-healing", scenario)
}

fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "pema_healing", true, false).await?;
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
                            key: "oteryn:npc.enpa_deia_pema".into(),
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
        super::npc_paid_travel_fixtures::talk(&owner, &deps, npc, 1, "hi").await?;
        {
            let mut states = owner.states.spell_states.lock().await;
            crate::gameplay_transport::actor_spell::tests::wound(
                &mut states,
                deps.actor,
                session,
                10,
            );
        }
        owner.clock_origin -= std::time::Duration::from_secs(1);
        let healed =
            super::npc_paid_travel_fixtures::talk(&owner, &deps, npc, 2, "healing please").await?;
        assert!(!healed.unknown);
        {
            let runtime = deps.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            let vitals = states
                .get(&runtime, deps.actor, session)
                .ok_or("vitals")?
                .vitals();
            assert_eq!(vitals.health, vitals.max_health);
        }
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
