//! The published lookout telescope dispatches into the fenced quest writer.
use super::fixture::{Harness, debug};
use super::{Dependencies, TestResult};
use crate::foundation::ChannelContentPin;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::{loader, sea_serpent};
use oteryn_protocol_oteryn::chat::ChatLine;
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, WorldObjectTarget, decode_use_result, encode_use_intent,
};
use std::sync::Arc;

#[test]
fn sea_serpent_native_use_persists_chance_and_replays_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run_async(async move |_| exercise(admin).await)?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED sea_serpent_native_use duration={:?}",
        started.elapsed()
    );
    Ok(())
}

fn lookout(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_crop};
    native_crop(
        native,
        NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z02-x124-y121.b3"
            )),
            region: (2, 124, 121),
            bounds: (31942, 31044, 31945, 31047),
            start: (31943, 31045),
            expected: vec![ExpectedTile {
                position: (31942, 31045),
                ids: vec![493, 2156, 938],
                counted_entry: None,
            }],
        },
    )
}

async fn exercise(admin: String) -> TestResult {
    super::reset();
    let world_id = super::fixture::fence(1)?.runtime_scope.world_id();
    let (controller, baseline) =
        super::death_ml_loss_composed::production_generation_with_pin(world_id, "content-1")?;
    let active = controller.active().ok_or("native content activation")?;
    let native = active.native_gameplay().ok_or("native item definitions")?;
    let world = lookout(native)?;
    let cells = world
        .movement_cells(baseline.qualified_room().movement_cells())
        .map_err(debug)?;
    let origin = crate::content::NativeEntryStart {
        x: 31943,
        y: 31045,
        floor: 2,
    };
    let room = baseline
        .qualified_room()
        .bundle_view(
            cells,
            world.frame_binding_digest(world_id),
            world.map_revision_digest(),
            origin,
        )
        .ok_or("lookout room")?;
    let pin = ChannelContentPin::from_activation(
        world_id,
        active.activation_sequence(),
        active.identity().server_artifact_digest(),
        active.identity().client_artifact_digest(),
        room.frame_binding().digest(),
        room.map_revision_digest(),
        (origin.x, origin.y, origin.floor),
    );
    let deps = Dependencies::with_content(
        pin.clone(),
        room,
        crate::spell::cast::v1_spell_book().map_err(debug)?,
    )?;
    let h = Harness::create_with_admission_seed(admin, "sea_serpent", false, false).await?;
    let evidence = super::house_transfer::RecoverySource::start(&h).await?;
    super::house_transfer::prepare_owned_session(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut owner = deps.owner(&h, &authority);
    owner.evidence = &evidence.evidence;
    owner.active_generation = Some(active);
    super::boss_activity::issue_content_and_reset_with_runtime_pin(
        &h,
        &pin,
        &pin,
        world.map_revision(),
    )
    .await?;
    let lowered = loader::load_embedded_quest_state("content-1").map_err(debug)?;
    owner.quest_catalogue = Some(Arc::new(lowered.catalogue().clone()));
    let mut admitted = deps.admitted;
    super::srx3_activity_source_fixture::prepare_player(&h, &owner, &mut admitted, 61).await?;
    owner.admit_quest_session(&admitted).await;
    let session = admitted.game_session_id;
    owner
        .states
        .chat
        .admit(session, "Lookout", owner.owner_now());

    let selector = sea_serpent::SELECTOR;
    let at = selector.position;
    let key = oteryn_world_bundle::bundle::placement_key(at.floor, at.x, at.y, selector.ordinal)
        .ok_or("telescope placement")?;
    let mut saw_water = false;
    let mut found_spot = false;
    for attempt in 0_u8..96 {
        let seed = sea_serpent::TRACKS[..5]
            .iter()
            .zip([1, 1, 0, 1, 0])
            .map(|(key, value)| ((*key).to_owned(), value))
            .collect::<Vec<_>>();
        super::qengine_outfit_stories::seed_quest_prerequisites(
            &h,
            &owner,
            &admitted,
            &seed,
            128 + attempt,
        )
        .await?;
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("world-use cooldown")?;
        let map = owner.bundle_map().ok_or("published World map")?;
        let mut placement = map.overlay.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let payload = encode_use_intent(&WorldObjectTarget {
            placement,
            expected_revision: map.facts.object_revision(key),
        })
        .map_err(debug)?;
        let command = admitted.continuity.next_command_id;
        let reply =
            super::quest_world_callbacks::wire(&owner, &mut admitted, command, 2, payload.clone())
                .await?;
        assert_eq!(
            decode_use_result(&reply).map_err(debug)?,
            UseDisposition::Committed
        );
        let character = admitted.item_fence.ok_or("Character fence")?.character_id;
        let stored = h
            .root
            .read_character_quest_state(owner.character, character)
            .await?;
        let state = sea_serpent::TRACKS.map(|track| stored.tracks().get(track).copied());
        let [
            Some(line),
            Some(bait),
            Some(direction),
            Some(success),
            Some(access),
            Some(outcome),
        ] = state
        else {
            return Err(format!("incomplete telescope state: {state:?}").into());
        };
        assert_eq!(success, 1);
        let feedback = owner.states.chat.drain(session);
        assert!(matches!(feedback.as_slice(), [ChatLine::Private { .. }]));
        match outcome {
            2 => {
                assert_eq!((line, bait, direction, access), (1, 1, 0, 0));
                saw_water = true;
            }
            3..=14 => assert_eq!((line, direction, access), (1, 1 + (outcome - 3) % 3, 0)),
            15 => assert_eq!((line, direction, access), (1, 4, 0)),
            16 => {
                assert_eq!((line, bait, direction, access), (2, 1, 0, 1));
                found_spot = true;
            }
            _ => return Err(format!("unexpected committed outcome {outcome}").into()),
        }
        assert_eq!(
            super::quest_world_callbacks::wire(&owner, &mut admitted, command, 2, payload.clone())
                .await?,
            reply,
        );
        assert!(owner.states.chat.drain(session).is_empty());
        if saw_water && found_spot {
            let receipts: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
            )
            .bind(sea_serpent::KEY)
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(receipts, i64::from(attempt) + 1);
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await?;
            return Ok(());
        }
    }
    Err("the telescope did not exercise water and success in 96 uses".into())
}
