//! The Zalamon trial through the production prison, NPC and World USE callers.

use super::fixture::{Harness, debug, id, run};
use super::questpc8_mystery_geometry_fixtures::arrive;
use super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session,
};
use crate::foundation::{ChannelContentPin, WorldId};
use crate::gameplay_transport::spell_premium_coordinator::{
    SpellPremiumCoordinator, purchase_fixture,
};
use crate::map::overlay::TilePos;
use oteryn_gameplay_quests::quest::loader::load_embedded_quest_state;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, WorldObjectTarget, decode_use_result, encode_use_intent,
};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;

const TRIAL: &str = "oteryn:quest-reward/children/trial-accept";
const CHILDREN: &str = "oteryn:quest.children_of_the_revolution_quest";
const FRONTIER: &str = "oteryn:quest-progress/quest/u8_54/the_new_frontier/";
const REVOLUTION: &str = "oteryn:quest-progress/quest/u8_54/children_of_the_revolution/";
const ZTISS: &str = "oteryn:npc.ztiss";
const ZALAMON: &str = "oteryn:npc.zalamon";

#[test]
fn children_of_the_revolution_earns_mission_eight_reports_map_and_investigates_temple() -> TestResult
{
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty(), "PostgreSQL admin route is empty");
    super::kill_reward_e2e_fixtures::scenario_thread::run("children-trial-prerequisite", || {
        run(async |admin| {
            let h =
                Harness::create_with_admission_seed(admin, "children_trial", true, false).await?;
            let result = exercise_trial(&h).await;
            h.cleanup().await?;
            result
        })
    })
}

async fn exercise_trial(h: &Harness) -> TestResult {
    let loaded = load_embedded_quest_state("content-1").map_err(debug)?;
    let catalogue = loaded.catalogue();
    assert!(
        catalogue.transition(TRIAL).is_some(),
        "embedded Zalamon trial"
    );
    let trial_gate = oteryn_gameplay_quests::quest::reward_callbacks::load_for(catalogue)?
        .into_iter()
        .find(|callback| callback.key == TRIAL)
        .ok_or("production trial gate")?;
    assert!(trial_gate.requires_premium);
    assert!(trial_gate.requires.iter().any(|guard| {
        guard.track == "oteryn:quest-progress/quest/u8_54/the_new_frontier/mission08"
            && guard.minimum == 2
            && guard.maximum == 2
    }));

    prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world_id = WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) = super::death_ml_loss_composed_fixtures::production_generation_with_pin(
        world_id,
        "content-1",
    )?;
    let active = controller.active().ok_or("active content")?;
    let start = TilePos {
        x: 33353,
        y: 31410,
        floor: -8,
    };
    let world = super::questpc8_native_slice::world(
        active.native_gameplay().ok_or("native content")?,
        &[
            (33349, 31406, 33357, 31414, -8),
            (33163, 31227, 33174, 31252, -11),
            (33278, 31217, 33284, 31223, -7),
            (33174, 31190, 33180, 31196, -7),
        ],
        &[],
        start,
    )?;
    let room = crate::content::qualify_native_entry_room(world_id).map_err(debug)?;
    let cells = world.movement_cells(room.movement_cells()).map_err(debug)?;
    let frame = world.frame_binding_digest(world_id);
    let map = world.map_revision_digest();
    let bundle_room = room
        .bundle_view(
            cells.clone(),
            frame,
            map,
            crate::content::NativeEntryStart {
                x: i32::from(start.x),
                y: i32::from(start.y),
                floor: 8,
            },
        )
        .ok_or("qualified Zalamon room")?;
    let channel_pin = ChannelContentPin::from_activation(
        world_id,
        pin.activation_sequence(),
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        frame,
        map,
        (i32::from(start.x), i32::from(start.y), 8),
    );
    let deps = Dependencies::with_content_and_capacity(
        channel_pin,
        room,
        crate::spell::cast::v1_spell_book().map_err(debug)?,
        3,
    )?;
    deps.runtime
        .lock()
        .await
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    let (consumer, clock, activation, _) = purchase_fixture::current(id(40));
    h.root
        .latch_premium_activation(
            &activation.activation_id,
            activation.switch_over_us,
            activation.switch_over_us + 60_000_000,
        )
        .await
        .map_err(debug)?;
    let premium = SpellPremiumCoordinator::new(
        consumer,
        Some(clock),
        Some(activation),
        Some(h.root.clone()),
    );
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(active);
    owner.quest_catalogue = Some(Arc::new(catalogue.clone()));
    owner.movement_cells = (&cells).into();
    owner.rooms = crate::gameplay_transport::SeamRooms::new(Some(&bundle_room), Some(&bundle_room));
    owner.premium_coordinator = Some(&premium);
    let mut admitted = deps.admitted;
    owner.admit_premium(id(40), admitted.game_session_id);
    assert!(
        owner
            .session_premium_gate(admitted.game_session_id)
            .await
            .premium_current()
    );
    owner
        .refresh_quest_session(admitted.game_session_id, true)
        .await;
    let [zalamon, ztiss] = super::classic_quests_fixtures::install_positions(
        &owner,
        &admitted,
        [ZALAMON, ZTISS],
        [
            Position {
                x: 33354,
                y: 31410,
                floor: -8,
            },
            Position {
                x: 33170,
                y: 31231,
                floor: -11,
            },
        ],
    )
    .await?;
    super::qd_world_fixtures::select_talk_capabilities(&owner, &mut admitted, &[3, 4, 11, 13])?;

    admitted.item_fence = Some(
        owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "character fence")?
            .ok_or("current session")?
            .item_fence(),
    );
    let before = quantity(h, 637).await?;
    let first = admitted.continuity.next_command_id;
    super::grizzly_npc_fixtures::wire::talk_refused(
        &mut owner,
        &mut admitted,
        zalamon,
        &[(first, "hi"), (first + 1, "mission")],
    )
    .await?;
    assert_eq!(quantity(h, 637).await?, before);
    let rows: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_tracks
         WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2",
    )
    .bind(id(41).as_slice())
    .bind(CHILDREN)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        rows, 0,
        "refusing the trial leaves all six missions unopened"
    );
    // The prison is a fixture arrival. The note, secret door and Ztiss deal
    // each go through their production map or NPC command and write their own state.
    arrive(&owner, &admitted, at(33165, 31250, -11)).await?;
    use_object(&mut owner, &mut admitted, at(33165, 31249, -11), 1).await?;
    assert_eq!(progress(h, FRONTIER, "mission07/hidden_note").await?, 1);
    assert_eq!(quantity(h, 8747).await?, 1);
    assert_eq!(progress(h, FRONTIER, "mission08").await?, 0);

    arrive(&owner, &admitted, at(33169, 31249, -11)).await?;
    use_object(&mut owner, &mut admitted, at(33170, 31248, -11), 2).await?;
    assert_eq!(progress(h, FRONTIER, "questline").await?, 23);
    assert_eq!(quantity(h, 8747).await?, 1);
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(deps.actor)
            .map_err(debug)?
            .position(),
        super::quest_world_callbacks_fixtures::local(Position {
            x: 33170,
            y: 31247,
            floor: -11
        }),
        "the secret door sends the character through to Ztiss",
    );

    arrive(&owner, &admitted, at(33169, 31231, -11)).await?;
    let ztiss_deal = oteryn_gameplay_quests::quest::reward_callbacks::load_for(catalogue)?
        .into_iter()
        .find(|callback| callback.key == "oteryn:quest-reward/new-frontier/ztiss-confirm")
        .ok_or("generated Ztiss reward input")?;
    let (disposition, _, _) = super::qengine_rewards_fixtures::wire_conversation(
        &mut owner,
        &mut admitted,
        ztiss,
        &ztiss_deal,
        false,
        vec!["hi", "guezt", "offer", "work", "yes", "yes"],
    )
    .await?;
    assert_eq!(disposition, NpcIntentDisposition::Ok);
    assert_eq!(progress(h, FRONTIER, "questline").await?, 24);
    assert_eq!(progress(h, FRONTIER, "mission08").await?, 2);
    assert_eq!(quantity(h, 8747).await?, 1);

    arrive(&owner, &admitted, start).await?;
    talk(
        &mut owner,
        &mut admitted,
        zalamon,
        ["hi", "mission", "yes", "yes"],
    )
    .await?;
    assert_eq!(progress(h, REVOLUTION, "mission00").await?, 1);
    assert_eq!(progress(h, REVOLUTION, "questline").await?, 1);
    assert_eq!(quantity(h, 637).await?, before);

    arrive(&owner, &admitted, at(33281, 31220, -7)).await?;
    use_object(&mut owner, &mut admitted, at(33281, 31219, -7), 1).await?;
    assert_eq!(progress(h, REVOLUTION, "mission00").await?, 1);
    assert_eq!(progress(h, REVOLUTION, "questline").await?, 2);
    assert_eq!(quantity(h, 637).await?, before + 1);

    arrive(&owner, &admitted, start).await?;
    talk(
        &mut owner,
        &mut admitted,
        zalamon,
        ["hi", "mission", "yes", "yes"],
    )
    .await?;
    assert_eq!(progress(h, REVOLUTION, "mission00").await?, 2);
    assert_eq!(progress(h, REVOLUTION, "questline").await?, 3);
    assert_eq!(quantity(h, 637).await?, before);

    talk(&mut owner, &mut admitted, zalamon, ["hi", "mission", "yes"]).await?;
    assert_eq!(progress(h, REVOLUTION, "mission01").await?, 1);
    assert_eq!(progress(h, REVOLUTION, "questline").await?, 4);
    assert_eq!(quantity(h, 637).await?, before);

    arrive(&owner, &admitted, at(33176, 31193, -7)).await?;
    super::desert_earned_route_fixtures::step(
        &mut owner,
        &mut admitted,
        oteryn_protocol_oteryn::world_spatial::StepDirection::East,
    )
    .await?;
    assert_eq!(progress(h, REVOLUTION, "mission01").await?, 2);
    assert_eq!(progress(h, REVOLUTION, "questline").await?, 5);
    assert_eq!(quantity(h, 637).await?, before);

    arrive(&owner, &admitted, start).await?;
    talk(&mut owner, &mut admitted, zalamon, ["hi", "mission"]).await?;
    assert_eq!(progress(h, REVOLUTION, "mission01").await?, 3);
    assert_eq!(progress(h, REVOLUTION, "questline").await?, 6);
    assert_eq!(quantity(h, 637).await?, before);
    Ok(())
}

const fn at(x: u16, y: u16, floor: i8) -> TilePos {
    TilePos { x, y, floor }
}

async fn talk<const N: usize>(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: EntityRef,
    words: [&str; N],
) -> TestResult {
    let first = admitted.continuity.next_command_id;
    let commands = words
        .into_iter()
        .enumerate()
        .map(|(offset, word)| Ok((first + u64::try_from(offset)?, word)))
        .collect::<TestResult<Vec<_>>>()?;
    super::grizzly_npc_fixtures::wire::talk(owner, admitted, npc, &commands).await
}

async fn use_object(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    at: TilePos,
    ordinal: u8,
) -> TestResult {
    let map = owner.bundle_map().ok_or("source map")?;
    let key = crate::map::view::placement_key(at, ordinal).ok_or("source placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = encode_use_intent(&WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    })
    .map_err(debug)?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    admitted.item_fence = Some(
        owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("current item fence")?
            .item_fence(),
    );
    let response = super::quest_world_callbacks_fixtures::wire(
        owner,
        admitted,
        admitted.continuity.next_command_id,
        2,
        payload,
    )
    .await?;
    assert_eq!(
        decode_use_result(&response).map_err(debug)?,
        UseDisposition::Committed,
        "{at:?}"
    );
    Ok(())
}

async fn progress(h: &Harness, prefix: &str, track: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT coalesce((SELECT value FROM game_character_quest_tracks
         WHERE character_id=encode($1,'hex')::uuid AND track_key=$2),0)",
    )
    .bind(id(41).as_slice())
    .bind(format!("{prefix}{track}"))
    .fetch_one(&h.pool)
    .await?)
}

async fn quantity(h: &Harness, item: u32) -> TestResult<i64> {
    super::questpc7_outfit_items_fixtures::quantity(h, item).await
}
