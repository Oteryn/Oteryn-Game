//! S-ENTNAME: enrich only the observer's bounded interest set with authoritative actor names.
use super::ComposedFreshAdmission;
use super::capabilities::SelectedCapabilities;
use super::connection::FreshAdmissionAuthority;
use super::world_spatial::{ChannelEntities, EntityKind, EntityRef, SessionVisibility};
use crate::foundation::{ExactActorRef, GameSessionId};
use oteryn_protocol_oteryn::entity_names::{
    CAPABILITY_WORLD_SPATIAL_ENTITIES_V2, EntityDisplayName,
};

pub(super) async fn observe<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    selected: SelectedCapabilities,
) -> Option<ChannelEntities> {
    if selected.contains(CAPABILITY_WORLD_SPATIAL_ENTITIES_V2) {
        authority.observe_named_visible_entities(actor).await
    } else {
        authority.observe_visible_entities(actor).await
    }
}

enum Source {
    Player(GameSessionId),
    Name(EntityDisplayName),
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn named_channel_entities(
        &self,
        observer: ExactActorRef,
    ) -> Option<ChannelEntities> {
        let mut channel = self.observe_channel_entities(observer).await?;
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
        // Durable reads run after dropping the runtime guard. The existing chat name is
        // already an admitted Character name; clients need not select chat to receive names.
        let retry = super::AdmissionRetryBudget::for_command(self.root);
        for (entity, source) in sources {
            let name = match source {
                Source::Name(name) => name,
                Source::Player(session) => {
                    let name = match self.states.chat.speaker_name(session) {
                        Some(name) => name,
                        None => self.chat_speaker_name_with_budget(session, &retry).await?.0,
                    };
                    EntityDisplayName::new(&name).ok()?
                }
            };
            let entry = channel.entities.iter_mut().find(|entry| {
                EntityRef {
                    identity: entry.identity,
                    generation: entry.generation,
                } == entity
            })?;
            entry.display_name = Some(name);
        }
        Some(channel)
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
