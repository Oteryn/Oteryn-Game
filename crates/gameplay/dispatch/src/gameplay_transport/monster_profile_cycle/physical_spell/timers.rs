//! Existing sealed windup/chain timers, with fresh facts for each exact source pulse.
use super::*;
use crate::creature_chain_attack::ChainMapFacts;
use crate::foundation::owner_timer::{OwnerClock, SemanticTimeMicros};
use crate::spell::chain::{CHAIN_PATH_SEARCH_TILES, TilePosition};

struct TurnClock(SemanticTimeMicros);
impl OwnerClock for TurnClock {
    fn now(&self) -> SemanticTimeMicros {
        self.0
    }
}

pub(super) struct CurrentChainMap {
    stamp: RuntimeWorkStamp,
    sights: BTreeMap<(TilePosition, TilePosition), bool>,
    paths: BTreeMap<(TilePosition, TilePosition), Vec<TilePosition>>,
}
impl ChainMapFacts for CurrentChainMap {
    fn sight(
        &mut self,
        runtime: &ChannelRuntimeV1,
        from: TilePosition,
        to: TilePosition,
        stamp: RuntimeWorkStamp,
    ) -> Option<bool> {
        (stamp == self.stamp && runtime.owner_fence().is_ok_and(|f| f.accepts_stamp(stamp)))
            .then(|| self.sights.get(&(from, to)).copied())
            .flatten()
    }
    fn path(
        &mut self,
        runtime: &ChannelRuntimeV1,
        from: TilePosition,
        to: TilePosition,
        stamp: RuntimeWorkStamp,
    ) -> Option<Vec<TilePosition>> {
        (stamp == self.stamp && runtime.owner_fence().is_ok_and(|f| f.accepts_stamp(stamp)))
            .then(|| self.paths.get(&(from, to)).cloned())
            .flatten()
    }
}

fn tile(at: MovementLocalPosition) -> TilePosition {
    TilePosition {
        x: at.x,
        y: at.y,
        floor: at.floor,
    }
}
fn position(at: TilePosition) -> MovementLocalPosition {
    MovementLocalPosition {
        x: at.x,
        y: at.y,
        floor: at.floor,
    }
}

async fn read_sight(
    runtime: &ChannelRuntimeV1,
    map: &mut CurrentMonsterMap<'_, '_>,
    from: MovementLocalPosition,
    to: MovementLocalPosition,
) -> Option<bool> {
    map.read_sight(runtime, from, to).await;
    let steps = crate::durability::native_world_read::source_sight_steps(from, to).ok()?;
    steps
        .iter()
        .all(|(at, exempt)| *exempt || map.view().known(*at))
        .then(|| map.view().sight(from, to))
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn prepare_chain_map(
    runtime: &ChannelRuntimeV1,
    map: &mut CurrentMonsterMap<'_, '_>,
    reader: &CurrentTarget<'_>,
    proposal: &ProfileAbilityProposal,
    stamp: RuntimeWorkStamp,
    attack: &mut ChannelAttackStates,
    now: u64,
) -> Option<CurrentChainMap> {
    let mut positions = vec![
        runtime
            .read_actor_position(proposal.issuer)
            .ok()?
            .position(),
    ];
    for (actor, _) in &reader.players {
        let at = runtime.read_actor_position(*actor).ok()?.position();
        if !positions.contains(&at) {
            positions.push(at);
        }
    }
    let mut result = CurrentChainMap {
        stamp,
        sights: BTreeMap::new(),
        paths: BTreeMap::new(),
    };
    let distance = i32::try_from(CHAIN_PATH_SEARCH_TILES).ok()?;
    for from in &positions {
        // Reuse the existing bounded Movement search for presentation traces. Its cardinal
        // geometry is the repository's explicit projection, not Crystal's diagonal trace.
        for dx in -distance..=distance {
            for dy in -distance..=distance {
                let at = MovementLocalPosition {
                    x: from.x.checked_add(dx)?,
                    y: from.y.checked_add(dy)?,
                    floor: from.floor,
                };
                map.read(runtime, at).await;
            }
        }
        for to in &positions {
            let pair = (tile(*from), tile(*to));
            result
                .sights
                .insert(pair, read_sight(runtime, map, *from, *to).await?);
            let mut path = Vec::new();
            if from != to {
                let waiting = attack.players_waiting(now);
                if !attack.path_budget.admits(now, false, waiting) {
                    return None;
                }
                let mut path_facts_known = true;
                let (route, work) = crate::movement::path_search::measured_route_with_distance(
                    *from,
                    *to,
                    false,
                    distance,
                    |at| {
                        let view = map.view();
                        if !view.known(at) {
                            path_facts_known = false;
                            return None;
                        }
                        view.admits(at).then_some(0)
                    },
                );
                attack.path_budget.charge(false, work);
                // A missing current Item/Map fact is not proof of a blocked tile.
                // Charge the real search work, then refuse before publishing a trace
                // or admitting a timer. Qualified empty cells remain known blockers.
                if !path_facts_known {
                    return None;
                }
                let mut at = *from;
                for step in route.unwrap_or_default() {
                    let (dx, dy) = crate::movement::path_search::delta(step);
                    at.x = at.x.checked_add(dx)?;
                    at.y = at.y.checked_add(dy)?;
                    path.push(tile(at));
                }
            }
            // A completed search with no route is a known empty trace. The source chain
            // owner appends the destination; unavailable map facts remain a refusal.
            result.paths.insert(pair, path);
        }
    }
    map.validate_transaction(runtime).await.then_some(result)
}

struct FactsKey {
    creature: String,
    entry: usize,
    occurrence: String,
    session: GameSessionId,
}
impl FactsKey {
    fn new(
        source: &SpellSource,
        proposal: &ProfileAbilityProposal,
        session: GameSessionId,
    ) -> Self {
        Self {
            creature: source.creature_key.clone(),
            entry: source.entry_index,
            occurrence: proposal.occurrence.id().as_str().to_owned(),
            session,
        }
    }
    fn matches(
        &self,
        source: &SpellSource,
        proposal: &ProfileAbilityProposal,
        session: GameSessionId,
    ) -> bool {
        self.creature == source.creature_key
            && self.entry == source.entry_index
            && self.occurrence == proposal.occurrence.id().as_str()
            && self.session == session
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn drain(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    attack: &mut ChannelAttackStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    parties: &ReferenceParties,
    now: u64,
) {
    let clock = TurnClock(SemanticTimeMicros::from_micros(now));
    let Ok(stamp) = runtime.issue_owner_work() else {
        return;
    };
    let mut windups = Vec::new();
    let mut players = Vec::new();
    for (source, proposal, session, due) in states.monster_spells.pending_windups() {
        if due.get() > now {
            continue;
        }
        if let Some(reader) = prepare_reader(
            runtime, states, attack, room, native, map, &source, &proposal, parties, now, stamp,
            false, None,
        )
        .await
        {
            players.extend(reader.players.iter().copied());
            windups.push((FactsKey::new(&source, &proposal, session), reader));
        }
    }
    // A later awaited read can invalidate the transaction behind an earlier
    // reader. Revalidate the complete bank before consuming any one-use pulse.
    if !map.validate_transaction(runtime).await {
        return;
    }
    let mut spells = std::mem::take(&mut states.monster_spells);
    let outcomes = spells.run_windup_due_current_with(
        runtime,
        stamp,
        states,
        &clock,
        |source, proposal, session, pulse_stamp| {
            if pulse_stamp != stamp {
                return None;
            }
            let index = windups
                .iter()
                .position(|(key, _)| key.matches(source, proposal, session))?;
            Some(Box::new(windups.swap_remove(index).1) as Box<dyn SpellWorldReader + '_>)
        },
    );
    if let Ok(outcomes) = outcomes {
        for (_, outcome) in outcomes {
            if let Ok(outcome) = outcome {
                record_hits(runtime, attack, &players, outcome, now);
            }
        }
    }
    let Some(mut chains) = states.monster_chains.take() else {
        states.monster_spells = spells;
        return;
    };
    let mut bank = Vec::new();
    players.clear();
    if let Ok(pending) = chains.pending_sources() {
        for pending in pending {
            if pending.due.get() > now {
                continue;
            }
            let Some(reader) = prepare_reader(
                runtime,
                states,
                attack,
                room,
                native,
                map,
                &pending.source,
                &pending.proposal,
                parties,
                now,
                stamp,
                false,
                Some(position(pending.from)),
            )
            .await
            else {
                continue;
            };
            let Ok(to) = runtime.read_actor_position(pending.proposal.target) else {
                continue;
            };
            let to = to.position();
            let Some(sight) = read_sight(runtime, map, position(pending.from), to).await else {
                continue;
            };
            if !map.validate_transaction(runtime).await {
                continue;
            }
            let facts = CurrentChainMap {
                stamp,
                sights: BTreeMap::from([((pending.from, tile(to)), sight)]),
                paths: BTreeMap::new(),
            };
            players.extend(reader.players.iter().copied());
            bank.push((
                FactsKey::new(&pending.source, &pending.proposal, pending.session),
                pending.from,
                reader,
                facts,
            ));
        }
    }
    if !map.validate_transaction(runtime).await {
        states.monster_chains = Some(chains);
        states.monster_spells = spells;
        return;
    }
    let outcomes = chains.run_due_current_with(
        runtime,
        stamp,
        states,
        &spells,
        &clock,
        |source, proposal, session, from, pulse_stamp| {
            if pulse_stamp != stamp {
                return None;
            }
            let index = bank.iter().position(|(key, origin, _, _)| {
                *origin == from && key.matches(source, proposal, session)
            })?;
            let (_, _, reader, map) = bank.swap_remove(index);
            Some((
                Box::new(reader) as Box<dyn SpellWorldReader + '_>,
                Box::new(map) as Box<dyn ChainMapFacts>,
            ))
        },
    );
    states.monster_chains = Some(chains);
    states.monster_spells = spells;
    if let Ok(outcomes) = outcomes {
        for (_, _, outcome) in outcomes {
            if let Ok(outcome) = outcome {
                record_hits(runtime, attack, &players, outcome, now);
            }
        }
    }
}
