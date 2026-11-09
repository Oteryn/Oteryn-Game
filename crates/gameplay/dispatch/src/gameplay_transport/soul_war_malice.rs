//! Malice's source room callbacks; Crystal201189f soul_war.lua:979-1147.
use super::{
    actor_spell::ChannelSpellStates, soul_war_callbacks as shared, soul_war_scene as scene,
};
use crate::content::{
    QualifiedNativeEntryRoom, bosses::activity_template::PinnedActivitySourceRoom,
    native_gameplay::NativeGameplayState,
};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, MovementLocalPosition, ThinkOccurrence};
use crate::movement::monster_map::CurrentMonsterMap;
use oteryn_gameplay_encounters::bosses::{
    goshnars_malice::{MaliceEffect, MaliceRoom},
    rotten_blood::RoomPosition,
};
use oteryn_gameplay_entities::carrier::soul_war::{SoulWarCreatureChange, SoulWarSpawnSource};
use std::sync::Arc;

const BOSS: &str = "oteryn:creature.goshnar_s_malice";
const CAGE: &str = "oteryn:creature.soul_cage";
const CENTER: RoomPosition = RoomPosition {
    x: 33709,
    y: 31599,
    floor: 14,
};
const CAGE_AT: MovementLocalPosition = MovementLocalPosition {
    x: 33709,
    y: 31596,
    floor: 14,
};

#[derive(Debug)]
pub(super) struct MaliceCallbacks {
    boss: ExactActorRef,
    // Immutable continuation provenance. Every use separately proves live scope authority.
    source: Arc<SoulWarSpawnSource>,
    clock: MaliceRoom,
}
impl MaliceCallbacks {
    pub(super) fn initialize(
        runtime: &mut ChannelRuntimeV1,
        native: &NativeGameplayState,
        room: &QualifiedNativeEntryRoom,
        boss: ExactActorRef,
        now_us: u64,
    ) -> Result<Self, ()> {
        if room.pinned_activity_source_room() != Some(PinnedActivitySourceRoom::GoshnarsMalice)
            || !shared::qualifies_source(runtime, native)
            || shared::source_actor(runtime, native, boss).as_deref() != Some(BOSS)
            || runtime.content_pin().frame_binding_digest() != room.frame_binding().digest()
            || runtime.map_revision_digest() != room.map_revision_digest()
            || !runtime.read_actor_position(boss).is_ok_and(|at| {
                at.position()
                    == MovementLocalPosition {
                        x: CENTER.x,
                        y: CENTER.y,
                        floor: CENTER.floor,
                    }
            })
            || scene::source_map(room)
                .and_then(|map| map.facts.movement_item_facts(409))
                .is_none()
        {
            return Err(());
        }
        Ok(Self {
            boss,
            source: Arc::new(
                runtime
                    .qualify_soul_war_spawn_source(boss)
                    .map_err(|_| ())?,
            ),
            clock: MaliceRoom::at_birth(now_us / 1_000),
        })
    }
    pub(super) fn committed_entry(&mut self, now_us: u64, elapsed_ms: u64) {
        self.clock
            .lever_pulled_with_elapsed(now_us / 1_000, elapsed_ms);
    }
    pub(super) fn next_due_ms(&self) -> Option<u64> {
        self.clock.next_due_ms()
    }
}

/// A copied CreatureEvent, independent of ordinary corpse/reward admission.
/// The original death identity also fences a retry after another handler waits.
#[allow(
    clippy::too_many_arguments,
    reason = "copied source handler validates the existing runtime, map role, native body and exact event/death seal together"
)]
pub(super) fn consume_cage_death(
    runtime: &ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    begin: &oteryn_gameplay_entities::carrier::native_source_death::SourceCreatureDeathSeal,
    event: &crate::content::QualifiedNativeDeathEvent,
    now_us: u64,
) -> Result<bool, ()> {
    use sha2::{Digest, Sha256};
    if event.name() != "SoulCageDeath" {
        return Ok(false);
    }
    if !begin.current_for(runtime)
        || !begin.events().iter().any(|copied| copied == event)
        || room.pinned_activity_source_room() != Some(PinnedActivitySourceRoom::GoshnarsMalice)
        || !shared::qualifies_source(runtime, native)
    {
        return Err(());
    }
    let current = runtime
        .companion_snapshot_including_dead(begin.actor())
        .map_err(|_| ())?;
    if current.state.master.is_some() {
        return Ok(true);
    }
    let mut hash = Sha256::new();
    hash.update(b"srx3:soul-cage-copied-death:v1");
    hash.update(begin.actor().corpse_identity());
    let identity: [u8; 32] = hash.finalize().into();
    states
        .soul_war_callbacks
        .malice
        .as_mut()
        .ok_or(())?
        .clock
        .cage_died(identity, now_us / 1_000)
        .map_err(|_| ())?;
    Ok(true)
}

fn cage(runtime: &ChannelRuntimeV1, native: &NativeGameplayState) -> Option<ExactActorRef> {
    runtime
        .positioned_actor_census()
        .ok()?
        .into_iter()
        .find_map(|(actor, _, session)| {
            (session.is_none()
                && shared::source_actor(runtime, native, actor).as_deref() == Some(CAGE))
            .then_some(actor)
        })
}
pub(super) async fn drive(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    occurrences: &[ThinkOccurrence],
    now: u64,
) -> Result<(), ()> {
    if !shared::qualifies_source(runtime, native) || !map.validate_transaction(runtime).await {
        return Err(());
    }
    let owner = states.soul_war_callbacks.malice.as_ref().ok_or(())?;
    if !runtime.soul_war_white_tile_source_current(&owner.source) {
        return Err(());
    }
    let boss = owner.boss;
    let mut next = owner.clock.clone();
    let effects = next.advance(
        now / 1_000,
        cage(runtime, native).is_some(),
        runtime.contains_live_creature(boss),
    );
    apply(runtime, states, room, native, map, &effects, now, 0).await?;
    states.soul_war_callbacks.malice.as_mut().ok_or(())?.clock = next;
    for &occurrence in occurrences
        .iter()
        .filter(|occurrence| occurrence.actor == boss)
    {
        let mut next = states
            .soul_war_callbacks
            .malice
            .as_ref()
            .ok_or(())?
            .clock
            .clone();
        let effects = next.on_think(occurrence, 1_000, now / 1_000);
        apply(
            runtime,
            states,
            room,
            native,
            map,
            &effects,
            now,
            occurrence.sequence,
        )
        .await?;
        states.soul_war_callbacks.malice.as_mut().ok_or(())?.clock = next;
    }
    Ok(())
}
#[allow(clippy::too_many_arguments)]
async fn apply(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    effects: &[MaliceEffect],
    now: u64,
    sequence: u64,
) -> Result<(), ()> {
    for effect in effects {
        let owner = states.soul_war_callbacks.malice.as_ref().ok_or(())?;
        let source = Arc::clone(&owner.source);
        let boss = owner.boss;
        match effect {
            MaliceEffect::SpawnCage => {
                // Lua schedules removal even if the forced create returns nil.
                let _ = shared::spawn_source_child(
                    runtime, room, native, map, &source, boss, CAGE, CAGE_AT, true, now, sequence,
                )
                .await;
            }
            MaliceEffect::RemoveCage => {
                if let Some(actor) = cage(runtime, native) {
                    runtime.remove_encounter_actor(actor).map_err(|_| ())?;
                    states.monster_melee.creature_ai.remove(actor);
                }
            }
            MaliceEffect::BuffMalice => {
                if runtime.contains_live_creature(boss) {
                    shared::change(
                        runtime,
                        boss,
                        SoulWarCreatureChange::Buff {
                            defense: 10,
                            reflection: 10,
                            maximum_health: 0,
                            health: 0,
                        },
                    )?;
                }
            }
            MaliceEffect::CreateWhiteTiles => {
                shared::white::create(runtime, states, room, native, boss, sequence)?;
            }
            MaliceEffect::RevertTilesAndDamage => {
                shared::white::apply(runtime, states, room, native, &source, now)?;
            }
        }
    }
    Ok(())
}

/// The original timer state is retained through detached Map/name reads.
#[derive(Clone)]
pub(super) struct PreparedMaliceTimer {
    boss: ExactActorRef,
    clock: MaliceRoom,
}
pub(super) fn prepare_due_timer(
    states: &ChannelSpellStates,
    cutoff_us: u64,
    now_us: u64,
) -> Option<PreparedMaliceTimer> {
    let owner = states.soul_war_callbacks.malice.as_ref()?;
    let due = owner.clock.next_due_ms()?.checked_mul(1_000)?;
    (due <= cutoff_us && due <= now_us).then(|| PreparedMaliceTimer {
        boss: owner.boss,
        clock: owner.clock.clone(),
    })
}
pub(super) fn due_timer_cells() -> [MovementLocalPosition; 1] {
    [CAGE_AT]
}

/// Synchronous publication while the detached read transaction still holds
/// its actual Item locks. No SQL, fresh Think or invented callback clock.
#[allow(
    clippy::too_many_arguments,
    reason = "timer, named actors, current Map/native scope and real execution time require independent reproof"
)]
pub(super) fn publish_due_timer(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    map: &crate::movement::monster_map::MonsterMap,
    prepared: &PreparedMaliceTimer,
    names: &[super::source_names::SourceNamedActor],
    now: u64,
) -> Result<(), ()> {
    let owner = states.soul_war_callbacks.malice.as_ref().ok_or(())?;
    if owner.boss != prepared.boss
        || owner.clock != prepared.clock
        || !map.matches(runtime)
        || !shared::qualifies_source(runtime, native)
        || !runtime.soul_war_white_tile_source_current(&owner.source)
    {
        return Err(());
    }
    let source = Arc::clone(&owner.source);
    let named_cage = super::source_names::creature_named(names, "Soul Cage")?;
    let named_boss = super::source_names::creature_named(names, "Goshnar's Malice")?;
    let mut next = prepared.clock.clone();
    let effects = next.advance(now / 1_000, named_cage.is_some(), named_boss.is_some());
    // Player/NPC aliases require their real removal owner. Retain this timer
    // rather than forging a creature removal or silently skipping precedence.
    if effects.contains(&MaliceEffect::RemoveCage)
        && named_cage
            .is_some_and(|actor| actor.role != super::source_names::SourceNameRole::Creature)
    {
        return Err(());
    }
    if effects.contains(&MaliceEffect::SpawnCage) && map.blocks_solid(CAGE_AT).is_none() {
        return Err(());
    }
    if effects.contains(&MaliceEffect::BuffMalice)
        && named_boss
            .is_some_and(|actor| actor.role != super::source_names::SourceNameRole::Creature)
    {
        return Err(());
    }
    for effect in effects {
        match effect {
            MaliceEffect::SpawnCage => {
                // This source create is forced. A proved existing centre is
                // sufficient even when occupied/solid; absent is not a tile.
                if map.blocks_solid(CAGE_AT).is_none() {
                    return Err(());
                }
                let _ = runtime.spawn_soul_war_child_from_source(
                    &source,
                    CAGE,
                    CAGE_AT,
                    |current, _| map.matches(current).then_some(true),
                );
            }
            MaliceEffect::RemoveCage => {
                if let Some(actor) = named_cage {
                    runtime
                        .remove_encounter_actor(actor.actor)
                        .map_err(|_| ())?;
                    states.monster_melee.creature_ai.remove(actor.actor);
                }
            }
            MaliceEffect::BuffMalice => {
                if let Some(actor) = named_boss {
                    if actor.role != super::source_names::SourceNameRole::Creature {
                        return Err(());
                    }
                    shared::change(
                        runtime,
                        actor.actor,
                        SoulWarCreatureChange::Buff {
                            defense: 10,
                            reflection: 10,
                            maximum_health: 0,
                            health: 0,
                        },
                    )?;
                }
            }
            MaliceEffect::RevertTilesAndDamage => {
                shared::white::apply(runtime, states, room, native, &source, now)?;
            }
            MaliceEffect::CreateWhiteTiles => return Err(()),
        }
    }
    states.soul_war_callbacks.malice.as_mut().ok_or(())?.clock = next;
    Ok(())
}
