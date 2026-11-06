//! Read the existing caster map producer into a private item-view instance.
//! Its handles never leave this function or alter the caster's handle table.
use super::*;
use oteryn_protocol_oteryn::cast::{DisplayTile, PublicMedia};
pub(super) fn display_map(
    snapshot: oteryn_protocol_oteryn::world_map::WorldMapViewSnapshot,
) -> Vec<PublicMedia> {
    let tiles: Vec<_> = snapshot
        .tiles
        .into_iter()
        .map(|t| DisplayTile {
            x: u16::try_from(t.position.x - snapshot.header.origin.x + 16).unwrap_or(0),
            y: u16::try_from(t.position.y - snapshot.header.origin.y + 16).unwrap_or(0),
            floor: t.position.floor,
            appearances: t.items.into_iter().map(|i| i.appearance_id).collect(),
        })
        .collect();
    if tiles.is_empty() {
        return vec![PublicMedia::Map {
            reset: true,
            last: true,
            tiles: vec![],
        }];
    }
    let count = tiles.len().div_ceil(32);
    tiles
        .chunks(32)
        .enumerate()
        .map(|(i, tiles)| PublicMedia::Map {
            reset: i == 0,
            last: i + 1 == count,
            tiles: tiles.to_vec(),
        })
        .collect()
}
// Handles and camera coordinates are derived only at the public projection boundary.
pub(super) fn project_actor(
    actors: &mut Vec<(u32, u16, u16, u16, u8)>,
    entity: &world_spatial::WorldSpatialEntity,
    origin: world_spatial::ActorPosition,
    hidden: bool,
) -> Option<bool> {
    if hidden {
        return Some(false);
    }
    let world_spatial::EntityDetail::Actor {
        appearance_ref,
        health_percent,
        ..
    } = entity.detail
    else {
        return Some(false);
    };
    let x = u16::try_from(entity.position.x.checked_sub(origin.x)?.checked_add(16)?).ok()?;
    let y = u16::try_from(
        entity
            .position
            .y
            .checked_sub(origin.y)?
            .checked_add(16)?
            .checked_add(i32::from(entity.position.floor + 15) * 32)?,
    )
    .ok()?;
    let appearance = if entity.kind == world_spatial::EntityKind::Player {
        1
    } else {
        u16::try_from(appearance_ref).ok()?
    };
    actors.push((
        u32::try_from(actors.len() + 1).ok()?,
        x,
        y,
        appearance,
        health_percent,
    ));
    Some(true)
}
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn cast_map(
        &self,
        admitted: connection::AdmittedSession,
    ) -> Option<(world_spatial::ActorPosition, Vec<PublicMedia>)> {
        let actor = admitted.runtime_actor?;
        let mut view = world_map::SessionMapView::default();
        let mut items =
            item_view::SessionItemView::resume(item_view::ItemViewContinuity::default())
                .with_map_view();
        let house = self
            .runtime
            .lock()
            .await
            .house_observer(actor)
            .ok()?
            .is_some();
        let update = if house {
            self.observe_house_item_map(admitted, &mut view, &mut items)
                .await?
        } else {
            bundle_map_position::observe(self, actor, &mut view, &mut items).await?
        };
        let world_map::MapUpdate::Snapshot(snapshot) = update.ok()?? else {
            return None;
        };
        let snapshot =
            oteryn_protocol_oteryn::world_map::decode_world_map_snapshot(&snapshot.payload).ok()?;
        Some((snapshot.header.origin, display_map(snapshot)))
    }
    pub(in crate::gameplay_transport) async fn cast_public_line(
        &self,
        admitted: connection::AdmittedSession,
        line: &oteryn_protocol_oteryn::chat::ChatLine,
    ) {
        let oteryn_protocol_oteryn::chat::ChatLine::Local {
            speaker,
            position,
            mode,
            text,
            ..
        } = line
        else {
            return;
        };
        self.cast_public_media(
            admitted,
            Some(world_spatial::EntityRef {
                identity: speaker.identity,
                generation: speaker.generation.get(),
            }),
            vec![PublicMedia::Speech {
                position: *position,
                mode: *mode,
                text: text.clone(),
            }],
        )
        .await;
    }
    pub(in crate::gameplay_transport) async fn cast_public_effects(
        &self,
        admitted: connection::AdmittedSession,
        events: &[oteryn_protocol_oteryn::presentation::PresentationEvent],
    ) {
        use oteryn_protocol_oteryn::presentation::PresentationEventKind;
        let events: Vec<_> = events
            .iter()
            .copied()
            .filter(|e| {
                matches!(
                    e.kind,
                    PresentationEventKind::MagicEffect { .. }
                        | PresentationEventKind::Missile { .. }
                )
            })
            .collect();
        if !events.is_empty() {
            self.cast_public_media(admitted, None, vec![PublicMedia::Effects(events)])
                .await;
        }
    }
    async fn cast_public_media(
        &self,
        admitted: connection::AdmittedSession,
        speaker: Option<world_spatial::EntityRef>,
        media: Vec<PublicMedia>,
    ) {
        let Some(node) = self.states.casts.get() else {
            return;
        };
        let Some(facts) = self.cast_facts(admitted).await else {
            return;
        };
        let Ok(Reply::Capture(permit)) = node.service.apply(Operation::Capture(facts)).await else {
            return;
        };
        let Some((visible, scene, consents, origin, shown)) =
            self.cast_projection_value(admitted).await
        else {
            let _ = node.service.apply(Operation::Slate(facts)).await;
            return;
        };
        if speaker.is_some_and(|id| !shown.contains(&id)) {
            return;
        }
        let media = media
            .into_iter()
            .filter_map(|m| relative_media(m, origin))
            .collect();
        let Some(current) = self.cast_facts(admitted).await else {
            return;
        };
        if facts.authority != current.authority {
            return;
        }
        let _ = node
            .service
            .apply(Operation::PublishView(
                permit, current, visible, scene, media, consents,
            ))
            .await;
    }
}

fn relative_media(
    mut media: PublicMedia,
    origin: world_spatial::ActorPosition,
) -> Option<PublicMedia> {
    use oteryn_protocol_oteryn::presentation::PresentationEventKind;
    let project = |p: &mut world_spatial::ActorPosition| -> Option<()> {
        p.x = p.x.checked_sub(origin.x)?.checked_add(16)?;
        p.y = p.y.checked_sub(origin.y)?.checked_add(16)?;
        u16::try_from(p.x).ok()?;
        u16::try_from(p.y).ok()?;
        Some(())
    };
    match &mut media {
        PublicMedia::Speech { position, .. } => project(position)?,
        PublicMedia::Effects(events) => {
            events.retain_mut(|e| match &mut e.kind {
                PresentationEventKind::MagicEffect { position, .. } => {
                    position.floor = position.floor.abs();
                    project(position).is_some()
                }
                PresentationEventKind::Missile { from, to, .. } => {
                    from.floor = from.floor.abs();
                    to.floor = to.floor.abs();
                    project(from).is_some() && project(to).is_some()
                }
                _ => false,
            });
            if events.is_empty() {
                return None;
            }
        }
        PublicMedia::Map { .. } => {}
    }
    Some(media)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn s17_hidden_actor_does_not_change_public_handles_or_bytes() -> Result<(), String> {
        let origin = world_spatial::ActorPosition {
            x: 30000,
            y: 31000,
            floor: -7,
        };
        let entity = |id: u8| world_spatial::WorldSpatialEntity {
            display_name: None,
            kind: world_spatial::EntityKind::Player,
            entity: world_spatial::EntityRef {
                identity: [id; 16],
                generation: 1,
            },
            position: origin,
            detail: world_spatial::EntityDetail::Actor {
                direction: world_spatial::StepDirection::South,
                appearance_ref: 0,
                health_percent: 100,
            },
        };
        let bytes =
            |input: &[(world_spatial::WorldSpatialEntity, bool)]| -> Result<Vec<u8>, String> {
                let mut actors = Vec::new();
                for (entity, hidden) in input {
                    project_actor(&mut actors, entity, origin, *hidden).ok_or("projection")?;
                }
                Message::Scene {
                    epoch: 1,
                    sequence: 1,
                    scene: oteryn_protocol_oteryn::cast::Scene {
                        tiles: vec![],
                        actors,
                    },
                }
                .encode()
                .map_err(|e| format!("{e:?}"))
            };
        assert_eq!(
            bytes(&[(entity(1), false), (entity(3), false)])?,
            bytes(&[(entity(1), false), (entity(2), true), (entity(3), false)])?
        );
        Ok(())
    }
    #[test]
    fn s17_public_speech_uses_the_same_relative_camera_as_map() {
        let origin = world_spatial::ActorPosition {
            x: 30000,
            y: 31000,
            floor: -7,
        };
        let media = PublicMedia::Speech {
            position: world_spatial::ActorPosition {
                x: 30001,
                y: 31002,
                floor: -7,
            },
            mode: oteryn_protocol_oteryn::chat::ChatSpeechMode::Say,
            text: "heard".into(),
        };
        assert_eq!(
            relative_media(media, origin),
            Some(PublicMedia::Speech {
                position: world_spatial::ActorPosition {
                    x: 17,
                    y: 18,
                    floor: -7
                },
                mode: oteryn_protocol_oteryn::chat::ChatSpeechMode::Say,
                text: "heard".into()
            })
        );
    }
}
