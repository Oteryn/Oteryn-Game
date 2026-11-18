//! Thieves Guild reward claims through literal USE on the committed map.
use super::fixture::{Harness, debug, id, run};
use super::quest_callbacks::wire_use_targets_retained;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

use crate::map::overlay::TilePos;
fn definition(key: &str) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: "Item".into(),
        production_key: key.into(),
        revision_ref: "definition-r1".into(),
    }
}


// Wiki base (curid 25436, oldid 1090720) and /Spoiler (curid 25437, oldid 1086330).
// Crystal 201189f8, data-global/startup/tables/chest.lua:2630-2670.
#[test]
fn source_chest_selectors_reach_durable_reward_dispatch() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("qcx-thieves-guild", || {
        for (x, y, floor, reward) in [
            (32309, 32209, -8, "oteryn:item.tibia.i8021"),
            (32309, 32211, -8, "oteryn:item.tibia.i7404"),
            (32309, 32213, -8, "oteryn:item.tibia.i8073"),
            (32367, 31781, -8, "oteryn:item.tibia.i8117"),
            (32551, 32652, -10, "oteryn:item.tibia.i7870"),
            (32902, 32143, -4, "oteryn:item.tibia.i7935"),
        ] {
            let started = std::time::Instant::now();
            let result = claim(x, y, floor, reward);
            eprintln!("POSTGRES_CASE_COMPLETED qcx_thieves_guild {reward} duration={:?} success={}", started.elapsed(), result.is_ok());
            result?;
        }
        Ok(())
    });
    eprintln!("POSTGRES_CASE_COMPLETED qcx_thieves_guild duration={:?} success={}", started.elapsed(), result.is_ok());
    result
}

fn claim(x: u16, y: u16, floor: i8, reward: &str) -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "qcx_thieves_guild", false, false)
            .await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activated) =
            super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("generation")?;
        let native = generation.native_gameplay().ok_or("native")?;
        let start = TilePos { x: x - 1, y, floor };
        let world = super::source_world_crop::world(start, TilePos { x, y, floor }, start, native)?;
        let cells = world
            .movement_cells(activated.qualified_room().movement_cells())
            .map_err(debug)?;
        let at = MovementLocalPosition {
            x: i32::from(x) - 1,
            y: i32::from(y),
            floor: -i16::from(floor),
        };
        deps.combat_room = Some(
            activated
                .qualified_room()
                .bundle_view(
                    cells.clone(),
                    activated.frame_binding().digest(),
                    world.map_revision_digest(),
                    crate::content::NativeEntryStart {
                        x: at.x,
                        y: at.y,
                        floor: at.floor,
                    },
                )
                .ok_or("qualified source reward Bundle room")?,
        );
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated.frame_binding().digest(),
            world.map_revision_digest(),
            (at.x, at.y, at.floor),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(deps.actor, at)
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?
                .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("fence")?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice())
            .bind(activated.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
        h.root
            .initialize_admission_equipment(
                &authority,
                &h.node,
                &fence,
                generation.identity().server_artifact_digest(),
                fresh.operation(),
            )
            .await
            .map_err(debug)?;
        h.root
            .grant_starter_items(
                &authority,
                &h.node,
                &fence,
                &NativeItemWeights(native),
                &crate::durability::starter_grant::StarterTemplate {
                    definition: definition("oteryn:item.tibia.i2854"),
                    ..crate::durability::starter_grant::StarterTemplate::main()
                },
            )
            .await
            .map_err(debug)?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        let key = oteryn_world_bundle::bundle::placement_key(floor, x, y, 1)
            .ok_or("reward placement")?;
        let mut placement = world.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let target = WorldObjectTarget { placement, expected_revision: 0 };
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        owner.movement_cells = (&cells).into();
        owner.admit_progression_session(&mut admitted).await;
        deps.initialize(&owner).await;
        assert_eq!(
            wire_use_targets_retained(&owner, &mut admitted, &[(1, &target)]).await?,
            [(CommandStatus::Accepted, UseDisposition::Committed)],
            "source USE at ({x}, {y}, {floor})"
        );
        let granted: i64 = sqlx::query_scalar("SELECT coalesce(sum(quantity), 0)::bigint FROM game_item_instances WHERE lifecycle = 1 AND definition_production_key = $1")
            .bind(reward)
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(granted, 1);
        Ok(())
    })
}
