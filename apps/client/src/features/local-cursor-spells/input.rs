//! C18b native flow: Shift+digits selects an ordinary spell; the next world click casts it.
use crate::combat_input::CombatCommand;
use crate::play::{Input, PlayLink, PlayView};
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
use oteryn_renderer::TileCoord;
use oteryn_session::SpellTargetPosition;
use std::num::NonZeroU32;

impl PlayView {
    pub fn cursor_spell_input(&mut self, events: &[NormalizedInputEvent]) {
        if self.death.dead {
            self.cursor_spell = None;
            return;
        }
        for event in events {
            match event {
                NormalizedInputEvent::FocusChanged { focused: false }
                | NormalizedInputEvent::Key {
                    code: KeyCode::ESCAPE,
                    ..
                } => {
                    self.cursor_spell = None;
                }
                NormalizedInputEvent::Key {
                    code,
                    state: ButtonState::Pressed,
                    modifiers,
                    repeat: false,
                } if *modifiers == Modifiers::one(Modifier::Shift) => {
                    let digit = match code.get() {
                        30..=38 => u32::from(code.get() - 29),
                        39 => 0,
                        _ => continue,
                    };
                    self.cursor_spell = self
                        .cursor_spell
                        .map_or(0, NonZeroU32::get)
                        .checked_mul(10)
                        .and_then(|value| value.checked_add(digit))
                        .and_then(NonZeroU32::new);
                    self.inputs
                        .retain(|input| !matches!(input, Input::TargetWalk(_)));
                }
                _ => {}
            }
        }
    }

    pub(in crate::play) fn cast_at_cursor(&mut self, tile: TileCoord) -> bool {
        let Some(spell) = self.cursor_spell.take() else {
            return false;
        };
        self.queue(Input::CastAt(
            spell,
            SpellTargetPosition {
                x: tile.x,
                y: tile.y,
                floor: self.floor,
            },
        ));
        true
    }
}

pub(in crate::play) fn send_cast_at(
    link: &PlayLink,
    spell: NonZeroU32,
    position: SpellTargetPosition,
) {
    link.request_combat(CombatCommand::CastAt { spell, position });
}
