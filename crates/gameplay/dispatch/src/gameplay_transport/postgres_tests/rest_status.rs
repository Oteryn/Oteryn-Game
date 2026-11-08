//! GAP-SV7 C10: native owner projects resting and food state from its actual actor.
use super::*;
use crate::foundation::ChannelContentPin;
use crate::map::boot::tests::{floor_items, floor_pins, teleport_bundle};
use oteryn_protocol_oteryn::rest_status::{CAPABILITY_REST_STATUS_V1, RestStatus};

#[test]
fn rest_status_native_owner_projects_food_and_bonuses_and_rejects_stale_session() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "rest status PostgreSQL thread panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "rest_status", false, false).await?;
        eprintln!(
            "POSTGRES_STARTED GAP-SV7 C10 rest_status_native_owner_projects_food_and_bonuses_and_rejects_stale_session"
        );
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let channel = crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?;
        let room = crate::content::qualify_native_entry_room(world).map_err(debug)?;
        let (bytes, pins) = teleport_bundle();
        let map = crate::map::boot::boot(&bytes, &floor_pins(pins), world, channel, floor_items)
            .map_err(debug)?;
        let cells = map.movement_cells(room.movement_cells()).map_err(debug)?;
        for (start, inside) in [((5, 1, 8), true), ((6, 1, 8), false)] {
            let pin = ChannelContentPin::from_activation(
                world,
                1,
                room.compiled().server_digest(),
                room.compiled().client_digest(),
                map.frame_binding_digest(world),
                map.map_revision_digest(),
                start,
            );
            let mut deps = Dependencies::with_content(
                pin,
                room.clone(),
                crate::spell::cast::v1_spell_book().map_err(debug)?,
            )?;
            deps.admitted.continuity.selected_capabilities =
                crate::gameplay_transport::capabilities::SelectedCapabilities::select(
                    &[crate::gameplay_transport::capabilities::OfferedCapability {
                        id: CAPABILITY_REST_STATUS_V1,
                        requires: &[],
                    }],
                    &[CAPABILITY_REST_STATUS_V1],
                )
                .ok_or("capability")?;
            let mut owner = deps.owner(&h, &authority);
            owner.movement_cells = (&cells).into();
            deps.initialize(&owner).await;
            let current =
                crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                    .current_session_at(deps.admitted.game_session_id)
                    .await?
                    .0;
            let mut admitted = deps.admitted;
            admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
            admitted.first_entry =
                crate::gameplay_transport::connection::FirstEntryOutcome::Positioned;
            let fence = admitted.item_fence.ok_or("fence")?;
            deps.runtime
                .lock()
                .await
                .bind_attacker_lease(
                    deps.actor,
                    admitted.game_session_id,
                    crate::foundation::CharacterLease::new(
                        crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                            .map_err(debug)?,
                        fence.character_lease_generation,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
            let (actor, session) = (deps.actor, deps.admitted.game_session_id);
            deps.runtime
                .lock()
                .await
                .initialize_first_entry_position(actor)
                .map_err(debug)?;
            owner.states.resting_areas.install(session, actor, 7);
            let now = owner.owner_now().get();
            {
                let runtime = deps.runtime.lock().await;
                let mut states = owner.states.spell_states.lock().await;
                let state = states.get_mut(&runtime, actor, session).ok_or("actor")?;
                state
                    .apply_owner_premium_transition(true, Some(u64::MAX))
                    .map_err(debug)?;
                *state = state.after_food(180, now / 1_000).ok_or("food")?;
            }
            let command_id = admitted.continuity.next_command_id;
            let (command_status, bytes) = super::cyclopedia::support::exchange(
                &owner,
                &mut admitted,
                command_id,
                oteryn_protocol_oteryn::rest_status::COMMAND_TYPE_REST_STATUS_QUERY,
                &[],
            )
            .await?;
            assert_eq!(
                command_status,
                oteryn_protocol_oteryn::CommandStatus::Accepted
            );
            let status = RestStatus::decode(&bytes).ok_or("decode")?;
            assert_eq!(status.inside_resting_area, inside);
            assert_eq!(status.streak, 7);
            assert_eq!(
                (status.health_multiplier, status.mana_multiplier),
                if inside { (2, 2) } else { (0, 0) }
            );
            assert_eq!((status.stamina, status.soul), (inside, inside));
            assert!((1..=180_000).contains(&status.food_remaining_ms));
            let mut stale = admitted;
            stale.game_session_id =
                crate::foundation::GameSessionId::decode(&id(99)).map_err(debug)?;
            assert!(owner.rest_status_page(stale, 902).await.is_none());
            let mut unselected = admitted;
            unselected.continuity.selected_capabilities =
                crate::gameplay_transport::capabilities::SelectedCapabilities::NONE;
            assert!(owner.rest_status_page(unselected, 903).await.is_none());
        }
        Ok(())
    })
}
