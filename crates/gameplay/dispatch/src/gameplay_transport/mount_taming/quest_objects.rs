//! Closed carried-tool callbacks routed through the existing World receipt owner.
use oteryn_gameplay_quests::quest::{targuna_ram, world_callbacks::Callback};

pub(super) fn accepts(callback: &Callback) -> bool {
    callback.item_on_target.is_some()
        && (callback.key == targuna_ram::KEY
            || matches!(
                callback.native_rule,
                Some(oteryn_gameplay_quests::quest::native_rules::NativeRule::IceSealPaint { .. })
            ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_gameplay_quests::quest::{native_rules::NativeRule, world_callbacks};

    #[test]
    fn registered_ice_paint_callbacks_reach_the_carried_tool_owner() -> Result<(), String> {
        let callbacks = world_callbacks::load()?;
        let paint: Vec<_> = callbacks
            .iter()
            .filter(|c| matches!(c.native_rule, Some(NativeRule::IceSealPaint { .. })))
            .collect();
        assert_eq!(paint.len(), 198);
        assert!(
            paint.iter().all(|callback| accepts(callback)),
            "registered seal tools cannot fall through to unrelated mining"
        );
        let ram = callbacks
            .iter()
            .find(|c| c.key == targuna_ram::KEY)
            .ok_or("Targuna callback")?;
        assert!(accepts(ram));
        let mut missing_tool = (*paint[0]).clone();
        missing_tool.item_on_target = None;
        assert!(!accepts(&missing_tool));
        let mut unrelated = (*paint[0]).clone();
        unrelated.native_rule = None;
        assert!(!accepts(&unrelated));
        unrelated.key = "oteryn:quest-callback/unrelated".into();
        assert!(!accepts(&unrelated));
        Ok(())
    }
}
