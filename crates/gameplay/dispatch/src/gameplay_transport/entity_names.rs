//! S-ENTNAME: enrich only the observer's bounded interest set with authoritative actor names.
use super::ComposedFreshAdmission;
use super::capabilities::SelectedCapabilities;
use super::connection::{AdmissionRefusal, FreshAdmissionAuthority};
use super::world_spatial::{ChannelEntities, EntityKind, EntityRef, SessionVisibility};
use crate::foundation::{ExactActorRef, GameSessionId};
use oteryn_protocol_oteryn::entity_names::EntityDisplayName;

pub(super) async fn observe<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    selected: SelectedCapabilities,
) -> Option<ChannelEntities> {
    Box::pin(observe_result(authority, actor, selected))
        .await
        .ok()
}

pub(super) async fn observe_result<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    selected: SelectedCapabilities,
) -> Result<ChannelEntities, AdmissionRefusal> {
    authority.observe_projected_entities(actor, selected).await
}

enum Source {
    Player(GameSessionId),
    Name(EntityDisplayName),
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn named_channel_entities(
        &self,
        observer: ExactActorRef,
        forecasts: bool,
    ) -> Result<ChannelEntities, AdmissionRefusal> {
        let (mut channel, sources) = Box::pin(self.named_entity_sources(observer, forecasts))
            .await
            .ok_or(AdmissionRefusal::Rejected)?;
        // Carry the authenticated observer's actual name through initial
        // publication. Unnamed peers can wait for the next visibility batch.
        let join_name = super::connection::join_recovery::join_name(observer);
        let retry = super::AdmissionRetryBudget::for_foreground_command(self.root)
            .map_err(|_| AdmissionRefusal::Rejected)?;
        let optional = super::AdmissionRetryBudget::for_optional_read(self.root);
        let joining = crate::durability::holder_deadline().is_some();
        for (entity, source) in sources {
            let own = entity.identity == channel.observer;
            let name = match source {
                Source::Name(name) => name,
                Source::Player(_) if own && join_name.is_some() => {
                    join_name.ok_or(AdmissionRefusal::Rejected)?
                }
                Source::Player(session) => {
                    let name = match self.states.chat.speaker_name(session) {
                        Some(name) => name,
                        None => {
                            let read = self.chat_speaker_name_with_budget(
                                session,
                                if joining && !own { &optional } else { &retry },
                            );
                            let (result, contended) =
                                Box::pin(crate::durability::with_holder_contention(read)).await;
                            match result {
                                Ok((name, _, _)) => name,
                                Err(AdmissionRefusal::Unavailable)
                                    if joining && !own && contended =>
                                {
                                    continue;
                                }
                                Err(error) => return Err(error),
                            }
                        }
                    };
                    EntityDisplayName::new(&name).map_err(|_| AdmissionRefusal::Rejected)?
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
                .ok_or(AdmissionRefusal::Rejected)?;
            entry.display_name = Some(name);
        }
        // The first interest pass selected the actors we actually named. Do not
        // let a deferred peer promote an out-of-range unnamed actor into the batch.
        SessionVisibility::default().defer_unnamed_actors(&mut channel);
        Ok(channel)
    }

    async fn named_entity_sources(
        &self,
        observer: ExactActorRef,
        forecasts: bool,
    ) -> Option<(ChannelEntities, Vec<(EntityRef, Source)>)> {
        let channel = self
            .observe_channel_entities_projected(observer, forecasts)
            .await?;
        // Use the same interest owner as the session: at most 256 name reads, and none for
        // actors outside this view. Objects have no display name.
        let selected = SessionVisibility::default()
            .snapshot(&channel, &mut |_| Ok(()))
            .ok()?;
        let sources = {
            let runtime = self.runtime.lock().await;
            let house = runtime.house_observer(observer).ok()?;
            let mut players = Vec::new();
            let mut hirelings = Vec::new();
            if let Some((session, _)) = house {
                for entry in runtime.house_visible_players(observer, session).ok()? {
                    let (session, _) = runtime.house_observer(entry.actor).ok()??;
                    players.push((entry.actor, session));
                }
                hirelings = runtime.house_hireling_entities(observer, session).ok()?;
            } else {
                players.extend(
                    runtime
                        .positioned_melee_census()
                        .ok()?
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
                let source = match entity.kind {
                    EntityKind::Player => {
                        Source::Player(players.iter().find(|(actor, _)| same(*actor))?.1)
                    }
                    EntityKind::Creature => {
                        let actor = visible
                            .creatures
                            .iter()
                            .find(|entry| same(entry.actor))?
                            .actor;
                        let creature = runtime.companion_snapshot(actor).ok()?;
                        Source::Name(
                            EntityDisplayName::new(&creature.state.policy.display_name).ok()?,
                        )
                    }
                    EntityKind::Npc => {
                        let name = if let Some(entry) =
                            hirelings.iter().find(|entry| same(entry.actor))
                        {
                            entry.name.as_str()
                        } else {
                            let actor = visible.npcs.iter().find(|entry| same(entry.actor))?.actor;
                            let key = runtime.npc_key(actor)?;
                            self.states.chat.npc_talk()?.display_name(key)?
                        };
                        Source::Name(EntityDisplayName::new(name).ok()?)
                    }
                    EntityKind::Corpse | EntityKind::GroundItem => continue,
                };
                sources.push((entity.entity, source));
            }
            sources
        };
        Some((channel, sources))
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
                source_icons:
                    oteryn_protocol_oteryn::world_spatial_entities::NativeCreatureIcons::default(),
                motion: None,
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
