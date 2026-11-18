//! The carried replica must pass real USE admission before a pillar can spawn.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CharacterId, CharacterLease, CommandId, CommandRef};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestTransition,
    world_callbacks::{Offering, Position},
    wrath_sleeping_dragon as quest,
};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};
use std::sync::Arc;

pub(super) const NAME: &str = "wrath/incarnation-3193";
const REPLICA: &str = "oteryn:item.tibia.i11362";
const BOSS_STATUS: &str = "oteryn:quest-progress/quest/u8_6/wrath_of_the_emperor/boss_status";
const FURY: &str = "oteryn:creature.fury_of_the_emperor";

#[test]
fn wrath_incarnation_selected_replica_spawns_and_death_reopens_pillar_postgres() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("wrath-incarnation", || {
        super::quest_world_callbacks::scenario(
            NAME,
            Position { x: 33049, y: 31089, floor: -15 },
            false,
        )
    })
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    at: Position,
) -> TestResult {
    let setup = "oteryn:quest-transition/wrath-incarnation-fixture";
    let catalogue = Arc::new(owner.quest_catalogue.as_ref().ok_or("catalogue")?
        .with_fixture_transition(QuestTransition {
            world_change: None,
            key: setup.into(),
            quest: quest::QUEST.into(),
            effects: [(quest::QUESTLINE, 30), (BOSS_STATUS, 1)]
                .into_iter()
                .map(|(track, value)| QuestEffect {
                    track: track.into(),
                    from: QuestComparison::Any,
                    effect: QuestEffectKind::Set(value),
                })
                .collect(),
            completes: false,
            experience: None,
        }).map_err(debug)?);
    owner.quest_catalogue = Some(catalogue.clone());
    let fence = owner.current_quest_fence(admitted.game_session_id).await.map_err(|()| "fence")?.ok_or("fence")?;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    owner.runtime.lock().await.bind_attacker_lease(
        actor,
        admitted.game_session_id,
        CharacterLease::new(CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?, fence.character_lease_generation).map_err(debug)?,
    ).map_err(debug)?;
    let result = owner.revision_sequencer.acquire(fence.character_id).await.commit_quest_transition(
        owner.root, owner.character, owner.holder, fence,
        QuestTransitionRequest {
            transition_key: setup.into(),
            cause: QuestCause::Command(CommandRef::new(admitted.game_session_id, CommandId::new(9000).map_err(debug)?)),
        },
        catalogue,
    ).await.map_err(debug)?;
    assert!(matches!(result, QuestTransitionOutcome::Committed(_)));
    owner.refresh_quest_session(admitted.game_session_id, false).await;
    let fence = owner.current_quest_fence(admitted.game_session_id).await.map_err(|()| "fence")?.ok_or("fence")?;
    super::quest_world_callbacks::input_item(owner, fence.item_fence(), &Offering {
        position: at, item: REPLICA.into(), count: 1,
    }, 9001, true).await?;
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(), &[4, 6, 11, 15, 18],
        ).ok_or("capabilities")?;
    let native = owner.active_generation.and_then(|g| g.native_gameplay()).ok_or("native")?;
    let reference = native.item_index().and_then(|index| index.definition_ref("Item", REPLICA, "definition-r1")).ok_or("replica definition")?;
    let key = oteryn_world_bundle::bundle::placement_key(at.floor, at.x, at.y, 1).ok_or("pillar")?;
    let map = owner.bundle_map().ok_or("map")?;
    let initial = map.facts.object_revision(key);
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::MIN),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget { placement, expected_revision: initial })),
        use_with_position: None,
    }).map_err(debug)?;
    let (next, disposition, _) = super::item_craft::exchange(
        owner, *admitted, admitted.continuity.next_command_id, &payload, Some(reference),
    ).await?;
    *admitted = next;
    assert_eq!(disposition, UseDisposition::Committed);
    assert!(owner.bundle_map().ok_or("map")?.facts.object_revision(key) > initial);
    let matching = {
        let runtime = owner.runtime.lock().await;
        runtime.current_live_creature_census().map_err(debug)?.into_iter().filter(|actor| {
            runtime.matches_live_creature_identity(*actor, FURY.as_bytes())
        }).collect::<Vec<_>>()
    };
    assert_eq!(matching.len(), 1);
    super::engenc2_cults::kill(owner, admitted, matching[0], FURY, 9002).await?;
    assert!(owner.runtime.lock().await.current_live_creature_census().map_err(debug)?.is_empty());
    let map = owner.bundle_map().ok_or("map")?;
    assert!(map.facts.object_revision(key) >= initial + 2, "death restored the pillar");
    assert_eq!(
        owner.drain_kill_rewards(admitted.game_session_id).await,
        crate::gameplay_transport::kill_reward::DrainOutcome::Drained
    );
    let (replica, remains): (Vec<u8>, Vec<u8>) = sqlx::query_as(
        "SELECT uuid_send(tool.item_instance_id), uuid_send(body.item_instance_id)
         FROM game_item_instances tool CROSS JOIN game_item_instances body
         JOIN game_item_ground_locations g ON g.item_instance_id=body.item_instance_id
         WHERE tool.definition_production_key=$1 AND tool.lifecycle=1
           AND body.definition_production_key='oteryn:item.tibia.i11361' AND body.lifecycle=1",
    ).bind(REPLICA).fetch_one(&h.pool).await?;
    super::questpc8_mystery_geometry::arrive(owner, admitted, crate::map::overlay::TilePos {
        x: at.x + 3, y: at.y, floor: at.floor,
    }).await?;
    let fence = owner.current_quest_fence(admitted.game_session_id).await.map_err(|()| "fence")?.ok_or("fence")?;
    let use_remains = UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::MIN),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Item(std::num::NonZeroU64::MIN)),
        use_with_position: None,
    };
    let outcome = owner.use_carried_quest_item(
        actor,
        crate::gameplay_transport::connection::UseCommand {
            game_session_id: admitted.game_session_id,
            command_id: 9003,
            item_fence: Some(fence.item_fence()),
        },
        &use_remains,
        Some(crate::gameplay_transport::item_view::ItemKey::Instance(replica.try_into().map_err(|_| "replica id")?)),
        Some(remains.try_into().map_err(|_| "remains id")?),
    ).await.map_err(|()| "remains use")?.ok_or("remains route")?;
    assert_eq!(outcome.disposition, UseDisposition::Committed);
    let samples: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_quest_corpse_samples WHERE transition_key='oteryn:quest-reward/wrath/banish-fury'",
    ).fetch_one(&h.pool).await?;
    assert_eq!(samples, 1);
    let replicas: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1",
    ).bind(REPLICA).fetch_one(&h.pool).await?;
    assert_eq!(replicas, 1);
    Ok(())
}
