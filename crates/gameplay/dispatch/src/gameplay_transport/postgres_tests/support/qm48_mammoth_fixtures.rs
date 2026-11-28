use super::fixture::{Harness, debug};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_world::map::view::MapFacts;

use super::{AdmittedSession, ComposedFreshAdmission, TestResult};

use oteryn_gameplay_quests::quest::{barbarian_mead, world_callbacks::Position};

use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

pub(super) const AT: Position = Position {
    x: 32154,
    y: 31028,
    floor: -5,
};

const MISSION: &str = "oteryn:quest-progress/quest/u8_0/barbarian_test/mission03";

const KEY: &str = "oteryn:quest-callback/qm48/mammoth";

pub(super) const BARREL: Position = Position {
    x: 32201,
    y: 31154,
    floor: -7,
};
pub(super) const BEAR: Position = Position {
    x: 32253,
    y: 31049,
    floor: -5,
};
const EMPTY_HORN: &str = "oteryn:item.tibia.i7140";
const FULL_HORN: &str = "oteryn:item.tibia.i7141";
const HUG: &str = "oteryn:quest-callback/barbarian-test/hug-bear";

pub(super) fn horn_start(position: Position) -> Position {
    if position == BARREL {
        Position {
            x: position.x + 1,
            ..position
        }
    } else {
        Position {
            y: position.y + 1,
            ..position
        }
    }
}

pub(super) fn horn_world(
    native: &crate::content::native_gameplay::NativeGameplayState,
    position: Position,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups_fixtures::{
        ExpectedTile, NativeCrop, native_scene_with_appearances,
    };
    let (bytes, floor, ids): (&[u8], _, _) = if position == BARREL {
        (
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z07-x125-y121.b3"
            )),
            7,
            vec![408, 2321, 7142],
        )
    } else {
        (
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z05-x125-y121.b3"
            )),
            5,
            vec![4559, 4728, 7174],
        )
    };
    let Position { x, y, .. } = position;
    let start = horn_start(position);
    native_scene_with_appearances(
        native,
        vec![NativeCrop {
            bytes,
            region: (floor, 125, 121),
            bounds: (x - 1, y - 1, x + 2, y + 2),
            start: (start.x, start.y),
            expected: vec![ExpectedTile {
                position: (x, y),
                ids,
                counted_entry: None,
            }],
        }],
        &[],
        &[7175],
    )
}

pub(super) fn horn_catalogue(
    original: &oteryn_gameplay_quests::quest::QuestStateCatalogue,
    callback: &oteryn_gameplay_quests::quest::world_callbacks::Callback,
) -> TestResult<oteryn_gameplay_quests::quest::QuestStateCatalogue> {
    use oteryn_gameplay_quests::quest::QuestStateCatalogue;
    let mut tracks = Vec::new();
    for (key, initial) in [
        (barbarian_mead::TRACKS[0], 4),
        (
            "oteryn:quest-progress/quest/u8_0/barbarian_test/mission02",
            1,
        ),
    ] {
        let mut track = original.track(key).ok_or("Barbarian Test track")?.clone();
        track.initial = initial;
        tracks.push(track);
    }
    let keys = [callback.key.as_str(), HUG];
    let transitions = keys
        .into_iter()
        .map(|key| {
            original
                .transition(key)
                .cloned()
                .ok_or("Barbarian Test transition")
        })
        .collect::<Result<_, _>>()?;
    QuestStateCatalogue::new(original.content_revision(), tracks, transitions).map_err(debug)
}

pub(super) async fn exercise_horn(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    position: Position,
) -> TestResult {
    use oteryn_gameplay_quests::quest::world_callbacks::Offering;
    use oteryn_protocol_oteryn::world_object::{
        MapUseTarget, UseIntent, UseTarget, encode_item_use_intent, encode_use_intent,
    };
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "quest fence")?
        .ok_or("quest fence")?;
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[4, 6, 11, 15, 18],
        )
        .ok_or("capabilities")?;
    let item = if position == BARREL {
        EMPTY_HORN
    } else {
        FULL_HORN
    };
    super::quest_world_callbacks_fixtures::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: horn_start(position),
            item: item.into(),
            count: 1,
        },
        951,
        true,
    )
    .await?;
    let source = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|n| n.item_index())
        .and_then(|i| i.definition_ref("Item", item, "definition-r1"))
        .ok_or("horn reference")?;
    let map = owner.bundle_map().ok_or("horn source map")?;
    let key = oteryn_world_bundle::bundle::placement_key(position.floor, position.x, position.y, 2)
        .ok_or("horn placement")?;
    let target = || {
        let mut placement = map.overlay.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        WorldObjectTarget {
            placement,
            expected_revision: map.facts.object_revision(key),
        }
    };
    let use_interval = std::time::Duration::from_millis(250);
    let tile = |point: Position| crate::map::overlay::TilePos {
        x: point.x,
        y: point.y,
        floor: point.floor,
    };
    let cue_cursor = || {
        map.facts
            .movement_effects_since(0)
            .last()
            .map_or(0, |entry| entry.0)
    };
    let assert_cue = |cursor, point, effect| {
        let cues = map.facts.movement_effects_since(cursor);
        assert_eq!(cues.len(), 1, "one cue from the successful horn action");
        assert_eq!((cues[0].1, cues[0].2), (tile(point), effect));
    };
    if position == BEAR {
        let cursor = cue_cursor();
        let before = bear_health(owner, admitted).await?;
        let payload = encode_use_intent(&target()).map_err(debug)?;
        let (next, result, _) = super::item_craft_fixtures::exchange(
            owner,
            *admitted,
            admitted.continuity.next_command_id,
            &payload,
            None,
        )
        .await?;
        *admitted = next;
        assert_eq!(
            result,
            UseDisposition::Committed,
            "bare USE on the awake bear resolves a hit"
        );
        assert!((10..=30).contains(&(before - bear_health(owner, admitted).await?)));
        assert_cue(cursor, horn_start(position), 3);
        tokio::time::sleep(use_interval).await;
    }
    let horn_use = || {
        encode_item_use_intent(&UseIntent {
            target: UseTarget::Item(std::num::NonZeroU64::MIN),
            use_with: None,
            use_with_map_item: Some(MapUseTarget::Object(target())),
            use_with_position: None,
        })
        .map_err(debug)
    };
    let payload = horn_use()?;
    let before_stun = owner.owner_now().get();
    let cursor = cue_cursor();
    let (next, result, _) = super::item_craft_fixtures::exchange(
        owner,
        *admitted,
        admitted.continuity.next_command_id,
        &payload,
        Some(source),
    )
    .await?;
    *admitted = next;
    assert_eq!(
        result,
        UseDisposition::Committed,
        "selected horn at its donor selector"
    );
    assert_cue(cursor, position, if position == BARREL { 13 } else { 32 });
    let appearance = || {
        let placement = map
            .overlay
            .base()
            .tile(position.x, position.y, position.floor)
            .ok_or("bear tile")?;
        let (pos, ordinal) = crate::map::view::placement_of(key).ok_or("bear position")?;
        map.facts
            .base_entry(pos, ordinal, placement.ids()[2])
            .map(|item| item.appearance_id)
            .ok_or("bear appearance")
    };
    if position == BEAR {
        assert_eq!(appearance()?, 7175);
        let due = map.facts.next_movement_decay_time().ok_or("bear decay")?;
        assert!((before_stun + 5_000_000..=owner.owner_now().get() + 5_000_000).contains(&due));
        assert_eq!(map.facts.expire_movement_items(due - 1), 0);
        assert_eq!(appearance()?, 7175);
        assert_eq!(map.facts.expire_movement_items(due), 1);
        assert_eq!(appearance()?, 7174);
        tokio::time::sleep(use_interval).await;
        let (next, result, _) = super::item_craft_fixtures::exchange(
            owner,
            *admitted,
            admitted.continuity.next_command_id,
            &horn_use()?,
            Some(source),
        )
        .await?;
        *admitted = next;
        assert_eq!(result, UseDisposition::Committed);
        let due = map.facts.next_movement_decay_time().ok_or("second bear decay")?;
        tokio::time::sleep(use_interval).await;
        let cursor = cue_cursor();
        let payload = encode_use_intent(&target()).map_err(debug)?;
        let (next, result, _) = super::item_craft_fixtures::exchange(
            owner,
            *admitted,
            admitted.continuity.next_command_id,
            &payload,
            None,
        )
        .await?;
        *admitted = next;
        assert_eq!(
            result,
            UseDisposition::Committed,
            "bare USE hugs the stunned bear"
        );
        assert_cue(cursor, position, 33);
        assert_eq!(appearance()?, 7174);
        assert_eq!(map.facts.expire_movement_items(due), 0, "hug invalidates the stun timer");
        let snapshot = owner
            .root
            .read_character_quest_state(owner.character, fence.character_id)
            .await
            .map_err(debug)?;
        assert_eq!(snapshot.tracks().get(barbarian_mead::TRACKS[0]), Some(&5));
        let achievements: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_account_achievements WHERE achievement_key = 'oteryn:achievement/bearhugger'",
        ).fetch_one(&h.pool).await?;
        assert_eq!(achievements, 1);
        tokio::time::sleep(use_interval).await;
        let before = bear_health(owner, admitted).await?;
        let payload = encode_use_intent(&target()).map_err(debug)?;
        let (next, result, _) = super::item_craft_fixtures::exchange(
            owner,
            *admitted,
            admitted.continuity.next_command_id,
            &payload,
            None,
        )
        .await?;
        *admitted = next;
        assert_eq!(result, UseDisposition::Committed);
        assert!((10..=30).contains(&(before - bear_health(owner, admitted).await?)));
    }
    assert_eq!(
        super::qengine_outfit_stories_fixtures::quantity(h, EMPTY_HORN).await?,
        0
    );
    assert_eq!(
        super::qengine_outfit_stories_fixtures::quantity(h, FULL_HORN).await?,
        1
    );
    Ok(())
}

async fn bear_health(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<u32> {
    let runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    Ok(crate::gameplay_transport::actor_spell::observe_vitals(
        &runtime,
        &states,
        admitted.runtime_actor.ok_or("bear trial actor")?,
        admitted.game_session_id,
    )
    .ok_or("bear trial vitals")?
    .1
    .health)
}

pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups_fixtures::{
        ExpectedTile, NativeCrop, native_scene_with_appearances,
    };
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
    lose_ack: bool,
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
    let target = WorldObjectTarget {
        placement,
        expected_revision: 0,
    };
    let payload = oteryn_protocol_oteryn::world_object::encode_use_intent(&target).map_err(debug)?;
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
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(1))
            .ok_or("world action clock")?;
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
        if lose_ack && command == 4 {
            owner
                .states
                .quest_callbacks
                .interrupt_after_commit
                .store(true, std::sync::atomic::Ordering::Release);
            assert!(
                super::item_craft_fixtures::exchange(
                    owner, *admitted, command, &payload, None
                )
                .await
                .is_err(),
                "the interrupted USE must leave its wire acknowledgement pending"
            );
            let map = owner.bundle_map().ok_or("map")?;
            assert!(owner.has_pending_quest_callback(session));
            let count: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
            )
            .bind(KEY)
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(count, 1, "the push committed before the interruption");
            assert_eq!(map.facts.object_revision(key), 0);
            assert_eq!(map.facts.next_movement_decay_time(), None);
            assert!(owner.reconcile_quest_callbacks(session).await);
            assert!(!owner.has_pending_quest_callback(session));
            assert!(owner.reconcile_quest_callbacks(session).await);
            break;
        }
        let reply = super::quest_world_callbacks_fixtures::wire(
            owner,
            admitted,
            command,
            2,
            payload.clone(),
        )
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
    let rows: Vec<(String, i64)> = sqlx::query_as("SELECT track_key,value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key IN ($2,$3) ORDER BY track_key")
        .bind(super::fixture::id(41).as_slice()).bind(MISSION).bind(barbarian_mead::TRACKS[0]).fetch_all(&h.pool).await?;
    assert_eq!(
        rows,
        vec![(MISSION.into(), 2), (barbarian_mead::TRACKS[0].into(), 7)]
    );
    if !lose_ack {
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
        let replay =
            super::quest_world_callbacks_fixtures::wire(owner, admitted, 4, 2, payload).await?;
        assert_eq!(
            oteryn_protocol_oteryn::world_object::decode_use_result(&replay).map_err(debug)?,
            UseDisposition::Committed
        );
    }
    let map = owner.bundle_map().ok_or("map")?;
    assert_eq!(map.facts.object_revision(key), 1);
    assert_eq!(map.facts.expire_movement_items(owner.owner_now().get()), 0);
    let decay_due = map.facts.next_movement_decay_time().ok_or("mammoth decay")?;
    assert_eq!(map.facts.expire_movement_items(decay_due - 1), 0);
    assert_eq!(map.facts.expire_movement_items(decay_due), 1);
    let (position, ordinal) = crate::map::view::placement_of(key).ok_or("mammoth placement")?;
    assert_eq!(
        map.facts.base_entry(position, ordinal, 7176).map(|item| item.appearance_id),
        Some(7176)
    );
    let effects_before = map.facts.movement_effects_since(0);
    let cue_due = map.facts.next_movement_decay_time().ok_or("mammoth sleep cue")?;
    assert!(cue_due >= decay_due + 55_000_000);
    assert_eq!(map.facts.expire_movement_items(cue_due - 1), 0);
    assert_eq!(map.facts.expire_movement_items(cue_due), 1);
    let effects_after = map.facts.movement_effects_since(0);
    assert_eq!(effects_after.len(), effects_before.len() + 1);
    assert_eq!(
        effects_before.last().map(|&(_, p, effect)| (p, effect)),
        Some((position, 33))
    );
    assert_eq!(effects_after.last().map(|&(_, p, effect)| (p, effect)), Some((position, 33)));
    assert_eq!(map.facts.next_movement_decay_time(), None);
    Ok(())
}

/// Only external NPC progress is arranged: the production callback and its bound
/// transition remain byte-for-byte unchanged, including the live actor admission.
pub(super) fn catalogue(
    original: &oteryn_gameplay_quests::quest::QuestStateCatalogue,
    callback: &oteryn_gameplay_quests::quest::world_callbacks::Callback,
) -> TestResult<oteryn_gameplay_quests::quest::QuestStateCatalogue> {
    use oteryn_gameplay_quests::quest::*;
    let mut tracks = callback
        .storage
        .iter()
        .zip([6, 1])
        .map(|(state, accepted)| {
            let mut track = original
                .track(&state.track)
                .cloned()
                .ok_or("mammoth track")?;
            track.initial = accepted;
            Ok(track)
        })
        .collect::<Result<Vec<_>, &str>>()?;
    tracks.push(QuestTrack {
        key: barbarian_mead::TRACKS[2].into(),
        quest: callback.quest.clone(),
        initial: 9,
        min: 0,
        max: 9,
    });
    QuestStateCatalogue::new(
        original.content_revision(),
        tracks,
        vec![
            original
                .transition(&callback.key)
                .ok_or("mammoth transition")?
                .clone(),
        ],
    )
    .map_err(debug)
}
