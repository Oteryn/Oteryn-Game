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
