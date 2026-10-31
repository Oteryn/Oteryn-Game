//! P11: the Achievement keys a committed NPC trade earns.
//!
//! Si, Ariki! (TibiaWiki id 277, `imports/tibiawiki/achievements/2026-09-29/
//! tibiawiki-achievements-facts.json`): "You've found the oriental traveller Yasir and were able
//! to trade with him". Crystal `201189f8` `data-global/npc/yasir.lua:820,826` and Canary
//! `47dfd51f` `data-otservbr-global/npc/yasir.lua:822,828` grant it when Yasir's trade opens;
//! Oteryn grants it on the first committed trade with him, the durable form of that text. Every
//! other Crystal NPC `addAchievement` is gated by quest state the NPC holds, not by the trade.

/// The grant request's source kind; the event id is the trade's TransactionId.
pub const SOURCE_KIND: &str = "oteryn:npc-trade";

/// (NPC, key).
const TRADES: [(&str, &str); 1] = [("oteryn:npc.yasir", "oteryn:achievement/si_ariki")];

/// The keys a committed trade with `npc_key` earns.
pub fn keys(npc_key: &str) -> Vec<&'static str> {
    TRADES
        .iter()
        .filter(|(npc, _)| *npc == npc_key)
        .map(|(_, key)| *key)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_trade_with_yasir_earns_si_ariki_and_no_other_npc_earns_anything() {
        assert_eq!(keys("oteryn:npc.yasir"), ["oteryn:achievement/si_ariki"]);
        assert!(keys("oteryn:npc.rashid").is_empty());
    }

    #[test]
    fn every_key_is_earnable_in_the_embedded_catalogue()
    -> Result<(), crate::achievement_catalogue::AchievementCatalogueError> {
        let catalogue =
            crate::achievement_catalogue::parse_shards(&crate::achievement_catalogue::SHARDS)?;
        for (_, key) in TRADES {
            assert!(
                catalogue.get(key).is_some_and(|entry| !entry.retired),
                "{key}"
            );
        }
        Ok(())
    }
}
