//! Shadows of Yalahar, TibiaWiki revisions 843939 and 1106237.
use super::fixture::{Harness, debug, id, run};
use super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session,
};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef, WorldId};
use crate::gameplay_transport::spell_premium_coordinator::{
    SpellPremiumCoordinator, purchase_fixture,
};
use oteryn_gameplay_quests::quest::loader::load_embedded_quest_state;
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestTransition,
};
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
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("qcx53-errand", || {
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
        super::death_ml_loss_composed_fixtures::production_generation_with_pin(world, "content-1")?;
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
    let mut admitted = deps.admitted;
    owner
        .refresh_quest_session(admitted.game_session_id, true)
        .await;
    let [telas, boveas, cerdras, dreadeye, milos, scutty] =
        super::classic_quests_fixtures::install_with_offsets(
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
    super::qd_world_fixtures::select_talk_capabilities(&owner, &mut admitted, &[3, 6, 25])?;
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
    Ok(())
}

async fn converse(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: EntityRef,
    command: &mut u64,
    keyword: &str,
) -> TestResult {
    super::classic_quests_fixtures::talk(owner, admitted, npc, *command, "hi").await?;
    super::qengine_rewards_fixtures::talk(owner, admitted, npc, *command + 1, keyword).await?;
    super::classic_quests_fixtures::talk(owner, admitted, npc, *command + 2, "bye").await?;
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
    let key = "oteryn:quest-transition/fixture/qcx53-in-service-mission-three";
    let catalogue = owner.quest_catalogue.as_ref().ok_or("quest catalogue")?;
    let fixture = Arc::new(
        catalogue
            .with_fixture_transition(QuestTransition {
                key: key.into(),
                quest: "oteryn:quest.in_service_of_yalahar_quest".into(),
                effects: vec![QuestEffect {
                    track: PREREQUISITE.into(),
                    from: QuestComparison::Eq(0),
                    effect: QuestEffectKind::Set(1),
                }],
                completes: false,
                experience: None,
                world_change: None,
            })
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
                transition_key: key.into(),
                cause: QuestCause::Command(CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(900).map_err(debug)?,
                )),
            },
            fixture,
        )
        .await
        .map_err(debug)?;
    assert!(matches!(result, QuestTransitionOutcome::Committed(_)));
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    Ok(())
}
