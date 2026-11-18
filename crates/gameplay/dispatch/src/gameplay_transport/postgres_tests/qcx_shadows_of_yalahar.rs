//! Shadows of Yalahar, TibiaWiki revisions 843939 and 1106237.
use super::fixture::{Harness, debug, id, run};
use super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session,
};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{ChannelContentPin, CommandId, CommandRef, WorldId};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use crate::gameplay_transport::spell_premium_coordinator::{
    SpellPremiumCoordinator, purchase_fixture,
};
use oteryn_gameplay_quests::quest::loader::load_embedded_quest_state;
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestTransition,
};
use oteryn_protocol_oteryn::world_object::UseDisposition;
use oteryn_protocol_oteryn::world_spatial::StepDirection;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;

const MISSION: &str = "oteryn:quest-progress/crystalserver/quest/u8_5/shadows_of_yalahar/mission01";
const PREREQUISITE: &str = "oteryn:quest-progress/quest/u8_4/in_service_of_yalahar/mission03";

#[test]
fn telas_entry_gate_and_all_five_research_notes() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("qcx53-errand", || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, "qcx53_errand", true, false).await?;
            let result = exercise(&h).await;
            h.cleanup().await?;
            result
        })
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED qcx53-errand duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

const SHADOWS: &str = "oteryn:quest.shadows_of_yalahar_quest";
const QUESTLINE: &str =
    "oteryn:quest-progress/crystalserver/quest/u8_5/shadows_of_yalahar/questline";
const MISSION_09: &str =
    "oteryn:quest-progress/crystalserver/quest/u8_5/shadows_of_yalahar/mission09";
const MISSION_10: &str =
    "oteryn:quest-progress/crystalserver/quest/u8_5/shadows_of_yalahar/mission10";
const DOOR_10: &str = "oteryn:quest-progress/crystalserver/quest/u8_5/shadows_of_yalahar/door_10";
const HELLGATE: (u16, u16, i8) = (32693, 31564, -13);

#[test]
fn hellgate_door_refuses_before_mission_ten() -> TestResult {
    hellgate_case(false)
}

#[test]
fn telas_mission_ten_start_opens_hellgate_door() -> TestResult {
    hellgate_case(true)
}

fn hellgate_case(start_mission: bool) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("qcx53-hellgate", move || {
        run(async |admin| {
            let h =
                Harness::create_with_admission_seed(admin, "qcx53_hellgate", true, false).await?;
            let result = exercise_hellgate(&h, start_mission).await;
            h.cleanup().await?;
            result
        })
    })
}

async fn exercise_hellgate(h: &Harness, start_mission: bool) -> TestResult {
    use super::quest_reward_claims::{committed_world, journey_step, journey_target, journey_use};

    prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) =
        super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
    let generation = controller.active().ok_or("active generation")?;
    let native = generation.native_gameplay().ok_or("native gameplay")?;
    let map = committed_world(HELLGATE, 5104, native)?;
    let (x, y, floor) = HELLGATE;
    let placement = oteryn_world_bundle::bundle::placement_key(floor, x, y, 1)
        .ok_or("Hellgate door placement")?;
    assert_eq!(map.facts().action_at(placement), Some(42521));
    let room = pin.qualified_room().clone();
    let cells = map.movement_cells(room.movement_cells()).map_err(debug)?;
    let pin = ChannelContentPin::from_activation(
        world,
        pin.activation_sequence(),
        generation.identity().server_artifact_digest(),
        generation.identity().client_artifact_digest(),
        map.frame_binding_digest(world),
        map.map_revision_digest(),
        (32692, 31564, 13),
    );
    let deps = Dependencies::with_content(
        pin,
        room,
        crate::spell::cast::v1_spell_book().map_err(debug)?,
    )?;
    deps.runtime
        .lock()
        .await
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    let mut owner = deps.owner(h, &authority);
    owner.movement_cells = (&cells).into();
    owner.active_generation = Some(generation);
    owner.quest_catalogue = Some(Arc::new(
        load_embedded_quest_state("content-1")
            .map_err(debug)?
            .catalogue()
            .clone(),
    ));
    deps.initialize(&owner).await;
    let mut admitted = deps.admitted.clone();
    admitted.first_entry = FirstEntryOutcome::Positioned;
    let current =
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(admitted.game_session_id)
            .await?
            .0;
    admitted.item_fence =
        Some(crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?);
    owner
        .refresh_quest_session(admitted.game_session_id, true)
        .await;
    super::qd_world::select_talk_capabilities(&owner, &mut admitted, &[3, 4, 6, 11, 15, 18, 25])?;

    let target = journey_target(&owner, HELLGATE, 1)?;
    assert_eq!(
        journey_use(&owner, &mut admitted, &target).await?,
        UseDisposition::Locked
    );
    if !start_mission {
        return Ok(());
    }

    // Seed the earlier missions and external ritual outcome through the
    // fenced quest writer. Telas must report Mission 9 and start Mission 10.
    owner.quest_catalogue = Some(
        commit_fixture_transition(
            &mut owner,
            &admitted,
            900,
            QuestTransition {
                key: "oteryn:quest-transition/fixture/qcx53-ritual-result".into(),
                quest: SHADOWS.into(),
                effects: vec![
                    fixture_effect(QUESTLINE, 0, 16),
                    fixture_effect(MISSION_09, 0, 2),
                ],
                completes: false,
                experience: None,
                world_change: None,
            },
        )
        .await?,
    );
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let [telas] = super::classic_quests::install(&owner, &admitted, ["oteryn:npc.telas"]).await?;
    let mut command = 1000;
    converse(&mut owner, &mut admitted, telas, &mut command, "mission").await?;
    assert_eq!(progress(h, QUESTLINE).await?, 17);
    assert_eq!(progress(h, MISSION_10).await?, 0);
    let target = journey_target(&owner, HELLGATE, 1)?;
    assert_eq!(
        journey_use(&owner, &mut admitted, &target).await?,
        UseDisposition::Locked
    );

    converse(&mut owner, &mut admitted, telas, &mut command, "mission").await?;
    assert_eq!(progress(h, MISSION_10).await?, 1);
    assert_eq!(progress(h, DOOR_10).await?, 1);
    let target = journey_target(&owner, HELLGATE, 1)?;
    assert_eq!(
        journey_use(&owner, &mut admitted, &target).await?,
        UseDisposition::Committed
    );
    journey_step(&owner, &mut admitted, StepDirection::East).await?;
    journey_step(&owner, &mut admitted, StepDirection::East).await?;
    let position = deps
        .runtime
        .lock()
        .await
        .read_actor_position(deps.actor)
        .map_err(debug)?
        .position();
    assert_eq!((position.x, position.y, position.floor), (32694, 31564, 13));
    Ok(())
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
    let (controller, pin) =
        super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
    let deps = Dependencies::with_content_and_capacity(
        pin.into_channel_pin(),
        crate::content::qualify_native_entry_room(world).map_err(debug)?,
        crate::spell::cast::v1_spell_book().map_err(debug)?,
        7,
    )?;
    deps.runtime
        .lock()
        .await
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    let generation = controller.active().ok_or("active generation")?;
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(generation);
    owner.quest_catalogue = Some(Arc::new(
        load_embedded_quest_state("content-1")
            .map_err(debug)?
            .catalogue()
            .clone(),
    ));
    let mut admitted = deps.admitted.clone();
    owner
        .refresh_quest_session(admitted.game_session_id, true)
        .await;
    let [telas, boveas, cerdras, dreadeye, milos, scutty] =
        super::classic_quests::install_with_offsets(
            &owner,
            &admitted,
            [
                "oteryn:npc.telas",
                "oteryn:npc.boveas",
                "oteryn:npc.cerdras",
                "oteryn:npc.dreadeye",
                "oteryn:npc.milos",
                "oteryn:npc.scutty",
            ],
            Some([(0, 1), (1, 1), (2, 1), (0, 2), (1, 2), (2, 2)]),
        )
        .await?;
    super::qd_world::select_talk_capabilities(&owner, &mut admitted, &[3, 6, 25])?;
    let mut command = 1000;
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

    converse(
        &mut owner,
        &mut admitted,
        scutty,
        &mut command,
        "research notes",
    )
    .await?;
    assert_eq!(
        progress(
            h,
            "oteryn:quest-progress/crystalserver/quest/u8_5/shadows_of_yalahar/research_scutty"
        )
        .await?,
        0,
        "research notes require the errand to be active",
    );

    level(h, 75).await?;
    owner.premium_coordinator = Some(&premium);
    converse(&mut owner, &mut admitted, telas, &mut command, "mission").await?;
    assert_eq!(
        progress(h, MISSION).await?,
        0,
        "Mission 3 must be started first"
    );

    start_prerequisite(&mut owner, &admitted).await?;
    level(h, 74).await?;
    converse(&mut owner, &mut admitted, telas, &mut command, "mission").await?;
    assert_eq!(progress(h, MISSION).await?, 0, "level 74 is too low");

    level(h, 75).await?;
    owner.premium_coordinator = None;
    converse(&mut owner, &mut admitted, telas, &mut command, "mission").await?;
    assert_eq!(progress(h, MISSION).await?, 0, "Premium is required");

    owner.premium_coordinator = Some(&premium);
    converse(&mut owner, &mut admitted, telas, &mut command, "mission").await?;
    assert_eq!(
        progress(h, MISSION).await?,
        1,
        "level 75, Premium and mission 3 unlock the errand"
    );

    assert_eq!(
        progress(
            h,
            "oteryn:quest-progress/crystalserver/quest/u8_5/shadows_of_yalahar/questline"
        )
        .await?,
        1,
        "acceptance opens the quest journal",
    );
    for npc in [scutty, boveas, milos, dreadeye, scutty] {
        converse(
            &mut owner,
            &mut admitted,
            npc,
            &mut command,
            "research notes",
        )
        .await?;
    }
    converse(&mut owner, &mut admitted, telas, &mut command, "mission").await?;
    assert_eq!(
        progress(h, MISSION).await?,
        1,
        "four researchers plus a repeat do not suffice"
    );

    converse(
        &mut owner,
        &mut admitted,
        cerdras,
        &mut command,
        "research notes",
    )
    .await?;
    converse(&mut owner, &mut admitted, telas, &mut command, "mission").await?;
    assert_eq!(
        progress(h, MISSION).await?,
        2,
        "five distinct researchers finish the errand"
    );
    assert_eq!(progress(h, QUESTLINE).await?, 2);
    Ok(())
}

async fn converse(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: EntityRef,
    command: &mut u64,
    keyword: &str,
) -> TestResult {
    super::classic_quests::talk(owner, admitted, npc, *command, "hi").await?;
    super::qengine_rewards::talk(owner, admitted, npc, *command + 1, keyword).await?;
    super::classic_quests::talk(owner, admitted, npc, *command + 2, "bye").await?;
    *command += 3;
    Ok(())
}

async fn progress(h: &Harness, track: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2),0)")
        .bind(id(41).as_slice()).bind(track).fetch_one(&h.pool).await?)
}

async fn level(h: &Harness, level: i64) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    let updated = sqlx::query("UPDATE game_character_progression_state SET level=$1 WHERE character_id=encode($2,'hex')::uuid")
        .bind(level).bind(id(41).as_slice()).execute(&mut *tx).await?;
    assert_eq!(updated.rows_affected(), 1);
    tx.commit().await?;
    Ok(())
}

async fn start_prerequisite(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult {
    commit_fixture_transition(
        owner,
        admitted,
        900,
        QuestTransition {
            key: "oteryn:quest-transition/fixture/qcx53-in-service-mission-three".into(),
            quest: "oteryn:quest.in_service_of_yalahar_quest".into(),
            effects: vec![fixture_effect(PREREQUISITE, 0, 1)],
            completes: false,
            experience: None,
            world_change: None,
        },
    )
    .await
    .map(|_| ())
}

fn fixture_effect(track: &str, from: i64, to: i64) -> QuestEffect {
    QuestEffect {
        track: track.into(),
        from: QuestComparison::Eq(from),
        effect: QuestEffectKind::Set(to),
    }
}

async fn commit_fixture_transition(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    command: u64,
    transition: QuestTransition,
) -> TestResult<Arc<oteryn_gameplay_quests::quest::QuestStateCatalogue>> {
    let key = transition.key.clone();
    let catalogue = owner.quest_catalogue.as_ref().ok_or("quest catalogue")?;
    let fixture = Arc::new(
        catalogue
            .with_fixture_transition(transition)
            .map_err(debug)?,
    );
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("fence")?;
    let result = owner
        .revision_sequencer
        .acquire(fence.character_id)
        .await
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: key,
                cause: QuestCause::Command(CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(command).map_err(debug)?,
                )),
            },
            fixture.clone(),
        )
        .await
        .map_err(debug)?;
    assert!(matches!(result, QuestTransitionOutcome::Committed(_)));
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    Ok(fixture)
}
