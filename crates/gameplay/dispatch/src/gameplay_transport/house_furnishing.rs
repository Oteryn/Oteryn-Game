//! WRAP-1 House-owned furniture and fluid transitions.
// Rules slice: the House intent route wires `dispatch` in the follow-up; until then only tests call it.
#![cfg_attr(not(test), allow(dead_code))]

use oteryn_protocol_oteryn::world_object::UseDisposition;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Wrap,
    Unwrap,
    PlaceKit,
    WeaponHit,
    Fill,
    PourGround,
    PourContainer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    pub action: Action,
    pub source: &'static str,
    pub output: &'static str,
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
}

impl State {
    pub fn apply(
        &mut self,
        authority: Authority,
        command: &Command,
        rules: &[Rule],
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
                } else {
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
    rules: &[Rule],
) -> UseDisposition {
    authority.map_or(UseDisposition::NothingToUse, |current| {
        state.apply(current, command, rules)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rules() -> Vec<Rule> {
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
}
