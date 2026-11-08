//! Real framed carried USE on the source Ram map object, with atomic PG state.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::{CharacterId, CharacterLease};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Offering, Position};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};

#[test]
fn srv7_quest_turnips_never_enter_the_unguarded_generic_taming_owner() {
    assert!(
        crate::content::native_gameplay::mount_taming::rules()
            .iter()
            .all(|rule| rule.item_key != "oteryn:item.tibia.i53073")
    );
}

#[test]
fn srv7_reliable_ram_real_map_use_guards_grants_and_replays() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("srv7_ram", || {
        super::quest_world_callbacks::scenario(
            "srv7-reliable-ram",
            Position {
                x: 31926,
                y: 31896,
                floor: -7,
            },
            false,
        )
    });
    eprintln!(
        "SRV7_PG_EXECUTED reliable_ram duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    assert!(started.elapsed().as_secs_f64() > 0.01);
    result
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    callback: &Callback,
    target: Position,
) -> TestResult {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    let actor = admitted.runtime_actor.ok_or("actor")?;
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
    let source = callback.item_on_target.as_ref().ok_or("turnips")?;
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: target,
            item: source.item.clone(),
            count: 1,
        },
        901,
        true,
    )
    .await?;
    let map = owner.bundle_map().ok_or("map")?;
    let key = oteryn_world_bundle::bundle::placement_key(target.floor, target.x, target.y, 1)
        .ok_or("ram key")?;
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
    let reference = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|n| n.item_index())
        .and_then(|index| index.definition_ref("Item", &source.item, "definition-r1"))
        .ok_or("turnip definition")?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let (next, outcome, _) = super::item_craft::exchange(
        owner,
        *admitted,
        admitted.continuity.next_command_id,
        &payload,
        Some(reference),
    )
    .await?;
    *admitted = next;
    assert_ne!(
        outcome,
        UseDisposition::Committed,
        "unearned turnips cannot tame"
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_appearance_unlocks WHERE unlock_key='oteryn:content.mount.reliable_ram/quest-grant'").fetch_one(&h.pool).await?;
    assert_eq!(count, 0);
    let (remaining, consumed, achievement): (i64,i64,i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i53073' AND lifecycle=1), (SELECT count(*) FROM game_quest_callback_consumptions), (SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/a_reliable_friend')").fetch_one(&h.pool).await?;
    assert_eq!(
        (remaining, consumed, achievement),
        (1, 0, 0),
        "refusal changes no value participant"
    );
    // Source state after Emiliana's real reward is the fixture boundary. The
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
        let key = "oteryn:quest-transition/srv7-fixture-prior-journey";
        let track = "oteryn:quest-progress/srv7-fixture-prior-journey";
        let quest = "oteryn:quest.srv7_fixture_prior_journey";
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
    for storage in &callback.storage {
        sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT character_id,$2,$3,$4,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid ON CONFLICT(character_id,track_key) DO UPDATE SET value=EXCLUDED.value,committed_character_revision=EXCLUDED.committed_character_revision")
            .bind(fence.character_id.as_bytes().as_slice()).bind(&storage.track).bind(&callback.quest).bind(storage.minimum).execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO game_character_quest_states(character_id,quest_key,pinned_content_revision,definition_hash,started_character_revision,committed_character_revision) SELECT character_id,$2,$3,$4,character_revision,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid ON CONFLICT(character_id,quest_key) DO UPDATE SET pinned_content_revision=EXCLUDED.pinned_content_revision,definition_hash=EXCLUDED.definition_hash")
        .bind(fence.character_id.as_bytes().as_slice()).bind(&callback.quest).bind(catalogue.content_revision()).bind(catalogue.definition_hash(&callback.quest).ok_or("quest hash")?.as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let command = admitted.continuity.next_command_id;
    let (next, outcome, actual) =
        super::item_craft::exchange(owner, *admitted, command, &payload, Some(reference)).await?;
    *admitted = next;
    assert_eq!(
        outcome,
        UseDisposition::Committed,
        "earned turnip on source Ram"
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_appearance_unlocks WHERE unlock_key='oteryn:content.mount.reliable_ram/quest-grant'").fetch_one(&h.pool).await?;
    assert_eq!(count, 1);
    let snapshot = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(
        snapshot
            .tracks()
            .get(oteryn_gameplay_quests::quest::targuna_ram::MISSION),
        Some(&5)
    );
    assert_eq!(
        snapshot
            .tracks()
            .get(oteryn_gameplay_quests::quest::targuna_ram::RAM),
        Some(&1)
    );
    assert_eq!(
        snapshot
            .tracks()
            .get(oteryn_gameplay_quests::quest::targuna_ram::TURNIPS),
        Some(&1)
    );
    let (remaining, consumed, achievement): (i64,i64,i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i53073' AND lifecycle=1), (SELECT count(*) FROM game_quest_callback_consumptions), (SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/a_reliable_friend')").fetch_one(&h.pool).await?;
    assert_eq!((remaining, consumed, achievement), (0, 1, 1));
    let (next, outcome, _) =
        super::item_craft::exchange(owner, *admitted, command, &actual, None).await?;
    *admitted = next;
    assert_eq!(
        outcome,
        UseDisposition::Committed,
        "historical replay after turnip consumption"
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_appearance_unlock_receipts WHERE unlock_key='oteryn:content.mount.reliable_ram/quest-grant'").fetch_one(&h.pool).await?;
    assert_eq!(count, 1);
    Ok(())
}
