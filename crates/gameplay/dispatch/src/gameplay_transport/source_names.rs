//! Current all-role names for source callbacks, separate from client interest.
use super::*;
use crate::durability::source_creature_names::{
    CurrentSourcePlayerName, PreparedSourcePlayerNames,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SourceNameRole {
    Player,
    Npc,
    Creature,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SourceNamedActor {
    pub actor: ExactActorRef,
    pub role: SourceNameRole,
    pub position: crate::foundation::MovementPositionSnapshot,
    pub name: String,
}

/// Game::getCreatureByName checks Players first, then NPCs, then Monsters.
/// Same-role ambiguity is represented explicitly; callers must not invent
/// an actor-ID sort as a substitute for the donor's registry iteration.
pub(super) fn creature_named<'a>(
    candidates: &'a [SourceNamedActor],
    name: &str,
) -> Result<Option<&'a SourceNamedActor>, ()> {
    for role in [
        SourceNameRole::Player,
        SourceNameRole::Npc,
        SourceNameRole::Creature,
    ] {
        let mut matches = candidates.iter().filter(|candidate| {
            candidate.role == role && candidate.name.eq_ignore_ascii_case(name)
        });
        if let Some(first) = matches.next() {
            return if matches.next().is_some() {
                Err(())
            } else {
                Ok(Some(first))
            };
        }
    }
    Ok(None)
}

/// A bounded owned census captured before releasing the Runtime. It is neither
/// a second actor registry nor a name cache; every seal rechecks the full census.
#[derive(Debug, Clone)]
pub(super) struct PreparedSourceNames {
    players: PreparedSourcePlayerNames,
    player_cohort: Vec<(ExactActorRef, GameSessionId)>,
    others: Vec<SourceNamedActor>,
    names: Vec<String>,
}
#[derive(Debug)]
pub(super) struct ReadSourceNames {
    plan: PreparedSourceNames,
    players: Vec<CurrentSourcePlayerName>,
}
impl ComposedFreshAdmission<'_, '_, '_> {
    fn source_other_names(&self, runtime: &ChannelRuntimeV1) -> Result<Vec<SourceNamedActor>, ()> {
        let mut result = Vec::new();
        for npc in runtime.visible_entities().npcs {
            let key = runtime.npc_key(npc.actor).ok_or(())?;
            let name = self
                .states
                .chat
                .npc_talk()
                .ok_or(())?
                .display_name(key)
                .ok_or(())?;
            result.push(SourceNamedActor {
                actor: npc.actor,
                role: SourceNameRole::Npc,
                name: name.to_owned(),
                position: runtime
                    .read_movement_actor_position(npc.actor)
                    .map_err(|_| ())?,
            });
        }
        for (actor, position) in runtime
            .source_registered_creature_census()
            .map_err(|_| ())?
        {
            let snapshot = runtime
                .companion_snapshot_including_dead(actor)
                .map_err(|_| ())?;
            result.push(SourceNamedActor {
                actor,
                role: SourceNameRole::Creature,
                position,
                name: snapshot.state.policy.display_name.clone(),
            });
        }
        if result.len() > runtime.actor_slot_capacity() {
            return Err(());
        }
        Ok(result)
    }
    /// Capture under Runtime; no SQL or await is permitted in this phase.
    pub(super) fn prepare_source_names(
        &self,
        runtime: &ChannelRuntimeV1,
        names: &[&str],
    ) -> Result<PreparedSourceNames, ()> {
        let player_cohort = runtime
            .positioned_melee_census()
            .map_err(|_| ())?
            .into_iter()
            .filter_map(|(actor, _, session)| session.map(|session| (actor, session)))
            .collect::<Vec<_>>();
        let players = crate::durability::source_creature_names::prepare_source_player_names(
            runtime,
            names,
            &player_cohort,
        )
        .map_err(|_| ())?;
        Ok(PreparedSourceNames {
            players,
            player_cohort,
            others: self.source_other_names(runtime)?,
            names: names.iter().map(|&name| name.to_owned()).collect(),
        })
    }
    /// The SQL future contains only owned observations and the actual scope
    /// lane. It has no Runtime argument or reference and needs no Player caster.
    pub(super) async fn read_source_names(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        plan: &PreparedSourceNames,
    ) -> Result<ReadSourceNames, ()> {
        let players = self
            .root
            .read_prepared_source_player_names(permit, self.character, self.holder, &plan.players)
            .await
            .map_err(|_| ())?;
        Ok(ReadSourceNames {
            plan: plan.clone(),
            players,
        })
    }
    /// Relock Runtime and reprove all current roles before exposing the names.
    /// New/retired actors, movement, lease changes and registered HP0 removal
    /// invalidate the old read rather than silently narrowing its candidate set.
    pub(super) fn seal_source_names(
        &self,
        runtime: &ChannelRuntimeV1,
        read: ReadSourceNames,
    ) -> Result<Vec<SourceNamedActor>, ()> {
        let plan = read.plan;
        let players = runtime
            .positioned_melee_census()
            .map_err(|_| ())?
            .into_iter()
            .filter_map(|(actor, _, session)| session.map(|session| (actor, session)))
            .collect::<Vec<_>>();
        if !plan.players.current_for(runtime)
            || players != plan.player_cohort
            || self.source_other_names(runtime)? != plan.others
        {
            return Err(());
        }
        let mut result = Vec::new();
        for player in read.players {
            result.push(SourceNamedActor {
                actor: player.actor,
                role: SourceNameRole::Player,
                name: player.name,
                position: runtime.read_actor_position(player.actor).map_err(|_| ())?,
            });
        }
        result.extend(plan.others.into_iter().filter(|entry| {
            plan.names
                .iter()
                .any(|name| entry.name.eq_ignore_ascii_case(name))
        }));
        Ok(result)
    }
}
