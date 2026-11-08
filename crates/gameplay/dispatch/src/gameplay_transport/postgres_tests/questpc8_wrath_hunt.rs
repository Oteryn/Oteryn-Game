//! Selected Wrath stage6: native imperial deaths, exact4/6 quotas, real Zlak.
use super::classic_quests::{progress, talk};
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::map::overlay::TilePos;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestTransition, world_callbacks::Position,
};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;
const QUEST: &str = "oteryn:quest.wrath_of_the_emperor_quest";
const STAGE: &str = "oteryn:quest-progress/chosen/wrath/stage";
const MAG: &str = "oteryn:quest-progress/chosen/wrath/magistrati";
const NOBLE: &str = "oteryn:quest-progress/chosen/wrath/nobles";

#[test]
fn questpc8_wrath_imperial_hunt_native_exact_quotas_and_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("questpc8-wrath-hunt", || {
        super::quest_world_callbacks::scenario(
            "authored-wrath-hunt",
            Position {
                x: 33082,
                y: 31220,
                floor: -7,
            },
            false,
        )
    });
    eprintln!(
        "POSTGRES_CASE_FINISHED questpc8_wrath_imperial_hunt duration={:?} result={result:?}",
        started.elapsed()
    );
    result
}

pub(super) fn require_bindings() -> TestResult {
    eprintln!(
        "POSTGRES_CASE_STARTED questpc8_wrath_imperial_hunt after PostgreSQL fixture initialization"
    );
    assert!(
        oteryn_gameplay_quests::quest::death_objectives::load()?
            .iter()
            .any(|o| o.key == "oteryn:quest-death/wrath-magistratus"),
        "WRATH_IMPERIAL_HUNT_RED: missing native imperial objectives"
    );
    Ok(())
}

pub(super) fn world() -> TestResult<crate::map::boot::BundleWorld> {
    let world_id = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, _) =
        super::death_ml_loss_composed::production_generation_with_pin(world_id, "content-1")?;
    let native = controller
        .active()
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    super::questpc8_native_slice::world(
        native,
        &[
            (33077, 31215, 33087, 31225, -7),
            (33055, 31140, 33090, 31200, -3),
        ],
        &[],
        TilePos {
            x: 33081,
            y: 31220,
            floor: -7,
        },
    )
}

async fn refused(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    npc: EntityRef,
    command: u64,
    text: &str,
) -> TestResult {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let result = owner
        .npc_talk(
            admitted.runtime_actor.ok_or("actor")?,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: command,
                item_fence: Some(fence.item_fence()),
            },
            NpcTalkIntent {
                npc_actor: npc,
                text: text.into(),
            },
        )
        .await;
    assert_eq!(result.disposition, NpcIntentDisposition::Rejected);
    assert!(!result.unknown);
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let npc = super::questpc8_wrath_hunt_npc::install(owner, admitted).await?;
    owner.admit_premium(id(40), admitted.game_session_id);
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    native
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(debug)?;
    let setup = "oteryn:quest-transition/questpc8-fixture-imperial-hunt-ready";
    let catalogue = Arc::new(
        owner
            .quest_catalogue
            .as_ref()
            .ok_or("catalogue")?
            .with_fixture_transition(QuestTransition {
                world_change: None,
                key: setup.into(),
                quest: QUEST.into(),
                effects: vec![QuestEffect {
                    track: STAGE.into(),
                    from: QuestComparison::Any,
                    effect: QuestEffectKind::Set(5),
                }],
                completes: false,
                experience: None,
            })
            .map_err(debug)?,
    );
    owner.quest_catalogue = Some(catalogue.clone());
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    {
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let result = slot
            .commit_quest_transition(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                QuestTransitionRequest {
                    transition_key: setup.into(),
                    cause: QuestCause::Command(CommandRef::new(
                        admitted.game_session_id,
                        CommandId::new(8000).map_err(debug)?,
                    )),
                },
                catalogue,
            )
            .await
            .map_err(debug)?;
        assert!(matches!(result, QuestTransitionOutcome::Committed(_)));
    }
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    talk(owner, admitted, npc, 8001, "hi").await?;
    refused(owner, admitted, npc, 8002, "imperial report").await?;
    talk(owner, admitted, npc, 8003, "imperial hunt").await?;
    assert_eq!(progress(owner, admitted, authority, MAG).await?, 0);
    assert_eq!(progress(owner, admitted, authority, NOBLE).await?, 0);
    super::questpc8_mystery_geometry::arrive(
        owner,
        admitted,
        TilePos {
            x: 33083,
            y: 31217,
            floor: -7,
        },
    )
    .await?;
    super::grizzly_npc::questpc8_campaign::hunt(
        owner,
        admitted,
        "oteryn:creature.lizard_magistratus",
        1,
        8100,
    )
    .await?;
    assert_eq!(
        progress(owner, admitted, authority, MAG).await?,
        0,
        "wrong native floor must not count"
    );
    super::questpc8_mystery_geometry::arrive(
        owner,
        admitted,
        TilePos {
            x: 33079,
            y: 31192,
            floor: -3,
        },
    )
    .await?;
    super::grizzly_npc::questpc8_campaign::hunt(
        owner,
        admitted,
        "oteryn:creature.lizard_magistratus",
        3,
        8101,
    )
    .await?;
    super::grizzly_npc::questpc8_campaign::hunt(
        owner,
        admitted,
        "oteryn:creature.lizard_noble",
        7,
        8102,
    )
    .await?;
    assert_eq!(progress(owner, admitted, authority, MAG).await?, 3);
    assert_eq!(
        progress(owner, admitted, authority, NOBLE).await?,
        6,
        "noble counter saturates at six"
    );
    super::questpc8_mystery_geometry::arrive(
        owner,
        admitted,
        TilePos {
            x: 33081,
            y: 31220,
            floor: -7,
        },
    )
    .await?;
    talk(owner, admitted, npc, 8200, "hi").await?;
    refused(owner, admitted, npc, 8201, "imperial report").await?;
    assert_eq!(
        progress(owner, admitted, authority, STAGE).await?,
        5,
        "ten kills with wrong quotas cannot advance"
    );
    owner
        .states
        .quest_sessions
        .lock()
        .map_err(debug)?
        .remove(&admitted.game_session_id);
    owner.admit_quest_session(admitted).await;
    super::questpc8_mystery_geometry::arrive(
        owner,
        admitted,
        TilePos {
            x: 33079,
            y: 31192,
            floor: -3,
        },
    )
    .await?;
    super::grizzly_npc::questpc8_campaign::hunt(
        owner,
        admitted,
        "oteryn:creature.lizard_magistratus",
        1,
        8103,
    )
    .await?;
    assert_eq!(progress(owner, admitted, authority, MAG).await?, 4);
    super::questpc8_mystery_geometry::arrive(
        owner,
        admitted,
        TilePos {
            x: 33081,
            y: 31220,
            floor: -7,
        },
    )
    .await?;
    talk(owner, admitted, npc, 8202, "hi").await?;
    talk(owner, admitted, npc, 8203, "imperial report").await?;
    assert_eq!(progress(owner, admitted, authority, STAGE).await?, 6);
    for (track, value) in [
        ("questline", 27),
        ("mission09", 1),
        ("interdimensional_potion", 1),
    ] {
        assert_eq!(
            progress(
                owner,
                admitted,
                authority,
                &format!("oteryn:quest-progress/quest/u8_6/wrath_of_the_emperor/{track}")
            )
            .await?,
            value
        );
    }
    talk(owner, admitted, npc, 8203, "imperial report").await?;
    refused(owner, admitted, npc, 8204, "imperial report").await?;
    let receipts:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE character_id=encode($1,'hex')::uuid AND transition_key='oteryn:quest-reward/chosen/wrath/imperial-report'").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(receipts, 1);
    eprintln!("POSTGRES_WRATH_IMPERIAL_NATIVE_EXACT_FOUR_SIX_COLD_REPLAY_PASS");
    Ok(())
}
