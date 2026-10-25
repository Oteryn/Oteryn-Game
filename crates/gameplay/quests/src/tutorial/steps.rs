//! The seven tutorial transitions in runtime coordinates (Tibia z, 0..=15).
//! Membership and current Character facts come from owners.

pub const ISLET_TEMPLE: (i32, i32, i16) = (32534, 32513, 7);
pub const ISLET_HOLE: (i32, i32, i16) = (32533, 32513, 7);
pub const FORCEFIELD_ARRIVAL: (i32, i32, i16) = (32544, 32515, 7);
pub const HOLE_TEXT: &str = "I should not climb down again but rather venture forth.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TutorialStep {
    Islet,
    Island,
    Departed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TutorialProgress {
    pub step: TutorialStep,
    pub hunting: bool,
    pub corruptor: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TutorialEvent {
    Login,
    Forcefield,
    Hunting,
    Corruptor,
    Death,
    Departure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TutorialTransition {
    pub progress: TutorialProgress,
    pub place_islet: bool,
    pub hint: Option<u8>,
    pub grant_corruptor_loot: bool,
}

/// Base and promoted vocation keys share the client's existing hint IDs.
pub(super) fn vocation_hint(vocation: &str) -> Option<u8> {
    match vocation {
        "knight" | "elite_knight" => Some(3),
        "paladin" | "royal_paladin" => Some(4),
        "sorcerer" | "master_sorcerer" => Some(5),
        "druid" | "elder_druid" => Some(6),
        "monk" | "exalted_monk" => Some(11),
        _ => None,
    }
}

/// `enabled` is the Newhaven-start gate, independent of the player's current position.
/// An unchanged event is a replay and requests no write. Login and death never place
/// access Characters; the hole and forcefield movement themselves remain world rules.
pub fn tutorial_transition(
    enabled: bool,
    stored: Option<TutorialProgress>,
    event: TutorialEvent,
    vocation: &str,
    access: bool,
) -> Option<TutorialTransition> {
    if !enabled {
        return None;
    }
    let vocationless = vocation == "none";
    let mut next = TutorialTransition {
        progress: stored.unwrap_or(TutorialProgress {
            step: TutorialStep::Departed,
            hunting: false,
            corruptor: false,
        }),
        place_islet: false,
        hint: None,
        grant_corruptor_loot: false,
    };
    match event {
        TutorialEvent::Login
            if vocationless
                && !access
                && stored.is_none_or(|row| row.step == TutorialStep::Departed) =>
        {
            next.progress.step = TutorialStep::Islet;
            next.progress.hunting = false;
            next.progress.corruptor = false;
            next.place_islet = true;
        }
        TutorialEvent::Forcefield => {
            next.progress = TutorialProgress {
                step: TutorialStep::Island,
                hunting: true,
                corruptor: true,
            };
            next.hint = vocationless.then_some(2);
        }
        TutorialEvent::Hunting if next.progress.hunting => {
            next.hint = vocation_hint(vocation);
            next.hint?;
            next.progress.hunting = false;
        }
        TutorialEvent::Corruptor if next.progress.corruptor => {
            next.progress.corruptor = false;
            next.grant_corruptor_loot = true;
        }
        TutorialEvent::Death
            if vocationless && !access && next.progress.step != TutorialStep::Departed =>
        {
            next.progress.step = TutorialStep::Islet;
            next.place_islet = true;
        }
        TutorialEvent::Departure if stored.is_some() => {
            next.progress = TutorialProgress {
                step: TutorialStep::Departed,
                hunting: false,
                corruptor: false,
            };
        }
        _ => return None,
    }
    if stored == Some(next.progress) {
        return None;
    }
    Some(next)
}
