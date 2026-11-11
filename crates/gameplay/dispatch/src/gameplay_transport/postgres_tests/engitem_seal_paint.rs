//! Three distinct native seals retain custody, replay and revision-fenced reset.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::FreshAdmissionAuthority;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::{
    ice_islands_seals as source,
    world_callbacks::{Callback, Offering, Position},
};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};

pub(super) const SEALS: [u16; 3] = [122, 120, 121];
pub(super) const START: Position = Position {
    x: 32381,
    y: 31157,
    floor: -7,
};

fn position(seal: u16) -> TestResult<Position> {
    let (x, y, floor) = source::targets::target(seal).ok_or("native seal")?.position;
    Ok(Position { x, y, floor })
}

fn source_appearances(
    map: &crate::map::boot::BundleMap,
    position: Position,
) -> TestResult<Vec<u16>> {
    let tile = map
        .overlay
        .base()
        .tile(position.x, position.y, position.floor)
        .ok_or("seal source tile")?;
    tile.ids()
        .iter()
        .zip(tile.depths())
        .filter(|(_, depth)| **depth == 0)
        .enumerate()
        .map(|(ordinal, (id, _))| {
            map.facts
                .base_entry(
                    crate::map::overlay::TilePos {
                        x: position.x,
                        y: position.y,
                        floor: position.floor,
                    },
                    u8::try_from(ordinal).ok()?,
                    *id,
                )
                .map(|entry| entry.appearance_id)
        })
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| "seal source appearances".into())
}

#[test]
fn engitem_native_seal_paint_preserves_tool_counts_occurrences_and_resets() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("engitem_seal_paint", || {
        super::quest_world_callbacks::scenario(
            "engitem-native-seal-paint",
            position(SEALS[0])?,
            false,
        )
    });
    eprintln!(
        "ENGITEM_PG_EXECUTED seal_paint duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result?;
    assert!(started.elapsed().as_secs_f64() > 0.01);
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    callback: &Callback,
    target: Position,
) -> TestResult {
    use crate::foundation::{CharacterId, CharacterLease};
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        crate::foundation::MovementLocalPosition {
            x: i32::from(START.x),
            y: i32::from(START.y),
            floor: -i16::from(START.floor),
        }
    );
    assert_eq!(
        source_appearances(owner.bundle_map().ok_or("map")?, START)?,
        vec![6685, 4726, 4733],
        "walkable source start has no seal under the player"
    );
    for seal in SEALS {
        let target = position(seal)?;
        assert_eq!(target.floor, START.floor);
        assert_eq!(
            target.x.abs_diff(START.x).max(target.y.abs_diff(START.y)),
            1,
            "every distinct source seal is in carried-use reach"
        );
    }
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(
            actor,
            admitted.game_session_id,
            CharacterLease::new(
                CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[4, 6, 11, 15, 18],
        )
        .ok_or("capabilities")?;
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: target,
            item: "oteryn:item.tibia.i7253".into(),
            count: 1,
        },
        951,
        true,
    )
    .await?;
    predecessor(owner, admitted, h, callback).await?;
    let original: serde_json::Value = sqlx::query_scalar(
        "SELECT jsonb_build_object('item',to_jsonb(i),'custody',to_jsonb(e)) \
         FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) \
         WHERE i.definition_production_key='oteryn:item.tibia.i7253' AND i.lifecycle=1",
    )
    .fetch_one(&h.pool)
    .await?;
    legacy_fresh_refused(owner, admitted, h).await?;
    let reference = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|n| n.item_index())
        .and_then(|i| i.definition_ref("Item", "oteryn:item.tibia.i7253", "definition-r1"))
        .ok_or("paint reference")?;
    let packed = [123_i64, 121 | (123 << 16), 121 | (122 << 16) | (123 << 32)];
    for (phase, seal) in SEALS.into_iter().enumerate() {
        let target = position(seal)?;
        let binding = source::targets::target(seal).ok_or("native seal")?;
        let key = oteryn_world_bundle::bundle::placement_key(
            target.floor,
            target.x,
            target.y,
            binding.ordinal,
        )
        .ok_or("seal key")?;
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        let map = owner.bundle_map().ok_or("map")?;
        assert_eq!(
            source_appearances(map, target)?,
            vec![6685, [4737, 4734, 4736][phase], 7178]
        );
        let mut placement = map.overlay.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let payload = encode_item_use_intent(&UseIntent {
            target: UseTarget::Item(std::num::NonZeroU64::new(3).ok_or("handle")?),
            use_with: None,
            use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
                placement,
                expected_revision: map.facts.object_revision(key),
            })),
            use_with_position: None,
        })
        .map_err(debug)?;
        let command = admitted.continuity.next_command_id;
        let (next, result, actual) =
            super::item_craft::exchange(owner, *admitted, command, &payload, Some(reference))
                .await?;
        *admitted = next;
        assert_eq!(result, UseDisposition::Committed, "source phase {phase}");
        let snapshot = owner
            .root
            .read_character_quest_state(owner.character, fence.character_id)
            .await
            .map_err(debug)?;
        assert_eq!(snapshot.tracks().get(source::TARGETS), Some(&packed[phase]));
        assert_eq!(
            snapshot.tracks().get(source::QUESTLINE),
            Some(&if phase < 2 { 8 } else { 9 })
        );
        assert_eq!(
            snapshot.tracks().get(source::MISSION04),
            Some(&if phase < 2 { 1 } else { 2 })
        );
        let (id, due) = {
            let map = owner.bundle_map().ok_or("map")?;
            let id = map
                .overlay
                .base()
                .tile(target.x, target.y, target.floor)
                .ok_or("tile")?
                .ids()[2];
            assert_eq!(
                map.facts
                    .base_entry(
                        crate::map::overlay::TilePos {
                            x: target.x,
                            y: target.y,
                            floor: target.floor
                        },
                        2,
                        id
                    )
                    .ok_or("seal")?
                    .appearance_id,
                7252
            );
            let due = map.facts.next_movement_decay_time().ok_or("source reset")?;
            assert_eq!(
                map.facts.expire_movement_items(due - 1),
                0,
                "no shortened production reset"
            );
            (id, due)
        };
        let (next, replay, _) =
            super::item_craft::exchange(owner, *admitted, command, &actual, None).await?;
        *admitted = next;
        assert_eq!(replay, UseDisposition::Committed, "exact receipt replay");
        if phase < 2 {
            let retained: serde_json::Value = sqlx::query_scalar(
                "SELECT jsonb_build_object('item',to_jsonb(i),'custody',to_jsonb(e)) \
                 FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) \
                 WHERE i.definition_production_key='oteryn:item.tibia.i7253' AND i.lifecycle=1",
            )
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(
                original, retained,
                "a retained tool is neither burned nor reminted"
            );
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_secs(2))
                .ok_or("clock")?;
            let mut repeated = oteryn_protocol_oteryn::world_object::decode_use_intent_target(
                &[4, 11, 15],
                &actual,
            )
            .map_err(debug)?;
            if let Some(MapUseTarget::Object(t)) = &mut repeated.use_with_map_item {
                t.expected_revision = owner.bundle_map().ok_or("map")?.facts.object_revision(key);
            }
            let bytes = encode_item_use_intent(&repeated).map_err(debug)?;
            let (next, refused, _) = super::item_craft::exchange(
                owner,
                *admitted,
                admitted.continuity.next_command_id,
                &bytes,
                Some(reference),
            )
            .await?;
            *admitted = next;
            assert_ne!(
                refused,
                UseDisposition::Committed,
                "the already painted occurrence cannot pay twice"
            );
            let after = owner
                .root
                .read_character_quest_state(owner.character, fence.character_id)
                .await
                .map_err(debug)?;
            assert_eq!(after.tracks(), snapshot.tracks());
        }
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(31))
            .ok_or("clock")?;
        let map = owner.bundle_map().ok_or("map")?;
        assert!(owner.owner_now().get() >= due);
        assert_eq!(
            map.facts.expire_movement_items(owner.owner_now().get()),
            1,
            "normal revision-fenced reset owner"
        );
        assert_eq!(
            map.facts
                .base_entry(
                    crate::map::overlay::TilePos {
                        x: target.x,
                        y: target.y,
                        floor: target.floor
                    },
                    2,
                    id
                )
                .ok_or("restored seal")?
                .appearance_id,
            7178
        );
        if phase < 2 {
            // A physical 30-second reset does not erase personal distinct-target credit.
            let mut repeated = oteryn_protocol_oteryn::world_object::decode_use_intent_target(
                &[4, 11, 15],
                &actual,
            )
            .map_err(debug)?;
            if let Some(MapUseTarget::Object(t)) = &mut repeated.use_with_map_item {
                t.expected_revision = map.facts.object_revision(key);
            }
            let bytes = encode_item_use_intent(&repeated).map_err(debug)?;
            let (next, refused, _) = super::item_craft::exchange(
                owner,
                *admitted,
                admitted.continuity.next_command_id,
                &bytes,
                Some(reference),
            )
            .await?;
            *admitted = next;
            assert_eq!(refused, UseDisposition::Rejected, "same seal after reset");
            let after = owner
                .root
                .read_character_quest_state(owner.character, fence.character_id)
                .await
                .map_err(debug)?;
            assert_eq!(after.tracks(), snapshot.tracks());
        }
    }
    let snapshot = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(snapshot.tracks().get(source::QUESTLINE), Some(&9));
    assert_eq!(snapshot.tracks().get(source::MISSION04), Some(&2));
    let counts: (i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1), \
         (SELECT count(*) FROM game_quest_callback_consumptions), \
         (SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i7253' AND lifecycle=1)",
    )
    .bind(source::QUEST)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        counts,
        (3, 1, 0),
        "three distinct quest receipts, one final source-tool BURN"
    );
    let atomic: bool = sqlx::query_scalar(
        "SELECT count(*)=1 AND bool_and(c.xmin=q.xmin) \
         FROM game_quest_callback_consumptions c \
         JOIN game_character_quest_receipts q USING(character_id,cause_id,cause_ordinal,transition_key) \
         WHERE q.quest_key=$1",
    )
    .bind(source::QUEST)
    .fetch_one(&h.pool)
    .await?;
    assert!(
        atomic,
        "final custody BURN and personal completion share one transaction"
    );
    Ok(())
}

async fn legacy_fresh_refused(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
) -> TestResult {
    use crate::durability::quest_state::{
        QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
    };
    use crate::durability::quest_world_callback::QuestWorldContext;
    use crate::foundation::{CommandId, CommandRef};
    use oteryn_gameplay_quests::quest::{QuestRefusal, ice_seal_paint};

    let callback = ice_seal_paint::callbacks()?
        .into_iter()
        .next()
        .ok_or("legacy seal callback")?;
    let binding = callback
        .selector
        .native
        .as_ref()
        .ok_or("legacy seal binding")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "legacy fence")?
        .ok_or("legacy fence")?;
    let actor = admitted.runtime_actor.ok_or("legacy actor")?;
    let map = owner.bundle_map().ok_or("legacy map")?;
    let position = binding.position;
    let key = oteryn_world_bundle::bundle::placement_key(
        position.floor,
        position.x,
        position.y,
        binding.ordinal,
    )
    .ok_or("legacy seal key")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let intent = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::new(3).ok_or("legacy tool handle")?),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
            placement,
            expected_revision: map.facts.object_revision(key),
        })),
        use_with_position: None,
    })
    .map_err(debug)?;
    let selected: Vec<u8> = sqlx::query_scalar(
        "SELECT uuid_send(i.item_instance_id) FROM game_item_instances i \
         JOIN game_item_container_entries e USING(item_instance_id) \
         WHERE i.definition_production_key='oteryn:item.tibia.i7253' AND i.lifecycle=1",
    )
    .fetch_one(&h.pool)
    .await?;
    let selected = selected.try_into().map_err(|_| "legacy selected UUID")?;
    let permit = owner.spell_lane_permit().await.ok_or("legacy lane")?;
    let context = {
        let runtime = owner.runtime.lock().await;
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
            placement_context: runtime.content_pin().frame_binding_digest().to_vec(),
            reset_epoch: owner.reset_epoch,
            actor_placement: actor.placement_identity(),
            movement_revision: runtime
                .read_actor_position(actor)
                .map_err(debug)?
                .revision(),
            party: Vec::new(),
        }
        .with_selected_tool(selected, intent)
        .ok_or("legacy tool context")?
    };
    let before = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let result = slot
        .commit_world_callback_with_items_selected(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: callback.key,
                cause: QuestCause::Use(CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(952).map_err(debug)?,
                )),
            },
            std::sync::Arc::clone(owner.quest_catalogue.as_ref().ok_or("catalogue")?),
            &permit,
            context,
            owner
                .active_generation
                .and_then(|active| active.native_gameplay()),
            None,
            &crate::durability::quest_state::grizzly::NoGrizzlyPremium,
        )
        .await
        .map_err(debug)?;
    assert!(
        matches!(
            result,
            QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
        ),
        "legacy receipt identity cannot authorize fresh repeated-seal progress: {result:?}"
    );
    drop(slot);
    drop(permit);
    let after = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(after.tracks(), before.tracks());
    let writes: (i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1), \
         (SELECT count(*) FROM game_quest_world_tool_reads), \
         (SELECT count(*) FROM game_quest_callback_consumptions)",
    )
    .bind(source::QUEST)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(writes, (0, 0, 0));
    Ok(())
}

async fn predecessor(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
    callback: &Callback,
) -> TestResult {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    // Source state after the Ice Islands assignment is the fixture boundary. The
    // final action itself uses the production framed command and receipt owner.
    // Seed only the persisted predecessor snapshot, as the established Grizzly
    // fixtures do. The protected world transition is never replaced or bypassed.
    {
        use crate::durability::quest_state::{
            QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
        };
        use crate::foundation::{CommandId, CommandRef};
        use oteryn_gameplay_quests::quest::{
            QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack,
            QuestTransition,
        };
        let key = "oteryn:quest-transition/engitem-paint-fixture-prior-journey";
        let track = "oteryn:quest-progress/engitem-paint-fixture-prior-journey";
        let quest = "oteryn:quest.engitem_paint_fixture_prior_journey";
        let setup = std::sync::Arc::new(
            QuestStateCatalogue::new(
                "content-1",
                vec![QuestTrack {
                    key: track.into(),
                    quest: quest.into(),
                    initial: 0,
                    min: 0,
                    max: 1,
                }],
                vec![QuestTransition {
                    key: key.into(),
                    quest: quest.into(),
                    world_change: None,
                    effects: vec![QuestEffect {
                        track: track.into(),
                        from: QuestComparison::Any,
                        effect: QuestEffectKind::Set(1),
                    }],
                    completes: false,
                    experience: None,
                }],
            )
            .map_err(debug)?,
        );
        let current = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|_| "fence")?
            .ok_or("fence")?;
        let mut slot = owner.revision_sequencer.acquire(current.character_id).await;
        let outcome = slot
            .commit_quest_transition(
                owner.root,
                owner.character,
                owner.holder,
                current,
                QuestTransitionRequest {
                    transition_key: key.into(),
                    cause: QuestCause::Command(CommandRef::new(
                        admitted.game_session_id,
                        CommandId::new(902).map_err(debug)?,
                    )),
                },
                setup,
            )
            .await
            .map_err(debug)?;
        assert!(matches!(outcome, QuestTransitionOutcome::Committed(_)));
    }
    let catalogue = owner.quest_catalogue.as_ref().ok_or("catalogue")?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for (track, value) in [
        (source::QUESTLINE, 8_i64),
        (source::TARGETS, 0),
        (source::MISSION04, 1),
        (oteryn_gameplay_quests::quest::ice_seal_paint::PAINT, -1),
    ] {
        sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT character_id,$2,$3,$4,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid ON CONFLICT(character_id,track_key) DO UPDATE SET value=EXCLUDED.value,committed_character_revision=EXCLUDED.committed_character_revision")
            .bind(fence.character_id.as_bytes().as_slice()).bind(track).bind(&callback.quest).bind(value).execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO game_character_quest_states(character_id,quest_key,pinned_content_revision,definition_hash,started_character_revision,committed_character_revision) SELECT character_id,$2,$3,$4,character_revision,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid ON CONFLICT(character_id,quest_key) DO UPDATE SET pinned_content_revision=EXCLUDED.pinned_content_revision,definition_hash=EXCLUDED.definition_hash")
        .bind(fence.character_id.as_bytes().as_slice()).bind(&callback.quest).bind(catalogue.content_revision()).bind(catalogue.definition_hash(&callback.quest).ok_or("quest hash")?.as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    Ok(())
}
