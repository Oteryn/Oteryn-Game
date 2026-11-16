//! Riding uses the real appearance owner, mounted PostgreSQL fixture and visibility codec.
use super::appearance_change_postgres::observer_visibility_tests;
use super::*;
use crate::content::native_gameplay::test_support::cosmetics_fixture::{
    activated_with_optional_cosmetics, cosmetics_test_packet,
};
use crate::gameplay_transport::{
    appearance_change,
    connection::{FreshAdmissionAuthority, UseCommand},
    world_spatial,
};
use oteryn_protocol_oteryn::{actor_motion, appearance, world_spatial_entities as wire};

#[test]
fn actor_motion_riding_dispatch_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "actor motion test thread")??;
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        let harness = Harness::create_appearance(admin, "actor_motion_riding").await?;
        prepare_current_session(&harness).await?;
        harness.seed_appearance_observer().await?;
        observer_visibility_tests::admit(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let mut packet = cosmetics_test_packet();
        packet["authoring"]["document"]["content_revision"] = "content-1".into();
        let mount_key = packet["authoring"]["document"]["mounts"][0]["key"]
            .as_str()
            .ok_or("mount key")?
            .to_owned();
        packet["authoring"]["document"]["grant_mappings"] = serde_json::json!([
            {"grant_key":"motion-mount", "target":{"kind":"Mount","mount_key":mount_key}, "compatible_provenance":["content-1"]}
        ]);
        let mut promotion = packet["authoring"]["promotions"][0].clone();
        promotion["path"] = "/grant_mappings".into();
        promotion["value"] = packet["authoring"]["document"]["grant_mappings"].clone();
        packet["authoring"]["promotions"]
            .as_array_mut()
            .ok_or("promotions")?
            .push(promotion);
        sqlx::query("INSERT INTO game_account_appearance_unlock_receipts VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'motion-mount','content-1',encode($3,'hex')::uuid,'quest',$1)")
            .bind(id(199).as_slice()).bind(id(40).as_slice()).bind(id(41).as_slice()).execute(&harness.pool).await?;
        sqlx::query("INSERT INTO game_account_appearance_unlocks VALUES(encode($1,'hex')::uuid,'motion-mount',encode($2,'hex')::uuid)")
            .bind(id(40).as_slice()).bind(id(199).as_slice()).execute(&harness.pool).await?;
        let (controller, pin) =
            activated_with_optional_cosmetics(world, None, Some(&serde_json::to_vec(&packet)?));
        let room = pin.qualified_room().clone();
        let (channel_pin, _, _, _) = pin.into_channel_parts();
        let dependencies = Dependencies::with_content(
            channel_pin,
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        let mut owner = dependencies.owner(&harness, &authority);
        owner.active_generation = controller.active();
        owner.rooms = crate::gameplay_transport::SeamRooms::new(
            Some(&dependencies.room),
            Some(&dependencies.room),
        );
        {
            let mut runtime = dependencies.runtime.lock().await;
            runtime
                .initialize_first_entry_position(dependencies.actor)
                .map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    dependencies.actor,
                    dependencies.admitted.game_session_id,
                    crate::foundation::CharacterLease::new(
                        crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                        2,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
        }
        dependencies.initialize(&owner).await;
        let entity = appearance::EntityRef {
            identity: dependencies.actor.placement_identity(),
            generation: dependencies.actor.actor_local_generation(),
        };
        let initial = owner
            .appearance_observation(
                dependencies.actor,
                dependencies.admitted.game_session_id,
                vec![entity],
            )
            .await
            .ok_or("appearance")?;
        let fence = crate::gameplay_transport::character_gameplay_fence::current(
            &owner,
            dependencies.admitted.game_session_id,
            false,
        )
        .await
        .ok_or("fence")?;
        let command = UseCommand {
            game_session_id: dependencies.admitted.game_session_id,
            command_id: 1,
            item_fence: Some(fence.item_fence()),
        };
        assert_eq!(
            appearance_change::dispatch_mount(
                &owner,
                dependencies.actor,
                command,
                &actor_motion::encode_mount(true)
            )
            .await,
            Some(false),
            "selection is required"
        );
        let change = appearance::Change {
            context: initial.catalogue.context,
            expected_root_revision: initial.catalogue.root_revision,
            prior: None,
            selection: appearance::Selection {
                outfit_key: initial
                    .catalogue
                    .outfits
                    .first()
                    .ok_or("outfit")?
                    .key
                    .clone(),
                colours: [0; 4],
                addons: 0,
                mount_key: Some(mount_key.clone()),
            },
            content_revision: "content-1".into(),
        };
        assert_eq!(
            crate::gameplay_transport::connection::appearance_change::dispatch(
                &owner,
                dependencies.actor,
                command,
                &change.encode().map_err(debug)?
            )
            .await,
            Some(appearance::ChangeResult::Committed(2))
        );
        assert_eq!(
            owner
                .current_mount(dependencies.actor, command.game_session_id)
                .await,
            Some(139),
            "legacy mount voices use the validated selected mount before riding"
        );
        let fence = crate::gameplay_transport::character_gameplay_fence::current(
            &owner,
            command.game_session_id,
            false,
        )
        .await
        .ok_or("new fence")?;
        let command = UseCommand {
            command_id: 2,
            item_fence: Some(fence.item_fence()),
            ..command
        };
        for (payload, accepted, expected) in [
            (&[8, 2][..], false, 0),
            (&[8, 1][..], true, 139),
            (&[8, 0][..], true, 0),
        ] {
            assert_eq!(
                appearance_change::dispatch_mount(&owner, dependencies.actor, command, payload)
                    .await,
                Some(accepted)
            );
            let channel = owner
                .observe_visible_entities(dependencies.actor)
                .await
                .ok_or("visibility")?;
            let mut visibility =
                world_spatial::SessionVisibility::with_objects(false).with_motion(true);
            let snapshot = visibility
                .snapshot(&channel, &mut |_| Ok(()))
                .map_err(debug)?;
            let (_, bytes) = world_spatial::encode_visibility_snapshot_lit(
                &[6, 13, 25, 502],
                &snapshot,
                &wire::EntityLights::new(),
            )
            .map_err(debug)?;
            let (decoded, _) = wire::decode_world_spatial_entities_snapshot_with_fields(
                &bytes,
                wire::EntityFields::selected(&[6, 13, 25, 502]),
            )
            .map_err(debug)?;
            assert_eq!(
                decoded.entities[0].motion.ok_or("motion")?.mount_look_type,
                expected
            );
            let observed = owner
                .appearance_observation(dependencies.actor, command.game_session_id, vec![entity])
                .await
                .ok_or("selected appearance")?;
            assert_eq!(
                observed
                    .catalogue
                    .selected
                    .ok_or("selection retained")?
                    .selection
                    .mount_key
                    .as_deref(),
                Some(mount_key.as_str())
            );
        }
        // A committed selection must retain its riding reconciliation while the
        // durable holder is busy; no runtime guard may be held across the wait.
        {
            use std::{future::Future, task::Poll};
            dependencies
                .runtime
                .lock()
                .await
                .set_riding(dependencies.actor, command.game_session_id, 140, 99)
                .map_err(debug)?;
            let held = harness.root.try_issue_semantic_pass()?;
            let mut refresh = std::pin::pin!(owner.refresh_riding(dependencies.actor, command));
            let first = std::future::poll_fn(|cx| Poll::Ready(refresh.as_mut().poll(cx))).await;
            let runtime_free = owner.runtime.try_lock().is_ok();
            held.run(|_, _| Box::pin(async { Ok(()) })).await?;
            assert!(
                first.is_pending(),
                "riding reconciliation must wait for the busy holder"
            );
            assert!(runtime_free, "riding retry releases the runtime guard");
            refresh
                .await
                .ok_or("riding reconciled after holder return")?;
            let motion = owner
                .runtime
                .lock()
                .await
                .actor_motion(dependencies.actor)
                .ok_or("reconciled motion")?;
            let (_, content) = owner.appearance_context().ok_or("cosmetics")?;
            assert_eq!(
                (motion.2, motion.3),
                content.mount_metadata(&mount_key).ok_or("selected mount")?,
                "retry installs the selected mount and replaces the old speed bonus"
            );
        }
        assert_eq!(
            owner.set_mount(dependencies.actor, command, false).await,
            Some(true)
        );
        let invalid = UseCommand {
            item_fence: None,
            ..command
        };
        assert_eq!(
            appearance_change::dispatch_mount(
                &owner,
                dependencies.actor,
                invalid,
                &actor_motion::encode_mount(true)
            )
            .await,
            Some(false)
        );
        real_connection(&owner, &dependencies, entity).await?;
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await?;
        eprintln!("ACTOR_MOTION_POSTGRES_CASE_COMPLETED");
        Ok(())
    })
}

// The admission/lease are the real fixture's committed facts. Only the already
// completed bootstrap response is written here; every game command, snapshot,
// delta, client reducer and UI projection below is the production implementation.
async fn real_connection(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    dependencies: &Dependencies,
    entity: wire::EntityRef,
) -> TestResult {
    use crate::gameplay_transport::{
        capabilities::{OfferedCapability, SelectedCapabilities},
        connection::{ConnectionEnd, FirstEntryOutcome, IDLE_LIVENESS, serve_admitted},
        tcp_tls::{read_frame, write_frame},
    };
    use oteryn_dev_client::{client_ui::actor_motion as ui, native_session as client};
    use oteryn_protocol_oteryn::{
        CommandStatus, MessageType, ServerAcceptedValue, decode_wire_envelope,
        encode_server_accepted,
    };
    let mut admitted = dependencies.admitted;
    let (current, _) =
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(owner.root.clone())
            .current_session_at(admitted.game_session_id)
            .await?;
    admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
    admitted.controller.as_mut().ok_or("controller")?.transport =
        current.current_transport().ok_or("transport")?;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        &[
            OfferedCapability {
                id: 6,
                requires: &[],
            },
            OfferedCapability {
                id: 13,
                requires: &[],
            },
            OfferedCapability {
                id: 25,
                requires: &[6],
            },
            OfferedCapability {
                id: 286,
                requires: &[6],
            },
            OfferedCapability {
                id: 502,
                requires: &[6, 13, 25],
            },
        ],
        &[6, 13, 25, 286, 502],
    )
    .ok_or("motion capability selection")?;
    let (mut server, stream) = tokio::io::duplex(256 * 1024);
    let serving = async {
        let bytes = read_frame(&mut server).await?;
        assert_eq!(
            decode_wire_envelope(&bytes).map_err(debug)?.message_type(),
            MessageType::ClientBootstrap
        );
        let accepted = encode_server_accepted(&ServerAcceptedValue {
            game_session_id: admitted.game_session_id,
            world_id: admitted.world_id,
            channel_id: admitted.channel_id,
            connection_generation: admitted.continuity.connection_generation,
            current_server_sequence: admitted.continuity.server_sequence,
            next_command_id: admitted.continuity.next_command_id,
            schema_revision: 1,
            selected_capabilities: admitted.continuity.selected_capabilities.as_slice(),
        })
        .map_err(debug)?;
        write_frame(&mut server, &accepted).await?;
        Ok::<_, Box<dyn std::error::Error>>(
            serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await,
        )
    };
    let driving = async {
        let mut session = client::Session::admit(
            stream,
            client::Admission {
                appearance_items: None,
                schema_revision: 1,
                character_id: oteryn_protocol_oteryn::CharacterId::decode(&id(41))
                    .map_err(debug)?,
                admission_material: b"committed fixture admission",
                client_build_id: "actor-motion-e2e",
                supported_capabilities: client::CLIENT_SUPPORTED_CAPABILITIES,
                deadline: Duration::from_secs(10),
            },
        )
        .await?;
        assert_eq!(
            session
                .world_entities()
                .and_then(|entities| entities.get(&entity))
                .and_then(|own| own.motion)
                .ok_or("joined motion")?
                .mount_look_type,
            0
        );
        let command = ui::mount_command(session.is_selected(502), true).ok_or("UI mount")?;
        let result = session
            .gated_command(command.capability, command.kind, &command.payload)
            .await?;
        assert_eq!(result.status, CommandStatus::Accepted);
        assert!(result.payload.is_empty());
        let own = loop {
            let own = *session
                .world_entities()
                .and_then(|entities| entities.get(&entity))
                .ok_or("own visibility")?;
            if own
                .motion
                .is_some_and(|motion| motion.mount_look_type == 139)
            {
                break own;
            }
            session.service_liveness(Duration::from_millis(20)).await?;
        };
        let (destination, forecast) =
            ui::forecast(&own, ui::StepDir::East).ok_or("UI known-walkable forecast")?;
        let step = session
            .step(oteryn_protocol_oteryn::world_spatial::StepDirection::East)
            .await?;
        assert_eq!(
            step.disposition,
            oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved
        );
        assert_eq!(step.step_wait_ms, forecast);
        let after = session
            .world_entities()
            .and_then(|entities| entities.get(&entity))
            .ok_or("stepped entity")?;
        assert_eq!([after.position.x, after.position.y], destination);
        let motion = after.motion.ok_or("stepped motion")?;
        assert_eq!(motion.step_duration_ms, forecast);
        assert_eq!(motion.mount_look_type, 139);
        assert_eq!(ui::walking_phase(2, u64::from(forecast) / 2, forecast), 1);
        let command = ui::mount_command(session.is_selected(502), false).ok_or("UI dismount")?;
        assert_eq!(
            session
                .gated_command(command.capability, command.kind, &command.payload)
                .await?
                .status,
            CommandStatus::Accepted
        );
        loop {
            if session
                .world_entities()
                .and_then(|entities| entities.get(&entity))
                .and_then(|own| own.motion)
                .is_some_and(|motion| motion.mount_look_type == 0)
            {
                break;
            }
            session.service_liveness(Duration::from_millis(20)).await?;
        }
        drop(session);
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let (end, ()) = tokio::time::timeout(Duration::from_secs(30), async {
        tokio::try_join!(serving, driving)
    })
    .await
    .map_err(|_| "actor motion real connection timed out")??;
    assert!(
        matches!(
            end,
            ConnectionEnd::AdmittedThenDisconnected(_) | ConnectionEnd::AdmittedThenControlLost(_)
        ),
        "{end:?}"
    );
    Ok(())
}
