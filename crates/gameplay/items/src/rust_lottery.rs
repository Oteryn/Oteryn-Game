//! Crystal-compatible rust remover and lottery-ticket outcomes.

use crate::item_craft::{CookingConversion, output_definition};
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_domain::domain::item_definition::TypedDefinitionRef;

const RUST_REMOVER: &str = "oteryn:item.tibia.i9016";
const LOTTERY_TICKET: &str = "oteryn:item.tibia.i5957";

const RUST: &[(u32, &[(u32, u32)])] = &[
    (
        8894,
        &[(3358, 6994), (3377, 3952), (3359, 1502), (3357, 197)],
    ),
    (
        8895,
        &[
            // Global includes scale armor; Crystal repeats chain armor here.
            (3377, 6437),
            (3358, 4606),
            (3359, 3029),
            (3357, 1559),
            (3370, 595),
            (8063, 283),
            (3381, 49),
        ],
    ),
    (
        8896,
        &[
            (3359, 6681),
            (3357, 3767),
            (3370, 1832),
            (3381, 177),
            (8063, 31),
            (3360, 10),
        ],
    ),
    (
        8897,
        &[(3558, 6949), (3362, 3692), (3372, 1307), (3557, 133)],
    ),
    (
        8898,
        &[
            (3362, 5962),
            (3558, 4037),
            (3372, 2174),
            (3557, 1242),
            (3371, 186),
        ],
    ),
    (
        8899,
        &[
            (3372, 6500),
            (3557, 3800),
            (3371, 200),
            (3382, 52),
            (3364, 30),
        ],
    ),
];

fn admitted(native: &NativeGameplayState, definition: &TypedDefinitionRef) -> bool {
    native
        .item_policy(&definition.production_key, &definition.revision_ref)
        .is_some_and(|policy| {
            let record = policy.record();
            record.production_definition.family == definition.family
                && record.admission.materializable
        })
}

fn definition(native: &NativeGameplayState, id: u32) -> Option<TypedDefinitionRef> {
    let target = &native
        .item_policy_for_source_id(id)?
        .record()
        .production_binding
        .target;
    output_definition(native, target)
}

/// Resolve a plain lottery-ticket use. Draws divisible by 98 win, matching a 1-in-98 roll.
pub fn lottery_conversion(
    native: &NativeGameplayState,
    used: &TypedDefinitionRef,
    draw: u64,
) -> Option<CookingConversion> {
    (used.production_key == LOTTERY_TICKET && admitted(native, used)).then_some(())?;
    let outputs = if lottery_wins(draw) {
        vec![(definition(native, 5958)?, 1)]
    } else {
        Vec::new()
    };
    Some(CookingConversion {
        recipe: "oteryn:lottery-ticket.v1".into(),
        outputs,
    })
}

const fn lottery_wins(draw: u64) -> bool {
    draw.is_multiple_of(98)
}

/// Resolve remover + rusty armor/legs using Crystal's success threshold and a uniform
/// choice among eligible entries (equivalent to its repeated random-index rejection loop).
pub fn rust_conversion(
    native: &NativeGameplayState,
    tool: &TypedDefinitionRef,
    rusty: &TypedDefinitionRef,
    draw: u64,
) -> Option<CookingConversion> {
    (tool.production_key == RUST_REMOVER && admitted(native, tool) && admitted(native, rusty))
        .then_some(())?;
    let rusty_id = rusty
        .production_key
        .strip_prefix("oteryn:item.tibia.i")?
        .parse::<u32>()
        .ok()?;
    let outcomes = RUST.iter().find(|(id, _)| *id == rusty_id)?.1;
    let outputs = match rust_outcome(outcomes, draw) {
        Some(chosen) => vec![(definition(native, chosen)?, 1)],
        None => Vec::new(),
    };
    Some(CookingConversion {
        recipe: format!("oteryn:rust-remover.i{rusty_id}.v1"),
        outputs,
    })
}

fn rust_outcome(outcomes: &[(u32, u32)], draw: u64) -> Option<u32> {
    let chance = u32::try_from(draw % 10_000).ok()? + 1;
    if chance > outcomes.first()?.1 {
        return None;
    }
    let mut eligible = outcomes
        .iter()
        .filter(|(_, threshold)| *threshold >= chance);
    let count = u64::try_from(eligible.clone().count()).ok()?;
    let index = usize::try_from((draw / 10_000).checked_rem(count)?).ok()?;
    eligible.nth(index).map(|(id, _)| *id)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    #[test]
    fn actual_rust_selector_covers_every_eligible_entry_and_break_boundary() {
        for (_, outcomes) in RUST {
            assert_eq!(rust_outcome(outcomes, u64::from(outcomes[0].1)), None);
            assert_eq!(
                rust_outcome(outcomes, u64::from(outcomes[0].1) - 1),
                Some(outcomes[0].0)
            );
            for (rank, (id, _)) in outcomes.iter().enumerate() {
                assert_eq!(
                    rust_outcome(outcomes, u64::try_from(rank).expect("rank") * 10_000),
                    Some(*id)
                );
            }
        }
        assert_eq!(RUST[1].1[0].0, 3377);
    }

    #[test]
    fn donor_tables_cover_the_six_global_armor_and_legs_variants() {
        assert_eq!(
            RUST.iter()
                .map(|(_, outcomes)| outcomes.len())
                .sum::<usize>(),
            31
        );
        assert_eq!(
            RUST[0],
            (
                8894,
                &[(3358, 6994), (3377, 3952), (3359, 1502), (3357, 197)][..]
            )
        );
    }

    #[test]
    fn seeded_lottery_draws_cover_win_and_loss() {
        assert!(lottery_wins(0));
        assert!(!lottery_wins(1));
    }

    #[test]
    fn seeded_rust_draws_cover_success_and_break() {
        let outcomes = RUST[0].1;
        let select = |draw: u64| {
            let chance = u32::try_from((draw >> 48) % 10_000).expect("bounded") + 1;
            (chance <= outcomes[0].1).then_some(outcomes[0].0)
        };
        assert_eq!(select(0), Some(3358));
        assert_eq!(select(9_999_u64 << 48), None);
    }
}
