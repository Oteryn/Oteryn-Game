//! Conditional merchants may open their source greeting; trade retains its gates.
pub(super) fn contains(key: &str) -> bool {
    crate::content::npc_catalogue::quest_shops::shops()
        .is_ok_and(|shops| shops.iter().any(|shop| shop.npc == key))
}

#[cfg(test)]
mod tests {
    #[test]
    fn conditional_merchants_have_greetings_without_reward_callbacks() {
        for key in [
            "oteryn:npc.yaman",
            "oteryn:npc.haroun",
            "oteryn:npc.nah_bob",
        ] {
            assert!(super::contains(key), "{key}");
            assert!(!super::super::reward_callback_npc(key), "{key}");
            assert!(super::super::reward_messages(key).is_some(), "{key}");
        }
        assert!(!super::contains("oteryn:npc.unregistered_merchant"));
        assert!(super::super::reward_messages("oteryn:npc.unregistered_merchant").is_none());
    }
}
