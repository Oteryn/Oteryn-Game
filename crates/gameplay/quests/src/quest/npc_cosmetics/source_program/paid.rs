//! Literal source fee closure. Payment mechanics remain the mature audited owner.
//! Crystal201189f: irmana.lua79–84/114–120/156–161/188–194;
//! Only Royal9 and Irmana4 are admitted; Simon and Gnommander remain held.
/// (gold, cash-only minimum); cash+bank never satisfies a cash-only source.
pub fn terms(npc: &str, transition: &str) -> Option<(u64, u64)> {
    match (npc, transition) {
        (
            "oteryn:npc.king_tibianus" | "oteryn:npc.queen_eloise" | "oteryn:npc.emperor_kruzak",
            "golden-buy-armor",
        ) => Some((500_000_000, 0)),
        (
            "oteryn:npc.king_tibianus" | "oteryn:npc.queen_eloise" | "oteryn:npc.emperor_kruzak",
            "golden-buy-helmet" | "golden-buy-boots",
        ) => Some((250_000_000, 0)),
        (
            "oteryn:npc.irmana",
            "noble-female-1-buy" | "noble-female-2-buy" | "noble-male-1-buy" | "noble-male-2-buy",
        ) => Some((150_000, 150_000)),
        _ => None,
    }
}
/// Source refusal text and whether its topic resets; no success-state writes.
pub fn insufficient(npc: &str, topic: u16) -> Option<(&'static str, u16)> {
    match npc {
        "oteryn:npc.irmana" => Some(("You do not have enough money.", topic)),
        "oteryn:npc.king_tibianus" | "oteryn:npc.queen_eloise" | "oteryn:npc.emperor_kruzak" => {
            Some(("You do not have enough money to donate that amount.", 2))
        }
        _ => None,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn finite_paid_source_terms_match_thirteen_admitted_money_effects()
    -> Result<(), Box<dyn std::error::Error>> {
        let program = super::super::embedded().map_err(|_| "source programme")?;
        let mut count = 0;
        for npc in &program.npcs {
            for transition in &npc.transitions {
                if let Some((price, cash_minimum)) = terms(&npc.npc, &transition.id) {
                    let effects: Vec<_> = transition
                        .effects
                        .iter()
                        .filter_map(|e| match e {
                            super::super::Effect::ConsumeMoney { gold } => Some((*gold, *gold)),
                            super::super::Effect::ConsumeMoneyBank { gold } => Some((*gold, 0)),
                            _ => None,
                        })
                        .collect();
                    assert_eq!(
                        effects,
                        vec![(price, cash_minimum)],
                        "exact pinned payment mode/amount: {}",
                        transition.source_key
                    );
                    count += 1;
                }
            }
        }
        assert_eq!(count, 13);
        assert_eq!(
            terms("oteryn:npc.irmana", "noble-female-1-buy"),
            Some((150000, 150000))
        );
        assert_eq!(
            terms("oteryn:npc.simon_the_beggar", "beggar-fur-pay-male"),
            None
        );
        assert_eq!(terms("oteryn:npc.gnommander", "cave-buy-male-second"), None);
        assert_eq!(terms("oteryn:npc.irmana", "golden-buy-armor"), None);
        assert_eq!(terms("oteryn:npc.gnommander", "cave-buy-male-third"), None);
        Ok(())
    }
}
