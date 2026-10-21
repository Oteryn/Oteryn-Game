//! Pure parameter spell classification and source-qualified feedback selection.

use crate::dispatch_plans::geometry::PlanRejected;
use oteryn_gameplay_content::content::spell_data::{Execution, SpellDefinition};

/// The `native_behavior` key of a native profile spell, empty for any other execution.
pub fn native_key(spell: &SpellDefinition) -> &str {
    match &spell.execution {
        Execution::NativeProfile(profile) => profile.spell()["execution"]["native_behavior"]["key"]
            .as_str()
            .unwrap_or(""),
        _ => "",
    }
}

pub fn named_player_spell(spell: &SpellDefinition) -> bool {
    matches!(
        spell.name.as_str(),
        "Heal Friend" | "Nature's Embrace" | "Restore Balance"
    ) && spell
        .authored
        .as_ref()
        .is_some_and(|p| p.header.targeting.parameter == "player_name")
        && matches!(
            spell.execution,
            Execution::Effects(_) | Execution::AbilityVariants(_)
        )
        && spell.needs_target
        && !spell.aggressive
}
/// Resolve the source feedback text and whether its effect is `poff`.
/// Dispatch maps these values to the private protocol feedback.
pub fn source_feedback(
    reason: &str,
    effect: Option<&str>,
) -> Result<(&'static str, bool), PlanRejected> {
    let text = match reason {
        "source_no_error" => "No error.",
        "player_with_this_name_is_not_online" => "A player with this name is not online.",
        "exiva_protected" => {
            "The character you are trying to find with Exiva is currently protected from your spell."
        }
        "name_is_ambiguous" => "Player name is ambiguous.",
        "not_possible" => "Sorry, not possible.",
        "not_enough_room" => "There is not enough room.",
        _ => return Err(PlanRejected),
    };
    let poff = match effect {
        None => false,
        Some("poff") => true,
        _ => return Err(PlanRejected),
    };
    Ok((text, poff))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn exact_source_cancel_feedback_and_unknown_reason_refusal() {
        let ambiguous = source_feedback("name_is_ambiguous", Some("poff")).unwrap();
        assert_eq!(ambiguous.0, "Player name is ambiguous.");
        assert!(ambiguous.1);
        let protected = source_feedback("exiva_protected", None).unwrap();
        assert_eq!(
            protected.0,
            "The character you are trying to find with Exiva is currently protected from your spell."
        );
        assert!(!protected.1);
        assert!(source_feedback("unqualified_source_message", None).is_err());
    }
    #[test]
    fn source_feedback_rejects_unknown_effects_and_preserves_all_known_reasons() {
        for (reason, text) in [
            ("source_no_error", "No error."),
            (
                "player_with_this_name_is_not_online",
                "A player with this name is not online.",
            ),
            ("not_possible", "Sorry, not possible."),
            ("not_enough_room", "There is not enough room."),
        ] {
            assert_eq!(source_feedback(reason, None), Ok((text, false)));
            assert_eq!(source_feedback(reason, Some("poff")), Ok((text, true)));
            assert_eq!(
                source_feedback(reason, Some("magic_blue")),
                Err(PlanRejected)
            );
        }
    }
}
