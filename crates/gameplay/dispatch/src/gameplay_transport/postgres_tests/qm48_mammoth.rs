//! M48: real native mammoth USE reads the live actor's Drunk condition.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

const AT: Position = Position {
    x: 32154,
    y: 31028,
    floor: -5,
};
const KEY: &str = "oteryn:quest-callback/qm48/mammoth";

#[test]
fn qm48_mammoth_live_drunk_refusal_expiry_cleanse_commit_replay() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("qm48-mammoth", || {
        super::quest_world_callbacks::scenario("qm48/mammoth", AT, false)
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED qm48_mammoth duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_scene_with_appearances};
    native_scene_with_appearances(
        native,
        vec![NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z05-x125-y121.b3"
            )),
            region: (5, 125, 121),
            bounds: (32152, 31026, 32156, 31030),
            start: (32154, 31027),
            expected: vec![ExpectedTile {
                position: (32154, 31028),
                ids: vec![6592, 7176],
                counted_entry: None,
            }],
        }],
        &[],
        &[7177],
    )
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use oteryn_gameplay_domain::domain::condition::{ConditionType, ConditionValues, StatusKind};
    use oteryn_gameplay_foundation::foundation::condition::{
        ApplicationFacts, ConditionDefinition, ConditionSourceKind,
    };
    use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};
    let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("POSTGRES_DATABASE_READY qm48_mammoth version={version}");
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let session = admitted.game_session_id;
    // Arrange only Sven's external accepted-mission state through the typed writer.
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let seeded = slot
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            crate::durability::quest_state::QuestTransitionRequest {
                transition_key: "oteryn:quest-transition/fixture-qm48-accepted".into(),
                cause: crate::durability::quest_state::QuestCause::Command(
                    crate::foundation::CommandRef::new(
                        session,
                        crate::foundation::CommandId::new(900).map_err(debug)?,
                    ),
                ),
            },
            owner.quest_catalogue.as_ref().ok_or("catalogue")?.clone(),
        )
        .await
        .map_err(debug)?;
    assert!(
        matches!(
            seeded,
            crate::durability::quest_state::QuestTransitionOutcome::Committed(_)
        ),
        "{seeded:?}"
    );
    drop(slot);
    let key =
        oteryn_world_bundle::bundle::placement_key(AT.floor, AT.x, AT.y, 1).ok_or("mammoth key")?;
    let mut placement = owner
        .bundle_map()
        .ok_or("map")?
        .overlay
        .base()
        .digest()
        .to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = oteryn_protocol_oteryn::world_object::encode_use_intent(&WorldObjectTarget {
        placement,
        expected_revision: 0,
    })
    .map_err(debug)?;
    let apply = |now, duration_ms| -> TestResult<_> {
        Ok((
            ConditionDefinition::new(
                "qm48.mead.drunk",
                1,
                ConditionValues::TimedStatus {
                    kind: StatusKind::Drunk,
                    duration_ms,
                },
            )
            .ok_or("definition")?,
            now,
        ))
    };
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(3))
        .ok_or("clock")?;
    for command in 1..=4 {
        if command > 1 {
            let now = owner.owner_now().get();
            let (definition, at) = apply(
                if command == 2 {
                    now.saturating_sub(2_000_000)
                } else {
                    now
                },
                if command == 2 { 1_000 } else { 10_000 },
            )?;
            let root = GameplayDecisionRoot::from_bytes([7; 32]);
            let runtime = owner.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            let conditions = &mut states
                .get_mut(&runtime, actor, session)
                .ok_or("state")?
                .conditions;
            conditions
                .apply(
                    &definition,
                    Some("mead".into()),
                    ConditionSourceKind::SelfUse,
                    &[],
                    &ApplicationFacts {
                        now: at,
                        base_speed: 220,
                        mana_shield_capacity: 0,
                        target_reentry_protected: false,
                        source_reentry_protected: false,
                        target_is_player: true,
                        decision_root: &root,
                        occurrence: DecisionOccurrenceId::from_bytes([u8::try_from(command)?; 16]),
                    },
                )
                .map_err(debug)?;
            if command == 3 {
                conditions.remove_type(ConditionType::Status(StatusKind::Drunk));
            }
        }
        let reply =
            super::quest_world_callbacks::wire(owner, admitted, command, 2, payload.clone())
                .await?;
        let result =
            oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?;
        assert_eq!(
            result,
            if command == 4 {
                UseDisposition::Committed
            } else {
                UseDisposition::Rejected
            },
            "command {command}"
        );
        let receipts: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
        )
        .bind(KEY)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(receipts, i64::from(command == 4));
    }
    let rows:Vec<(String,i64)> = sqlx::query_as("SELECT track_key,value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key IN ('oteryn:quest-progress/storage/41174','oteryn:quest-progress/storage/41177') ORDER BY track_key").bind(super::fixture::id(41).as_slice()).fetch_all(&h.pool).await?;
    assert_eq!(rows.iter().map(|r| r.1).collect::<Vec<_>>(), vec![7, 2]);
    let runtime = owner.runtime.lock().await;
    owner
        .states
        .spell_states
        .lock()
        .await
        .get_mut(&runtime, actor, session)
        .ok_or("state")?
        .conditions
        .cleanse_negative();
    drop(runtime);
    let replay = super::quest_world_callbacks::wire(owner, admitted, 4, 2, payload).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&replay).map_err(debug)?,
        UseDisposition::Committed
    );
    let map = owner.bundle_map().ok_or("map")?;
    assert_eq!(map.facts.object_revision(key), 1);
    assert_eq!(map.facts.expire_movement_items(owner.owner_now().get()), 0);
    assert_eq!(
        map.facts
            .expire_movement_items(owner.owner_now().get() + 5_000_000),
        1
    );
    Ok(())
}

/// A fixture adds only the source NPC's accepted state; the actual action stays production.
pub(super) fn catalogue(
    original: &oteryn_gameplay_quests::quest::QuestStateCatalogue,
    callback: &oteryn_gameplay_quests::quest::world_callbacks::Callback,
) -> TestResult<oteryn_gameplay_quests::quest::QuestStateCatalogue> {
    use oteryn_gameplay_quests::quest::*;
    let mut tracks = callback
        .storage
        .iter()
        .map(|s| original.track(&s.track).cloned().ok_or("mammoth track"))
        .collect::<Result<Vec<_>, _>>()?;
    tracks.push(QuestTrack {
        key: "oteryn:quest-progress/storage/41179".into(),
        quest: callback.quest.clone(),
        initial: 0,
        min: 0,
        max: 10,
    });
    let seed = QuestTransition {
        world_change: None,
        key: "oteryn:quest-transition/fixture-qm48-accepted".into(),
        quest: callback.quest.clone(),
        effects: tracks
            .iter()
            .zip([6, 1, 10])
            .map(|(t, value)| QuestEffect {
                track: t.key.clone(),
                from: QuestComparison::Any,
                effect: QuestEffectKind::Set(value),
            })
            .collect(),
        completes: false,
        experience: None,
    };
    QuestStateCatalogue::new(
        original.content_revision(),
        tracks,
        vec![
            original
                .transition(&callback.key)
                .ok_or("mammoth transition")?
                .clone(),
            seed,
        ],
    )
    .map_err(debug)
}
