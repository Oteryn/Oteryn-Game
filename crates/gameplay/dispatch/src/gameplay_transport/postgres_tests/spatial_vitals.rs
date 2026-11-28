use super::{
    Dependencies, TestResult,
    fixture::{Harness, debug, fence, run},
};
use crate::foundation::{ChannelContentPin, MovementFacing, MovementLocalPosition, NpcPlacement};
use crate::gameplay_transport::{
    actor_spell::tests::{FACTS, wound},
    capabilities::SelectedCapabilities,
    connection::FreshAdmissionAuthority,
    world_spatial::VisibleKind,
};

#[test]
fn spatial_vitals_project_live_player_and_creature_health_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "spatial vitals test thread")??;
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        let harness = Harness::create(admin, "spatial_vitals", true).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = fence(1)?.runtime_scope.world_id();
        let deps = Dependencies::with_content_and_capacity(
            ChannelContentPin::test(world),
            crate::content::qualify_native_entry_room(world).map_err(debug)?,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
            3,
        )?;
        let owner = deps.owner(&harness, &authority);
        let player = deps.actor;
        let session = deps.admitted.game_session_id;
        deps.initialize(&owner).await;
        let (creature, npc) = {
            let mut runtime = deps.runtime.lock().await;
            runtime
                .initialize_first_entry_position(player)
                .map_err(debug)?;
            let start = runtime
                .read_actor_position(player)
                .map_err(debug)?
                .position();
            let creature = runtime
                .admit_pinned_monster_lab_creature(
                    MovementLocalPosition {
                        x: start.x + 1,
                        ..start
                    },
                    "oteryn:creature.rat",
                    20,
                )
                .map_err(debug)?;
            assert!(runtime.companion_snapshot(creature).is_err());
            let max_health = runtime
                .creature_combat_facts(creature, 0)
                .map_err(debug)?
                .maximum_health;
            runtime
                .commit_monster_lab_damage(creature, b"vis3-spatial-vitals", max_health * 3 / 4)
                .map_err(debug)?;
            let npc = runtime
                .place_dynamic_npc(
                    "oteryn:npc.cipfried",
                    NpcPlacement {
                        x: u16::try_from(start.x + 2)?,
                        y: u16::try_from(start.y)?,
                        floor: i8::try_from(-start.floor)?,
                        direction: MovementFacing::South,
                    },
                )
                .map_err(debug)?;
            (creature, npc)
        };
        {
            let mut states = owner.states.spell_states.lock().await;
            wound(&mut states, player, session, FACTS.max_health * 2 / 5);
        }

        let view = owner
            .observe_projected_entities(player, SelectedCapabilities::NONE)
            .await
            .map_err(debug)?;
        let health = |actor: crate::foundation::ExactActorRef| {
            view.entities
                .iter()
                .find(|entry| entry.identity == actor.placement_identity())
                .map(|entry| (entry.kind, entry.health_percent))
        };
        eprintln!("POSTGRES_STARTED VIS-3 spatial vitals projection");
        assert_eq!(
            [health(player), health(creature), health(npc)],
            [
                Some((VisibleKind::Player, 40)),
                Some((VisibleKind::Creature, 25)),
                Some((VisibleKind::Npc, 100)),
            ]
        );
        Ok(())
    })
}
