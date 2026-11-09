//! Pinned Vemiath/Chagorz/Bakragore onThink effects on actual native owners.
use super::*;
use oteryn_gameplay_encounters::bosses::rotten_blood::{
    bakragore_think as bak, chagorz_think as chag, vemiath_think as vem,
};
use oteryn_gameplay_entities::carrier::rotten_blood::{RottenBloodIconKey, RottenBloodIconSource};
pub(super) mod bak_preparation;
mod effects;
mod footprints;
mod plan;
mod portals;
pub(super) mod step;
#[derive(Debug, Clone)]
struct Player {
    actor: ExactActorRef,
    session: GameSessionId,
    id: u64,
    position: RoomPosition,
    max_health: u32,
    taint: u8,
}
#[derive(Debug, Clone)]
enum Engine {
    Vemiath {
        think: vem::VemiathThinkOwner,
        agony: chag::AgonyStacks,
    },
    Chagorz(chag::ChagorzThinkOwner),
    Bakragore {
        think: bak::BakragoreThinkOwner,
        agony: chag::AgonyStacks,
    },
}
#[derive(Debug)]
struct AdvancedBoss {
    actor: ExactActorRef,
    key: String,
    source: Arc<RottenBloodSpawnSource>,
    last: Option<u64>,
    engine: Engine,
    players: Vec<(ExactActorRef, GameSessionId, u64)>,
    next_player: u64,
    teleport: chag::TeleportCycle,
    essence_count: Option<u8>,
    portal_source: crate::durability::creature_source_items::RottenPortalSource,
    portal_ordinal: u64,
    paused: bool,
}
#[derive(Debug, Clone)]
enum AdvancedEffect {
    Spawn {
        source: Arc<RottenBloodSpawnSource>,
        parent: ExactActorRef,
        sequence: u64,
        ordinal: u64,
        key: &'static str,
        around: RoomPosition,
        offset: u8,
        close: bool,
    },
    Icon {
        source: Arc<RottenBloodIconSource>,
        actor: ExactActorRef,
        session: GameSessionId,
        count: u8,
    },
    Damage {
        source: Arc<RottenBloodIconSource>,
        boss: ExactActorRef,
        actor: ExactActorRef,
        session: GameSessionId,
        amount: u32,
    },
    Transform {
        at: RoomPosition,
        from: u16,
        to: u16,
    },
}
#[cfg(test)]
#[derive(Debug, Clone, Copy)]
pub(super) struct AgonyCommit {
    pub(super) actor: ExactActorRef,
    pub(super) before: u32,
    pub(super) after: u32,
    pub(super) amount: u32,
}
#[derive(Debug, Default)]
pub(in crate::gameplay_transport) struct AdvancedCallbacks {
    bosses: Vec<AdvancedBoss>,
    pending: VecDeque<AdvancedEffect>,
    pending_due: Option<u64>,
    portals: Vec<Arc<crate::durability::creature_source_items::RottenPortalCallback>>,
    active_portals: Vec<Arc<crate::durability::creature_source_items::RottenPortalCallback>>,
    steps: Vec<step::PendingStep>,
    footprints: Vec<footprints::Pending>,
    first_think: Option<bak_preparation::Pending>,
    #[cfg(test)]
    commits: Vec<AgonyCommit>,
    #[cfg(test)]
    massive_commits: Vec<(ExactActorRef, u8, u32, u32, u32)>,
}
impl AdvancedCallbacks {
    pub(super) fn pending_for_player(&self, actor: ExactActorRef, session: GameSessionId) -> bool {
        self.first_think.is_some()
            || self.footprints.iter().any(footprints::Pending::blocking)
            || step::pending_for(&self.steps, actor, session)
    }
    pub(super) fn critical_scope_pending(&self) -> bool {
        self.footprints.iter().any(footprints::Pending::blocking)
            || !self.steps.is_empty()
            || self
                .first_think
                .as_ref()
                .is_some_and(|pending| !pending.resolved)
    }
    #[cfg(test)]
    pub(super) fn massive_commit_for_test(
        &self,
        actor: ExactActorRef,
        phase: u8,
    ) -> Option<(u32, u32, u32)> {
        self.massive_commits
            .iter()
            .rev()
            .find_map(|(target, stage, amount, before, after)| {
                (*target == actor && *stage == phase).then_some((*amount, *before, *after))
            })
    }
    #[cfg(test)]
    pub(in crate::gameplay_transport) fn source_state_for_test(
        &self,
    ) -> Vec<(String, Option<u64>, Option<u8>)> {
        self.bosses
            .iter()
            .map(|b| (b.key.clone(), b.last, b.essence_count))
            .collect()
    }
    pub(super) fn next_due_time(&self) -> Option<u64> {
        self.pending_due
            .into_iter()
            .chain(self.portals.iter().map(|p| p.due_us()))
            .chain(self.footprints.iter().map(|p| p.due_us()))
            .chain(self.first_think.as_ref().map(|_| 0))
            .chain(
                self.bosses
                    .iter()
                    .filter_map(|b| b.teleport.next_at_ms().map(|t| t.saturating_mul(1000))),
            )
            .min()
    }
    // PARTIAL: feeds the source-fenced death getter, not an invented count.
    #[allow(
        dead_code,
        reason = "PARTIAL Rotten Blood: captured Bakragore death-loot count consumer is pending"
    )]
    pub(super) fn actual_bakragore_essence_count(&self, target: ExactActorRef) -> Option<u8> {
        self.bosses
            .iter()
            .find(|boss| boss.actor == target && boss.key == bak::BAKRAGORE_KEY)
            .and_then(|boss| boss.essence_count)
    }
    #[cfg(test)]
    pub(super) fn agony_commit(&self, actor: ExactActorRef) -> Option<AgonyCommit> {
        self.commits
            .iter()
            .rev()
            .find(|c| c.actor == actor)
            .copied()
    }
}
fn source_room(
    key: &str,
) -> Option<crate::content::bosses::activity_template::PinnedActivitySourceRoom> {
    use crate::content::bosses::activity_template::PinnedActivitySourceRoom as Room;
    match key {
        vem::VEMIATH_KEY => Some(Room::Vemiath),
        chag::CHAGORZ_KEY => Some(Room::Chagorz),
        bak::BAKRAGORE_KEY => Some(Room::Bakragore),
        _ => None,
    }
}
fn center(key: &str) -> RoomPosition {
    match key {
        vem::VEMIATH_KEY => vem::VEMIATH_ROOM_CENTRE,
        chag::CHAGORZ_KEY => chag::CHAGORZ_ROOM_CENTRE,
        _ => bak::BAKRAGORE_ROOM_CENTRE,
    }
}
// PARTIAL: the parent due-only port awaits actual source-death arbiter wiring;
// retain its refusal rather than manufacture a timer execution or fresh Think.
#[allow(
    dead_code,
    reason = "PARTIAL Rotten Blood: original-deadline advanced arbiter is not wired yet"
)]
#[allow(
    clippy::too_many_arguments,
    reason = "due-only work retains the same separately fenced physical owners"
)]
pub(super) async fn drive_due_through(
    _runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    _room: &QualifiedNativeEntryRoom,
    _native: &NativeGameplayState,
    _map: &mut CurrentMonsterMap<'_, '_>,
    eligible_through_us: u64,
    now: u64,
) -> Result<Option<u64>, ()> {
    let due = states.rotten_blood_callbacks.advanced.next_due_time();
    // Keep this boundary refusing until the actual death-notice regression
    // proves the retained original deadline, independently of retry wakes.
    if due.is_some_and(|due| due <= eligible_through_us.min(now)) {
        return Err(());
    }
    Ok(due)
}
#[allow(
    clippy::too_many_arguments,
    reason = "actual native think consumes separately fenced runtime, map and Player owners"
)]
pub(super) async fn drive(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    occurrences: &[ThinkOccurrence],
    now: u64,
) -> Result<Option<u64>, ()> {
    effects::drain(runtime, states, room, native, map, now).await?;
    let retained = states
        .rotten_blood_callbacks
        .advanced
        .first_think
        .as_ref()
        .filter(|p| p.resolved)
        .map(|p| p.occurrence);
    let mut work = Vec::with_capacity(occurrences.len() + usize::from(retained.is_some()));
    if let Some(occurrence) = retained {
        work.push(occurrence);
    }
    for occurrence in occurrences {
        if !work.contains(occurrence) {
            work.push(*occurrence);
        }
    }
    for occurrence in &work {
        let actor = occurrence.actor;
        let Some(key) = super::super::soul_war_callbacks::source_actor(runtime, native, actor)
        else {
            continue;
        };
        let Some(kind) = source_room(&key) else {
            continue;
        };
        if room.pinned_activity_source_room() != Some(kind)
            || !runtime.contains_live_creature(actor)
        {
            continue;
        }
        if key == bak::BAKRAGORE_KEY
            && bak_preparation::await_claim(runtime, states, native, *occurrence, now)?
        {
            // This synchronous source callback has not committed its global
            // decision. Do not advance native AI/ordinary casts past it.
            return Err(());
        }
        if !states
            .rotten_blood_callbacks
            .advanced
            .bosses
            .iter()
            .any(|b| b.actor == actor)
        {
            if states.rotten_blood_callbacks.advanced.bosses.len() >= 8 {
                return Err(());
            }
            let source = Arc::new(
                runtime
                    .qualify_rotten_blood_spawn_source(actor, native)
                    .map_err(|_| ())?,
            );
            let engine = match key.as_str() {
                vem::VEMIATH_KEY => Engine::Vemiath {
                    think: Default::default(),
                    agony: Default::default(),
                },
                chag::CHAGORZ_KEY => Engine::Chagorz(Default::default()),
                _ => Engine::Bakragore {
                    think: Default::default(),
                    agony: Default::default(),
                },
            };
            states
                .rotten_blood_callbacks
                .advanced
                .bosses
                .push(AdvancedBoss {
                    actor,
                    key: key.clone(),
                    source,
                    last: None,
                    engine,
                    players: Vec::new(),
                    next_player: 0,
                    teleport: chag::TeleportCycle::new(center(&key)),
                    essence_count: None,
                    portal_source: crate::durability::creature_source_items::RottenPortalSource::from_current_native_think(runtime,room,native,occurrence).map_err(|_| ())?,
                    portal_ordinal: 0,
                    paused: false,
                });
        }
        let index = states
            .rotten_blood_callbacks
            .advanced
            .bosses
            .iter()
            .position(|b| b.actor == actor)
            .ok_or(())?;
        if states.rotten_blood_callbacks.advanced.bosses[index]
            .last
            .is_some_and(|last| last >= occurrence.sequence)
        {
            continue;
        }
        let census = runtime.positioned_actor_census().map_err(|_| ())?;
        let boss_pos = pos(runtime
            .read_actor_position(actor)
            .map_err(|_| ())?
            .position());
        let mut players = Vec::new();
        for (a, p, s) in &census {
            let Some(session) = s else {
                continue;
            };
            let Some(player) = states.get(runtime, *a, *session) else {
                continue;
            };
            if player.vitals().health == 0 {
                continue;
            }
            let max_health = player.vitals().max_health;
            let taint = states
                .rotten_blood_callbacks
                .taint(*a, *session)
                .ok_or(())?;
            let boss = &mut states.rotten_blood_callbacks.advanced.bosses[index];
            let id = if let Some((_, _, id)) = boss
                .players
                .iter()
                .find(|(known, s, _)| known == a && s == session)
            {
                *id
            } else {
                if boss.players.len() >= 64 {
                    return Err(());
                }
                boss.next_player = boss.next_player.checked_add(1).ok_or(())?;
                boss.players.push((*a, *session, boss.next_player));
                boss.next_player
            };
            players.push(Player {
                actor: *a,
                session: *session,
                id,
                position: pos(p.position()),
                max_health,
                taint,
            });
        }
        let maximum = players
            .iter()
            .filter(|p| near(center(&key), p.position, 15))
            .map(|p| p.taint)
            .max()
            .unwrap_or(0);
        let count = |child: &str, range: u32| {
            census
                .iter()
                .filter(|(a, p, s)| {
                    s.is_none()
                        && runtime.contains_live_creature(*a)
                        && near(boss_pos, pos(p.position()), range)
                        && super::super::soul_war_callbacks::source_actor(runtime, native, *a)
                            .as_deref()
                            == Some(child)
                })
                .count()
                .min(255) as u8
        };
        let jaw_count = count(
            vem::ELDER_BLOODJAW_KEY,
            if key == vem::VEMIATH_KEY {
                17
            } else if key == bak::BAKRAGORE_KEY {
                20
            } else {
                15
            },
        );
        let pillar_count = count(
            vem::PILLAR_KEY,
            if key == vem::VEMIATH_KEY {
                17
            } else if key == bak::BAKRAGORE_KEY {
                20
            } else {
                16
            },
        );
        let mushroom_count = count(bak::MUSHROOM_KEY, 20);
        let icon = Arc::new(
            runtime
                .qualify_rotten_blood_icon_source(actor, native)
                .map_err(|_| ())?,
        );
        let active_essences = if key == bak::BAKRAGORE_KEY {
            let mask = states
                .rotten_blood_callbacks
                .prepared_bak
                .as_ref()
                .and_then(|proof| proof.current_mask(runtime))
                .ok_or(())?;
            // Both the source FLAMES and donor ESSENCES use this exact order:
            // Murcion, Ichgahal, Chagorz, Vemiath. An absent header refuses;
            // only an actual sealed zero header means no prepared essences.
            std::array::from_fn(|index| mask & (1 << index) != 0)
        } else {
            [false; 4]
        };
        let callback_now = states
            .rotten_blood_callbacks
            .advanced
            .first_think
            .as_ref()
            .filter(|p| p.resolved && p.occurrence == *occurrence)
            .map_or(now, |p| p.started_us);
        let boss = &mut states.rotten_blood_callbacks.advanced.bosses[index];
        let mut engine = boss.engine.clone();
        let planned = plan::plan(
            &mut engine,
            boss_pos,
            &players,
            elder::bloodjaw_target_count(maximum),
            jaw_count,
            pillar_count,
            mushroom_count,
            callback_now / 1000,
            boss.paused,
            active_essences,
            census.iter().any(|(a, p, session)| {
                session.is_none()
                    && runtime.contains_live_creature(*a)
                    && near(center(&key), pos(p.position()), 15)
                    && super::super::soul_war_callbacks::source_actor(runtime, native, *a)
                        .as_deref()
                        == Some(bak::ELDER_BLOODJAW_KEY)
            }),
        )?;
        if states.rotten_blood_callbacks.advanced.pending.len() + planned.len() > 128 {
            return Err(());
        }
        let boss = &mut states.rotten_blood_callbacks.advanced.bosses[index];
        for (ordinal, effect) in planned.into_iter().enumerate() {
            use plan::Planned as P;
            let effect = match effect {
                P::StartTeleport => {
                    boss.teleport.start(callback_now / 1000);
                    continue;
                }
                P::EssenceCount(count) => {
                    boss.essence_count = Some(count);
                    continue;
                }
                P::ResetEssence => continue,
                P::Spawn {
                    key,
                    around,
                    offset,
                    close,
                } => AdvancedEffect::Spawn {
                    source: Arc::clone(&boss.source),
                    parent: actor,
                    sequence: occurrence.sequence,
                    ordinal: ordinal as u64 + 1,
                    key,
                    around,
                    offset,
                    close,
                },
                P::Icon { id, count } => {
                    let Some(player) = players.iter().find(|p| p.id == id) else {
                        return Err(());
                    };
                    AdvancedEffect::Icon {
                        source: Arc::clone(&icon),
                        actor: player.actor,
                        session: player.session,
                        count,
                    }
                }
                P::Damage { id, amount } => {
                    let Some(player) = players.iter().find(|p| p.id == id) else {
                        return Err(());
                    };
                    AdvancedEffect::Damage {
                        source: Arc::clone(&icon),
                        boss: actor,
                        actor: player.actor,
                        session: player.session,
                        amount,
                    }
                }
                P::Transform { at, from, to } => AdvancedEffect::Transform { at, from, to },
            };
            states
                .rotten_blood_callbacks
                .advanced
                .pending
                .push_back(effect);
        }
        let boss = &mut states.rotten_blood_callbacks.advanced.bosses[index];
        boss.engine = engine;
        boss.last = Some(occurrence.sequence);
        if states
            .rotten_blood_callbacks
            .advanced
            .first_think
            .as_ref()
            .is_some_and(|pending| pending.resolved && pending.occurrence == *occurrence)
        {
            states.rotten_blood_callbacks.advanced.first_think = None;
        }
    }
    effects::drain(runtime, states, room, native, map, now).await?;
    portals::plan(runtime, states, room, native, now)?;
    Ok(states.rotten_blood_callbacks.advanced.next_due_time())
}

#[cfg(test)]
impl super::RottenBloodCallbacks {
    pub(in crate::gameplay_transport) fn portal_diagnostics_for_test(
        &self,
        runtime: &ChannelRuntimeV1,
    ) -> Vec<String> {
        let a = &self.advanced;
        let mut rows = vec![format!(
            "pending_effects={} pending_due={:?} portal_queue={} active_portals={} steps={} first_think={}",
            a.pending.len(),
            a.pending_due,
            a.portals.len(),
            a.active_portals.len(),
            a.steps.len(),
            a.first_think.is_some()
        )];
        for boss in &a.bosses {
            rows.push(format!(
                "boss={:?} key={} last={:?} deadline_ms={:?} paused={} current={:?}",
                boss.actor,
                boss.key,
                boss.last,
                boss.teleport.next_at_ms(),
                boss.paused,
                runtime
                    .companion_snapshot(boss.actor)
                    .map(|s| (s.health, s.position))
            ));
        }
        if let Ok(census) = runtime.positioned_actor_census() {
            for (actor, position, session) in census {
                if session.is_some() {
                    continue;
                }
                if let Ok(s) = runtime.companion_snapshot(actor) {
                    rows.push(format!(
                        "native={:?} key={} health={} at={:?}",
                        actor,
                        s.state.policy.definition_key,
                        s.health,
                        position.position()
                    ));
                }
            }
        }
        rows
    }
}
