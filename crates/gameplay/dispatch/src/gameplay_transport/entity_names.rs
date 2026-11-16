//! S-ENTNAME: enrich only the observer's bounded interest set with authoritative actor names.
use super::ComposedFreshAdmission;
use super::appearance_change::observation::ObservationError;
use super::capabilities::SelectedCapabilities;
use super::connection::{AdmissionRefusal, FreshAdmissionAuthority};
use super::world_spatial::{ChannelEntities, EntityKind, EntityRef, SessionVisibility};
use crate::foundation::{ExactActorRef, GameSessionId};
use oteryn_protocol_oteryn::entity_names::{
    CAPABILITY_WORLD_SPATIAL_ENTITIES_V2, EntityDisplayName,
};

/// `HolderBusy` is transient: the durable holder stayed busy for this read's whole budget, or
/// a selected actor left between the two runtime reads. The next refresh observes again.
pub(super) async fn observe<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    selected: SelectedCapabilities,
) -> Result<ChannelEntities, ObservationError> {
    if selected.contains(CAPABILITY_WORLD_SPATIAL_ENTITIES_V2) {
        let r = authority.observe_named_visible_entities(actor).await;
        if let Err(e) = &r {
            crate::gameplay_transport::operator_event(&format!("[DBG-prof] named_observe_err {e:?}"));
        }
        r
    } else {
        authority
            .observe_visible_entities(actor)
            .await
            .ok_or(ObservationError::Unavailable)
    }
}

enum Source {
    Player(ExactActorRef, GameSessionId),
    Name(EntityDisplayName),
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn named_channel_entities(
        &self,
        observer: ExactActorRef,
    ) -> Result<ChannelEntities, ObservationError> {
        use ObservationError::{HolderBusy, Unavailable};
        let mut channel = self
            .observe_channel_entities(observer)
            .await
            .ok_or(Unavailable)?;
        // Use the same interest owner as the session: at most 256 name reads, and none for
        // actors outside this view. Objects have no display name.
        let selected = SessionVisibility::default()
            .snapshot(&channel, &mut |_| Ok(()))
            .map_err(|_| Unavailable)?;
        let sources = {
            let runtime = self.runtime.lock().await;
            let house = runtime.house_observer(observer).map_err(|_| Unavailable)?;
            let mut players = Vec::new();
            let mut hirelings = Vec::new();
            if let Some((session, _)) = house {
                for entry in runtime
                    .house_visible_players(observer, session)
                    .map_err(|_| Unavailable)?
                {
                    let (session, _) = runtime
                        .house_observer(entry.actor)
                        .map_err(|_| Unavailable)?
                        .ok_or(Unavailable)?;
                    players.push((entry.actor, session));
                }
                hirelings = runtime
                    .house_hireling_entities(observer, session)
                    .map_err(|_| Unavailable)?;
            } else {
                players.extend(
                    runtime
                        .positioned_melee_census()
                        .map_err(|_| Unavailable)?
                        .into_iter()
                        .filter_map(|(actor, _, session)| session.map(|session| (actor, session))),
                );
            }
            let visible = runtime.visible_entities();
            let mut sources = Vec::new();
            for entity in &selected.entities {
                let same = |actor: ExactActorRef| {
                    actor.placement_identity() == entity.entity.identity
                        && actor.actor_local_generation() == entity.entity.generation
                };
                // An actor missing here left after the channel read: the next refresh no
                // longer sees it, so this tick is skipped instead of naming it `None`.
                let source = match entity.kind {
                    EntityKind::Player => {
                        let (actor, session) = players
                            .iter()
                            .find(|(actor, _)| same(*actor))
                            .ok_or(HolderBusy)?;
                        Source::Player(*actor, *session)
                    }
                    EntityKind::Creature => {
                        let actor = visible
                            .creatures
                            .iter()
                            .find(|entry| same(entry.actor))
                            .ok_or(HolderBusy)?
                            .actor;
                        let creature =
                            runtime.companion_snapshot(actor).map_err(|_| Unavailable)?;
                        Source::Name(
                            EntityDisplayName::new(&creature.state.policy.display_name)
                                .map_err(|_| Unavailable)?,
                        )
                    }
                    EntityKind::Npc => {
                        let name =
                            if let Some(entry) = hirelings.iter().find(|entry| same(entry.actor)) {
                                entry.name.as_str()
                            } else {
                                let actor = visible
                                    .npcs
                                    .iter()
                                    .find(|entry| same(entry.actor))
                                    .ok_or(HolderBusy)?
                                    .actor;
                                let key = runtime.npc_key(actor).ok_or(Unavailable)?;
                                self.states
                                    .chat
                                    .npc_talk()
                                    .and_then(|talk| talk.display_name(key))
                                    .ok_or(Unavailable)?
                            };
                        Source::Name(EntityDisplayName::new(name).map_err(|_| Unavailable)?)
                    }
                    EntityKind::Corpse | EntityKind::GroundItem => continue,
                };
                sources.push((entity.entity, source));
            }
            sources
        };
        // Durable reads run after dropping the runtime guard. The existing chat name is
        // already an admitted Character name; clients need not select chat to receive names.
        // Holder contention that outlasts the shared budget is `HolderBusy`, like the
        // appearance read; every other failure stays a lost observer.
        let retry = super::AdmissionRetryBudget::for_foreground_command(self.root)
            .map_err(|_| HolderBusy)?;
        for (entity, source) in sources {
            let name = match source {
                Source::Name(name) => name,
                Source::Player(actor, session) => {
                    let name = match self.states.chat.speaker_name(session) {
                        Some(name) => name,
                        None => match self.chat_speaker_name_with_budget(session, &retry).await {
                            Ok((name, _, _)) => name,
                            Err(AdmissionRefusal::Unavailable) => return Err(HolderBusy),
                            // A peer that ended its session (logout, relog) after the runtime
                            // read is gone from the next refresh; one that still holds it is
                            // a lost observer.
                            Err(_) if !self.holds_session(actor, session).await => {
                                return Err(HolderBusy);
                            }
                            Err(_) => return Err(Unavailable),
                        },
                    };
                    EntityDisplayName::new(&name).map_err(|_| Unavailable)?
                }
            };
            let entry = channel
                .entities
                .iter_mut()
                .find(|entry| {
                    EntityRef {
                        identity: entry.identity,
                        generation: entry.generation,
                    } == entity
                })
                .ok_or(Unavailable)?;
            entry.display_name = Some(name);
        }
        Ok(channel)
    }

    async fn holds_session(&self, actor: ExactActorRef, session: GameSessionId) -> bool {
        let runtime = self.runtime.lock().await;
        matches!(runtime.house_observer(actor), Ok(Some((held, _))) if held == session)
            || runtime.positioned_melee_census().is_ok_and(|census| {
                census
                    .into_iter()
                    .any(|(entry, _, held)| entry == actor && held == Some(session))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gameplay_transport::world_spatial::{
        ActorPosition, ChannelEntity, VisibilityUpdate, VisibleKind,
    };
    use oteryn_protocol_oteryn::world_spatial::StepDirection;

    #[test]
    fn entity_names_stationary_name_changes_are_spatial_updates()
    -> Result<(), oteryn_protocol_oteryn::world_spatial::WorldSpatialError> {
        let mut channel = ChannelEntities {
            content_generation: [1; 32],
            observer: [1; 16],
            entities: vec![ChannelEntity {
                display_name: Some(EntityDisplayName::new("Before")?),
                health_percent: 100,
                direction: StepDirection::South,
                kind: VisibleKind::Player,
                identity: [1; 16],
                generation: 1,
                position: ActorPosition {
                    x: 0,
                    y: 0,
                    floor: 7,
                },
                revision: 1,
            }],
            lights: Default::default(),
            observed_at_ms: 0,
        };
        let mut view = SessionVisibility::default();
        view.snapshot(&channel, &mut |_| Ok(()))?;
        channel.entities[0].display_name = Some(EntityDisplayName::new("After")?);
        let VisibilityUpdate::Delta(delta) = view.refresh(&channel, &mut |_| Ok(()))? else {
            return Err(oteryn_protocol_oteryn::world_spatial::WorldSpatialError::Malformed);
        };
        assert_eq!(delta.update.len(), 1);
        assert_eq!(
            delta.update[0].display_name,
            channel.entities[0].display_name
        );
        assert_eq!(
            view.refresh(&channel, &mut |_| Ok(()))?,
            VisibilityUpdate::Unchanged
        );
        Ok(())
    }
}
