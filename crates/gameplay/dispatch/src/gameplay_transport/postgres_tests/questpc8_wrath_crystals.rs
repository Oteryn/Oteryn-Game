//! Chosen four-crystal route uses exact native objects and a retained Snake Sceptre.
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef};
use crate::map::overlay::TilePos;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestTransition,
    world_callbacks::{Offering, Position},
};
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, WorldObjectTarget, decode_use_result, encode_use_intent,
};
use std::sync::Arc;
const QUEST: &str = "oteryn:quest.wrath_of_the_emperor_quest";
const STAGE: &str = "oteryn:quest-progress/chosen/wrath/stage";
const COUNT: &str = "oteryn:quest-progress/chosen/wrath/crystals";
pub(super) const CRYSTALS: [(u16, u16, u16); 4] = [
    (33049, 31089, 1),
    (33095, 31091, 1),
    (33096, 31114, 2),
    (33049, 31115, 1),
];
const APPROACHES: [TilePos; 4] = [
    TilePos {
        x: 33048,
        y: 31089,
        floor: -15,
    },
    TilePos {
        x: 33094,
        y: 31091,
        floor: -15,
    },
    TilePos {
        x: 33097,
        y: 31114,
        floor: -15,
    },
    TilePos {
        x: 33048,
        y: 31115,
        floor: -15,
    },
];
#[test]
fn questpc8_wrath_four_native_crystals_require_tool_order_and_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("questpc8-wrath-crystals", || {
        super::quest_world_callbacks::scenario(
            "authored-wrath-crystals",
            Position {
                x: 33049,
                y: 31089,
                floor: -15,
            },
            false,
        )
    });
    eprintln!(
        "POSTGRES_CASE_FINISHED questpc8_wrath_crystals duration={:?} result={result:?}",
        started.elapsed()
    );
    result
}
pub(super) fn require_bindings() -> TestResult {
    eprintln!(
        "POSTGRES_CASE_STARTED questpc8_wrath_crystals after PostgreSQL fixture initialization"
    );
    assert!(
        oteryn_gameplay_quests::quest::world_callbacks::load()?
            .iter()
            .any(|c| c.key == "oteryn:quest-callback/authored-wrath-crystal-1"),
        "WRATH_CRYSTALS_RED: missing four native crystal callbacks"
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
            (33045, 31085, 33053, 31093, -15),
            (33091, 31087, 33099, 31095, -15),
            (33092, 31110, 33100, 31118, -15),
            (33045, 31111, 33053, 31119, -15),
        ],
        &[],
        APPROACHES[0],
    )
}
fn target(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    index: usize,
) -> TestResult<WorldObjectTarget> {
    let (x, y, ordinal) = CRYSTALS[index];
    let map = owner.bundle_map().ok_or("map")?;
    let mut placement = map.overlay.base().digest().to_vec();
    let key = oteryn_world_bundle::bundle::placement_key(-15, x, y, u8::try_from(ordinal)?)
        .ok_or("crystal placement")?;
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    })
}
async fn use_target(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: u64,
    target: &WorldObjectTarget,
) -> TestResult<UseDisposition> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let bytes = super::quest_world_callbacks::wire(
        owner,
        admitted,
        command,
        2,
        encode_use_intent(target).map_err(debug)?,
    )
    .await?;
    decode_use_result(&bytes).map_err(|e| format!("{e:?}").into())
}
async fn count(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2),0)").bind(id(41).as_slice()).bind(COUNT).fetch_one(&h.pool).await?)
}
pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    owner.admit_premium(id(40), admitted.game_session_id);
    let setup = "oteryn:quest-transition/questpc8-fixture-crystals-ready";
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
                    effect: QuestEffectKind::Set(7),
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
                        CommandId::new(9000).map_err(debug)?,
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
    let first = target(owner, 0)?;
    assert_eq!(
        use_target(owner, admitted, 1, &first).await?,
        UseDisposition::Rejected
    );
    assert_eq!(count(h).await?, 0);
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: Position {
                x: APPROACHES[0].x,
                y: APPROACHES[0].y,
                floor: -15,
            },
            item: "oteryn:item.tibia.i3031".into(),
            count: 1,
        },
        9100,
        true,
    )
    .await?;
    assert_eq!(
        use_target(owner, admitted, 2, &first).await?,
        UseDisposition::Rejected
    );
    assert_eq!(count(h).await?, 0);
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: Position {
                x: APPROACHES[0].x,
                y: APPROACHES[0].y,
                floor: -15,
            },
            item: "oteryn:item.tibia.i11371".into(),
            count: 1,
        },
        9101,
        true,
    )
    .await?;
    let tool:serde_json::Value=sqlx::query_scalar("SELECT to_jsonb(i) FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE e.character_id=encode($1,'hex')::uuid AND i.definition_production_key='oteryn:item.tibia.i11371'").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    super::questpc8_mystery_geometry::arrive(owner, admitted, APPROACHES[1]).await?;
    let wrong_order = target(owner, 1)?;
    assert_eq!(
        use_target(owner, admitted, 3, &wrong_order).await?,
        UseDisposition::Rejected
    );
    assert_eq!(count(h).await?, 0);
    for (index, &approach) in APPROACHES.iter().enumerate() {
        super::questpc8_mystery_geometry::arrive(owner, admitted, approach).await?;
        let object = target(owner, index)?;
        let command = 4 + u64::try_from(index)? * 3;
        assert_eq!(
            use_target(owner, admitted, command, &object).await?,
            UseDisposition::Committed
        );
        assert_eq!(count(h).await?, i64::try_from(index + 1)?);
        assert_eq!(
            use_target(owner, admitted, command, &object).await?,
            UseDisposition::Committed
        );
        assert_eq!(
            use_target(owner, admitted, command + 1, &object).await?,
            UseDisposition::StaleState
        );
        let current = target(owner, index)?;
        assert!(current.expected_revision > object.expected_revision);
        assert_eq!(
            use_target(owner, admitted, command + 2, &current).await?,
            UseDisposition::Rejected
        );
        assert_eq!(count(h).await?, i64::try_from(index + 1)?);
        if index == 1 {
            owner
                .states
                .quest_sessions
                .lock()
                .map_err(debug)?
                .remove(&admitted.game_session_id);
            owner.admit_quest_session(admitted).await;
        }
    }
    assert_eq!(
        super::classic_quests::progress(owner, admitted, authority, STAGE).await?,
        8
    );
    let after:serde_json::Value=sqlx::query_scalar("SELECT to_jsonb(i) FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE e.character_id=encode($1,'hex')::uuid AND i.definition_production_key='oteryn:item.tibia.i11371'").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        tool, after,
        "reusable source sceptre remains byte-identical"
    );
    let receipts:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE character_id=encode($1,'hex')::uuid AND transition_key LIKE 'oteryn:quest-callback/authored-wrath-crystal-%'").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(receipts, 4);
    let completed:bool=sqlx::query_scalar("SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2").bind(id(41).as_slice()).bind(QUEST).fetch_one(&h.pool).await?;
    assert!(
        !completed,
        "four influences do not grant the final quest reward"
    );
    eprintln!("POSTGRES_WRATH_FOUR_NATIVE_CRYSTALS_EXACT_TOOL_ORDER_COLD_REPLAY_PASS");
    Ok(())
}
