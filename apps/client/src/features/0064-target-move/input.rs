//! Ctrl+arrows turn, Ctrl+F follows the selected actor, Escape stops walking/following.
use crate::combat_input::PlayCommand;
use crate::play::{Input, PlayLink, PlayView, mailbox::lock};
use oteryn_client_runtime::server_mechanic::ServerMechanicCommand;
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
use oteryn_renderer::TileCoord;
use oteryn_session::ActorPosition;
use oteryn_session::target_move::{TargetMoveIntent, encode_target_move_intent};

impl PlayView {
    pub fn target_move_input(&mut self, link: &PlayLink, events: &[NormalizedInputEvent]) -> bool {
        for event in events {
            let NormalizedInputEvent::Key {
                code,
                state: ButtonState::Pressed,
                modifiers,
                repeat,
            } = event
            else {
                continue;
            };
            let intent = if *code == KeyCode::ESCAPE && *modifiers == Modifiers::NONE {
                TargetMoveIntent::Stop
            } else if (code.get() == 9 && *modifiers == Modifiers::one(Modifier::Control))
                || *modifiers
                    == Modifiers::one(match self.preferences().rotation_modifier.as_str() {
                        "ALT" => Modifier::Alt,
                        "SHIFT" => Modifier::Shift,
                        _ => Modifier::Control,
                    })
            {
                if code.get() == 9 && *modifiers == Modifiers::one(Modifier::Control) {
                    let Some(target) = self.scene.target().and_then(|target| target.entity) else {
                        return true;
                    };
                    // Re-prove visibility and generation after a selection outlives a delta.
                    if !self.entities.iter().any(|entity| entity.entity == target) {
                        return true;
                    }
                    TargetMoveIntent::Follow(target)
                } else if let Some(direction) =
                    crate::input::arrow_step(std::slice::from_ref(event))
                {
                    TargetMoveIntent::Turn(super::super::step_direction(direction))
                } else {
                    continue;
                }
            } else if crate::input::arrow_step(std::slice::from_ref(event)).is_some() {
                TargetMoveIntent::Stop
            } else {
                continue;
            };
            // Manual walking replaces autowalk without Escape's separate combat stop.
            let manual = intent == TargetMoveIntent::Stop && *code != KeyCode::ESCAPE;
            if lock(&link.mailbox).world_input_blocked() {
                return true;
            }
            if *repeat {
                return !manual;
            }
            oteryn_client_ui::target_move::cancel(&mut lock(&link.mailbox).target_walk);
            self.creature_drag.source = None;

            self.inputs.retain(|input| {
                !matches!(
                    input,
                    crate::play::Input::Arrow(_)
                        | crate::play::Input::TargetWalk(_)
                ) && (manual || !matches!(input, crate::play::Input::Attack(_)))
            });
            if !manual && intent == TargetMoveIntent::Stop
                && lock(&link.mailbox).battle.combat.is_some()
            {
                // Domain 10 proves capability 17 is selected; its receipt clears the target.
                link.request_combat(crate::combat_input::CombatCommand::Stop);
            }
            if !crate::play::mailbox::lock(&link.mailbox).target_move_selected {
                return !manual;
            }
            if let Ok(payload) = encode_target_move_intent(intent) {
                let _ = link
                    .commands
                    .send(crate::combat_input::PlayCommand::ServerMechanic(
                        ServerMechanicCommand {
                            capability: 64,
                            command_type: 81,
                            payload,
                        },
                    ));
            }
            return !manual;
        }
        false
    }
}

impl PlayView {
    pub(in crate::play) fn walk_to(&mut self, tile: TileCoord) {
        self.queue(Input::TargetWalk(ActorPosition {
            x: tile.x,
            y: tile.y,
            floor: self.floor,
        }));
    }
}

impl PlayLink {
    pub(in crate::play) fn request_target_walk(&self, position: ActorPosition) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if mailbox.world_input_blocked() {
            return true;
        }
        if mailbox.target_walk.pending {
            return false;
        }
        if let Some(position) =
            oteryn_client_ui::target_move::request(&mut mailbox.target_walk, position)
            && self
                .commands
                .send(PlayCommand::TargetWalk(position))
                .is_err()
        {
            mailbox.target_walk.pending = false;
            mailbox.target_walk.feedback = Some(tr!("Session ended"));
        }
        true
    }

    /// Arrival ends a click or minimap walk; a blocked step or a dropped walk re-requests it.
    pub(in crate::play) fn target_walk_follow_up(&self, own: ActorPosition) {
        let now = std::time::Instant::now();
        let mut mailbox = lock(&self.mailbox);
        oteryn_client_ui::target_move::progress(&mut mailbox.target_walk, own, now);
        if mailbox.world_input_blocked() {
            return;
        }
        if let Some(goal) = oteryn_client_ui::target_move::retry(&mut mailbox.target_walk, now)
            && self.commands.send(PlayCommand::TargetWalk(goal)).is_err()
        {
            mailbox.target_walk.pending = false;
            mailbox.target_walk.feedback = Some(tr!("Session ended"));
        }
    }
}
