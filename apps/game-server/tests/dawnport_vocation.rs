#![allow(clippy::unwrap_used)]

// Library unit-test placement: import the actual registered adapter and real durable owners.
use durability::character_build::{
    BuildCause, BuildFormula, BuildOccurrence, DurableBuildState, MAX_BUILD_LEVEL, MIN_SKILL_LEVEL,
    NO_VOCATION, cumulative_progress,
};
use durability::character_progression::CharacterProgressionError;
use oteryn_game_server::durability;

use crate::gameplay_transport::vocation_choice::vocation_choice_request;

/// Synthetic arithmetic table, not native Dawnport facts or a production policy.
struct Table;
impl BuildFormula for Table {
    fn required(&self, vocation: &str, family: usize, level: u16) -> Option<u64> {
        let floor = if family == 0 { 0 } else { MIN_SKILL_LEVEL };
        if family > 7 || !(floor + 1..=MAX_BUILD_LEVEL).contains(&level) {
            return None;
        }
        let base = match vocation {
            NO_VOCATION => 100,
            "knight" => 50,
            _ => return None,
        };
        Some(base * (family as u64 + 1))
    }
    fn digest(&self) -> [u8; 32] {
        [7; 32]
    }
    fn content_revision(&self) -> &str {
        "test:conversion"
    }
}

fn occurrence(tag: u8) -> BuildOccurrence {
    let mut bytes = [0_u8; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes[15] = tag;
    BuildOccurrence::from_bytes(bytes).unwrap()
}

fn trained() -> DurableBuildState {
    let mut skills = [(12, 0); 7];
    for (family, skill) in skills.iter_mut().enumerate() {
        skill.1 = family as u64 + 3;
    }
    DurableBuildState::new(NO_VOCATION, (3, 17), skills).unwrap()
}

#[test]
fn the_real_writer_request_preserves_all_eight_cumulative_families() {
    let before = trained();
    let request =
        vocation_choice_request(occurrence(1), before.clone(), "knight", &Table, None).unwrap();
    assert_eq!(request.occurrence, occurrence(1));
    assert_eq!(request.cause, BuildCause::VocationChoice);
    assert_eq!(request.before, before);
    assert_eq!(request.after.vocation(), "knight");
    assert_eq!(request.after.magic(), (6, 17));
    assert_ne!(request.after.magic(), before.magic());
    let old = std::iter::once(before.magic()).chain(before.skills());
    let new = std::iter::once(request.after.magic()).chain(request.after.skills());
    for (family, ((old_level, old_progress), (new_level, new_progress))) in old.zip(new).enumerate()
    {
        let floor = if family == 0 { 0 } else { MIN_SKILL_LEVEL };
        assert_eq!(
            cumulative_progress(old_level, old_progress, floor, |level| {
                Table.required(NO_VOCATION, family, level)
            }),
            cumulative_progress(new_level, new_progress, floor, |level| {
                Table.required("knight", family, level)
            }),
            "family {family}"
        );
    }
    for (family, skill) in request.after.skills().iter().enumerate() {
        assert_eq!(*skill, (14, family as u64 + 3));
    }
    assert_eq!(request.pruned_stance, None);
}

#[test]
fn a_qualified_prune_stays_in_the_single_existing_build_request() {
    let request = vocation_choice_request(
        occurrence(2),
        trained(),
        "knight",
        &Table,
        Some("stance:owner-qualified".into()),
    )
    .unwrap();
    assert_eq!(request.cause, BuildCause::VocationChoice);
    assert_eq!(
        request.pruned_stance.as_deref(),
        Some("stance:owner-qualified")
    );
    assert_eq!(request.after.magic(), (6, 17));
}

#[test]
fn repeated_input_preserves_occurrence_and_every_request_field() {
    let build =
        || vocation_choice_request(occurrence(3), trained(), "knight", &Table, None).unwrap();
    assert_eq!(build(), build());
    let other = vocation_choice_request(occurrence(4), trained(), "knight", &Table, None).unwrap();
    assert_ne!(build().occurrence, other.occurrence);
    assert_eq!(build().after, other.after);
    // This is deterministic request assembly, not a durable exactly-once/replay assertion.
}

#[test]
fn invalid_key_errors_come_from_the_real_conversion_seam() {
    for target in ["", "bad key", "key/with/slashes", &"x".repeat(129)] {
        assert!(matches!(
            vocation_choice_request(occurrence(5), trained(), target, &Table, None),
            Err(CharacterProgressionError::InvalidInput)
        ));
    }
}

#[test]
fn unreachable_formula_entries_are_not_reinterpreted_as_choice_admission() {
    // The owning BuildFormula contract permits None for an unreachable level. Native choice
    // admission is deliberately outside this request builder; no first-level probe invents it.
    let request = vocation_choice_request(
        occurrence(6),
        DurableBuildState::default(),
        "unreachable",
        &Table,
        None,
    )
    .unwrap();
    assert_eq!(request.after.vocation(), "unreachable");
    assert_eq!(request.after.magic(), (0, 0));
    assert_eq!(request.after.skills(), [(MIN_SKILL_LEVEL, 0); 7]);
}
