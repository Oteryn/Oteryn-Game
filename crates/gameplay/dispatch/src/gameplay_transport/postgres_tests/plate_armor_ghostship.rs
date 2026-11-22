use super::fixture::{Harness, debug, id, run};
use super::npc_paid_travel_fixtures::talk;
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::native_gameplay::{
    NativeGameplayState, test_support::activated_with_loot_tables,
};
use crate::content::{NativeEntryStart, NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::foundation::{
    ChannelContentPin, MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements,
};
use crate::gameplay_transport::connection::{
    FirstEntryOutcome, FreshAdmissionAuthority, UseCommand,
};
use crate::gameplay_transport::{
    ComposedFreshAdmission, newhaven_travel::Departure, npc_talk::NpcTalkBook,
};
use crate::map::overlay::TilePos;
use crate::map::view::MapFacts;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use oteryn_protocol_oteryn::world_spatial::{
    StepDirection, StepDisposition, decode_step_result, encode_step_intent,
};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;

const START: TilePos = TilePos {
    x: 32957,
    y: 32022,
    floor: -6,
};
const GHOSTSHIP: (i32, i32, i16) = (33324, 32173, 6);
const DARASHIA: (i32, i32, i16) = (33289, 32481, 6);

#[test]
fn plate_armor_postgres_fearless_two_confirmations_ghostship_to_coffin() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    execute(true)
}

#[test]
fn plate_armor_postgres_darashia_branch_and_retry_preserve_fare_and_arrival() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    execute(false)
}

fn execute(ghostship: bool) -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(ghostship).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Captain Fearless PostgreSQL scenario panicked")??;
    Ok(())
}

fn scenario(ghostship: bool) -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "plate_armor_ghostship", true, false)
            .await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world_id = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) = activated_with_loot_tables(world_id, None);
        let active = controller.active().ok_or("active content")?;
        let native = active.native_gameplay().ok_or("native gameplay")?;
        let world = source_world(native)?;
        let base = pin.qualified_room().clone();
        let cells = world.movement_cells(base.movement_cells()).map_err(debug)?;
        let start = NativeEntryStart {
            x: START.x.into(),
            y: START.y.into(),
            floor: 6,
        };
        let room = base
            .bundle_view(
                cells.clone(),
                world.frame_binding_digest(world_id),
                world.map_revision_digest(),
                start,
            )
            .ok_or("bundle room")?;
        let digest = pin.identity().server_artifact_digest();
        let mut deps = Dependencies::with_content_and_capacity(
            ChannelContentPin::from_activation(
                world_id,
                pin.activation_sequence(),
                digest,
                pin.identity().client_artifact_digest(),
                world.frame_binding_digest(world_id),
                world.map_revision_digest(),
                (start.x, start.y, start.floor),
            ),
            base,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
            2,
        )?;
        deps.combat_room = Some(room);
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = (&cells).into();
        deps.initialize(&owner).await;
        let catalogue = NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"),
            npc_catalogue_preproduction_limits(),
        )
        .map_err(debug)?;
        owner
            .states
            .chat
            .install_npc_talk(Arc::new(NpcTalkBook::from_model(
                &catalogue.service_model_for_project().map_err(debug)?,
            )));
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
            // Crystal world-npc.xml:705-707.
            let report = runtime
                .place_npcs(
                    &NpcPlacementTable {
                        npcs: vec![NpcPlacements {
                            key: "oteryn:npc.captain_fearless".into(),
                            placements: vec![NpcPlacement {
                                x: 32957,
                                y: 32021,
                                floor: START.floor,
                                direction: MovementFacing::South,
                            }],
                        }],
                    },
                    |_| true,
                    true,
                )
                .map_err(debug)?;
            let actor = report.actors.first().ok_or("Captain Fearless")?;
            EntityRef {
                identity: actor.placement_identity(),
                generation: actor.actor_local_generation(),
            }
        };
        super::item_look_fixtures::seed_main_backpack(&h, &[("oteryn:item.tibia.i3031", 100)])
            .await?;
        assert!(owner.ensure_source_map_initialized().await);

        talk(&owner, &deps, npc, 1, "hi").await?;
        let quote = talk(&owner, &deps, npc, 2, "darashia").await?;
        assert_eq!(
            quote.conversation.ok_or("Darashia quote")?.lines,
            ["Do you seek a passage to Darashia for 60 gold?",]
        );
        let warning = talk(&owner, &deps, npc, 3, "yes").await?;
        assert_eq!(
            warning.conversation.ok_or("Ghostship warning")?.lines,
            ["I warn you! This route is haunted by a ghostship. Do you really want to go there?",]
        );
        assert!(!warning.inventory_changed);
        assert_eq!(position(&deps).await?, (START.x.into(), START.y.into(), 6));
        assert_eq!(travel_state(&h).await?, (100, 0, 0));

        let declined = talk(&owner, &deps, npc, 4, "no").await?;
        assert_eq!(
            declined.conversation.ok_or("declined sailing")?.lines,
            ["We would like to serve you some time.",]
        );
        assert_eq!(travel_state(&h).await?, (100, 0, 0));
        talk(&owner, &deps, npc, 5, "darashia").await?;
        talk(&owner, &deps, npc, 6, "yes").await?;
        let command_id = branch_command(&deps, digest, ghostship)?;

        if ghostship {
            assert!(
                talk(&owner, &deps, npc, command_id, "yes")
                    .await?
                    .inventory_changed
            );
        } else {
            retry_darashia(&h, &owner, &deps, npc, command_id).await?;
        }
        assert_eq!(
            position(&deps).await?,
            if ghostship { GHOSTSHIP } else { DARASHIA }
        );
        let saved: (i32, i32, i16) =
            sqlx::query_as("SELECT tile_x,tile_y,tile_floor FROM game_character_last_positions")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(saved, position(&deps).await?);
        assert_eq!(travel_state(&h).await?, (40, 1, 0));
        let receipt: (String, i64, i64, bool) = sqlx::query_as(
            "SELECT route, fare_gold_units, base_fare_gold_units, arrived_at IS NOT NULL
             FROM game_newhaven_travel_receipts",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(
            receipt,
            (
                "oteryn:service.travel.captain_fearless/darashia".into(),
                60,
                60,
                true
            )
        );
        if ghostship {
            sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
                .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
            sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
                .bind(id(42).as_slice()).bind(id(43).as_slice())
                .bind(digest.as_slice())
                .bind(pin.identity().client_artifact_digest().as_slice())
                .bind(world.frame_binding_digest(world_id).as_slice()).execute(&h.pool).await?;
            reach_coffin(&h, &owner, &deps).await?;
        }
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await?;
        Ok(())
    })
}

// Choose a confirmation command on either side of the occurrence-based draw.
// Uses the base's decision API so these cases can also run on the parent commit.
fn branch_command(deps: &Dependencies, digest: [u8; 32], ghostship: bool) -> TestResult<u64> {
    use oteryn_simulation_determinism::{
        DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
    };

    let root = GameplayDecisionRoot::from_bytes(digest);
    for command in 7..4096 {
        let occurrence = crate::gameplay_transport::npc_trade_command::derived(
            b"oteryn:npc-travel:v1:0",
            deps.admitted.game_session_id,
            command,
        );
        let sample = deterministic_decision_u64(
            &root,
            DecisionOccurrenceId::from_bytes(occurrence),
            "captain-fearless-darashia",
            0,
        )?;
        if sample < u64::MAX - u64::MAX % 10 && (sample.is_multiple_of(10)) == ghostship {
            return Ok(command);
        }
    }
    Err("no confirmation command for the requested branch".into())
}

async fn retry_darashia(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
    npc: EntityRef,
    command_id: u64,
) -> TestResult {
    let session = deps.admitted.game_session_id;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(debug)?
        .ok_or("current session")?;
    let command = UseCommand {
        game_session_id: session,
        command_id,
        item_fence: Some(fence.item_fence()),
    };
    sqlx::raw_sql(
        "CREATE FUNCTION plate_travel_fail() RETURNS trigger LANGUAGE plpgsql AS $$
           BEGIN RAISE EXCEPTION 'injected pending arrival failure'; END; $$;
         CREATE TRIGGER plate_travel_fail BEFORE INSERT ON game_character_pending_arrivals
         FOR EACH ROW EXECUTE FUNCTION plate_travel_fail()",
    )
    .execute(&h.pool)
    .await?;
    let failed = owner
        .npc_talk(
            deps.actor,
            command,
            oteryn_protocol_oteryn::npc_service::NpcTalkIntent {
                npc_actor: npc,
                text: "yes".into(),
            },
        )
        .await;
    assert!(failed.unknown);
    assert_eq!(travel_state(h).await?, (100, 0, 0));
    assert_eq!(position(deps).await?, (START.x.into(), START.y.into(), 6));
    let request = {
        let holds = owner
            .states
            .newhaven_travel
            .holds
            .lock()
            .map_err(|_| "travel holds")?;
        let Departure::Paid(request) = &holds.get(&session).ok_or("retained departure")?.route
        else {
            return Err("expected paid departure".into());
        };
        (**request).clone()
    };
    let selected = request.arrival.destination;
    assert_eq!((selected.x, selected.y, selected.floor), DARASHIA);
    sqlx::raw_sql("DROP TRIGGER plate_travel_fail ON game_character_pending_arrivals; DROP FUNCTION plate_travel_fail()")
        .execute(&h.pool).await?;
    let result = owner
        .resume_npc_bank(deps.actor, command)
        .await
        .ok_or("travel retry")?;
    assert!(!result.unknown && result.inventory_changed);
    let current = owner
        .current_quest_fence(session)
        .await
        .map_err(debug)?
        .ok_or("retry session")?;
    let mut slot = owner.revision_sequencer.acquire(current.character_id).await;
    assert_eq!(
        slot.commit_npc_travel(
            &h.root,
            owner.character,
            &h.node,
            current,
            request.clone(),
            &owner.premium_coordinator,
        )
        .await
        .map_err(debug)?
        .map_err(debug)?,
        None
    );
    let mut different_destination = request;
    different_destination.arrival.destination.x = GHOSTSHIP.0;
    different_destination.arrival.destination.y = GHOSTSHIP.1;
    assert!(matches!(
        slot.commit_npc_travel(
            &h.root, owner.character, &h.node, current, different_destination, &owner.premium_coordinator,
        ).await,
        Err(crate::durability::character_progression::CharacterProgressionError::ConflictingOccurrence)
    ));
    Ok(())
}

async fn reach_coffin(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
) -> TestResult {
    use super::quest_world_callbacks_fixtures::wire;
    use StepDirection::{East, North, South, West};

    let mut admitted = deps.admitted;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("coffin session")?;
    admitted.item_fence = Some(fence.item_fence());
    admitted.first_entry = FirstEntryOutcome::Positioned;
    owner.admit_progression_session(&mut admitted).await;
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[4, 6, 11, 15, 18],
        )
        .ok_or("movement and use capabilities")?;
    // Cross the committed rope bridge, take the aft hatch, then walk below deck.
    for (direction, count) in [
        (South, 8),
        (West, 4),
        (South, 3),
        (West, 1),
        (East, 7),
        (North, 2),
    ] {
        for _ in 0..count {
            let command = admitted.continuity.next_command_id;
            let reply = wire(
                owner,
                &mut admitted,
                command,
                1,
                encode_step_intent(direction).to_vec(),
            )
            .await?;
            assert_eq!(
                decode_step_result(&reply).map_err(debug)?,
                StepDisposition::Moved
            );
        }
    }
    assert_eq!(position(deps).await?, (33326, 32182, 7));
    let map = owner.bundle_map().ok_or("Ghostship map")?;
    let key = oteryn_world_bundle::bundle::placement_key(-7, 33327, 32182, 1)
        .ok_or("Plate Armor coffin placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let target = WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    };
    let command = admitted.continuity.next_command_id;
    let payload =
        oteryn_protocol_oteryn::world_object::encode_use_intent(&target).map_err(debug)?;
    let result = wire(owner, &mut admitted, command, 2, payload).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&result).map_err(debug)?,
        UseDisposition::Committed
    );
    let plate_armor: (String, i64) = sqlx::query_as(
        "SELECT i.definition_production_key,i.quantity FROM game_reward_claim_mint_receipts r
         JOIN game_item_instances i USING(item_instance_id)
         WHERE r.claim_production_key='oteryn:reward-claim.kv.platearmor'",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(plate_armor, ("oteryn:item.tibia.i3357".into(), 1));
    Ok(())
}

async fn position(deps: &Dependencies) -> TestResult<(i32, i32, i16)> {
    let position = deps
        .runtime
        .lock()
        .await
        .read_actor_position(deps.actor)
        .map_err(debug)?
        .position();
    Ok((position.x, position.y, position.floor))
}

async fn travel_state(h: &Harness) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as(
        "SELECT (SELECT COALESCE(sum(i.quantity),0)::bigint FROM game_item_container_entries e
                 JOIN game_item_instances i USING(item_instance_id)
                 WHERE i.definition_production_key='oteryn:item.tibia.i3031'),
                (SELECT count(*) FROM game_newhaven_travel_receipts),
                (SELECT count(*) FROM game_character_pending_arrivals)",
    )
    .fetch_one(&h.pool)
    .await?)
}

fn source_world(native: &NativeGameplayState) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::bundle::{BuildClass, Identity};
    use oteryn_world_bundle_compiler::{compile, project, source};

    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let source = source::project_regions(&root, &[(128, 125), (130, 125), (130, 126)])?;
    let (registry, palette) = source::registry(&root, &source)?;
    let identity: Identity = serde_json::from_slice(&std::fs::read(
        root.join("content/world/pins/oteryn.identity.json"),
    )?)?;
    let mut pins = crate::map::BundlePins {
        digest: [0; 32],
        project_format_version: identity.project_format_version.clone(),
        world_schema_version: identity.world_schema_version.clone(),
        content_revision: identity.content_revision.clone(),
        production: false,
    };
    let compiled = compile::compile(
        &compile::Input {
            regions: &source.regions,
            palette: &palette,
            identity,
            world: source::world_extent(&root)?,
            build_class: BuildClass::NonProduction,
            draft_areas: project::draft_areas(&source.index)?,
            families: &source.families,
        },
        &registry,
    )?;
    pins.digest = compiled.digest;
    let base = crate::map::load(&compiled.bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &compiled.bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: START,
        },
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |key| crate::map::boot::native_item_definition(Some(native), key),
    )
    .map_err(debug)
}
