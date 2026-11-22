//! Exact selected knife use through the ordinary wire route on authored map placements.
use super::fixture::{Harness, debug, id, run};
use super::quest_world_callbacks_fixtures::{wire, world_with_objects};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue, authored_bindings::Bindings, world_callbacks::Position,
};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, decode_use_result,
    encode_item_use_intent,
};
use serde_json::json;
use std::sync::Arc;

const AT: Position = Position {
    x: 32700,
    y: 31200,
    floor: -7,
};
const QUEST: &str = "oteryn:quest.an_interest_in_botany_quest";
const TRACK: &str = "oteryn:quest-progress/quest/u8_6/an_interest_in_botany/questline";
const KNIFE: &str = "oteryn:item.tibia.i5908";
const BOOK: &str = "oteryn:item.tibia.i11699";
const DONOR: &str = "https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/actions/tools/skinning.lua";

#[test]
fn quest_skinning_botany_requires_almanach_and_ordered_storage_without_replacing_plants()
-> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    exercise(false)
}

#[test]
fn quest_skinning_map_corpse_replacement_and_progress_commit_together_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    exercise(true)
}

fn exercise(transform: bool) -> TestResult {
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(transform).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "quest skinning scenario panicked")??;
    eprintln!(
        "QUEST_SKINNING_PG_COMPLETED transform={transform} duration={:?}",
        started.elapsed()
    );
    Ok(())
}

fn scenario(transform: bool) -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "quest_skinning", true, false).await?;
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
            super::death_ml_loss_composed_fixtures::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("active generation")?;
        let native = generation.native_gameplay().ok_or("native content")?;
        let knife = native
            .item_index()
            .and_then(|index| index.definition_ref("Item", KNIFE, "definition-r1"))
            .ok_or("obsidian knife definition")?;
        let other = Position { y: AT.y + 1, ..AT };
        let appearance = if transform { 4011 } else { 10735 };
        let mut first = callback("first-sample", AT, appearance, 1)?;
        if transform {
            // Exercise the documented Crystal minotaur spent form using an authored quest gate.
            first
                .item_on_target
                .as_mut()
                .ok_or("knife input")?
                .target_replacement = Some(4012);
        }
        let second = callback("second-sample", other, 10697, 2)?;
        let bytes = serde_json::to_vec(&json!({
            "schema": 1,
            "quests": [{
                "quest": QUEST,
                "recipe_sha256": "2222222222222222222222222222222222222222222222222222222222222222",
                "sources": [DONOR],
                "tracks": [{"key": TRACK, "initial": 1, "minimum": 0, "maximum": 3}],
                "stages": [
                    {"key":"first-sample","completes":false,"trigger":{"kind":"map_use","callback":&first}},
                    {"key":"second-sample","completes":true,"trigger":{"kind":"map_use","callback":&second}}
                ]
            }]
        }))?;
        let base = QuestStateCatalogue::empty("content-1").ok_or("catalogue revision")?;
        let catalogue = Bindings::parse(&bytes)?.attach(&base)?;
        let world = world_with_objects(&first, AT, &[(other, 10697)], &[])?;
        let cells = world
            .movement_cells(activated.qualified_room().movement_cells())
            .map_err(debug)?;
        let frame = world.frame_binding_digest(deps.admitted.world_id);
        let entry = crate::content::NativeEntryStart {
            x: i32::from(AT.x - 1),
            y: i32::from(AT.y),
            floor: -i16::from(AT.floor),
        };
        let room = activated
            .qualified_room()
            .bundle_view(cells.clone(), frame, world.map_revision_digest(), entry)
            .ok_or("bundle room")?;
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            frame,
            world.map_revision_digest(),
            (entry.x, entry.y, entry.floor),
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
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: entry.x,
                    y: entry.y,
                    floor: entry.floor,
                },
            )
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        owner.movement_cells = (&cells).into();
        owner.rooms = crate::gameplay_transport::SeamRooms::new(Some(&room), Some(&room));
        owner.quest_catalogue = Some(Arc::new(catalogue));
        deps.initialize(&owner).await;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice()).bind(frame.as_slice()).execute(&h.pool).await?;
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = super::FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(
            owner
                .current_quest_fence(admitted.game_session_id)
                .await
                .map_err(debug)?
                .ok_or("quest fence")?
                .item_fence(),
        );
        admitted.continuity.selected_capabilities =
            super::SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 15, 18])
                .ok_or("map use capabilities")?;
        super::item_look_fixtures::seed_main_backpack(&h, &[(KNIFE, 1)]).await?;
        owner.admit_quest_session(&admitted).await;

        let map = owner.bundle_map().ok_or("map")?;
        let target = |pos: Position| -> TestResult<_> {
            let key = oteryn_world_bundle::bundle::placement_key(pos.floor, pos.x, pos.y, 1)
                .ok_or("placement")?;
            let mut placement = map.overlay.base().digest().to_vec();
            placement.extend_from_slice(&key.to_be_bytes());
            Ok((
                key,
                encode_item_use_intent(&UseIntent {
                    target: UseTarget::ItemByDefinition(knife),
                    use_with: None,
                    use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
                        placement,
                        expected_revision: 0,
                    })),
                    use_with_position: None,
                })
                .map_err(debug)?,
            ))
        };
        let (first_key, first_use) = target(AT)?;
        let (second_key, second_use) = target(other)?;
        assert_ne!(
            send(&owner, &mut admitted, 1, first_use.clone()).await?,
            UseDisposition::Committed
        );
        super::quest_world_callbacks_fixtures::input_item(
            &owner,
            admitted.item_fence.ok_or("item fence")?,
            &oteryn_gameplay_quests::quest::world_callbacks::Offering {
                position: Position { x: AT.x - 1, ..AT },
                item: BOOK.into(),
                count: 1,
            },
            60,
            true,
        )
        .await?;
        owner.clock_origin -= std::time::Duration::from_secs(2);
        assert_ne!(
            send(&owner, &mut admitted, 2, second_use.clone()).await?,
            UseDisposition::Committed
        );
        let refused: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(refused, 0);
        assert_eq!(map.facts.object_revision(first_key), 0);

        owner.clock_origin -= std::time::Duration::from_secs(2);
        assert_eq!(
            send(&owner, &mut admitted, 3, first_use.clone()).await?,
            UseDisposition::Committed
        );
        assert_eq!(
            send(&owner, &mut admitted, 3, first_use).await?,
            UseDisposition::Committed
        );
        let replacement: Option<(i64, bool)> = sqlx::query_as(
            "SELECT t.state_item_id, r.xmin=q.created_xact_id::text::xid FROM game_terrain_tool_transforms t
             JOIN game_terrain_tool_receipts r USING (world_id,channel_id,placement,reset_epoch)
             JOIN game_character_quest_receipts q ON q.cause_id=r.game_session_id AND q.cause_ordinal=r.command_id"
        ).fetch_optional(&h.pool).await?;
        assert_eq!(replacement, transform.then_some((4012, true)));
        assert_eq!(map.facts.object_revision(first_key), u64::from(transform));
        owner.clock_origin -= std::time::Duration::from_secs(2);
        assert_eq!(
            send(&owner, &mut admitted, 4, second_use).await?,
            UseDisposition::Committed
        );
        assert_eq!(map.facts.object_revision(second_key), 0);
        let stage: i64 =
            sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
                .bind(TRACK)
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(stage, 3);
        assert_eq!(
            super::qengine_voucher_stories_fixtures::quantity(&h, 11699).await?,
            1
        );
        assert_eq!(
            super::qengine_voucher_stories_fixtures::quantity(&h, 5908).await?,
            1
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

fn callback(
    stage: &str,
    position: Position,
    appearance: u16,
    before: i64,
) -> TestResult<oteryn_gameplay_quests::quest::world_callbacks::Callback> {
    Ok(serde_json::from_value(json!({
        "key": format!("oteryn:quest-callback/botany-engine-{stage}"), "quest": QUEST,
        "trigger": "use",
        "selector":{"position":position,"ordinal":1,"appearance":appearance},
        "reversible":false, "requires_world_admission":true, "passages":[],
        "storage":[{"track":TRACK,"minimum":before,"maximum":before,"set":before+1}],
        "offerings":[], "present":[{"item":BOOK,"count":1}],
        "item_on_target":{"item":KNIFE,"count":1,"selected_instance":true,"reusable":true},
        "return_on_failure":false, "sources":[DONOR]
    }))?)
}

async fn send(
    owner: &super::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut super::AdmittedSession,
    command: u64,
    payload: Vec<u8>,
) -> TestResult<UseDisposition> {
    Ok(decode_use_result(&wire(owner, admitted, command, 2, payload).await?).map_err(debug)?)
}
