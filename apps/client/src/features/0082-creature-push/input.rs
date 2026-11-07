//! A primary-button gesture becomes a click on release or a creature push on another tile.
use crate::play::{PlayLink, PlayView, mailbox::lock};
use oteryn_client_runtime::server_mechanic::ServerMechanicCommand;
use oteryn_input_actions::{ButtonState, Modifiers, MouseButton, NormalizedInputEvent};
use oteryn_renderer::{BatchError, TileCoord};
use oteryn_session::{ActorPosition, EntityKind, creature_push::CreaturePushIntent};

impl PlayView {
    pub fn world_pointer_input(
        &mut self,
        link: &PlayLink,
        events: &[NormalizedInputEvent],
    ) -> Result<bool, BatchError> {
        let mut consumed = false;
        for event in events {
            match event {
                NormalizedInputEvent::PointerMoved { position, .. } => {
                    self.creature_drag.pointer =
                        Some((f64::from(position.x().get()), f64::from(position.y().get())));
                }
                NormalizedInputEvent::FocusChanged { focused: false }
                | NormalizedInputEvent::CaptureChanged { captured: false }
                | NormalizedInputEvent::DeviceLost => self.creature_drag = Default::default(),
                NormalizedInputEvent::MouseButton {
                    button,
                    state,
                    modifiers,
                } if *button == MouseButton::PRIMARY => {
                    let tile = self
                        .creature_drag
                        .pointer
                        .and_then(|(x, y)| crate::input::click_tile(self.scene.view(), x, y));
                    let mailbox = lock(&link.mailbox);
                    let blocked = mailbox.world_input_blocked()
                        || mailbox.world_context.pending
                        || mailbox.world_context.targeting.is_some()
                        || self.cursor_spell.is_some();
                    drop(mailbox);
                    if blocked {
                        self.creature_drag.source = None;
                        continue;
                    }
                    match state {
                        ButtonState::Pressed if *modifiers == Modifiers::NONE => {
                            self.creature_drag.source = tile.and_then(|tile| {
                                let target = self
                                    .entities
                                    .iter()
                                    .rev()
                                    .find(|entity| {
                                        entity.position.floor == self.floor
                                            && TileCoord::new(entity.position.x, entity.position.y)
                                                == tile
                                            && matches!(
                                                entity.kind,
                                                EntityKind::Player | EntityKind::Creature
                                            )
                                    })
                                    .map(|entity| entity.entity);
                                target.map(|target| (tile, self.floor, Some(target)))
                            });
                            consumed |= self.creature_drag.source.is_some();
                        }
                        ButtonState::Released => {
                            let Some((source, floor, target)) = self.creature_drag.source.take()
                            else {
                                continue;
                            };
                            consumed = true;
                            let Some(tile) = tile else {
                                continue;
                            };
                            if floor != self.floor || *modifiers != Modifiers::NONE {
                                continue;
                            }
                            if source == tile {
                                self.primary_click(tile, link)?;
                            } else if let Some(target) = target {
                                let valid = self.entities.iter().any(|entity| {
                                    entity.entity == target
                                        && entity.position
                                            == ActorPosition {
                                                x: source.x,
                                                y: source.y,
                                                floor,
                                            }
                                });
                                let (selected, stop_walk) = {
                                    let mailbox = lock(&link.mailbox);
                                    (mailbox.creature_push_selected, mailbox.target_move_selected)
                                };
                                if valid && selected {
                                    if stop_walk {
                                        let _ = link.commands.send(
                                            crate::combat_input::PlayCommand::ServerMechanic(
                                                ServerMechanicCommand {
                                                    capability: 64,
                                                    command_type: 81,
                                                    payload: vec![16, 1],
                                                },
                                            ),
                                        );
                                    }
                                    self.inputs.retain(|input| {
                                        !matches!(
                                            input,
                                            crate::play::Input::Arrow(_)
                                                | crate::play::Input::Attack(_)
                                                | crate::play::Input::TargetWalk(_)
                                        )
                                    });
                                    let intent = CreaturePushIntent {
                                        target,
                                        destination: ActorPosition {
                                            x: tile.x,
                                            y: tile.y,
                                            floor,
                                        },
                                    };
                                    if let Some(command) =
                                        oteryn_client_ui::creature_push::request(intent)
                                    {
                                        let _ = link.commands.send(
                                            crate::combat_input::PlayCommand::ServerMechanic(
                                                command,
                                            ),
                                        );
                                    }
                                }
                            }
                        }
                        _ => self.creature_drag.source = None,
                    }
                }
                _ => {}
            }
        }
        Ok(consumed)
    }
}

impl PlayView {
    /// Observe pointer/reset events even when a panel consumes the input batch.
    pub fn observe_creature_pointer(&mut self, events: &[NormalizedInputEvent]) {
        for event in events {
            match event {
                NormalizedInputEvent::PointerMoved { position, .. } => {
                    self.creature_drag.pointer =
                        Some((f64::from(position.x().get()), f64::from(position.y().get())))
                }
                NormalizedInputEvent::FocusChanged { focused: false }
                | NormalizedInputEvent::CaptureChanged { captured: false }
                | NormalizedInputEvent::DeviceLost => self.creature_drag = Default::default(),
                _ => {}
            }
        }
    }
}
