//! Views of the native House owner. Ephemeral actors never enter durable Hireling receipts.
use super::*;

fn direction(facing: Option<crate::foundation::MovementFacing>) -> StepDirection {
    match facing {
        Some(crate::foundation::MovementFacing::North) => StepDirection::North,
        Some(crate::foundation::MovementFacing::East) => StepDirection::East,
        Some(crate::foundation::MovementFacing::West) => StepDirection::West,
        Some(crate::foundation::MovementFacing::South) | None => StepDirection::South,
    }
}

pub(super) fn entities(
    runtime: &ChannelRuntimeV1,
    observer: ExactActorRef,
    observed_at_ms: u64,
) -> Result<Option<world_spatial::ChannelEntities>, CarrierError> {
    let Some((session, _)) = runtime.house_observer(observer)? else {
        return Ok(None);
    };
    let mut entities = Vec::new();
    for entry in runtime.house_visible_players(observer, session)? {
        let (entry_session, _) = runtime
            .house_observer(entry.actor)?
            .ok_or(CarrierError::WrongScope)?;
        let movement = runtime.read_house_movement(entry.actor, entry_session)?;
        entities.push(world_spatial::ChannelEntity {
            health_percent: 100,
            direction: direction(movement.facing()),
            kind: world_spatial::VisibleKind::Player,
            identity: entry.actor.placement_identity(),
            generation: entry.generation,
            position: ActorPosition {
                x: entry.position.x,
                y: entry.position.y,
                floor: entry.position.floor,
            },
            revision: entry.revision,
        });
    }
    entities.extend(
        runtime
            .house_hireling_entities(observer, session)?
            .into_iter()
            .map(|entry| world_spatial::ChannelEntity {
                health_percent: 100,
                direction: direction(Some(entry.direction)),
                kind: world_spatial::VisibleKind::Hireling,
                identity: entry.actor.placement_identity(),
                generation: entry.actor.actor_local_generation(),
                position: ActorPosition {
                    x: entry.position.x,
                    y: entry.position.y,
                    floor: entry.position.floor,
                },
                revision: entry.revision,
            }),
    );
    Ok(Some(world_spatial::ChannelEntities {
        content_generation: runtime.content_pin().client_artifact_digest(),
        observer: observer.placement_identity(),
        entities,
        lights: world_spatial::EntityLights::new(),
        observed_at_ms,
    }))
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Call under the current runtime lock after the durable House read was reconciled.
    pub(in crate::gameplay_transport) fn hireling_actor_bindings(
        runtime: &ChannelRuntimeV1,
        observer: ExactActorRef,
        session: GameSessionId,
    ) -> Option<oteryn_protocol_oteryn::hireling::management::Reply> {
        use oteryn_protocol_oteryn::hireling::management::{HirelingActorBinding, Reply};
        let bindings = runtime
            .house_hireling_entities(observer, session)
            .ok()?
            .into_iter()
            .map(|entry| {
                Some(HirelingActorBinding {
                    hireling: crate::durability::house_hireling::npc_identity(&entry.key)?,
                    entity: EntityRef {
                        identity: entry.actor.placement_identity(),
                        generation: entry.actor.actor_local_generation(),
                    },
                })
            })
            .collect::<Option<Vec<_>>>()?;
        let reply = Reply::Projection(bindings);
        reply.encode()?;
        Some(reply)
    }
}

#[cfg(test)]
#[path = "house_hireling_view/tests.rs"]
mod tests;
