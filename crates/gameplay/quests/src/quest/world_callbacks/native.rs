//! Closed source bindings and committed feedback for native world rules.
use super::{Callback, NativeRule, Position, Trigger};
use crate::quest::{QuestTrackChange, cults_of_tibia as cults, unnatural_selection as dance};

pub(super) fn valid_native_callback(callback: &Callback, rule: NativeRule) -> bool {
    let (trigger, position, action, ranges, source) = match rule {
        NativeRule::IceSealPaint { seal } => {
            return crate::quest::ice_islands_seals::callbacks::admitted(callback, seal);
        }
        NativeRule::CultsSymbol { symbol } => {
            let Some(position) = cults::SYMBOL_POSITIONS.get(usize::from(symbol)).copied() else {
                return false;
            };
            (
                Trigger::Use,
                position,
                Some(5535),
                [(0, 7), (0, i64::from(i32::MAX)), (0, i64::from(i32::MAX))],
                "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/cults_of_tibia/actions_cult_symbol.lua:",
            )
        }
        NativeRule::UnnaturalDance { tile } => {
            let Some(position) = tile
                .checked_sub(1)
                .and_then(|index| dance::TILES.get(usize::from(index)))
                .copied()
            else {
                return false;
            };
            // The nine positions are literal source config entries. StepIn observes
            // entry into the tile; it never guesses an absent UID or modifies map data.
            (
                Trigger::StepIn,
                position,
                None,
                [(1, 16), (-1, 3), (-1, 15)],
                "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/unnatural_selection/movements_mission3_dance_dance_evolution.lua:",
            )
        }
    };
    callback.quest == rule.quest()
        && callback.selector.native.is_none()
        && callback.trigger == trigger
        && callback.selector.position
            == Some(Position {
                x: position.0,
                y: position.1,
                floor: position.2,
            })
        && callback.selector.action == action
        && callback.selector.unique.is_none()
        && callback.selector.appearance.is_none()
        && callback.storage.len() == 3
        && rule
            .tracks()
            .into_iter()
            .zip(ranges)
            .all(|(track, (minimum, maximum))| {
                callback
                    .storage
                    .iter()
                    .filter(|storage| {
                        storage.track == track
                            && storage.preserve
                            && storage.minimum == minimum
                            && storage.maximum == maximum
                    })
                    .count()
                    == 1
            })
        && callback
            .sources
            .iter()
            .any(|reference| reference.starts_with(source))
        && callback.lever.is_none()
        && !callback.reversible
        && callback.passages.is_empty()
        && callback.teleport.is_none()
        && callback.facing.is_none()
        && callback.effects.is_empty()
        && callback.minimum_level.is_none()
        && callback.experience.is_none()
        && callback.monk_reward.is_none()
        && callback.offerings.is_empty()
        && callback.item_on_target.is_none()
        && !callback.return_on_failure
        && callback.failure_message.is_none()
        && callback.success_message.is_none()
        && callback.effects == Default::default()
}

impl Callback {
    /// Presentation follows a newly committed receipt. Replayed receipts do not
    /// repeat success text; the dispatch owner supplies that distinction.
    pub fn committed_message<'a>(&'a self, changes: &[QuestTrackChange]) -> Option<&'a str> {
        match self.native_rule {
            // Crystal actions_cult_symbol.lua:43-47. Mission-only repair is silent.
            Some(NativeRule::CultsSymbol { .. }) => {
                let count = changes
                    .iter()
                    .find(|change| change.track == cults::OBJECTS)?;
                if count.after <= count.before {
                    return None;
                }
                if count.after < 10 {
                    Some(
                        "Your body reacts to this strange green substance as you reach out to touch it. You feel an urge for more of this energy.",
                    )
                } else if count.after == 10 {
                    Some(
                        "You gathered an impressive amount of power from simply touching the strange green symbols of the Barkless. But how...?",
                    )
                } else {
                    None
                }
            }
            // Crystal movements_mission3_dance_dance_evolution.lua:33-60.
            Some(NativeRule::UnnaturalDance { .. }) => {
                let step = changes
                    .iter()
                    .find(|change| change.track == dance::DANCE_STATUS_TRACK)?;
                if step.after == 1 {
                    Some("You did it wrong. now you have to start again.")
                } else if (step.before, step.after) == (1, 2) {
                    Some("Dance for the mighty Krunus!")
                } else if (step.before, step.after) == (16, 17) {
                    Some("Krunus should be pleased.")
                } else {
                    None
                }
            }
            Some(NativeRule::IceSealPaint { .. }) => None,
            None => self.success_message.as_deref(),
        }
    }
}
