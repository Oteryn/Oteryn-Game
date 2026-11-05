//! WRAP-1 House-owned furniture and fluid transitions.
// Furniture conversions are drafted by `house_wrap` and fenced again by the durable owner.

use oteryn_protocol_oteryn::world_object::UseDisposition;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Wrap,
    // Purchased decoration-kit metadata unwraps before this generic transition.
    #[cfg_attr(not(test), allow(dead_code))]
    Unwrap,
    // Retained deterministic rules slice; this command admits furniture wrap/unwrap only.
    #[cfg_attr(not(test), allow(dead_code))]
    PlaceKit,
    // Retained deterministic rules slice; this command admits furniture wrap/unwrap only.
    #[cfg_attr(not(test), allow(dead_code))]
    WeaponHit,
    // Retained deterministic rules slice; this command admits furniture wrap/unwrap only.
    #[cfg_attr(not(test), allow(dead_code))]
    Fill,
    // Retained deterministic rules slice; this command admits furniture wrap/unwrap only.
    #[cfg_attr(not(test), allow(dead_code))]
    PourGround,
    // Retained deterministic rules slice; this command admits furniture wrap/unwrap only.
    #[cfg_attr(not(test), allow(dead_code))]
    PourContainer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule<'a> {
    pub action: Action,
    pub source: &'a str,
    pub output: &'a str,
    pub amount: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub definition: String,
    pub amount: u32,
    pub liquid: Option<String>,
    pub hit_points: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Authority {
    pub generation: u64,
    pub acl_revision: u64,
    pub may_edit: bool,
}

/// Persisted by the physical House owner. The transition is deterministic so retrying the same
/// expected revision cannot duplicate a kit, remains, or liquid.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct State {
    pub revision: u64,
    pub house: BTreeMap<u64, Item>,
    pub inventory: BTreeMap<u64, Item>,
    pub ground_liquids: BTreeMap<u64, (String, u32)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    pub revision: u64,
    pub generation: u64,
    pub acl_revision: u64,
    pub action: Action,
    pub source: u64,
    pub target: Option<u64>,
    /// Pre-sampled uniform roll in 1..=7 from the production command owner.
    pub destruction_roll: u8,
}

impl State {
    pub fn apply(
        &mut self,
        authority: Authority,
        command: &Command,
        rules: &[Rule<'_>],
    ) -> UseDisposition {
        if (self.revision, authority.generation, authority.acl_revision)
            != (command.revision, command.generation, command.acl_revision)
        {
            return UseDisposition::StaleState;
        }
        if !authority.may_edit {
            return UseDisposition::NothingToUse;
        }
        let Some(before) = self
            .house
            .get(&command.source)
            .cloned()
            .or_else(|| self.inventory.get(&command.source).cloned())
        else {
            return UseDisposition::StaleState;
        };
        let Some(rule) = rules
            .iter()
            .find(|rule| rule.action == command.action && rule.source == before.definition)
        else {
            return UseDisposition::NothingToUse;
        };
        match command.action {
            Action::Wrap => {
                self.house.remove(&command.source);
                self.inventory.insert(
                    command.source,
                    Item {
                        definition: rule.output.into(),
                        ..before
                    },
                );
            }
            Action::Unwrap | Action::PlaceKit => {
                if self.inventory.remove(&command.source).is_none() {
                    return UseDisposition::NothingToUse;
                }
                self.house.insert(
                    command.source,
                    Item {
                        definition: rule.output.into(),
                        ..before
                    },
                );
            }
            Action::WeaponHit => {
                let Some(item) = self.house.get_mut(&command.source) else {
                    return UseDisposition::NothingToUse;
                };
                if item.hit_points > rule.amount {
                    item.hit_points -= rule.amount;
                } else if command.destruction_roll == 1 {
                    *item = Item {
                        definition: rule.output.into(),
                        amount: 1,
                        liquid: None,
                        hit_points: 0,
                    };
                }
            }
            Action::Fill => {
                let Some(target) = command.target.and_then(|id| self.inventory.get_mut(&id)) else {
                    return UseDisposition::StaleState;
                };
                if target.liquid.is_some() || before.liquid.is_none() {
                    return UseDisposition::NothingToUse;
                }
                target.liquid = before.liquid;
                target.amount = rule.amount;
            }
            Action::PourGround => {
                let Some(source) = self.inventory.get_mut(&command.source) else {
                    return UseDisposition::NothingToUse;
                };
                let Some(liquid) = source.liquid.take() else {
                    return UseDisposition::NothingToUse;
                };
                let amount = source.amount;
                source.amount = 0;
                self.ground_liquids
                    .insert(command.target.unwrap_or_default(), (liquid, amount));
            }
            Action::PourContainer => {
                let Some(target_id) = command.target.filter(|id| *id != command.source) else {
                    return UseDisposition::NothingToUse;
                };
                let Some(mut source) = self.inventory.remove(&command.source) else {
                    return UseDisposition::NothingToUse;
                };
                let accepted = self.inventory.get_mut(&target_id).is_some_and(|target| {
                    if target.liquid.is_none() && source.liquid.is_some() {
                        target.liquid = source.liquid.take();
                        target.amount = source.amount;
                        source.amount = 0;
                        true
                    } else {
                        false
                    }
                });
                self.inventory.insert(command.source, source);
                if !accepted {
                    return UseDisposition::NothingToUse;
                }
            }
        }
        self.revision += 1;
        UseDisposition::Committed
    }
}

/// Production command boundary. House admission supplies the current authority; `None` is the
/// deliberate refusal for a Channel/outdoor session. The mutable state is borrowed from the
/// physical House owner, so no transport copy can commit an effect.
pub fn dispatch(
    state: &mut State,
    authority: Option<Authority>,
    command: &Command,
    rules: &[Rule<'_>],
) -> UseDisposition {
    authority.map_or(UseDisposition::NothingToUse, |current| {
        state.apply(current, command, rules)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rules() -> Vec<Rule<'static>> {
        vec![
            Rule {
                action: Action::Wrap,
                source: "chair",
                output: "chair-kit",
                amount: 0,
            },
            Rule {
                action: Action::PlaceKit,
                source: "chair-kit",
                output: "chair",
                amount: 0,
            },
            Rule {
                action: Action::Unwrap,
                source: "chair-kit",
                output: "chair",
                amount: 0,
            },
            Rule {
                action: Action::WeaponHit,
                source: "chair",
                output: "wood-remains",
                amount: 2,
            },
            Rule {
                action: Action::Fill,
                source: "well",
                output: "",
                amount: 10,
            },
            Rule {
                action: Action::PourGround,
                source: "vial",
                output: "",
                amount: 0,
            },
            Rule {
                action: Action::PourContainer,
                source: "vial",
                output: "",
                amount: 0,
            },
        ]
    }
    fn auth(access: bool) -> Authority {
        Authority {
            generation: 4,
            acl_revision: 7,
            may_edit: access,
        }
    }
    fn cmd(action: Action, source: u64, target: Option<u64>, revision: u64) -> Command {
        Command {
            revision,
            generation: 4,
            acl_revision: 7,
            action,
            source,
            target,
            destruction_roll: 1,
        }
    }
    #[test]
    fn operations_are_atomic_and_fresh_acl_fenced() {
        let mut state = State::default();
        state.house.insert(
            1,
            Item {
                definition: "chair".into(),
                amount: 1,
                liquid: None,
                hit_points: 3,
            },
        );
        state.house.insert(
            2,
            Item {
                definition: "well".into(),
                amount: 1,
                liquid: Some("water".into()),
                hit_points: 0,
            },
        );
        for id in [3, 4] {
            state.inventory.insert(
                id,
                Item {
                    definition: "vial".into(),
                    amount: 0,
                    liquid: None,
                    hit_points: 0,
                },
            );
        }
        state.inventory.insert(
            5,
            Item {
                definition: "chair-kit".into(),
                amount: 1,
                liquid: None,
                hit_points: 3,
            },
        );
        assert_eq!(
            dispatch(&mut state, None, &cmd(Action::Wrap, 1, None, 0), &rules()),
            UseDisposition::NothingToUse
        );
        assert_eq!(
            dispatch(
                &mut state,
                Some(auth(false)),
                &cmd(Action::Wrap, 1, None, 0),
                &rules()
            ),
            UseDisposition::NothingToUse
        );
        for (revision, command) in [
            (0, cmd(Action::Wrap, 1, None, 0)),
            (1, cmd(Action::PlaceKit, 1, None, 1)),
            (2, cmd(Action::Unwrap, 5, None, 2)),
            (3, cmd(Action::WeaponHit, 1, None, 3)),
            (4, cmd(Action::WeaponHit, 1, None, 4)),
            (5, cmd(Action::Fill, 2, Some(3), 5)),
            (6, cmd(Action::PourContainer, 3, Some(4), 6)),
            (7, cmd(Action::PourGround, 4, Some(99), 7)),
        ] {
            assert_eq!(state.revision, revision);
            assert_eq!(
                dispatch(&mut state, Some(auth(true)), &command, &rules()),
                UseDisposition::Committed
            );
        }
        assert_eq!(state.house[&1].definition, "wood-remains");
        assert_eq!(state.ground_liquids[&99], ("water".into(), 10));
        assert_eq!(
            dispatch(
                &mut state,
                Some(auth(true)),
                &cmd(Action::Wrap, 1, None, 7),
                &rules()
            ),
            UseDisposition::StaleState
        );
    }

    #[test]
    fn refused_fluid_transfer_does_not_lose_the_source_liquid() {
        let mut state = State::default();
        state.inventory.insert(
            1,
            Item {
                definition: "vial".into(),
                amount: 10,
                liquid: Some("water".into()),
                hit_points: 0,
            },
        );
        state.inventory.insert(
            2,
            Item {
                definition: "vial".into(),
                amount: 10,
                liquid: Some("oil".into()),
                hit_points: 0,
            },
        );
        let before = state.clone();

        assert_eq!(
            dispatch(
                &mut state,
                Some(auth(true)),
                &cmd(Action::PourContainer, 1, Some(2), 0),
                &rules(),
            ),
            UseDisposition::NothingToUse
        );
        assert_eq!(state, before);
    }
}

// Crystal 201189f8, data-crystal/scripts/actions/other/destroy.lua:1-251.
const DESTROY_WEAPONS: &[u32] = &[
    860, 3264, 3265, 3266, 3267, 3268, 3269, 3270, 3271, 3272, 3273, 3274, 3275, 3276, 3278, 3279,
    3280, 3281, 3282, 3283, 3284, 3285, 3286, 3288, 3289, 3290, 3291, 3292, 3294, 3295, 3296, 3297,
    3298, 3299, 3300, 3301, 3302, 3305, 3309, 3310, 3311, 3312, 3313, 3314, 3315, 3316, 3317, 3318,
    3319, 3320, 3321, 3322, 3323, 3324, 3325, 3326, 3327, 3328, 3329, 3331, 3332, 3333, 3334, 3335,
    3336, 3337, 3338, 3339, 3340, 3341, 3342, 3343, 3344, 3346, 3348, 6101, 6528, 6553, 7379, 7380,
    7381, 7382, 7383, 7384, 7385, 7386, 7387, 7388, 7389, 7390, 7391, 7392, 7402, 7403, 7404, 7405,
    7406, 7407, 7408, 7409, 7410, 7411, 7412, 7413, 7414, 7415, 7416, 7417, 7418, 7419, 7420, 7421,
    7422, 7423, 7424, 7425, 7426, 7427, 7428, 7429, 7430, 7431, 7432, 7433, 7434, 7435, 7436, 7437,
    7449, 7450, 7451, 7452, 7453, 7454, 7455, 7456, 7773, 7774, 8096, 8097, 8098, 8099, 8100, 8101,
    8102, 8103, 8104, 9373, 9375, 9376, 9385, 9386, 9387, 9396, 10388, 10389, 10390, 10391, 10392,
    10406, 11657, 11692, 11693, 12510, 12673, 12683, 12731, 13987, 13991, 14001, 14040, 14043,
    14250, 16160, 16161, 16162, 16175, 17109, 17784, 17812, 17813, 17824, 17828, 17859, 20065,
    20066, 20067, 20068, 20069, 20070, 20071, 20072, 20073, 20074, 20075, 20076, 20077, 20078,
    20079, 20080, 20081, 21171, 21172, 21173, 21174, 21176, 21177, 21178, 21179, 21180, 21219,
    22171, 22172, 22727, 22759, 22760, 22762, 27449, 27450, 27451, 27452, 27453, 27454, 27523,
    27525, 27651, 28464, 28723, 28724, 28725, 28829, 28830, 28831, 28832, 29419, 29421, 29422,
    30283, 30395, 30396, 30398, 31580, 31614, 32093, 32616, 50163, 50164, 50165, 50168,
];

pub(super) fn is_destroy_weapon(item_id: u32) -> bool {
    DESTROY_WEAPONS.binary_search(&item_id).is_ok()
}
pub(super) fn destroy_roll_succeeds(roll: u8, attack: i32) -> bool {
    (1..=80).contains(&roll) && i32::from(roll) <= attack.max(10)
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct DestroyRemains {
    pub item_id: u32,
    pub decay_target: u32,
    pub duration_ms: u32,
}
impl DestroyRemains {
    pub(super) const fn new(item_id: u32, decay_target: u32, duration_ms: u32) -> Self {
        Self {
            item_id,
            decay_target,
            duration_ms,
        }
    }
}
pub(super) fn destroy_remains(item_id: u32) -> Option<DestroyRemains> {
    match item_id {
        2295 => Some(DestroyRemains::new(3145, 2295, 120_000)),
        2296 => Some(DestroyRemains::new(3146, 2296, 120_000)),
        4995 => Some(DestroyRemains::new(4996, 4995, 300_000)),
        _ => None,
    }
}
#[cfg(test)]
mod destroy_tests {
    use super::*;
    #[test]
    fn destroy_roll_uses_inclusive_eighty_sided_attack_threshold() {
        for attack in [0, 9, 10] {
            for roll in 0..=u8::MAX {
                assert_eq!(
                    destroy_roll_succeeds(roll, attack),
                    (1..=10).contains(&roll)
                );
            }
        }
        for roll in 0..=u8::MAX {
            assert_eq!(destroy_roll_succeeds(roll, 40), (1..=40).contains(&roll));
            assert_eq!(destroy_roll_succeeds(roll, 80), (1..=80).contains(&roll));
        }
    }
    #[test]
    fn destroy_remains_match_crystal_decay_pairs() {
        assert_eq!(
            destroy_remains(2295),
            Some(DestroyRemains::new(3145, 2295, 120_000))
        );
        assert_eq!(
            destroy_remains(2296),
            Some(DestroyRemains::new(3146, 2296, 120_000))
        );
        assert_eq!(
            destroy_remains(4995),
            Some(DestroyRemains::new(4996, 4995, 300_000))
        );
        assert_eq!(destroy_remains(3145), None);
    }
    #[test]
    fn destroy_weapons_are_the_exact_sorted_crystal_set() {
        assert_eq!(DESTROY_WEAPONS.len(), 249);
        use sha2::{Digest, Sha256};
        let digest: [u8; 32] = DESTROY_WEAPONS
            .iter()
            .fold(Sha256::new(), |hash, id| {
                hash.chain_update(id.to_be_bytes())
            })
            .finalize()
            .into();
        assert_eq!(
            digest,
            [
                242, 205, 21, 190, 71, 106, 16, 26, 158, 227, 211, 186, 162, 52, 146, 86, 71, 218,
                17, 66, 184, 197, 196, 243, 129, 10, 237, 195, 125, 206, 39, 124
            ]
        );
        assert!(DESTROY_WEAPONS.windows(2).all(|v| v[0] < v[1]));
        for id in 0..=50169 {
            assert_eq!(is_destroy_weapon(id), DESTROY_WEAPONS.contains(&id));
        }
        for id in [860, 7452, 50163, 50164, 50165, 50168] {
            assert!(is_destroy_weapon(id));
        }
        for id in [859, 3277, 3345, 50166, 50167, 50169] {
            assert!(!is_destroy_weapon(id));
        }
    }
}
