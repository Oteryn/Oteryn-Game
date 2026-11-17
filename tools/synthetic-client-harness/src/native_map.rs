//! Headless live-map and C05 item-selection scenario through the native task, mailbox and rendered scene.
use oteryn_client::combat_input::{CombatCommand, CombatReport};
use oteryn_client::play::{PlayView, Stepper, play_channel, run_session};
use oteryn_protocol_oteryn::world_map::*;
use oteryn_renderer::TileCoord;
use oteryn_session::{
    ActorPosition, AppliedDelta, LogoutResult, SessionError, SessionEvent, StepDirection,
    StepOutcome,
};
use std::time::Duration;
use tokio::sync::mpsc;

pub(super) struct MapSession {
    pub(super) initial: WorldMapViewSnapshot,
    pub(super) incoming: mpsc::UnboundedReceiver<SessionEvent>,
    pub(super) pending: Vec<SessionEvent>,
    pub(super) appearances: Vec<oteryn_session::VisibleAppearance>,
}

impl Stepper for MapSession {
    fn visible_appearances(&self) -> Vec<oteryn_session::VisibleAppearance> {
        self.appearances.clone()
    }
    fn feature_snapshot(&self, domain: u32) -> Option<Vec<u8>> {
        (domain == STATE_DOMAIN_WORLD_MAP_VIEW)
            .then(|| encode_world_map_snapshot(&self.initial).ok())
            .flatten()
    }
    async fn step(&mut self, _: StepDirection) -> Result<StepOutcome, SessionError> {
        Err(SessionError::Timeout("synthetic map"))
    }
    async fn serve(&mut self, _: Duration) -> Result<(), SessionError> {
        let event = self
            .incoming
            .recv()
            .await
            .ok_or(SessionError::Timeout("synthetic map"))?;
        if let SessionEvent::WorldSpatialEntities(delta) = &event {
            self.appearances
                .retain(|entry| !delta.value.leave.contains(&entry.entity.entity));
            for entity in &delta.value.update {
                if let Some(entry) = self
                    .appearances
                    .iter_mut()
                    .find(|entry| entry.entity.entity == entity.entity)
                {
                    entry.entity = *entity;
                }
            }
        }
        self.pending.push(event);
        Ok(())
    }
    fn drain(&mut self) -> Vec<SessionEvent> {
        std::mem::take(&mut self.pending)
    }
    async fn combat(&mut self, _: CombatCommand) -> Result<CombatReport, SessionError> {
        Err(SessionError::Timeout("synthetic map"))
    }
    async fn logout(&mut self) -> Result<Option<LogoutResult>, SessionError> {
        Err(SessionError::Timeout("synthetic map"))
    }
}

#[test]
fn native_live_map_snapshot_then_delta_changes_the_scene_and_item_selection()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(async {
                    let position = ActorPosition {
                        x: 100,
                        y: 200,
                        floor: 0,
                    };
                    let initial = WorldMapViewSnapshot {
                        header: MapViewHeader {
                            content_generation: [1; 32],
                            bundle_digest: [2; 32],
                            reset_epoch: 1,
                            origin: position,
                            first_visible_floor: 0,
                        },
                        tiles: vec![MapTile {
                            position,
                            items: vec![MapItem {
                                definition: MapDefinition::Item(std::num::NonZeroU32::MIN),
                                count: 1,
                                sub_type: 0,
                                origin: MapOrigin::DisplayOnly,
                                appearance_id: 0,
                            }],
                            more: false,
                            ground_speed: 0,
                        }],
                    };
                    let payload = encode_world_map_snapshot(&initial)
                        .map_err(|error| format!("{error:?}"))?;
                    let initial = decode_world_map_snapshot(&payload)
                        .map_err(|error| format!("{error:?}"))?;
                    let mut view = PlayView::new(TileCoord::new(100, 200), 0, &[])?;
                    let empty_sprites = view.scene().sprites().len();
                    view.set_live_map(&initial)?;
                    assert!(view.scene().sprites().len() > empty_sprites);
                    assert_eq!(
                        view.scene()
                            .sprites()
                            .instances()
                            .iter()
                            .filter(|q| q.size == [4.0; 2] || q.size == [12.0; 2])
                            .count(),
                        31 * 25 + 2
                    );
                    let (link, commands, outbox) = play_channel();
                    let (send, incoming) = mpsc::unbounded_channel();
                    let task = tokio::spawn(run_session(
                        MapSession {
                            initial: initial.clone(),
                            incoming,
                            pending: Vec::new(),
                            appearances: Vec::new(),
                        },
                        commands,
                        outbox,
                    ));
                    let mut added = initial.tiles[0].clone();
                    added.position.x += 1;
                    added.items[0].count = 7;
                    added.items[0].origin = MapOrigin::Handle(std::num::NonZeroU64::MIN);
                    let added_position = added.position;
                    let upsert = WorldMapViewDelta {
                        header: initial.header,
                        tiles: vec![added],
                        cleared: Vec::new(),
                    };
                    let clear = WorldMapViewDelta {
                        header: initial.header,
                        tiles: Vec::new(),
                        cleared: vec![position, added_position],
                    };
                    for (revision, delta, expected) in
                        [(2, upsert, empty_sprites + 2), (3, clear, empty_sprites)]
                    {
                        let payload = encode_world_map_delta(&initial.header, &delta)
                            .map_err(|error| format!("{error:?}"))?;
                        let value = decode_world_map_delta(&initial.header, &payload)
                            .map_err(|error| format!("{error:?}"))?;
                        send.send(SessionEvent::WorldMap(AppliedDelta {
                            server_sequence: revision,
                            base_revision: revision - 1,
                            new_revision: revision,
                            value,
                        }))?;
                        tokio::time::timeout(Duration::from_secs(5), async {
                            loop {
                                view.tick(&link).map_err(|error| format!("{error:?}"))?;
                                if view
                                    .scene()
                                    .sprites()
                                    .instances()
                                    .iter()
                                    .filter(|q| q.size != [4.0; 2] && q.size != [12.0; 2])
                                    .count()
                                    == expected
                                {
                                    break;
                                }
                                tokio::task::yield_now().await;
                            }
                            Ok::<_, String>(())
                        })
                        .await??;
                        assert_eq!(
                            view.scene()
                                .sprites()
                                .instances()
                                .iter()
                                .filter(|q| q.size == [4.0; 2] || q.size == [12.0; 2])
                                .count(),
                            31 * 25 + 1 + if revision == 2 { 2 } else { 0 }
                        );
                        if revision == 2 {
                            view.click(TileCoord::new(added_position.x, added_position.y))?;
                            let item = view
                                .scene()
                                .target()
                                .and_then(|target| target.item)
                                .ok_or("live ground item was not selectable")?;
                            assert_eq!(item.definition(), 1);
                            assert_eq!(item.quantity(), 7);
                            assert!(matches!(
                                item,
                                oteryn_client::play::GroundItem::Map {
                                    item: MapItem {
                                        origin: MapOrigin::Handle(_),
                                        ..
                                    },
                                    ..
                                }
                            ));
                        } else {
                            assert_eq!(view.scene().target(), None);
                        }
                    }
                    drop(link);
                    task.abort();
                    Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
                })
        })?
        .join()
        .map_err(|_| "native map scenario thread failed")?
}

#[test]
fn native_live_appearance_snapshot_reaches_visible_scene()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(async {
                    let position = ActorPosition {
                        x: 100,
                        y: 200,
                        floor: 0,
                    };
                    let initial = WorldMapViewSnapshot {
                        header: MapViewHeader {
                            content_generation: [1; 32],
                            bundle_digest: [2; 32],
                            reset_epoch: 0,
                            origin: position,
                            first_visible_floor: 0,
                        },
                        tiles: Vec::new(),
                    };
                    let entity = oteryn_session::WorldSpatialEntity {
                        display_name: Some(
                            oteryn_session::EntityDisplayName::new("Live Rat")
                                .map_err(|e| format!("{e:?}"))?,
                        ),
                        kind: oteryn_session::EntityKind::Creature,
                        entity: oteryn_session::EntityRef {
                            identity: [8; 16],
                            generation: 1,
                        },
                        position,
                        detail: oteryn_session::EntityDetail::Actor {
                            direction: StepDirection::South,
                            appearance_ref: 1,
                            health_percent: 100,
                        },
                    };
                    let (link, commands, outbox) = play_channel();
                    let (send, incoming) = mpsc::unbounded_channel();
                    let task = tokio::spawn(run_session(
                        MapSession {
                            initial,
                            incoming,
                            pending: Vec::new(),
                            appearances: vec![oteryn_session::VisibleAppearance {
                                forge_stacks: None,
                                entity,
                                look: None,
                                light: None,
                            }],
                        },
                        commands,
                        outbox,
                    ));
                    let mut view = PlayView::new(TileCoord::new(100, 200), 0, &[])?;
                    // Wake the production loop to publish its presentation snapshot.
                    send.send(SessionEvent::WorldSpatial(oteryn_session::AppliedDelta {
                        server_sequence: 1,
                        base_revision: 1,
                        new_revision: 2,
                        value: oteryn_session::WorldSpatialObservation {
                            content_generation: [1; 32],
                            actor_position: position,
                        },
                    }))?;
                    tokio::time::timeout(Duration::from_secs(5), async {
                        loop {
                            view.tick(&link).map_err(|e| format!("{e:?}"))?;
                            if view.scene().name_count() == 1 {
                                break;
                            }
                            tokio::task::yield_now().await;
                        }
                        Ok::<_, String>(())
                    })
                    .await??;
                    assert_eq!(view.scene().sprites().len(), 10);
                    let mut renamed = entity;
                    renamed.display_name = Some(
                        oteryn_session::EntityDisplayName::new("Rat")
                            .map_err(|e| format!("{e:?}"))?,
                    );
                    for (sequence, update, leave, expected_names, expected_sprites) in [
                        (2, vec![renamed], vec![], 1, 5),
                        (3, vec![], vec![entity.entity], 0, 1),
                    ] {
                        send.send(SessionEvent::WorldSpatialEntities(AppliedDelta {
                            server_sequence: sequence,
                            base_revision: sequence,
                            new_revision: sequence + 1,
                            value: oteryn_session::WorldSpatialEntitiesDelta {
                                content_generation: [1; 32],
                                actor_position: position,
                                enter: vec![],
                                update,
                                leave,
                            },
                        }))?;
                        tokio::time::timeout(Duration::from_secs(5), async {
                            loop {
                                view.tick(&link).map_err(|e| format!("{e:?}"))?;
                                if view.scene().name_count() == expected_names
                                    && view.scene().sprites().len() == expected_sprites
                                {
                                    break;
                                }
                                tokio::task::yield_now().await;
                            }
                            Ok::<_, String>(())
                        })
                        .await??;
                    }

                    drop(link);
                    task.abort();
                    Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
                })
        })?
        .join()
        .map_err(|_| "appearance scenario thread failed")?
}
