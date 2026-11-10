//! Accepted SW-3 Q1a's actual two-second, per-Instance current-actor consumer.
use super::super::soul_war_scene as scene;
use super::*;
use crate::foundation::GameSessionId;
use crate::map::boot::BundleCollisionIndexExt;
use oteryn_gameplay_encounters::soul_war_taint_teleport::{
    DueFacts, TaintTeleportEffect, TaintTeleportMagic, TaintTeleportOwner, TeleportCandidate,
};

type Binding = (
    [u8; 32],
    [u8; 32],
    [u8; 32],
    [u8; 32],
    u64,
    crate::foundation::RuntimeScopeRefV1,
    crate::foundation::ScopeOwnershipGeneration,
);
#[derive(Debug, Default)]
pub(super) struct TaintOne {
    binding: Option<Binding>,
    owner: TaintTeleportOwner,
    actors: Vec<(u64, ExactActorRef, Option<GameSessionId>)>,
    next_check: Option<u64>,
    sequence: u32,
}
impl TaintOne {
    pub(super) fn at_birth(now: u64) -> Self {
        Self {
            next_check: now.checked_add(2_000_000),
            ..Default::default()
        }
    }
    pub(super) fn next_due(&self) -> Option<u64> {
        self.next_check
            .into_iter()
            .chain(
                self.owner
                    .next_pending_due_ms()
                    .and_then(|due| due.checked_mul(1000)),
            )
            .min()
    }
    fn current_metadata_actor(
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: Option<GameSessionId>,
    ) -> bool {
        runtime.read_actor_position(actor).is_ok()
            && session.is_none_or(|session| runtime.player_control_facts(actor, session).is_ok())
    }
    fn id(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: Option<GameSessionId>,
    ) -> Option<u64> {
        if !Self::current_metadata_actor(runtime, actor, session) {
            return None;
        }
        if let Some((id, _, _)) = self
            .actors
            .iter()
            .find(|(_, a, s)| *a == actor && *s == session)
        {
            return Some(*id);
        }
        // Metadata follows actual physical ownership. Original timer/cooldown IDs
        // remain in their owner; removing a retired reference cannot rebind them.
        let capacity = runtime.actor_slot_capacity();
        if self.actors.len() >= capacity {
            self.actors.retain(|(_, actor, session)| {
                Self::current_metadata_actor(runtime, *actor, *session)
            });
        }
        if self.actors.len() >= capacity {
            return None;
        }
        self.actors.try_reserve(1).ok()?;
        self.sequence = self.sequence.checked_add(1)?;
        let id = (u64::from(actor.actor_local_id()) << 32) | u64::from(self.sequence);
        self.actors.push((id, actor, session));
        Some(id)
    }
}
pub(super) async fn drive(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    now: u64,
) -> Result<(), ()> {
    drive_due_through(runtime, states, room, native, map, now, now).await
}
#[allow(clippy::too_many_arguments)]
pub(super) async fn drive_due_through(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    eligible_through_us: u64,
    now: u64,
) -> Result<(), ()> {
    let Some(geometry) = states.soul_war_callbacks.taints.geometry(runtime) else {
        states.soul_war_callbacks.taint_one = TaintOne::default();
        return Ok(());
    };
    if !qualifies_source(runtime, native)
        || runtime.content_pin().frame_binding_digest() != room.frame_binding().digest()
        || runtime.map_revision_digest() != room.map_revision_digest()
    {
        states.soul_war_callbacks.taint_one = TaintOne::default();
        return Ok(());
    }
    let binding = (
        runtime.content_pin().server_artifact_digest(),
        runtime.map_revision_digest(),
        runtime.content_pin().frame_binding_digest(),
        runtime.content_pin().client_artifact_digest(),
        runtime.content_pin().activation_sequence(),
        runtime.binding().runtime_scope(),
        runtime.binding().scope_generation(),
    );
    // The caller holds the spell-state guard throughout this pass. Copy only
    // its read facts so the timer owner itself remains in its retained slot
    // across Map awaits, including cancellation of the surrounding future.
    let players = states
        .soul_war_callbacks
        .taints
        .current_player_read(runtime, now)
        .ok_or(())?
        .to_vec();
    let dead_players = states
        .soul_war_callbacks
        .taint_one
        .actors
        .iter()
        .map(|(_, actor, _)| *actor)
        .chain(players.iter().map(|player| player.actor))
        .filter(|actor| states.is_dead(*actor))
        .collect::<Vec<_>>();
    let owner = &mut states.soul_war_callbacks.taint_one;
    if owner.binding.is_some_and(|b| b != binding) {
        *owner = TaintOne::at_birth(now);
    }
    owner.binding = Some(binding);
    drive_current(
        owner,
        runtime,
        &players,
        &dead_players,
        room,
        native,
        map,
        now,
        geometry,
        eligible_through_us,
    )
    .await
}
#[allow(
    clippy::too_many_arguments,
    reason = "one source callback consumes the current actor, Quest, Map and native owners"
)]
async fn drive_current(
    owner: &mut TaintOne,
    runtime: &mut ChannelRuntimeV1,
    players: &[crate::durability::quest_state::soul_war::CurrentSoulWarTaintFacts],
    dead_players: &[ExactActorRef],
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    now: u64,
    geometry: taints::Geometry,
    eligible_through_us: u64,
) -> Result<(), ()> {
    let periodic_eligible = owner
        .next_check
        .is_some_and(|due| due <= now && due <= eligible_through_us);
    let mut candidates = Vec::with_capacity(players.len());
    for player in players {
        if !periodic_eligible {
            continue;
        }
        if dead_players.contains(&player.actor)
            || player.count == 0
            || runtime
                .player_control_facts(player.actor, player.session)
                .is_err()
        {
            continue;
        }
        let Ok(at) = runtime.read_actor_position(player.actor) else {
            continue;
        };
        if !geometry.contains(at.position()) {
            continue;
        }
        let Some(id) = owner.id(runtime, player.actor, Some(player.session)) else {
            continue;
        };
        candidates.push(TeleportCandidate {
            actor: id,
            position: geometry.source_position(at.position()).ok_or(())?,
            has_teleport_taint: true,
            in_boss_zone: true,
        });
        // Delayed landing always reads this current position, not the warning's.
        if map.read(runtime, at.position()).await.is_none() && !map.view().known(at.position()) {
            // Unavailable current facts are not a confirmed blocked/absent tile;
            // retain the original due callback for a later qualified owner pass.
            return Ok(());
        }
    }
    // Source disappearance drops work even when the player has no longer got a
    // current Quest row. Exact references are retained solely for due checks.
    let due_players = owner
        .owner
        .pending_pairs_through(eligible_through_us / 1000, now / 1000)
        .map(|(_, player)| player)
        .collect::<std::collections::BTreeSet<_>>();
    for (id, actor, session) in &owner.actors {
        if !due_players.contains(id) {
            continue;
        }
        if session.is_some_and(|session| runtime.player_control_facts(*actor, session).is_ok())
            && let Ok(at) = runtime.read_actor_position(*actor)
            && map.read(runtime, at.position()).await.is_none()
            && !map.view().known(at.position())
        {
            return Ok(());
        }
    }
    if !map.validate_transaction(runtime).await {
        return Ok(());
    }
    {
        // Expire cooldowns on every real owner turn, including offline targets.
        // advance itself retains later pending moves without re-executing them.
        let actors = &owner.actors;
        let effects = owner.owner.advance_through(
            eligible_through_us / 1000,
            now / 1000,
            |monster, player| {
                let (_, monster, _) = actors.iter().find(|(id, _, _)| *id == monster)?;
                let (_, player, session) = actors.iter().find(|(id, _, _)| *id == player)?;
                let session = (*session)?;
                runtime.player_control_facts(*player, session).ok()?;
                if dead_players.contains(player) {
                    return None;
                }
                if !runtime.contains_live_creature(*monster) {
                    return None;
                }
                let key = source_actor(runtime, native, *monster)?;
                let line =
                    oteryn_gameplay_encounters::soul_war_taint_teleport::teleport_line(&key)?;
                let before = runtime.read_actor_position(*monster).ok()?;
                let target = runtime.read_actor_position(*player).ok()?.position();
                if !map.view().matches(runtime)
                    || map.view().blocks_solid(target) != Some(false)
                    || map.view().blocks_projectile(target) != Some(false)
                {
                    return None;
                }
                // Accepted Q1a permits creatures on the landing tile. The canonical
                // Movement owner still proves the exact actor/current context; the
                // ordinary relocate helper's occupancy rejection is intentionally
                // not this source action's rule.
                let speech =
                    crate::map::facts::MovementSpeech::capture_monster(runtime, *monster, 36, line);
                runtime
                    .borrow_movement_position()
                    .commit_creature_relocation(*monster, before, target)
                    .ok()?;
                if let Some(speech) = speech
                    && let Some(map) = source_map(room, geometry)
                {
                    map.facts.emit_movement_speech(speech);
                }
                Some(DueFacts {
                    monster_position: geometry.source_position(before.position())?,
                    player_position: geometry.source_position(target)?,
                    destination_allows_teleport: true,
                })
            },
            |_| true,
        );
        emit(room, geometry, &effects);
        owner.actors.retain(|(_, actor, session)| {
            TaintOne::current_metadata_actor(runtime, *actor, *session)
        });
    }
    let Ok(census) = runtime.positioned_actor_census() else {
        return Ok(());
    };
    let harvesters = census
        .into_iter()
        .filter_map(|(actor, _, session)| {
            (session.is_none()
                && source_actor(runtime, native, actor).is_some_and(|key| {
                    let allowed = match geometry {
                        taints::Geometry::Boss(_) => key == "oteryn:creature.dreadful_harvester",
                        taints::Geometry::World(_) => key != "oteryn:creature.dreadful_harvester",
                    };
                    allowed
                        && oteryn_gameplay_encounters::soul_war_taint_teleport::teleport_line(&key)
                            .is_some()
                })
                && runtime.contains_live_creature(actor))
            .then_some(actor)
        })
        .collect::<Vec<_>>();
    if harvesters.is_empty() {
        owner.next_check = None;
        return Ok(());
    }
    let due = *owner
        .next_check
        .get_or_insert(now.saturating_add(2_000_000));
    if due > now || due > eligible_through_us {
        return Ok(());
    }
    for actor in harvesters {
        let Ok(at) = runtime.read_actor_position(actor) else {
            continue;
        };
        let Some(id) = owner.id(runtime, actor, None) else {
            continue;
        };
        let occurrence = format!("taint-one:{due}");
        let Some(roll) = attacks::draw(native.source_digest(), actor, &occurrence, 0, 100) else {
            continue;
        };
        let effects = owner
            .owner
            .think(
                id,
                source_actor(runtime, native, actor).as_deref().ok_or(())?,
                geometry.source_position(at.position()).ok_or(())?,
                now / 1000,
                &candidates,
                || roll + 1,
            )
            .map_err(|_| ())?;
        if !effects.is_empty() {
            emit(room, geometry, &effects);
        }
    }
    // Accepted per-Instance timer coalesces a late wakeup, never fabricates
    // missed RNG checks or creature thinks.
    owner.next_check = Some(now.saturating_add(2_000_000));
    Ok(())
}
fn emit(
    room: &QualifiedNativeEntryRoom,
    geometry: taints::Geometry,
    effects: &[TaintTeleportEffect],
) {
    for effect in effects {
        if let TaintTeleportEffect::Magic { position, effect } = effect {
            let number = match effect {
                TaintTeleportMagic::MortArea => 18,
                TaintTeleportMagic::Teleport => 11,
            };
            if let Some(at) = geometry.physical_position(*position)
                && let Some(map) = source_map(room, geometry)
                && let Some(at) = scene::wire_position(at)
            {
                map.facts.emit_movement_effect(at, number);
            }
        }
    }
}

fn source_map(
    room: &QualifiedNativeEntryRoom,
    geometry: taints::Geometry,
) -> Option<&crate::map::boot::BundleMap> {
    match geometry {
        taints::Geometry::Boss(_) => scene::source_map(room),
        taints::Geometry::World(_) => match room.movement_cells().index() {
            crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) => {
                Some(index.map())
            }
            _ => None,
        },
    }
}

#[cfg(test)]
mod metadata_capacity_tests {
    use super::*;
    type TestResult = Result<(), Box<dyn std::error::Error>>;
    fn id(tag: u8) -> [u8; 16] {
        [
            tag, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, tag,
        ]
    }
    #[test]
    fn srx3_taint_one_metadata_accepts_all_current_owned_actors_above_8192() -> TestResult {
        let debug = |e| format!("{e:?}");
        let world = crate::foundation::WorldId::decode(&id(1)).map_err(debug)?;
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            crate::foundation::ChannelId::decode(&id(2)).map_err(debug)?,
            crate::foundation::NodeId::decode(&id(3)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            8193,
            crate::content::channel_content_pin::ChannelContentPin::test(world),
        )?;
        let mut owner = TaintOne::default();
        // These are genuine current fixed-slot refs, not IDs forged as authority.
        // Metadata owns no HP, placement, source policy or gameplay permission.
        for ordinal in 0..8193 {
            let actor = runtime.admit_pinned_test_creature(MovementLocalPosition {
                x: ordinal,
                y: 0,
                floor: 7,
            })?;
            assert!(
                owner.id(&runtime, actor, None).is_some(),
                "current physical capacity is the bound; an unrelated8192 history cap cannot disable actor{ordinal}"
            );
        }
        assert_eq!(runtime.positioned_actor_census()?.len(), 8193);
        Ok(())
    }
    fn metadata_runtime(
        capacity: usize,
        tag: u8,
    ) -> Result<ChannelRuntimeV1, Box<dyn std::error::Error>> {
        let world = crate::foundation::WorldId::decode(&id(tag)).map_err(|e| format!("{e:?}"))?;
        Ok(ChannelRuntimeV1::from_committed_assignment(
            world,
            crate::foundation::ChannelId::decode(&id(2)).map_err(|e| format!("{e:?}"))?,
            crate::foundation::NodeId::decode(&id(3)).map_err(|e| format!("{e:?}"))?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            capacity,
            crate::content::channel_content_pin::ChannelContentPin::test(world),
        )?)
    }
    #[test]
    fn srx3_taint_metadata_keeps_registered_hp0_but_never_rebinds_retired_generation() -> TestResult
    {
        let mut runtime = metadata_runtime(1, 21)?;
        let at = MovementLocalPosition {
            x: 1,
            y: 2,
            floor: 7,
        };
        let first = runtime.admit_pinned_test_creature(at)?;
        let mut owner = TaintOne::default();
        let original = owner
            .id(&runtime, first, None)
            .ok_or("owned actor metadata")?;
        runtime.set_test_creature_health(first, 0)?;
        assert!(!runtime.contains_live_creature(first));
        assert_eq!(owner.id(&runtime, first, None), Some(original));
        runtime.remove_test_actor(first)?;
        assert_eq!(owner.id(&runtime, first, None), None);
        let replacement = runtime.admit_pinned_test_creature(at)?;
        assert_ne!(replacement, first);
        let next = owner
            .id(&runtime, replacement, None)
            .ok_or("replacement metadata")?;
        assert_ne!(
            next, original,
            "original timer IDs must not identify a reused slot"
        );
        assert_eq!(owner.actors.len(), runtime.actor_slot_capacity());
        assert_eq!(owner.actors[0].1, replacement);
        assert_eq!(owner.id(&runtime, first, None), None);
        let other = metadata_runtime(1, 22)?;
        assert_eq!(owner.id(&other, replacement, None), None);
        assert_eq!(owner.actors.len(), 1);
        Ok(())
    }
    #[test]
    fn srx3_taint_metadata_requires_current_player_session() -> TestResult {
        let (mut runtime, actor, session) =
            crate::gameplay_transport::actor_spell::tests::runtime_with_player(91);
        runtime.initialize_movement_test_position(
            actor,
            MovementLocalPosition {
                x: 1,
                y: 2,
                floor: 7,
            },
        )?;
        let mut owner = TaintOne::default();
        let original = owner
            .id(&runtime, actor, Some(session))
            .ok_or("current Player metadata")?;
        let wrong = GameSessionId::decode(&id(92)).map_err(|e| format!("{e:?}"))?;
        assert_ne!(wrong, session);
        assert_eq!(owner.id(&runtime, actor, Some(wrong)), None);
        assert_eq!(owner.id(&runtime, actor, Some(session)), Some(original));
        assert_eq!(owner.actors.len(), 1);
        runtime.remove_test_actor(actor)?;
        assert_eq!(owner.id(&runtime, actor, Some(session)), None);
        Ok(())
    }
}
