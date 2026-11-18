//! Lower Roshamuul bookkeeping from Crystal 201189f8, data-global/npc/sandomo.lua:53-59,97-165.

pub const GOLD_PER_CLUSTER: u32 = 100;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Contributions {
    pub mortar: u32,
    pub frazzlemaws: u32,
    pub silencers: u32,
}

impl Contributions {
    pub fn gold(self) -> u64 {
        u64::from(self.mortar) * 100 + u64::from(self.frazzlemaws) + u64::from(self.silencers)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Registry {
    pub recent: Contributions,
    pub registered_gold: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Topic {
    #[default]
    Idle,
    Repairs,
    Books,
    Confirm,
    Quantity,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Reply {
    pub topic: Topic,
    pub registry: Registry,
    pub clusters: u32,
    pub text: String,
}

impl Registry {
    /// Return the complete proposed result so a caller can persist the registry
    /// and award the exchange together before publishing the reply.
    pub fn talk(self, topic: Topic, message: &str) -> Option<Reply> {
        let keyword = |word| {
            super::npc_rules::keyword::matches(
                message,
                word,
                super::npc_rules::NpcKeywordMode::MsgContains,
            )
            .unwrap_or(false)
        };
        let mut reply = Reply {
            topic,
            registry: self,
            clusters: 0,
            text: String::new(),
        };
        let recent_gold = self.recent.gold();
        reply.text = if keyword("gratitude") {
            reply.topic = Topic::Repairs;
            "What did you do for us? Did you help with repairs?".into()
        } else if keyword("repairs") {
            if topic != Topic::Repairs {
                return None;
            }
            if recent_gold == 0 {
                reply.topic = Topic::Idle;
                "Come back after you have done at least one of the tasks.".into()
            } else {
                reply.topic = Topic::Books;
                format!(
                    "You delivered {} mortar, killed {} frazzlemaws and hunted {} silencers. That earns you {recent_gold} inquisition gold. Shall I add it to my books?",
                    self.recent.mortar, self.recent.frazzlemaws, self.recent.silencers
                )
            }
        } else if topic == Topic::Books {
            if !keyword("book") && !keyword("books") {
                return None;
            }
            reply.topic = Topic::Confirm;
            format!(
                "You earned {recent_gold} inquisition gold recently and have {} in my books. Shall I record your recent work and reset its counters?",
                self.registered_gold
            )
        } else if topic == Topic::Confirm {
            if !keyword("yes") {
                return None;
            }
            reply.topic = Topic::Idle;
            // Crystal assigns the recent total to the book, replacing its prior balance.
            reply.registry.registered_gold = recent_gold;
            reply.registry.recent = Contributions::default();
            format!("I recorded {recent_gold} inquisition gold. Ask for your record or trade.")
        } else if keyword("record") {
            if self.registered_gold > 0 {
                format!(
                    "I have {} inquisition gold in my books for you.",
                    self.registered_gold
                )
            } else {
                "I have no inquisition gold in my books for you.".into()
            }
        } else if keyword("trade") {
            if self.registered_gold < u64::from(GOLD_PER_CLUSTER) {
                reply.topic = Topic::Idle;
                "You do not have enough registered inquisition gold to trade.".into()
            } else {
                reply.topic = Topic::Quantity;
                "One cluster costs 100 inquisition gold. How many clusters do you want?".into()
            }
        } else if topic == Topic::Quantity {
            let maximum = self.registered_gold / u64::from(GOLD_PER_CLUSTER);
            let quantity = message.trim().parse::<f64>().ok().filter(|value| {
                (1.0..=f64::from(u32::MAX)).contains(value) && value.fract() == 0.0
            });
            match quantity.map(|value| value as u32) {
                Some(quantity) if u64::from(quantity) <= maximum => {
                    reply.topic = Topic::Idle;
                    reply.clusters = quantity;
                    reply.registry.registered_gold -=
                        u64::from(quantity) * u64::from(GOLD_PER_CLUSTER);
                    format!(
                        "You have {} inquisition gold left in my books.",
                        reply.registry.registered_gold
                    )
                }
                Some(_) => {
                    format!("You can ask for up to {maximum} clusters.")
                }
                _ => "Tell me a positive whole number.".into(),
            }
        } else {
            return None;
        };
        Some(reply)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roshamuul_registration_replaces_the_book_and_exchange_charges_quantity()
    -> Result<(), &'static str> {
        let initial = Registry {
            recent: Contributions {
                mortar: 2,
                frazzlemaws: 1,
                silencers: 1,
            },
            registered_gold: 900,
        };
        let mut reply = initial.talk(Topic::Idle, "gratitude").ok_or("gratitude")?;
        for message in ["repairs", "books", "yes"] {
            reply = reply
                .registry
                .talk(reply.topic, message)
                .ok_or("bookkeeping")?;
        }
        assert_eq!(reply.registry.recent, Contributions::default());
        assert_eq!(reply.registry.registered_gold, 202);
        let trade = reply.registry.talk(reply.topic, "trade").ok_or("trade")?;
        for message in ["0", "1.5", "3"] {
            let refused = trade.registry.talk(trade.topic, message).ok_or("refusal")?;
            assert_eq!(refused.topic, Topic::Quantity);
            assert_eq!(refused.registry, trade.registry);
            assert_eq!(refused.clusters, 0);
        }
        let bought = trade.registry.talk(trade.topic, "2.0").ok_or("exchange")?;
        assert_eq!(bought.clusters, 2);
        assert_eq!(bought.registry.registered_gold, 2);
        assert_eq!(bought.topic, Topic::Idle);
        assert!(bought.registry.talk(bought.topic, "2").is_none());
        Ok(())
    }
}
