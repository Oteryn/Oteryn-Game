//! Closed carried-tool callbacks routed through the existing World receipt owner.
use oteryn_gameplay_quests::quest::{
    authored_bindings::Bindings,
    targuna_ram,
    world_callbacks::{Callback, ItemOnTarget, Trigger},
};

pub(crate) fn accepts(callback: &Callback, bindings: Option<&Bindings>) -> bool {
    callback.item_on_target.is_some()
        && (callback.key == targuna_ram::KEY
            || matches!(
                callback.native_rule,
                Some(oteryn_gameplay_quests::quest::native_rules::NativeRule::IceSealPaint { .. })
            )
            // Crystal actions_trough.lua:3-16 is one use of this authored
            // path. Only the registered native target can take the selected item.
            || (callback.trigger == Trigger::Use
                && callback.requires_world_admission
                && callback.selector.native.is_some()
                && callback.item_on_target.as_ref().is_some_and(|item| item.selected_instance)
                && bindings.is_some_and(|bindings| bindings.world_callbacks().any(|native| native == callback))))
}

pub(in crate::gameplay_transport) fn accepts_source(
    input: &ItemOnTarget,
    definition: &str,
) -> bool {
    input.item == definition || input.alternatives.iter().any(|item| item == definition)
}

/// Select only the wire subject observed in the fenced inventory snapshot.
/// This read does not grant mutation authority; the World writer locks custody.
pub(crate) fn matches_source(
    subject: &oteryn_protocol_oteryn::world_object::UseTarget,
    resolved: Option<[u8; 16]>,
    candidate: [u8; 16],
    current_definition: Option<std::num::NonZeroU32>,
) -> bool {
    use oteryn_protocol_oteryn::world_object::UseTarget;
    match subject {
        UseTarget::Item(_) => resolved == Some(candidate),
        UseTarget::ItemByDefinition(definition) => {
            resolved.is_none() && current_definition == Some(*definition)
        }
        UseTarget::WorldObject(_) => false,
    }
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
            paint.iter().all(|callback| accepts(callback, None)),
            "registered seal tools cannot fall through to unrelated mining"
        );
        let ram = callbacks
            .iter()
            .find(|c| c.key == targuna_ram::KEY)
            .ok_or("Targuna callback")?;
        assert!(accepts(ram, None));
        let retained: Vec<_> = callbacks
            .iter()
            .filter(|callback| {
                oteryn_gameplay_quests::quest::ice_seal_paint::historical_only(&callback.key)
            })
            .collect();
        assert_eq!(retained.len(), 9);
        assert!(retained.iter().all(|callback| !accepts(callback, None)));
        let mut missing_tool = (*paint[0]).clone();
        missing_tool.item_on_target = None;
        assert!(!accepts(&missing_tool, None));
        let mut unrelated = (*paint[0]).clone();
        unrelated.native_rule = None;
        assert!(!accepts(&unrelated, None));
        unrelated.key = "oteryn:quest-callback/unrelated".into();
        assert!(!accepts(&unrelated, None));
        Ok(())
    }
    #[test]
    fn world_tool_source_accepts_current_definition_and_exact_handle() -> Result<(), String> {
        use oteryn_protocol_oteryn::{item_view::ItemHandle, world_object::UseTarget};
        let definition = std::num::NonZeroU32::MIN;
        let other = std::num::NonZeroU32::new(2).ok_or("other definition")?;
        let hotkey = UseTarget::ItemByDefinition(definition);
        assert!(matches_source(&hotkey, None, [1; 16], Some(definition)));
        assert!(!matches_source(&hotkey, None, [1; 16], None));
        assert!(!matches_source(&hotkey, None, [1; 16], Some(other)));
        assert!(!matches_source(
            &hotkey,
            Some([1; 16]),
            [1; 16],
            Some(definition)
        ));
        let handle = UseTarget::Item(ItemHandle::new(7).ok_or("handle")?);
        assert!(matches_source(
            &handle,
            Some([1; 16]),
            [1; 16],
            Some(definition)
        ));
        assert!(!matches_source(
            &handle,
            Some([2; 16]),
            [1; 16],
            Some(definition)
        ));
        assert!(!matches_source(&handle, None, [1; 16], Some(definition)));
        Ok(())
    }
}
