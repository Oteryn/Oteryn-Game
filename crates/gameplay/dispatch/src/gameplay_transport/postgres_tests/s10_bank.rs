//! Source-bound bank population through NPC_TALK and the fenced bank writer.
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
fn s10_eva_postgres_bank_deposit_and_withdraw() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "S10 bank PostgreSQL thread panicked")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED S10 eva_bank duration={:?}",
        started.elapsed()
    );
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "s10_eva_bank", true, false).await?;
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
                            key: "oteryn:npc.eva".into(),
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
        super::item_look::seed_main_backpack(&h, &[("oteryn:item.tibia.i3031", 100)]).await?;
        for (command, text) in [(1, "hi"), (2, "deposit 20 gold"), (3, "yes")] {
            super::npc_paid_travel::talk(&owner, &deps, npc, command, text).await?;
        }
        let balance = || async {
            sqlx::query_scalar::<_, i64>(
                "SELECT COALESCE(sum(balance),0)::bigint FROM game_account_bank_balances",
            )
            .fetch_one(&h.pool)
            .await
        };
        assert_eq!(balance().await?, 20);
        owner.clock_origin -= std::time::Duration::from_secs(1);
        for (command, text) in [(4, "withdraw 5 gold"), (5, "yes")] {
            super::npc_paid_travel::talk(&owner, &deps, npc, command, text).await?;
        }
        assert_eq!(balance().await?, 15);
        let entries: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_bank_entries")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(entries, 2);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
