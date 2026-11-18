//! Mounted dispatch fixtures for authored quest item-on-target operations.
use super::super::fixture::{Harness, debug, id, run};
use super::super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session,
};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1};
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use crate::map::{
    boot::{BundleMap, BundleWorld},
    view::MapFacts,
};
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue, QuestTrack,
    authored_bindings::{Bindings, BoundQuest, Stage, Track, Trigger},
    world_callbacks::{self, Callback, Offering, Position},
};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};
use std::num::NonZeroU64;

pub(in super::super) fn run_world(
    admin: String,
    name: &'static str,
    world: BundleWorld,
    callback: Callback,
    tracks: Vec<QuestTrack>,
    exercise: impl for<'o, 'a, 'f, 's> AsyncFnOnce(
        &'o ComposedFreshAdmission<'a, 'f, 's>,
        AdmittedSession,
        &'o Harness,
        CurrentCharacterGameplayFence,
    ) -> TestResult,
) -> TestResult {
    let trigger = callback
        .selector
        .native
        .as_ref()
        .ok_or("fixture native selector")?
        .position;
    run(async move |_| {
        let h = Harness::create_with_admission_seed(admin, name, false, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activated) =
            super::super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("generation")?;
        let cells = world
            .movement_cells(activated.qualified_room().movement_cells())
            .map_err(debug)?;
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated.frame_binding().digest(),
            world.map_revision_digest(),
            (
                i32::from(trigger.x) - 1,
                i32::from(trigger.y),
                -i16::from(trigger.floor),
            ),
        );
        let room = activated
            .qualified_room()
            .bundle_view(
                cells.clone(),
                activated.frame_binding().digest(),
                world.map_revision_digest(),
                crate::content::NativeEntryStart {
                    x: i32::from(trigger.x) - 1,
                    y: i32::from(trigger.y),
                    floor: -i16::from(trigger.floor),
                },
            )
            .ok_or("mosaic bundle room")?;
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            8,
            pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        let origin = super::local(Position {
            x: trigger.x - 1,
            ..trigger
        });
        runtime
            .initialize_pinned_test_position(deps.actor, origin)
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        owner.movement_cells = (&cells).into();
        owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&room));
        let quest = callback.quest.clone();
        let completes = callback.completes;
        let trigger = match callback.trigger {
            world_callbacks::Trigger::Use => Trigger::MapUse {
                callback: Box::new(callback),
            },
            world_callbacks::Trigger::StepIn => Trigger::StepIn {
                callback: Box::new(callback),
            },
            _ => return Err("unsupported fixture trigger".into()),
        };
        let base = QuestStateCatalogue::empty("content-1").ok_or("fixture base catalogue")?;
        let bindings = Bindings {
            schema: 1,
            quests: vec![BoundQuest {
                quest,
                recipe_sha256: "1".repeat(64),
                occurrence: None,
                admission: None,
                tracks: tracks
                    .into_iter()
                    .map(|track| Track {
                        key: track.key,
                        initial: track.initial,
                        minimum: track.min,
                        maximum: track.max,
                    })
                    .collect(),
                stages: vec![Stage {
                    key: "source".into(),
                    completes,
                    trigger,
                }],
                sources: vec!["PostgreSQL dispatch fixture".into()],
            }],
        };
        owner.quest_catalogue = Some(std::sync::Arc::new(bindings.attach(&base)?));
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        admitted.item_fence = Some(fence.item_fence());
        owner
            .runtime
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
        sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice()).bind(activated.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        let fresh = super::super::fresh_fixture::fresh::Source::new(0)?.request()?;
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
        let native = generation.native_gameplay().ok_or("native")?;
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
        owner.admit_quest_session(&admitted).await;

        exercise(&owner, admitted, &h, fence).await?;
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

pub(in super::super) fn placement(
    map: &BundleMap,
    at: Position,
    ordinal: u8,
) -> TestResult<(u64, WorldObjectTarget)> {
    let key = crate::map::view::placement_key(
        crate::map::overlay::TilePos {
            x: at.x,
            y: at.y,
            floor: at.floor,
        },
        ordinal,
    )
    .ok_or("placement key")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok((
        key,
        WorldObjectTarget {
            placement,
            expected_revision: map.facts.object_revision(key),
        },
    ))
}

pub(in super::super) async fn use_tool(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    fence: CurrentCharacterGameplayFence,
    tool: &str,
    at: Position,
    ordinal: u8,
) -> TestResult<(AdmittedSession, UseDisposition)> {
    super::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: at,
            item: tool.into(),
            count: 1,
        },
        3302,
        true,
    )
    .await?;
    let mut admitted = admitted;
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[4, 6, 11, 15, 18],
        )
        .ok_or("capabilities")?;
    let map = owner.bundle_map().ok_or("bundle map")?;
    let (_, target) = placement(map, at, ordinal)?;
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(NonZeroU64::new(3).ok_or("tool handle")?),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(target)),
        use_with_position: None,
    })
    .map_err(debug)?;
    let item = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|native| native.item_index())
        .and_then(|index| index.definition_ref("Item", tool, "definition-r1"))
        .ok_or("mirror definition")?;
    let (next, result, _) = super::super::item_craft::exchange(
        owner,
        admitted,
        admitted.continuity.next_command_id,
        &payload,
        Some(item),
    )
    .await?;
    Ok((next, result))
}
