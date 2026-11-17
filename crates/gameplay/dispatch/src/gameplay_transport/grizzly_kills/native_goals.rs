//! Personal native goals share the retained lethal queue and exact principal fence.
use super::*;
use oteryn_gameplay_quests::quest::native_kill_goals as goals;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Recipient {
    actor: ExactActorRef,
    session: GameSessionId,
    character: [u8; 16],
    lease_generation: u64,
    goals: Vec<String>,
    source: crate::durability::quest_state::kill_objectives::NativeObjectiveDeath,
}

pub(super) fn active(tracks: &std::collections::BTreeMap<String, i64>) -> Vec<String> {
    goals::load()
        .map(|goals| {
            goals
                .iter()
                .filter(|g| g.active(tracks))
                .map(|g| g.key.clone())
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn tracks_creature(creature: &str) -> bool {
    goals::load().is_ok_and(|goals| {
        goals
            .iter()
            .any(|g| g.origins.iter().any(|p| p.creature == creature))
    })
}

pub(super) fn freeze(
    runtime: &mut ChannelRuntimeV1,
    entry: &PendingKillSettlement,
    present: bool,
    snapshots: &[TaskSnapshot],
) -> Option<Recipient> {
    if !present {
        return None;
    }
    let principal = entry.facts.principal;
    let snapshot = snapshots.iter().find(|s| {
        s.actor == principal.actor
            && s.session == principal.session
            && &s.character == principal.character.as_bytes()
            && s.lease_generation == principal.lease_generation
            && s.observed_revision > 0
            && !s.active_goals.is_empty()
    })?;
    let origin = runtime.creature_death_spawn_cell(entry.target)?;
    let actions: Vec<_> = goals::load()
        .ok()?
        .iter()
        .filter(|g| {
            snapshot.active_goals.contains(&g.key)
                && g.matches(&entry.creature, origin.x, origin.y, origin.floor)
        })
        .map(|g| g.key.clone())
        .collect();
    if actions.is_empty() {
        return None;
    }
    let character = crate::domain::CharacterId::from_bytes(snapshot.character).ok()?;
    let source =
        crate::durability::quest_state::kill_objectives::NativeObjectiveDeath::capture_goal(
            runtime,
            entry.target,
            character,
            snapshot.observed_revision,
        )?;
    Some(Recipient {
        actor: snapshot.actor,
        session: snapshot.session,
        character: snapshot.character,
        lease_generation: snapshot.lease_generation,
        goals: actions,
        source,
    })
}

pub(in crate::gameplay_transport) fn requests(
    entry: &PendingKillSettlement,
) -> Result<
    Vec<(
        &str,
        &crate::durability::quest_state::kill_objectives::NativeObjectiveDeath,
    )>,
    CharacterProgressionError,
> {
    let Some(recipient) = entry.grizzly.as_ref().and_then(|f| f.native_goals.as_ref()) else {
        return Ok(Vec::new());
    };
    let principal = entry.facts.principal;
    if principal.actor != recipient.actor
        || principal.session != recipient.session
        || principal.character.as_bytes() != &recipient.character
        || principal.lease_generation != recipient.lease_generation
    {
        return Err(CharacterProgressionError::AuthorityRejected);
    }
    Ok(recipient
        .goals
        .iter()
        .map(|goal| (goal.as_str(), &recipient.source))
        .collect())
}

pub(in crate::gameplay_transport) fn session(
    entry: &PendingKillSettlement,
) -> Option<GameSessionId> {
    entry
        .grizzly
        .as_ref()?
        .native_goals
        .as_ref()
        .map(|r| r.session)
}
