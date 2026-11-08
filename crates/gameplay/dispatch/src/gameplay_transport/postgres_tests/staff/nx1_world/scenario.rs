use super::*;

pub(super) fn run() -> TestResult {
    super::run(async |admin| {
        let signing = Signing::new()?;
        let h = Harness::create_with_admission_seed(admin, "nx1_staff_world", true, false).await?;
        prepare_current_session(&h).await?;
        h.seed_pvp_target().await?;
        observer::admit(&h).await?;
        signing.bootstrap(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, pin) =
            super::super::super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let active = controller.active().ok_or("generation")?;
        deps.room = pin.qualified_room().clone();
        let (bytes, pins) = super::super::super::creature_push::bundle(None)?;
        let base = crate::map::load(&bytes, &pins).map_err(debug)?;
        let world = boot(
            &bytes,
            &BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: pins,
                start: TilePos {
                    x: 2,
                    y: 2,
                    floor: -7,
                },
            },
            deps.admitted.world_id,
            deps.admitted.channel_id,
            |key| {
                Some(crate::map::facts::ItemDefinition {
                    reference: std::num::NonZeroU32::new(match key {
                        "oteryn:item.tibia.i3944" => 4,
                        "oteryn:item.tibia.i3945" => 5,
                        "oteryn:item.tibia.i12368" => 6,
                        "oteryn:item.tibia.i12369" => 7,
                        _ => return None,
                    })?,
                    solid: Some(false),
                    blocks_projectile: false,
                    pickupable: false,
                    door: false,
                })
            },
        )
        .map_err(debug)?;
        let cells = world
            .movement_cells(deps.room.movement_cells())
            .map_err(debug)?;
        let content = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            active.identity().server_artifact_digest(),
            active.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            world.map_revision_digest(),
            (2, 2, 7),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            3,
            content,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = (&cells).into();
        deps.initialize(&owner).await;
        bind_tools_actor(&owner,deps.actor,deps.admitted.game_session_id).await?;
        let target = observer::initialize(&deps, &owner).await?;
        bind_tools_actor(&owner,target,GameSessionId::decode(&id(110)).map_err(debug)?).await?;
        {
            let mut runtime = owner.runtime.lock().await;
            for x in [3, 4] {
                let at = runtime.read_actor_position(target).map_err(debug)?;
                runtime
                    .borrow_movement_position()
                    .commit_cardinal(at, MovementLocalPosition { x, y: 2, floor: 7 })
                    .map_err(debug)?;
            }
        }
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[6, 51, 330])
                .ok_or("tools selection")?;
        admitted.item_fence = Some(
            owner
                .current_quest_fence(admitted.game_session_id)
                .await
                .map_err(|_| "fence")?
                .ok_or("fence")?
                .item_fence(),
        );
        let mut chain = Chain {
            signing: &signing,
            seq: 1,
            previous: signing.config.newest_activation_sha256.clone(),
            operation: 120,
        };
        let (inspect, reply) = chain
            .action(
                &owner,
                admitted,
                rules::Action::Inspect {
                    character_id: hex(&id(41)),
                },
            )
            .await?;
        assert_eq!(reply.result.code, wire::StaffCode::Ok);
        assert!(reply.info.contains("Character"));
        assert!(reply.info.to_ascii_lowercase().contains("position"));
        assert_eq!(
            send_tools(&owner, admitted, &inspect, false).await?.result,
            reply.result
        );
        let (teleport, reply) = chain
            .action(
                &owner,
                admitted,
                rules::Action::Teleport {
                    character_id: hex(&id(41)),
                    x: 3,
                    y: 2,
                    floor: 7,
                },
            )
            .await?;
        assert_eq!(reply.result.code, wire::StaffCode::Ok);
        applied(&owner, admitted, &teleport).await?;
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position(),
            MovementLocalPosition {
                x: 3,
                y: 2,
                floor: 7
            }
        );
        let (goto, _) = chain
            .action(
                &owner,
                admitted,
                rules::Action::TeleportToPlayer {
                    character_id: hex(&id(41)),
                    destination_character_id: hex(&id(101)),
                },
            )
            .await?;
        applied(&owner, admitted, &goto).await?;
        let (bring, _) = chain
            .action(
                &owner,
                admitted,
                rules::Action::Bring {
                    character_id: hex(&id(101)),
                },
            )
            .await?;
        applied(&owner, admitted, &bring).await?;
        for enabled in [true, false] {
            let (ghost, _) = chain
                .action(
                    &owner,
                    admitted,
                    rules::Action::Ghost {
                        character_id: hex(&id(41)),
                        enabled,
                    },
                )
                .await?;
            applied(&owner, admitted, &ghost).await?;
            let seen = owner
                .observe_visible_entities(target)
                .await
                .ok_or("observer view")?;
            assert_eq!(
                seen.entities
                    .iter()
                    .any(|e| e.identity == deps.actor.placement_identity()),
                !enabled
            );
            assert!(
                seen.entities
                    .iter()
                    .any(|e| e.identity == target.placement_identity())
            );
            assert!(
                owner
                    .observe_visible_entities(deps.actor)
                    .await
                    .ok_or("self view")?
                    .entities
                    .iter()
                    .any(|e| e.identity == deps.actor.placement_identity())
            );
        }
        let (invisible, _) = chain
            .action(
                &owner,
                admitted,
                rules::Action::Invisible {
                    character_id: hex(&id(41)),
                    enabled: true,
                },
            )
            .await?;
        applied(&owner, admitted, &invisible).await?;
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|_| "fence")?
            .ok_or("fence")?
            .item_fence();
        assert!(
            owner
                .root
                .staff_live_status(&authority, owner.holder, fence)
                .await?
                .invisible
        );
        let victim = GameSessionId::decode(&id(110)).map_err(debug)?;
        assert!(
            owner
                .states
                .server_messages
                .join(admitted.game_session_id, 1)
        );
        assert!(owner.states.server_messages.join(victim, 1));
        let (notice, _) = chain
            .action(
                &owner,
                admitted,
                rules::Action::Notify {
                    character_id: hex(&id(101)),
                    text: "Staff notice".into(),
                },
            )
            .await?;
        applied(&owner, admitted, &notice).await?;
        assert!(
            owner
                .states
                .server_messages
                .take(admitted.game_session_id, 1)
                .is_none()
        );
        assert!(
            matches!(owner.states.server_messages.take(victim,1),Some(protocol::zones_nologout::ServerMessage::StatusText(text)|protocol::zones_nologout::ServerMessage::Warning(text)|protocol::zones_nologout::ServerMessage::Advance(text)) if text.contains("Staff notice"))
        );
        let (broadcast, _) = chain
            .action(
                &owner,
                admitted,
                rules::Action::Broadcast {
                    character_id: hex(&id(41)),
                    text: "World staff announcement".into(),
                },
            )
            .await?;
        applied(&owner, admitted, &broadcast).await?;
        assert!(
            owner
                .states
                .server_messages
                .take(admitted.game_session_id, 1)
                .is_some()
        );
        assert!(owner.states.server_messages.take(victim, 1).is_some());
        send_tools(&owner, admitted, &broadcast, false).await?;
        assert!(
            owner.states.server_messages.take(victim, 1).is_none(),
            "replay must not repeat announcement"
        );
        let (blocked, _) = chain
            .action(
                &owner,
                admitted,
                rules::Action::Teleport {
                    character_id: hex(&id(41)),
                    x: 5,
                    y: 2,
                    floor: 7,
                },
            )
            .await?;
        let blocked_request: rules::Request = serde_json::from_slice(&blocked.canonical)?;
        assert_eq!(
            owner
                .root
                .staff_world_effect_state(
                    &authority,
                    owner.holder,
                    fence,
                    &blocked_request.operation_id
                )
                .await?,
            Some(crate::durability::staff::StaffWorldEffectState::Refused)
        );
        send_tools(&owner, admitted, &blocked, false).await?;
        assert_ne!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position(),
            MovementLocalPosition {
                x: 5,
                y: 2,
                floor: 7
            }
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
