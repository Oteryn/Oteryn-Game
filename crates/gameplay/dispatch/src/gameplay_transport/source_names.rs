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

#[derive(Clone, Copy)]
struct NativeHealthNamesFrame {
    scope: crate::foundation::RuntimeScopeRefV1,
    generation: u64,
    activation: u64,
    pins: [[u8; 32]; 4],
}
impl NativeHealthNamesFrame {
    fn capture(runtime: &ChannelRuntimeV1) -> Result<Self, ()> {
        runtime.owner_fence().map_err(|_| ())?;
        Ok(Self {
            scope: runtime.binding().runtime_scope(),
            generation: runtime.binding().scope_generation().get(),
            activation: runtime.content_pin().activation_sequence(),
            pins: [
                runtime.content_pin().server_artifact_digest(),
                runtime.content_pin().client_artifact_digest(),
                runtime.content_pin().frame_binding_digest(),
                runtime.content_pin().map_revision_digest(),
            ],
        })
    }
    fn current_for(&self, runtime: &ChannelRuntimeV1) -> bool {
        runtime.owner_fence().is_ok()
            && runtime.binding().runtime_scope() == self.scope
            && runtime.binding().scope_generation().get() == self.generation
            && runtime.content_pin().activation_sequence() == self.activation
            && self.pins
                == [
                    runtime.content_pin().server_artifact_digest(),
                    runtime.content_pin().client_artifact_digest(),
                    runtime.content_pin().frame_binding_digest(),
                    runtime.content_pin().map_revision_digest(),
                ]
    }
}
/// Owned capture/outcome retain the original frame even when name facts are
/// unavailable. Unknown names never become a positive `Other` observation.
pub(super) struct PreparedNativeHealthNames {
    frame: NativeHealthNamesFrame,
    plan: Option<PreparedSourceNames>,
}
impl PreparedNativeHealthNames {
    pub(super) fn requires_sql(&self) -> bool {
        self.plan.is_some()
    }
    pub(super) fn unread(self) -> ReadNativeHealthNames {
        ReadNativeHealthNames {
            frame: self.frame,
            read: None,
        }
    }
}
pub(super) struct ReadNativeHealthNames {
    frame: NativeHealthNamesFrame,
    read: Option<ReadSourceNames>,
}
/// Immutable fixed-name DATA for one held physical owner turn. It is never
/// stored on an actor, timer or callback queue. Native movement can change
/// poses; every use rechecks the same external identities and current pins.
pub(super) struct NativeHealthNamesData {
    frame: NativeHealthNamesFrame,
    players: Option<PreparedSourcePlayerNames>,
    player_cohort: Vec<(ExactActorRef, GameSessionId)>,
    npcs: Vec<(ExactActorRef, String)>,
    names: Vec<(
        ExactActorRef,
        oteryn_gameplay_entities::NativeHealthGuardName,
    )>,
}
struct CurrentNativeHealthNames<'a> {
    runtime: &'a ChannelRuntimeV1,
    names: &'a NativeHealthNamesData,
}
impl oteryn_gameplay_entities::NativeHealthNameRead for CurrentNativeHealthNames<'_> {
    fn observed(
        &self,
        actor: ExactActorRef,
        at: crate::foundation::MovementPositionSnapshot,
    ) -> Option<oteryn_gameplay_entities::NativeHealthGuardName> {
        if self.runtime.read_movement_actor_position(actor).ok()? != at {
            return None;
        }
        self.names
            .names
            .iter()
            .find_map(|(candidate, name)| (*candidate == actor).then_some(*name))
    }
}
fn native_health_targets(runtime: &ChannelRuntimeV1) -> Result<Vec<ExactActorRef>, ()> {
    let mut targets = Vec::new();
    for (actor, _) in runtime
        .source_registered_creature_census()
        .map_err(|_| ())?
    {
        let key = runtime.creature_spell_target_atom(actor).map_err(|_| ())?;
        if !oteryn_gameplay_content::content::native_monster_callbacks::requires_native_health_event(
            &key,
        ) {
            continue;
        }
        if runtime.native_health_event(actor).map_err(|_| ())?.is_some_and(|event| {
            event.kind() == oteryn_gameplay_content::content::native_monster_callbacks::NativeHealthEventKind::HealthForgotten
        }) && runtime.companion_snapshot_including_dead(actor).map_err(|_| ())?.health > 0 {
            targets.push(actor);
        }
    }
    Ok(targets)
}
impl NativeHealthNamesData {
    pub(super) fn validate_frame(&self, runtime: &ChannelRuntimeV1) -> Result<(), ()> {
        self.frame.current_for(runtime).then_some(()).ok_or(())
    }
    fn validate(&self, runtime: &ChannelRuntimeV1) -> Result<(), ()> {
        self.validate_frame(runtime)?;
        if let Some(players) = &self.players {
            if !players.identities_current_for(runtime) {
                return Err(());
            }
            let cohort = runtime
                .positioned_melee_census()
                .map_err(|_| ())?
                .into_iter()
                .filter_map(|(actor, _, session)| session.map(|session| (actor, session)))
                .collect::<Vec<_>>();
            if cohort != self.player_cohort {
                return Err(());
            }
            let npcs = runtime.visible_entities().npcs;
            if npcs.len() != self.npcs.len()
                || self.npcs.iter().any(|(actor, key)| {
                    !npcs.iter().any(|npc| npc.actor == *actor)
                        || runtime.npc_key(*actor) != Some(key.as_str())
                })
            {
                return Err(());
            }
            // Each captured NPC key/name came from this Channel's OnceLock
            // NpcTalkBook. It cannot change within this pinned owner turn;
            // current actor/key membership above rechecks its name owner.
        }
        Ok(())
    }
    pub(super) fn observe(
        &self,
        runtime: &ChannelRuntimeV1,
        target: ExactActorRef,
    ) -> Result<Option<oteryn_gameplay_entities::NativeHealthChangeObservation>, ()> {
        self.validate_frame(runtime)?;
        // An unrelated valid target has no dependency on the optional names
        // cohort. Errors are never converted into an absent health callback.
        if matches!(
            runtime.observe_native_health_change(
                target,
                &|_: ExactActorRef, _: crate::foundation::MovementPositionSnapshot| None::<
                    oteryn_gameplay_entities::NativeHealthGuardName,
                >
            ),
            Ok(None)
        ) {
            return Ok(None);
        }
        self.validate(runtime)?;
        if self.players.is_none() && native_health_targets(runtime)?.contains(&target) {
            // A HealthForgotten actor born during a turn with no name read
            // waits for a fresh detached cohort; no foreign identity is guessed.
            return Err(());
        }
        runtime
            .observe_native_health_change(
                target,
                &CurrentNativeHealthNames {
                    runtime,
                    names: self,
                },
            )
            .map_err(|_| ())
    }
    /// Collect available proofs, not absence claims. The numeric owner still
    /// requires a proof for each actually selected HealthForgotten target.
    pub(super) fn observations(
        &self,
        runtime: &ChannelRuntimeV1,
    ) -> Result<Vec<oteryn_gameplay_entities::NativeHealthChangeObservation>, ()> {
        self.validate_frame(runtime)?;
        let targets = match native_health_targets(runtime) {
            Ok(targets) => targets,
            Err(()) => return Ok(Vec::new()), // No proof; never a no-event assertion.
        };
        Ok(targets
            .into_iter()
            .filter_map(|target| self.observe(runtime, target).ok().flatten())
            .collect())
    }
}
impl ComposedFreshAdmission<'_, '_, '_> {
    /// Pure capture under a short Runtime guard; no target means no name SQL.
    pub(super) fn prepare_native_health_names(
        &self,
        runtime: &ChannelRuntimeV1,
    ) -> Result<Option<PreparedSourceNames>, ()> {
        if native_health_targets(runtime)?.is_empty() {
            return Ok(None);
        }
        self.prepare_source_names(runtime, &["shadow tentacle", "possessed tree"])
            .map(Some)
    }
    pub(super) fn capture_native_health_names(
        &self,
        runtime: &ChannelRuntimeV1,
    ) -> Result<PreparedNativeHealthNames, ()> {
        let frame = NativeHealthNamesFrame::capture(runtime)?;
        // An unavailable name/candidate fact affects only a required health
        // callback. It does not grant absence or block unrelated work.
        let plan = self.prepare_native_health_names(runtime).ok().flatten();
        Ok(PreparedNativeHealthNames { frame, plan })
    }
    pub(super) async fn read_prepared_native_health_names(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        prepared: PreparedNativeHealthNames,
    ) -> ReadNativeHealthNames {
        let read = match prepared.plan {
            Some(plan) => self.read_source_names(permit, &plan).await.ok(),
            None => None,
        };
        ReadNativeHealthNames {
            frame: prepared.frame,
            read,
        }
    }
    pub(super) async fn read_native_health_names(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
    ) -> Result<ReadNativeHealthNames, ()> {
        let prepared = {
            let runtime = self.runtime.lock().await;
            self.capture_native_health_names(&runtime)?
        };
        Ok(self
            .read_prepared_native_health_names(permit, prepared)
            .await)
    }
    pub(super) fn seal_native_health_names(
        &self,
        runtime: &ChannelRuntimeV1,
        outcome: ReadNativeHealthNames,
    ) -> Result<NativeHealthNamesData, ()> {
        if !outcome.frame.current_for(runtime) {
            return Err(());
        }
        let unknown = || NativeHealthNamesData {
            frame: outcome.frame,
            players: None,
            player_cohort: Vec::new(),
            npcs: Vec::new(),
            names: Vec::new(),
        };
        let Some(read) = outcome.read else {
            return Ok(unknown());
        };
        if !read
            .plan
            .names
            .iter()
            .map(String::as_str)
            .eq(["shadow tentacle", "possessed tree"])
        {
            return Ok(unknown());
        }
        let players = read.plan.players.clone();
        let player_cohort = read.plan.player_cohort.clone();
        let mut npcs = Vec::new();
        for entry in read
            .plan
            .others
            .iter()
            .filter(|entry| entry.role == SourceNameRole::Npc)
        {
            let Some(key) = runtime.npc_key(entry.actor) else {
                return Ok(unknown());
            };
            npcs.push((entry.actor, key.to_owned()));
        }
        // The initial result seal still checks exact poses and full membership.
        let matched = match self.seal_source_names(runtime, read) {
            Ok(matched) => matched,
            Err(()) => return Ok(unknown()),
        };
        let covered = player_cohort
            .iter()
            .map(|(actor, _)| *actor)
            .chain(npcs.iter().map(|(actor, _)| *actor));
        let names = covered
            .map(|actor| {
                let name = match matched
                    .iter()
                    .find(|entry| entry.actor == actor)
                    .map(|entry| entry.name.to_ascii_lowercase())
                    .as_deref()
                {
                    Some("shadow tentacle") => {
                        oteryn_gameplay_entities::NativeHealthGuardName::ShadowTentacle
                    }
                    Some("possessed tree") => {
                        oteryn_gameplay_entities::NativeHealthGuardName::PossessedTree
                    }
                    _ => oteryn_gameplay_entities::NativeHealthGuardName::Other,
                };
                (actor, name)
            })
            .collect();
        let data = NativeHealthNamesData {
            frame: outcome.frame,
            players: Some(players),
            player_cohort,
            npcs,
            names,
        };
        if data.validate(runtime).is_err() {
            return Ok(unknown());
        }
        Ok(data)
    }
    pub(super) fn seal_native_health_observations(
        &self,
        runtime: &ChannelRuntimeV1,
        read: ReadNativeHealthNames,
    ) -> Result<Vec<oteryn_gameplay_entities::NativeHealthChangeObservation>, ()> {
        self.seal_native_health_names(runtime, read)?
            .observations(runtime)
    }
}

#[cfg(test)]
mod native_health_unknown_tests {
    use super::*;

    #[test]
    fn unknown_names_allow_unrelated_targets_but_never_bypass_retired_frame() -> Result<(), String>
    {
        let id = |tag| [tag, 0, 0, 0, 0, tag, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, tag];
        let world =
            crate::foundation::WorldId::decode(&id(1)).map_err(|error| format!("{error:?}"))?;
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            crate::foundation::ChannelId::decode(&id(2)).map_err(|error| format!("{error:?}"))?,
            crate::foundation::NodeId::decode(&id(3)).map_err(|error| format!("{error:?}"))?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            3,
            crate::foundation::ChannelContentPin::test(world),
        )
        .map_err(|error| format!("{error:?}"))?;
        let session = GameSessionId::decode(&id(4)).map_err(|error| format!("{error:?}"))?;
        let reserved = runtime
            .reserve_fresh_session(session)
            .map_err(|error| format!("{error:?}"))?;
        let player = runtime
            .commit_fresh_session(reserved)
            .map_err(|error| format!("{error:?}"))?;
        runtime
            .initialize_first_entry_position(player)
            .map_err(|error| format!("{error:?}"))?;
        let creature = runtime
            .admit_test_creature(crate::foundation::MovementLocalPosition {
                x: 1,
                y: 1,
                floor: 7,
            })
            .map_err(|error| format!("{error:?}"))?;
        let data = NativeHealthNamesData {
            frame: NativeHealthNamesFrame::capture(&runtime).map_err(|_| "current frame")?,
            players: None,
            player_cohort: Vec::new(),
            npcs: Vec::new(),
            names: Vec::new(),
        };
        assert!(matches!(data.observe(&runtime, player), Ok(None)));
        assert!(matches!(data.observe(&runtime, creature), Ok(None)));
        assert!(
            data.observations(&runtime)
                .is_ok_and(|proofs| proofs.is_empty()),
            "unknown names produce no proof, not a global unavailable gate"
        );
        runtime.retire_owner_cycle();
        assert!(
            data.observe(&runtime, player).is_err(),
            "an unrelated target never bypasses scope/frame retirement"
        );
        assert!(data.observations(&runtime).is_err());
        Ok(())
    }
}
