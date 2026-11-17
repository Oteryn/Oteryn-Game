//! Weighted item exchanges. The durable caller supplies a bounded server RNG draw.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WeightedOutcome {
    pub weight: u32,
    /// None is a blank: consume the input without granting an item.
    pub item: Option<&'static str>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RandomItemExchange {
    pub input: &'static str,
    pub outcomes: &'static [WeightedOutcome],
}

impl RandomItemExchange {
    /// Resolve the inventory effects before handing them to the existing atomic
    /// NPC item participant. Missing input refuses before consuming an RNG draw.
    pub fn effects(
        &self,
        available: u32,
        draw: impl FnOnce(u32) -> u32,
    ) -> Option<Vec<super::NpcRuleEffect>> {
        if available == 0 {
            return None;
        }
        let total = self.total_weight()?;
        let item = self.draw(draw(total))?;
        let mut effects = vec![super::NpcRuleEffect::TakeItem {
            item: self.input.into(),
            count: 1,
            subtype: None,
            optional: false,
        }];
        if let Some(item) = item {
            effects.push(super::NpcRuleEffect::GiveItem {
                item: item.into(),
                count: 1,
                subtype: None,
            });
        }
        Some(effects)
    }

    /// Reject empty, zero-weight or overflowing tables before requesting a draw.
    pub fn total_weight(&self) -> Option<u32> {
        if self.outcomes.is_empty() {
            return None;
        }
        self.outcomes.iter().try_fold(0_u32, |sum, outcome| {
            if outcome.weight == 0 {
                None
            } else {
                sum.checked_add(outcome.weight)
            }
        })
    }

    /// A zero-based uniform draw in 0..total_weight. Outer None rejects input;
    /// inner None represents a valid blank outcome.
    pub fn draw(&self, mut draw: u32) -> Option<Option<&'static str>> {
        if draw >= self.total_weight()? {
            return None;
        }
        for outcome in self.outcomes {
            if draw < outcome.weight {
                return Some(outcome.item);
            }
            draw -= outcome.weight;
        }
        None
    }
}

/// Crystal 201189f871d23597b82caf88fd611454a2ddefb8:
/// data/scripts/actions/items/lottery_ticket.lua:4-9 draws math.random(98),
/// transforms 5957 to 5958 on 1 and removes the ticket otherwise.
/// data-global/npc/sandra.lua:108-110 issues the unopened ticket for 100 vials.
pub const SANDRA_LOTTERY: RandomItemExchange = RandomItemExchange {
    input: "oteryn:item.tibia.i5957",
    outcomes: &[
        WeightedOutcome {
            weight: 1,
            item: Some("oteryn:item.tibia.i5958"),
        },
        WeightedOutcome {
            weight: 97,
            item: None,
        },
    ],
};
