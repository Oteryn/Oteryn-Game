//! Read the existing caster map producer into a private item-view instance.
//! Its handles never leave this function or alter the caster's handle table.
use super::*;
use oteryn_protocol_oteryn::cast::{DisplayTile, PublicMedia};
#[derive(Clone, Copy)]
pub(in crate::gameplay_transport) struct Viewport {
    pub(super) origin: world_spatial::ActorPosition,
    pub(super) first_visible_floor: i16,
}
fn native_position(
    mut position: world_spatial::ActorPosition,
) -> Option<world_spatial::ActorPosition> {
    position.floor = position.floor.checked_abs()?.checked_neg()?;
    (-15..=0).contains(&position.floor).then_some(position)
}
impl Viewport {
    pub(in crate::gameplay_transport) fn matches_runtime(
        self,
        origin: world_spatial::ActorPosition,
    ) -> bool {
        same_camera(origin, self.origin)
    }
    pub(super) fn visible(self, position: world_spatial::ActorPosition) -> bool {
        native_position(position).is_some_and(|p| {
            p.floor <= self.first_visible_floor
                && oteryn_protocol_oteryn::world_map::in_window(self.origin, p)
        })
    }
}
pub(super) fn display_map(
    snapshot: oteryn_protocol_oteryn::world_map::WorldMapViewSnapshot,
) -> Vec<PublicMedia> {
    let tiles: Vec<_> = snapshot
        .tiles
        .into_iter()
        .filter(|t| t.position.floor <= snapshot.header.first_visible_floor)
        .map(|t| DisplayTile {
            x: u16::try_from(t.position.x - snapshot.header.origin.x + 16).unwrap_or(0),
            y: u16::try_from(t.position.y - snapshot.header.origin.y + 16).unwrap_or(0),
            floor: t.position.floor,
            appearances: t.items.into_iter().map(|i| i.appearance_id).collect(),
        })
        .collect();
    if tiles.is_empty() {
        return vec![
            PublicMedia::Camera {
                floor: snapshot.header.origin.floor,
            },
            PublicMedia::Map {
                reset: true,
                last: true,
                tiles: vec![],
            },
        ];
    }
    let count = tiles.len().div_ceil(32);
    std::iter::once(PublicMedia::Camera {
        floor: snapshot.header.origin.floor,
    })
    .chain(
        tiles
            .chunks(32)
            .enumerate()
            .map(|(i, tiles)| PublicMedia::Map {
                reset: i == 0,
                last: i + 1 == count,
                tiles: tiles.to_vec(),
            }),
    )
    .collect()
}
pub(super) fn same_camera(
    runtime: world_spatial::ActorPosition,
    map: world_spatial::ActorPosition,
) -> bool {
    native_position(runtime) == Some(map)
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
            .checked_add(i32::from(native_position(entity.position)?.floor + 15) * 32)?,
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
    ) -> Option<(Viewport, Vec<PublicMedia>)> {
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
        Some((
            Viewport {
                origin: snapshot.header.origin,
                first_visible_floor: snapshot.header.first_visible_floor,
            },
            display_map(snapshot),
        ))
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
        let Some((viewport, _)) = self.cast_map(admitted).await else {
            let _ = node.service.apply(Operation::Slate(facts)).await;
            return;
        };
        let Some((visible, scene, consents, shown)) =
            self.cast_projection_value(admitted, viewport).await
        else {
            let _ = node.service.apply(Operation::Slate(facts)).await;
            return;
        };
        if speaker.is_some_and(|id| !shown.contains(&id)) {
            return;
        }
        let media = media
            .into_iter()
            .filter_map(|m| relative_media(m, viewport))
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

fn relative_media(mut media: PublicMedia, viewport: Viewport) -> Option<PublicMedia> {
    use oteryn_protocol_oteryn::presentation::PresentationEventKind;
    let project = |p: &mut world_spatial::ActorPosition| -> Option<()> {
        p.x = p.x.checked_sub(viewport.origin.x)?.checked_add(16)?;
        p.y = p.y.checked_sub(viewport.origin.y)?.checked_add(16)?;
        u16::try_from(p.x).ok()?;
        u16::try_from(p.y).ok()?;
        Some(())
    };
    match &mut media {
        PublicMedia::Speech { position, .. } => {
            if !viewport.visible(*position) {
                return None;
            }
            position.floor = native_position(*position)?.floor;
            project(position)?;
        }
        PublicMedia::Effects(events) => {
            events.retain_mut(|e| match &mut e.kind {
                PresentationEventKind::MagicEffect { position, .. } => {
                    if !viewport.visible(*position) {
                        return false;
                    }
                    position.floor = position.floor.saturating_abs();
                    project(position).is_some()
                }
                PresentationEventKind::Missile { from, to, .. } => {
                    if !viewport.visible(*from) || !viewport.visible(*to) {
                        return false;
                    }
                    from.floor = from.floor.saturating_abs();
                    to.floor = to.floor.saturating_abs();
                    project(from).is_some() && project(to).is_some()
                }
                _ => false,
            });
            if events.is_empty() {
                return None;
            }
        }
        PublicMedia::Map { .. } | PublicMedia::Camera { .. } => {}
    }
    Some(media)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn s17_review_runtime_camera_matches_native_map_floor() {
        let runtime = world_spatial::ActorPosition {
            x: 30000,
            y: 31000,
            floor: 7,
        };
        assert!(same_camera(
            runtime,
            world_spatial::ActorPosition {
                floor: -7,
                ..runtime
            }
        ));
    }
    #[test]
    fn s17_review_actor_floor_matches_native_map_plane() -> Result<(), String> {
        let at = world_spatial::ActorPosition {
            x: 30000,
            y: 31000,
            floor: 7,
        };
        let entity = world_spatial::WorldSpatialEntity {
            display_name: None,
            kind: world_spatial::EntityKind::Player,
            entity: world_spatial::EntityRef {
                identity: [1; 16],
                generation: 1,
            },
            position: at,
            detail: world_spatial::EntityDetail::Actor {
                direction: world_spatial::StepDirection::South,
                appearance_ref: 0,
                health_percent: 100,
            },
        };
        let mut actors = Vec::new();
        project_actor(&mut actors, &entity, at, false).ok_or("actor")?;
        assert_eq!(actors[0].2, 16 + 8 * 32);
        Ok(())
    }
    #[test]
    fn s17_review_runtime_speech_uses_native_floor() {
        let runtime = world_spatial::ActorPosition {
            x: 30000,
            y: 31000,
            floor: 7,
        };
        let media = PublicMedia::Speech {
            position: runtime,
            mode: oteryn_protocol_oteryn::chat::ChatSpeechMode::Say,
            text: "heard".into(),
        };
        assert_eq!(
            relative_media(
                media,
                Viewport {
                    origin: world_spatial::ActorPosition {
                        floor: -7,
                        ..runtime
                    },
                    first_visible_floor: -7
                }
            ),
            Some(PublicMedia::Speech {
                position: world_spatial::ActorPosition {
                    x: 16,
                    y: 16,
                    floor: -7
                },
                mode: oteryn_protocol_oteryn::chat::ChatSpeechMode::Say,
                text: "heard".into()
            })
        );
    }
    #[test]
    fn s17_review_roof_hidden_tiles_are_not_exported() -> Result<(), String> {
        use oteryn_protocol_oteryn::world_map::*;
        let origin = world_spatial::ActorPosition {
            x: 30000,
            y: 31000,
            floor: -7,
        };
        let item = MapItem {
            definition: MapDefinition::Terrain(std::num::NonZeroU32::new(1).ok_or("definition")?),
            count: 1,
            sub_type: 0,
            origin: MapOrigin::BaseOrdinal {
                ordinal: 0,
                object_revision: 0,
            },
            appearance_id: 100,
        };
        let tiles = [-7, -6]
            .into_iter()
            .map(|floor| MapTile {
                position: world_spatial::ActorPosition { floor, ..origin },
                items: vec![item],
                more: false,
                ground_speed: 100,
            })
            .collect();
        let snapshot = WorldMapViewSnapshot {
            header: MapViewHeader {
                content_generation: [1; 32],
                bundle_digest: [2; 32],
                reset_epoch: 1,
                origin,
                first_visible_floor: -7,
            },
            tiles,
        };
        let encoded = encode_world_map_snapshot(&snapshot).map_err(|e| format!("{e:?}"))?;
        let snapshot = decode_world_map_snapshot(&encoded).map_err(|e| format!("{e:?}"))?;
        let media = display_map(snapshot);
        let count: usize = media
            .iter()
            .map(|m| {
                if let PublicMedia::Map { tiles, .. } = m {
                    tiles.len()
                } else {
                    0
                }
            })
            .sum();
        assert_eq!(
            count, 1,
            "tile above caster's visible-floor boundary must not enter cast bytes"
        );
        Ok(())
    }
    #[test]
    fn s17_review_roof_hides_actor_speech_and_effect_positions() {
        use oteryn_protocol_oteryn::presentation::{
            PresentationEvent, PresentationEventKind, PresentationSource,
        };
        let origin = world_spatial::ActorPosition {
            x: 30000,
            y: 31000,
            floor: -7,
        };
        let viewport = Viewport {
            origin,
            first_visible_floor: -7,
        };
        let position = world_spatial::ActorPosition { floor: 6, ..origin };
        assert!(!viewport.visible(position));
        assert!(viewport.visible(world_spatial::ActorPosition { floor: 7, ..origin }));
        assert!(
            relative_media(
                PublicMedia::Speech {
                    position,
                    mode: oteryn_protocol_oteryn::chat::ChatSpeechMode::Say,
                    text: "hidden".into()
                },
                viewport
            )
            .is_none()
        );
        let effect = PresentationEvent {
            decision_ordinal: 1,
            emission_ordinal: 1,
            kind: PresentationEventKind::MagicEffect {
                position,
                effect_id: 10,
                source: PresentationSource::Others,
            },
        };
        assert!(relative_media(PublicMedia::Effects(vec![effect]), viewport).is_none());
        let visible = PresentationEvent {
            kind: PresentationEventKind::MagicEffect {
                position: world_spatial::ActorPosition { floor: 7, ..origin },
                effect_id: 10,
                source: PresentationSource::Others,
            },
            ..effect
        };
        assert!(
            relative_media(PublicMedia::Effects(vec![visible]), viewport)
                .is_some_and(|media| media.encode().is_ok())
        );
    }
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
            relative_media(
                media,
                Viewport {
                    origin,
                    first_visible_floor: origin.floor
                }
            ),
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
