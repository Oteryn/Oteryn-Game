//! Literal USE of Crystal's retained telescope, with durable receipt replay.
use super::fixture::{Harness, debug, id, run};
use super::l6_world_pickups_fixtures::{ExpectedTile, NativeCrop, native_crop};
use super::quest_world_callbacks_fixtures::wire;
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::NativeEntryStart;
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::durability::quest_world_callback::QuestWorldContext;
use crate::foundation::{ChannelContentPin, CommandId, CommandRef, WorldId};
use oteryn_gameplay_quests::quest::{loader, sea_serpent as hunt};
use oteryn_gameplay_world::map::view::MapFacts;
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, WorldObjectTarget, decode_use_result, encode_use_intent,
};
use std::sync::Arc;

#[test]
fn rngroute_sea_serpent_telescope_use_replays_the_receipted_roll_and_state() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run("rngroute-telescope", || {
        run(async |admin| {
            reset();
            let h = Harness::create_with_admission_seed(admin, "rngroute_telescope", true, false)
                .await?;
            let result = exercise(&h).await;
            h.cleanup().await?;
            result
        })
    })
}

async fn exercise(h: &Harness) -> TestResult {
    prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) = activated_with_loot_tables(world, None);
    let generation = controller.active().ok_or("active generation")?;
    let native = generation.native_gameplay().ok_or("native items")?;
    let bundle = native_crop(
        native,
        NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z02-x124-y121.b3"
            )),
            region: (2, 124, 121),
            bounds: (31942, 31044, 31944, 31046),
            start: (31943, 31045),
            expected: vec![ExpectedTile {
                position: (31942, 31045),
                ids: vec![493, 2156, 938],
                counted_entry: None,
            }],
        },
    )?;
    let base = pin.qualified_room().clone();
    let cells = bundle
        .movement_cells(base.movement_cells())
        .map_err(debug)?;
    let frame = bundle.frame_binding_digest(world);
    let map_revision = bundle.map_revision_digest();
    let origin = (31943, 31045, 2);
    let room = base
        .bundle_view(
            cells.clone(),
            frame,
            map_revision,
            NativeEntryStart {
                x: origin.0,
                y: origin.1,
                floor: origin.2,
            },
        )
        .ok_or("lookout room")?;
    let mut deps = Dependencies::with_content_and_capacity(
        ChannelContentPin::from_activation(
            world,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            frame,
            map_revision,
            origin,
        ),
        base,
        crate::spell::cast::v1_spell_book().map_err(debug)?,
        2,
    )?;
    deps.combat_room = Some(room);
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(generation);
    owner.movement_cells = (&cells).into();
    let catalogue = Arc::new(
        loader::load_embedded_quest_state("content-1")
            .map_err(debug)?
            .catalogue()
            .clone(),
    );
    owner.quest_catalogue = Some(catalogue.clone());
    deps.runtime
        .lock()
        .await
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    deps.initialize(&owner).await;
    let mut admitted = deps.admitted;
    let session = admitted.game_session_id;
    owner
        .states
        .chat
        .admit(session, "Lookout", owner.owner_now());
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "fence")?
        .ok_or("live fence")?;
    admitted.item_fence = Some(fence.item_fence());
    assert!(owner.ensure_source_map_initialized().await);
    let ready = hunt::TRACKS[..4]
        .iter()
        .zip([1, 1, 1, 0])
        .map(|(key, value)| ((*key).to_owned(), value))
        .collect::<Vec<_>>();
    super::qengine_outfit_stories_fixtures::seed_quest_prerequisites(
        h, &owner, &admitted, &ready, 125,
    )
    .await?;

    let object =
        oteryn_world_bundle::bundle::placement_key(-2, 31942, 31045, 2).ok_or("telescope key")?;
    let map = owner.bundle_map().ok_or("lookout map")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&object.to_be_bytes());
    let intent = encode_use_intent(&WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(object),
    })
    .map_err(debug)?;
    let command = admitted.continuity.next_command_id;
    let result = wire(&owner, &mut admitted, command, 2, intent.clone()).await?;
    assert_eq!(
        decode_use_result(&result).map_err(debug)?,
        UseDisposition::Committed
    );
    assert_eq!(
        owner.states.chat.drain(session).len(),
        1,
        "a fresh use reports one result"
    );

    // Ask the durable caller again, independently of the wire command-result cache.
    let context = {
        let runtime = deps.runtime.lock().await;
        QuestWorldContext {
            map_revision: crate::map::overlay::map_revision(map.overlay.base()),
            content_revision: format!(
                "sha256:{}",
                runtime
                    .content_pin()
                    .server_artifact_digest()
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>()
            ),
            placement_context: frame.to_vec(),
            reset_epoch: owner.reset_epoch,
            actor_placement: deps.actor.placement_identity(),
            movement_revision: runtime
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .revision(),
            party: Vec::new(),
        }
    };
    let permit = owner.spell_lane_permit().await.ok_or("permit")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let replay = slot
        .commit_world_callback(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: hunt::KEY.into(),
                cause: QuestCause::Use(CommandRef::new(
                    session,
                    CommandId::new(command).map_err(debug)?,
                )),
            },
            catalogue,
            &permit,
            context,
            Some(native),
        )
        .await?;
    let QuestTransitionOutcome::AlreadyCommitted(receipt) = replay else {
        return Err(format!("USE did not produce a replayable receipt: {replay:?}").into());
    };
    drop(slot);
    drop(permit);
    let after = |key: &str| {
        receipt
            .changes
            .iter()
            .find(|change| change.track == key)
            .map(|change| change.after)
            .ok_or("receipt track")
    };
    let roll = after(hunt::DRAW)?;
    assert!((1..=240).contains(&roll));
    let appearance = 1 + (roll - 1) % 10;
    let detail = (roll - 1) / 10;
    let (line, bait, direction, access) = match appearance {
        1..=4 => (1, 1, 0, 0),
        5..=8 => (1, 1 - detail % 2, 1 + detail / 8, 0),
        9 => (1, 1 - detail % 2, 4, 0),
        10 => (2, 1, 0, 1),
        _ => return Err("roll outside Crystal outcome space".into()),
    };
    assert_eq!(
        [
            after(hunt::TRACKS[0])?,
            after(hunt::TRACKS[1])?,
            after(hunt::TRACKS[3])?,
            after(hunt::TRACKS[4])?
        ],
        [line, bait, direction, access],
    );
    let stored = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await?;
    for change in receipt.changes {
        assert_eq!(stored.tracks().get(&change.track), Some(&change.after));
    }
    assert_eq!(
        wire(&owner, &mut admitted, command, 2, intent).await?,
        result
    );
    assert!(owner.states.chat.drain(session).is_empty());
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
    )
    .bind(hunt::KEY)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(count, 1);
    Ok(())
}
