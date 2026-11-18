//! The selected base object is removed and restored by its Channel decay deadline.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue, QuestTrack,
    world_callbacks::{self, Callback, Offering, Position},
};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};
use std::num::NonZeroU64;

const TRACK: &str = "oteryn:quest-progress/quest/u8_1/to_blind_the_enemy/questline";
const QUEST: &str = "oteryn:quest.to_blind_the_enemy_quest";
const MIRROR: &str = "oteryn:item.tibia.i3463";

#[test]
fn scrying_ball_use_consumes_mirror_and_restores_exact_shared_object_after_sixty_seconds()
-> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("scrying-ball", move || scenario(admin))
}

fn scenario(admin: String) -> TestResult {
    // See Crystal 201189f8 data-global/scripts/quests/to_blind_the_enemy_quest/mirror.lua:10-21.
    let trigger = Position {
        x: 32588,
        y: 31644,
        floor: -2,
    };
    let decoy = Position {
        x: trigger.x,
        y: trigger.y + 1,
        ..trigger
    };
    let callback: Callback = serde_json::from_value(serde_json::json!({
        "key": "oteryn:quest-callback/fixture/scrying-ball",
        "quest": QUEST,
        "trigger": "use",
        "selector": {
            "position": trigger, "action": 4001, "appearance": 132,
            "native": { "position": trigger, "appearance": 132, "ordinal": 1 }
        },
        "requires_world_admission": true,
        "reversible": false,
        "passages": [],
        "offerings": [],
        "return_on_failure": false,
        "storage": [{ "track": TRACK, "minimum": 1, "maximum": 1, "set": 2 }],
        "item_on_target": {
            "item": MIRROR, "count": 1, "target_remove": true, "selected_instance": true
        },
        "effects": { "reset_after_ms": 60000 },
        "sources": ["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/to_blind_the_enemy_quest/mirror.lua:10-21"]
    }))?;
    let world =
        super::quest_world_callbacks::world_with_objects(&callback, trigger, &[(decoy, 132)], &[])?;
    run(async move |_| {
        let h = Harness::create_with_admission_seed(admin, "scrying_ball", false, false).await?;
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
        let origin = super::quest_world_callbacks::local(Position {
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
        owner.quest_catalogue = Some(std::sync::Arc::new(
            QuestStateCatalogue::new(
                "content-1",
                vec![QuestTrack {
                    key: TRACK.into(),
                    quest: QUEST.into(),
                    initial: 1,
                    min: 0,
                    max: 2,
                }],
                vec![world_callbacks::primary_transition(&callback)?],
            )
            .map_err(debug)?,
        ));
        owner
            .states
            .quest_callbacks
            .fixture
            .set(vec![callback])
            .map_err(|_| "callback fixture already installed")?;
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

        super::quest_world_callbacks::input_item(
            &owner,
            fence.item_fence(),
            &Offering {
                position: trigger,
                item: MIRROR.into(),
                count: 1,
            },
            3302,
            true,
        )
        .await?;
        admitted.continuity.selected_capabilities =
            crate::gameplay_transport::capabilities::SelectedCapabilities::select(
                owner.offered_capabilities(),
                &[4, 6, 11, 15, 18],
            )
            .ok_or("capabilities")?;
        let map = owner.bundle_map().ok_or("bundle map")?;
        let key = crate::map::view::placement_key(
            crate::map::overlay::TilePos {
                x: trigger.x,
                y: trigger.y,
                floor: trigger.floor,
            },
            1,
        )
        .ok_or("ball key")?;
        let decoy_key = crate::map::view::placement_key(
            crate::map::overlay::TilePos {
                x: decoy.x,
                y: decoy.y,
                floor: decoy.floor,
            },
            1,
        )
        .ok_or("decoy key")?;
        let mut placement = map.overlay.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let payload = encode_item_use_intent(&UseIntent {
            target: UseTarget::Item(NonZeroU64::new(3).ok_or("tool handle")?),
            use_with: None,
            use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
                placement,
                expected_revision: map.facts.object_revision(key),
            })),
            use_with_position: None,
        })
        .map_err(debug)?;
        let item = native
            .item_index()
            .and_then(|index| index.definition_ref("Item", MIRROR, "definition-r1"))
            .ok_or("mirror definition")?;
        let clock = owner.freeze_owner_clock_for_test();
        owner
            .states
            .test_owner_time
            .store(2_000_000, std::sync::atomic::Ordering::Relaxed);
        let (_, result, _) = super::item_craft::exchange(
            &owner,
            admitted,
            admitted.continuity.next_command_id,
            &payload,
            Some(item),
        )
        .await?;
        assert_eq!(result, UseDisposition::Committed);
        assert!(map.facts.base_hidden(key));
        assert!(!map.facts.base_hidden(decoy_key));
        assert_eq!(map.facts.object_revision(key), 1);
        assert_eq!(map.facts.object_revision(decoy_key), 0);
        assert_eq!(map.facts.next_movement_decay_time(), Some(62_000_000));
        assert_eq!(map.facts.expire_movement_items(61_999_999), 0);
        assert!(map.facts.base_hidden(key));
        assert_eq!(map.facts.expire_movement_items(62_000_000), 1);
        assert!(!map.facts.base_hidden(key));
        assert_eq!(map.facts.object_revision(key), 2);
        assert_eq!(map.facts.expire_movement_items(62_000_001), 0);
        let copy = owner
            .root
            .read_character_quest_state(owner.character, fence.character_id)
            .await
            .map_err(debug)?;
        assert_eq!(copy.tracks().get(TRACK), Some(&2));
        let mirrors: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1",
        ).bind(MIRROR).fetch_one(&h.pool).await?;
        assert_eq!(mirrors, 0);
        drop(clock);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
