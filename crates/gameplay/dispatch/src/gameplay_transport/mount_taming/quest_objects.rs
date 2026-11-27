//! Closed carried-tool callbacks routed through the existing World receipt owner.
use oteryn_gameplay_quests::quest::{
    rottin_walls, targuna_ram,
    world_callbacks::{Callback, ItemOnTarget},
};

/// Authored Top of the City pick-on-dirt step; the pick is the fenced selected tool.
pub(crate) const TOP_OF_THE_CITY_DIG: &str = "oteryn:quest-callback/authored-top-of-the-city-dig";

pub(crate) fn accepts(callback: &Callback) -> bool {
    callback.item_on_target.is_some()
        && (callback.key == targuna_ram::KEY
            || callback.key == TOP_OF_THE_CITY_DIG
            || (callback
                .item_on_target
                .as_ref()
                .is_some_and(|tool| tool.selected_instance)
                && ((callback.requires_world_admission
                    && callback.quest == "oteryn:quest.the_explorer_society_quest")
                    || matches!(
                        callback.key.as_str(),
                        "oteryn:quest-callback/druid-ceiron-waterskin"
                            | "oteryn:quest-callback/authored-repenters-fill-vial"
                            | "oteryn:quest-callback/authored-repenters-mushroom-01"
                            | "oteryn:quest-callback/authored-repenters-mushroom-02"
                            | "oteryn:quest-callback/authored-repenters-mushroom-03"
                            | "oteryn:quest-callback/authored-repenters-mushroom-04"
                            | "oteryn:quest-callback/authored-repenters-mushroom-05"
                            | "oteryn:quest-callback/authored-repenters-mushroom-06"
                            | "oteryn:quest-callback/authored-repenters-mushroom-07"
                            | "oteryn:quest-callback/authored-repenters-mushroom-08"
                            | "oteryn:quest-callback/authored-repenters-mushroom-09"
                            | "oteryn:quest-callback/authored-repenters-mushroom-10"
                            | "oteryn:quest-callback/authored-repenters-mushroom-11"
                            | "oteryn:quest-callback/authored-repenters-mushroom-12"
                            | "oteryn:quest-callback/authored-repenters-refill-vial"
                            | "oteryn:quest-callback/authored-venore-machine-1"
                            | "oteryn:quest-callback/authored-venore-machine-2"
                            | "oteryn:quest-callback/authored-venore-machine-3"
                            | "oteryn:quest-callback/authored-venore-machine-4"
                            | "oteryn:quest-callback/authored-venore-machine-5"
                            | "oteryn:quest-callback/authored-venore-machine-6"
                            | "oteryn:quest-callback/authored-venore-machine-7"
                            | "oteryn:quest-callback/authored-venore-machine-8"
                            | "oteryn:quest-callback/authored-venore-machine-9"
                            | "oteryn:quest-callback/qc-lizard-sanctuary-altar-1"
                            | "oteryn:quest-callback/qc-lizard-sanctuary-altar-2"
                            | "oteryn:quest-callback/alawars-vault-folda-shaft"
                            | "oteryn:quest-callback/an-interest-in-botany/s2"
                            | "oteryn:quest-callback/an-interest-in-botany/s3"
                            | "oteryn:quest-callback/an-interest-in-botany/s3-web"
                            | "oteryn:quest-callback/authored-ape-city-amphora-1"
                            | "oteryn:quest-callback/authored-ape-city-amphora-2"
                            | "oteryn:quest-callback/authored-ape-city-amphora-3"
                            | "oteryn:quest-callback/authored-ape-city-amphora-4"
                            | "oteryn:quest-callback/authored-ape-city-snake-destroyer"
                            | "oteryn:quest-callback/authored-new-frontier/bait-tree-1"
                            | "oteryn:quest-callback/authored-new-frontier/bait-tree-2"
                            | "oteryn:quest-callback/authored-new-frontier/bait-tree-3"
                            | "oteryn:quest-callback/authored-shattered-isles-ring"
                            | "oteryn:quest-callback/authored-shattered-isles-s4"
                            | "oteryn:quest-callback/authored-threatened-dreams/book-table-1"
                            | "oteryn:quest-callback/authored-threatened-dreams/book-table-2"
                            | "oteryn:quest-callback/explorer-society/collect-bells"
                            | "oteryn:quest-callback/explorer-society/cut-ice"
                            | "oteryn:quest-callback/to-blind-the-enemy/mirror"
                            | "oteryn:quest-callback/postman-folda-mailbox"
                            | "oteryn:quest-callback/postman-letterbag-delivery"
                            | "oteryn:quest-callback/rafzan-swamp-gas-33026-32183"
                            | "oteryn:quest-callback/rafzan-swamp-gas-33067-32121"
                            | "oteryn:quest-callback/rafzan-swamp-gas-33092-32061"
                            | "oteryn:quest-callback/questpc7-goblin-sign-1"
                            | "oteryn:quest-callback/questpc7-goblin-sign-2"
                            | "oteryn:quest-callback/questpc7-goblin-sign-3"
                            | "oteryn:quest-callback/questpc7-goblin-trunk-1"
                            | "oteryn:quest-callback/questpc7-goblin-trunk-1-last"
                            | "oteryn:quest-callback/questpc7-goblin-trunk-2"
                            | "oteryn:quest-callback/questpc7-goblin-trunk-2-last"
                            | "oteryn:quest-callback/questpc7-goblin-trunk-3"
                            | "oteryn:quest-callback/questpc7-goblin-trunk-3-last"
                            | "oteryn:quest-callback/ape-city-cask-32495"
                            | "oteryn:quest-callback/ape-city-cask-32497"
                            | "oteryn:quest-callback/ape-city-cask-32499"
                            | "oteryn:quest-callback/ape-city-cask-32501"
                            | "oteryn:quest-callback/ape-city-cask-32503"
                            | "oteryn:quest-callback/ape-city-cask-32505"
                            | "oteryn:quest-callback/ape-city-cask-32507"
                            | "oteryn:quest-callback/thieves-guild-cathedral-note"
                            | "oteryn:quest-callback/thieves-guild-fish-room-key"
                            | "oteryn:quest-callback/thieves-guild-sarina-vase"
                            | "oteryn:quest-callback/elvenbane-mirror-crystal"
                            | "oteryn:quest-callback/inquisition-eclipse-brew-32657-31927"
                            | "oteryn:quest-callback/inquisition-eclipse-brew-32658-31927"
                            | "oteryn:quest-callback/inquisition-eclipse-brew-32657-31928"
                            | "oteryn:quest-callback/inquisition-eclipse-brew-32658-31928"
                            | "oteryn:quest-callback/tibia-tales/fill-ale-bottle"
                            | "oteryn:quest-callback/tibia-tales/poison-slime-pool-31821"
                            | "oteryn:quest-callback/tibia-tales/poison-slime-pool-31822"
                    )
                    || (callback
                        .key
                        .starts_with("oteryn:quest-callback/tibia-tales/sanctify-grave-")
                        && callback
                            .selector
                            .action
                            .is_some_and(|action| (4208..=4223).contains(&action)))
                    || [
                        (
                            "oteryn:quest.bigfoot_s_burden_quest",
                            "oteryn:quest-callback/bigfoot-daily/",
                        ),
                        (
                            "oteryn:quest.dangerous_depths_quest",
                            "oteryn:quest-callback/dangerous-depths-daily/",
                        ),
                        (
                            "oteryn:quest.spike_task",
                            "oteryn:quest-callback/spike-daily/",
                        ),
                    ]
                    .iter()
                    .any(|(quest, prefix)| {
                        callback.quest == *quest && callback.key.starts_with(prefix)
                    })))
            || rottin_walls::accepts(callback)
            || callback.effects.place_item == Some(12187)
            || matches!(
                callback.native_rule,
                Some(oteryn_gameplay_quests::quest::native_rules::NativeRule::IceSealPaint { .. })
            ))
}

/// Match the exact production key of the required tool or an authored alternative.
pub(crate) fn is_required_tool(required: &ItemOnTarget, production_key: &str) -> bool {
    production_key == required.item
        || required
            .alternatives
            .iter()
            .any(|item| item == production_key)
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
    fn ape_city_casks_reach_the_carried_tool_owner_for_both_crowbars() -> Result<(), String> {
        let callbacks = world_callbacks::load()?;
        let casks: Vec<_> = callbacks
            .iter()
            .filter(|callback| {
                callback
                    .key
                    .starts_with("oteryn:quest-callback/ape-city-cask-")
            })
            .collect();
        assert_eq!(casks.len(), 7);
        for cask in casks {
            assert!(
                accepts(cask),
                "{} must reach the carried-tool owner",
                cask.key
            );
            let tool = cask.item_on_target.as_ref().ok_or("cask tool")?;
            for key in ["oteryn:item.tibia.i3304", "oteryn:item.tibia.i9598"] {
                assert!(
                    is_required_tool(tool, key),
                    "{} must accept {key}",
                    cask.key
                );
            }
            assert!(!is_required_tool(tool, "oteryn:item.tibia.i3457"));
        }
        Ok(())
    }

    #[test]
    fn authored_journey_map_tools_reach_the_carried_tool_owner() -> Result<(), String> {
        use oteryn_gameplay_quests::quest::loader::load_embedded_quest_state;

        let lowered =
            load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
        let callbacks = world_callbacks::load_for(lowered.catalogue())?;
        for (prefix, expected_count) in [
            ("authored-ape-city-", 5),
            ("authored-new-frontier/", 3),
            ("authored-shattered-isles-", 2),
            ("explorer-society/", 3),
            ("druid-ceiron-waterskin", 1),
            ("to-blind-the-enemy/", 1),
        ] {
            let selected_tools: Vec<_> = callbacks
                .iter()
                .filter(|callback| {
                    callback
                        .key
                        .strip_prefix("oteryn:quest-callback/")
                        .is_some_and(|key| key.starts_with(prefix))
                        && callback
                            .item_on_target
                            .as_ref()
                            .is_some_and(|tool| tool.selected_instance)
                })
                .collect();
            assert_eq!(selected_tools.len(), expected_count, "{prefix}");
            let unhandled: Vec<_> = selected_tools
                .into_iter()
                .filter(|callback| !accepts(callback))
                .map(|callback| callback.key.as_str())
                .collect();
            assert!(
                unhandled.is_empty(),
                "unhandled {prefix} tools: {unhandled:?}"
            );
        }
        Ok(())
    }

    #[test]
    fn gnome_daily_world_tools_reach_the_carried_tool_owner() -> Result<(), String> {
        let callbacks = world_callbacks::load()?;
        let tools: Vec<_> = callbacks
            .iter()
            .filter(|callback| {
                ["bigfoot-daily", "dangerous-depths-daily", "spike-daily"]
                    .iter()
                    .any(|owner| {
                        callback
                            .key
                            .starts_with(&format!("oteryn:quest-callback/{owner}/"))
                    })
                    && callback.item_on_target.is_some()
            })
            .collect();
        assert_eq!(tools.len(), 126);
        assert!(tools.iter().all(|callback| accepts(callback)));
        let mut unrelated = tools[0].clone();
        unrelated.quest = "oteryn:quest.tibia_tales".into();
        assert!(!accepts(&unrelated));
        Ok(())
    }

    #[test]
    fn thieves_guild_tools_use_the_selected_carried_instance() -> Result<(), String> {
        let callbacks = world_callbacks::load()?;
        for name in ["cathedral-note", "fish-room-key", "sarina-vase"] {
            let key = format!("oteryn:quest-callback/thieves-guild-{name}");
            let callback = callbacks
                .iter()
                .find(|callback| callback.key == key)
                .ok_or(key)?;
            assert!(accepts(callback));
        }
        Ok(())
    }

    #[test]
    fn mirror_and_eclipse_brews_reach_the_carried_tool_owner() -> Result<(), String> {
        let callbacks = world_callbacks::load()?;
        let routes: Vec<_> = callbacks
            .iter()
            .filter(|callback| {
                callback.key == "oteryn:quest-callback/elvenbane-mirror-crystal"
                    || callback
                        .key
                        .starts_with("oteryn:quest-callback/inquisition-eclipse-brew-")
            })
            .collect();
        assert_eq!(routes.len(), 5);
        for callback in routes {
            assert!(accepts(callback), "{}", callback.key);
        }
        Ok(())
    }

    #[test]
    fn authored_daily_task_tools_reach_the_fenced_world_owner() -> Result<(), String> {
        let callbacks = world_callbacks::load()?;
        for (prefix, expected) in [("authored-venore-machine-", 9), ("authored-repenters-", 14)] {
            let tools: Vec<_> = callbacks
                .iter()
                .filter(|callback| {
                    callback
                        .key
                        .strip_prefix("oteryn:quest-callback/")
                        .is_some_and(|key| key.starts_with(prefix))
                })
                .collect();
            assert_eq!(tools.len(), expected);
            for callback in tools {
                assert!(callback.requires_world_admission);
                assert!(accepts(callback), "unrouted task tool {}", callback.key);
            }
        }
        Ok(())
    }

    #[test]
    fn explorer_society_selected_tools_reach_the_fenced_world_owner() -> Result<(), String> {
        let callbacks = world_callbacks::load()?;
        let tools: Vec<_> = callbacks
            .iter()
            .filter(|callback| callback.quest == "oteryn:quest.the_explorer_society_quest")
            .filter(|callback| {
                callback
                    .item_on_target
                    .as_ref()
                    .is_some_and(|tool| tool.selected_instance)
            })
            .collect();
        assert_eq!(tools.len(), 11);
        let unrouted: Vec<_> = tools
            .iter()
            .filter(|callback| !callback.requires_world_admission || !accepts(callback))
            .map(|callback| callback.key.as_str())
            .collect();
        assert!(
            unrouted.is_empty(),
            "unrouted Explorer Society tools: {unrouted:?}"
        );
        Ok(())
    }

    #[test]
    fn rafzan_swamp_gas_reaches_the_selected_tool_owner() -> Result<(), String> {
        let callbacks = world_callbacks::load()?;
        let sites = callbacks
            .iter()
            .filter(|callback| {
                callback
                    .key
                    .starts_with("oteryn:quest-callback/rafzan-swamp-gas-")
            })
            .collect::<Vec<_>>();
        assert_eq!(sites.len(), 3);
        for site in sites {
            assert!(accepts(site));
            let tool = site.item_on_target.as_ref().ok_or("gas tool")?;
            assert_eq!(tool.item, "oteryn:item.tibia.i17672");
            assert_eq!(
                tool.replacement.as_deref(),
                Some("oteryn:item.tibia.i17673")
            );
        }
        Ok(())
    }

    #[test]
    fn rafzan_signs_and_trunks_reach_the_selected_tool_owner() -> Result<(), String> {
        let callbacks = world_callbacks::load()?;
        let sites = callbacks
            .iter()
            .filter(|callback| {
                matches!(callback.selector.appearance, Some(17677 | 17679))
                    && callback
                        .key
                        .starts_with("oteryn:quest-callback/questpc7-goblin-")
            })
            .collect::<Vec<_>>();
        assert_eq!(sites.len(), 9);
        for site in sites {
            let tool = site.item_on_target.as_ref().ok_or("site tool")?;
            assert!(tool.selected_instance, "{}", site.key);
            assert!(accepts(site), "{}", site.key);
            assert_eq!(
                tool.reusable,
                tool.item.ends_with("17676") && !site.key.ends_with("-last")
            );
        }
        Ok(())
    }

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
        let retained: Vec<_> = callbacks
            .iter()
            .filter(|callback| {
                oteryn_gameplay_quests::quest::ice_seal_paint::historical_only(&callback.key)
            })
            .collect();
        assert_eq!(retained.len(), 9);
        assert!(retained.iter().all(|callback| !accepts(callback)));
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
    #[test]
    fn authored_top_of_the_city_dig_reaches_the_carried_tool_owner() -> Result<(), String> {
        let bindings =
            oteryn_gameplay_quests::quest::authored_bindings::Bindings::parse(include_bytes!(
                "../../../../../../content/quests/authored-bindings/top_of_the_city_quest.json"
            ))?;
        let dig = bindings
            .world_callbacks()
            .find(|callback| callback.key == TOP_OF_THE_CITY_DIG)
            .ok_or("Top of the City dig callback")?;
        let tool = dig
            .item_on_target
            .as_ref()
            .ok_or("dig requires a carried tool")?;
        assert_eq!(
            tool.item, "oteryn:item.tibia.i3456",
            "the pick is the selected tool"
        );
        assert!(
            accepts(dig),
            "pick-on-dirt must reach the fenced tool owner"
        );
        assert!(tool.selected_instance);
        assert!(is_required_tool(tool, "oteryn:item.tibia.i3456"));
        assert!(
            !is_required_tool(tool, "oteryn:item.tibia.i3457"),
            "a shovel is the wrong tool"
        );
        let fenced_without_pick = ["oteryn:item.tibia.i3457", "oteryn:item.tibia.i3003"];
        assert!(
            !fenced_without_pick
                .iter()
                .any(|key| is_required_tool(tool, key)),
            "a fenced inventory without the pick selects nothing"
        );
        let others: Vec<_> = bindings
            .world_callbacks()
            .filter(|callback| callback.key != TOP_OF_THE_CITY_DIG)
            .collect();
        assert!(!others.is_empty());
        assert!(others.iter().all(|callback| !accepts(callback)));
        let mut bare_use = dig.clone();
        bare_use.item_on_target = None;
        assert!(!accepts(&bare_use), "a bare USE is not a carried-tool use");
        let mut unrelated = dig.clone();
        unrelated.key = "oteryn:quest-callback/authored-top-of-the-city-crate".into();
        assert!(!accepts(&unrelated));
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

    #[test]
    fn tibia_tales_selected_tools_reach_the_world_receipt_owner() -> Result<(), String> {
        let callbacks = world_callbacks::load()?;
        let tools: Vec<_> = callbacks
            .iter()
            .filter(|callback| {
                callback
                    .key
                    .starts_with("oteryn:quest-callback/tibia-tales/")
                    && callback.item_on_target.is_some()
            })
            .collect();
        assert_eq!(tools.len(), 35);
        assert!(tools.into_iter().all(accepts));
        Ok(())
    }
}
