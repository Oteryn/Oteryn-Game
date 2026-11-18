use super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult,
    fixture::{Harness, debug, id},
    prepare_current_session,
};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::foundation::{
    ChannelContentPin, ChannelRuntimeV1, CharacterId, CharacterLease, MovementLocalPosition, NodeId,
};
use crate::gameplay_transport::{
    FreshAdmissionAuthority, SeamRooms, connection::FirstEntryOutcome,
};
use crate::map::view::{MapFacts, placement_key};
use oteryn_gameplay_quests::quest::{
    classic_talk::ROTTIN_WALLCOUNT,
    world_callbacks::{Offering, Position},
};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};

const HAMMER: &str = "oteryn:item.tibia.i3460";
const TARGET: Position = Position {
    x: 32647,
    y: 32216,
    floor: -7,
};
const START: Position = Position {
    x: 32647,
    y: 32217,
    floor: -7,
};

#[test]
fn rottin_hammer_repairs_counts_and_resets_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run_async(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "rottin_walls", false, false).await?;
        super::reset();
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
        let native = generation.native_gameplay().ok_or("wall policies")?;
        let definition = crate::content::ProjectV2DefinitionRef {
            family: crate::content::ProjectV2Family::Item,
            key: HAMMER.into(),
            revision: "definition-r1".into(),
        };
        assert!(
            oteryn_gameplay_items::item_craft::output_definition(native, &definition).is_some()
        );

        let world = world(native)?;
        let entry = activated.qualified_room();
        let cells = world
            .movement_cells(entry.movement_cells())
            .map_err(debug)?;
        let map_revision = world.map_revision_digest();
        let frame_binding = world.frame_binding_digest(deps.admitted.world_id);
        let start = crate::content::NativeEntryStart {
            x: i32::from(START.x),
            y: i32::from(START.y),
            floor: -i16::from(START.floor),
        };
        let room = entry
            .bundle_view(cells.clone(), frame_binding, map_revision, start)
            .ok_or("qualified wall room")?;
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            frame_binding,
            map_revision,
            (start.x, start.y, start.floor),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            NodeId::decode(&id(1)).map_err(debug)?,
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
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: start.x,
                    y: start.y,
                    floor: start.floor,
                },
            )
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);

        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        owner.movement_cells = (&cells).into();
        owner.rooms = SeamRooms::new(Some(&room), Some(&room));
        owner.quest_catalogue = Some(std::sync::Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        ));
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.runtime_actor = Some(deps.actor);
        let fence =
            super::death_ml_loss_composed::current_fence(&owner, admitted.game_session_id).await?;
        admitted.item_fence = Some(fence.item_fence());
        owner
            .runtime
            .lock()
            .await
            .bind_attacker_lease(
                deps.actor,
                admitted.game_session_id,
                CharacterLease::new(
                    CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice()).bind(frame_binding.as_slice()).execute(&h.pool).await?;
        let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
        h.root
            .initialize_admission_equipment(
                &authority,
                &h.node,
                &fence.item_fence(),
                generation.identity().server_artifact_digest(),
                fresh.operation(),
            )
            .await
            .map_err(debug)?;
        let template = crate::durability::starter_grant::StarterTemplate {
            definition: crate::durability::item_mint::TypedDefinitionRef {
                family: "Item".into(),
                production_key: "oteryn:item.tibia.i2854".into(),
                revision_ref: "definition-r1".into(),
            },
            ..crate::durability::starter_grant::StarterTemplate::main()
        };
        h.root
            .grant_starter_items(
                &authority,
                &h.node,
                &fence.item_fence(),
                &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native),
                &template,
            )
            .await
            .map_err(debug)?;
        owner.admit_progression_session(&mut admitted).await;
        deps.initialize(&owner).await;
        let result = exercise(&mut owner, &mut admitted, &h, fence).await;
        drop(owner);
        drop(authority);
        drop(seal);
        result.and(h.cleanup().await)
    })
}

fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::native_world_crop::{ExpectedTile, NativeCrop, native_scene_with_appearances};
    native_scene_with_appearances(
        native,
        vec![NativeCrop {
            bytes: include_bytes!(
                "../../../../../../content/world/placements/region-z07-x127-y125.b3"
            ),
            region: (7, 127, 125),
            bounds: (32646, 32216, 32648, 32218),
            start: (START.x, START.y),
            expected: vec![ExpectedTile {
                position: (TARGET.x, TARGET.y),
                ids: vec![11732, 4144, 12183],
                counted_entry: None,
            }],
        }],
        &[],
        &[6474],
    )
}

async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    fence: CurrentCharacterGameplayFence,
) -> TestResult {
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[4, 6, 11, 15, 18],
        )
        .ok_or("capabilities")?;
    let position = crate::map::overlay::TilePos {
        x: TARGET.x,
        y: TARGET.y,
        floor: TARGET.floor,
    };
    let key = placement_key(position, 2).ok_or("wall key")?;
    let source_id = *owner
        .bundle_map()
        .ok_or("map")?
        .overlay
        .base()
        .tile(position.x, position.y, position.floor)
        .ok_or("wall tile")?
        .ids()
        .get(2)
        .ok_or("wall source")?;
    assert_eq!(
        use_hammer(owner, admitted, key, false).await?,
        UseDisposition::NothingToUse,
        "a hammer definition request must resolve to a carried hammer",
    );
    let state = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(
        state.tracks().get(ROTTIN_WALLCOUNT).copied().unwrap_or(0),
        0
    );
    super::world_input_item::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: TARGET,
            item: HAMMER.into(),
            count: 1,
        },
        951,
        true,
    )
    .await?;
    let carried_hammer = || {
        sqlx::query_scalar::<_, serde_json::Value>(
            "SELECT to_jsonb(i) FROM game_item_instances i WHERE definition_production_key=$1 AND lifecycle=1",
        )
        .bind(HAMMER)
        .fetch_one(&h.pool)
    };
    let original_hammer = carried_hammer().await?;
    for count in 1..=7 {
        // The first repair proves that the definition hotkey now resolves to
        // the carried hammer. Subsequent repairs select its exact item handle.
        let from_handle = count != 1;
        let result = use_hammer(owner, admitted, key, from_handle).await?;
        assert_eq!(
            result,
            if count == 7 {
                UseDisposition::Rejected
            } else {
                UseDisposition::Committed
            },
        );
        let state = owner
            .root
            .read_character_quest_state(owner.character, fence.character_id)
            .await
            .map_err(debug)?;
        assert_eq!(state.tracks().get(ROTTIN_WALLCOUNT), Some(&count.min(6)));
        let map = owner.bundle_map().ok_or("map")?;
        if count == 7 {
            assert_eq!(
                map.facts
                    .base_entry(position, 2, source_id)
                    .ok_or("capped wall")?
                    .appearance_id,
                12183,
                "a capped repair must leave the broken wall unchanged"
            );
            assert_eq!(map.facts.object_revision(key), 12);
            break;
        }
        assert_eq!(
            map.facts
                .base_entry(position, 2, source_id)
                .ok_or("repaired wall")?
                .appearance_id,
            6474
        );
        let now = owner.owner_now().get();
        let due = map.facts.next_movement_decay_time().ok_or("wall reset")?;
        assert!((119_000_000..=120_000_000).contains(&due.saturating_sub(now)));
        if count == 1 {
            assert_eq!(
                use_hammer(owner, admitted, key, true).await?,
                UseDisposition::NothingToUse,
                "a wall must break again before it can earn more credit",
            );
            let state = owner
                .root
                .read_character_quest_state(owner.character, fence.character_id)
                .await
                .map_err(debug)?;
            assert_eq!(state.tracks().get(ROTTIN_WALLCOUNT), Some(&1));
        }
        let map = owner.bundle_map().ok_or("map")?;
        assert_eq!(map.facts.expire_movement_items(due - 1), 0);
        assert_eq!(map.facts.expire_movement_items(due), 1);
        assert_eq!(
            map.facts
                .base_entry(position, 2, source_id)
                .ok_or("restored wall")?
                .appearance_id,
            12183
        );
        assert_eq!(map.facts.object_revision(key), u64::try_from(count)? * 2);
    }
    let hammer: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1")
        .bind(HAMMER)
        .fetch_one(&h.pool).await?;
    assert_eq!(hammer, 1);
    assert_eq!(
        carried_hammer().await?,
        original_hammer,
        "successful and capped repairs must preserve the exact carried tool"
    );
    let tool_reads: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_quest_world_tool_reads WHERE transition_key = $1",
    )
    .bind("oteryn:quest-callback/rottin-repair-42501")
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        tool_reads, 6,
        "each repair must prove custody of its hammer"
    );
    Ok(())
}

async fn use_hammer(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    key: u64,
    from_handle: bool,
) -> TestResult<UseDisposition> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let map = owner.bundle_map().ok_or("map")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let reference = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|n| n.item_index())
        .and_then(|i| i.definition_ref("Item", HAMMER, "definition-r1"))
        .ok_or("hammer reference")?;
    let payload = encode_item_use_intent(&UseIntent {
        // exchange resolves the real inventory snapshot handle when requested.
        target: UseTarget::ItemByDefinition(reference),
        use_with: None,
        use_with_position: None,
        use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
            placement,
            expected_revision: map.facts.object_revision(key),
        })),
    })
    .map_err(debug)?;
    let (next, result, _) = super::item_craft::exchange(
        owner,
        *admitted,
        admitted.continuity.next_command_id,
        &payload,
        from_handle.then_some(reference),
    )
    .await?;
    *admitted = next;
    Ok(result)
}
