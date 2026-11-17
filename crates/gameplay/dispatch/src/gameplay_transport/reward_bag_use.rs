//! REWARD-CHEST-WIRE-1: carried reward containers use CRAFT-1's fenced consume/mint owner.
//! Outer None delegates an ordinary item; inner None is an unknown outcome.
use super::connection::{UseCommand, UseOutcome};
use super::item_ref_admission::NativeItemWeights;
use super::{ComposedFreshAdmission, item_view::ItemKey};
use crate::durability::item_craft::ItemCraftRequest;
use crate::foundation::{CommandId, CommandRef, ExactActorRef};
use oteryn_gameplay_items::reward_bag::is_reward_bag;
use oteryn_protocol_oteryn::world_object::{UseDisposition, UseIntent, encode_item_use_intent};

/// Existing bags retain their historical instance-tagged receipt.
const INTENT_TAG: &[u8] = b"oteryn:reward-bag-use:v1";

impl ComposedFreshAdmission<'_, '_, '_> {
    /// W19 binds the original client USE bytes before the handle can retire.
    /// This is an adapter to the existing craft owner, including its exact replay.
    pub(super) async fn use_reward_bag_intent(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
        source: Option<ItemKey>,
    ) -> Result<Option<UseOutcome>, ()> {
        let Some(fence) = command.item_fence else {
            return Ok(None);
        };
        let command_ref = CommandRef::new(
            command.game_session_id,
            CommandId::new(command.command_id).map_err(|_| ())?,
        );
        let encoded = encode_item_use_intent(intent).map_err(|_| ())?;
        let historical = self
            .root
            .read_event_container_command_source(self.character, fence, command_ref)
            .await
            .map_err(|_| ())?;
        let id = if let Some(id) = historical {
            id
        } else {
            let Some(ItemKey::Instance(id)) = source else {
                return Ok(None);
            };
            let inventory = match self
                .root
                .read_item_use_inventory(self.character, self.holder, fence)
                .await
            {
                Ok(inventory) => inventory,
                Err(crate::durability::item_use::ItemUseError::AuthorityRejected) => {
                    return Ok(Some(UseOutcome::rejected()));
                }
                Err(_) => return Err(()),
            };
            let carried = inventory
                .backpack
                .iter()
                .flat_map(|bag| &bag.entries)
                .chain(inventory.nested.iter().map(|(_, entry)| entry))
                .find(|entry| entry.item.item_instance_id == id);
            if !carried.is_some_and(|entry| {
                oteryn_gameplay_items::event_items::is_event_reward(
                    &entry.item.definition.production_key,
                )
            }) {
                return Ok(None);
            }
            if intent.use_with.is_some()
                || intent.use_with_map_item.is_some()
                || intent.use_with_position.is_some()
            {
                return Ok(Some(UseOutcome {
                    disposition: UseDisposition::NothingToUse,
                    ..UseOutcome::rejected()
                }));
            }
            id
        };
        match self
            .use_reward_bag_request(actor, command, id, encoded)
            .await
        {
            Some(Some(outcome)) => Ok(Some(outcome)),
            Some(None) => Err(()),
            None => Ok(None),
        }
    }

    pub(super) async fn use_reward_bag(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        item: ItemKey,
    ) -> Option<Option<UseOutcome>> {
        let ItemKey::Instance(id) = item else {
            return None;
        };
        self.use_reward_bag_request(actor, command, id, [INTENT_TAG, id.as_slice()].concat())
            .await
    }

    /// Replay precedes inventory, liveness and trade reads. Fresh writes recheck the
    /// source through the same generation-fenced craft transaction as legacy bags.
    async fn use_reward_bag_request(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        id: [u8; 16],
        intent: Vec<u8>,
    ) -> Option<Option<UseOutcome>> {
        let native = self.active_generation?.native_gameplay()?;
        let fence = command.item_fence?;
        let outcome = |disposition| {
            Some(Some(UseOutcome {
                disposition,
                ..UseOutcome::rejected()
            }))
        };
        let Ok(command_id) = CommandId::new(command.command_id) else {
            return outcome(UseDisposition::Rejected);
        };
        let Some(permit) = self.spell_lane_permit().await else {
            return Some(None);
        };
        let request = ItemCraftRequest {
            command: CommandRef::new(command.game_session_id, command_id),
            source: id,
            tool: None,
            ingredient: None,
            gathering: None,
            enchanting: None,
            station: None,
            intent,
        };
        let content = NativeItemWeights(native).with_wheel(self.premium_coordinator);
        match self
            .root
            .craft_item(
                &permit,
                self.character,
                self.holder,
                fence,
                request.clone(),
                native,
                &content,
                true,
            )
            .await
        {
            Ok(Some(result)) => return outcome(result),
            Ok(None) => {}
            Err(_) => return Some(None),
        }
        let inventory = match self
            .root
            .read_item_use_inventory(self.character, self.holder, fence)
            .await
        {
            Ok(inventory) => inventory,
            Err(_) => return outcome(UseDisposition::Rejected),
        };
        let carried = inventory
            .backpack
            .iter()
            .flat_map(|bag| &bag.entries)
            .chain(inventory.nested.iter().map(|(_, entry)| entry))
            .find(|entry| entry.item.item_instance_id == id)?;
        if !is_reward_bag(&carried.item.definition.production_key) {
            return None;
        }
        {
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            if states.is_dead(actor) || !states.spell_idle(&runtime, actor, command.game_session_id)
            {
                return outcome(UseDisposition::Rejected);
            }
        }
        if !self
            .states
            .player_trades
            .physical(command.game_session_id, Some(id))
        {
            return outcome(UseDisposition::Rejected);
        }
        match self
            .root
            .craft_item(
                &permit,
                self.character,
                self.holder,
                fence,
                request,
                native,
                &content,
                false,
            )
            .await
        {
            Ok(Some(result)) => outcome(result),
            _ => Some(None),
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use crate::durability::item_mint::TypedDefinitionRef;
    use oteryn_gameplay_items::reward_bag::{reward_bag_conversion, reward_bags};

    fn item(key: &str) -> TypedDefinitionRef {
        TypedDefinitionRef {
            family: "Item".into(),
            production_key: key.into(),
            revision_ref: "definition-r1".into(),
        }
    }

    #[test]
    fn primal_bag_is_playable_and_mints_every_pinned_reward() {
        use crate::content::native_gameplay::test_support::{
            activated_with_item_keys, activated_with_loot_tables,
        };
        let world = activated_with_item_keys(None)
            .active()
            .expect("active")
            .identity()
            .world_id();
        let (controller, _) = activated_with_loot_tables(world, None);
        let active = controller.active().expect("active");
        let native = active.native_gameplay().expect("native");
        let bag = reward_bags()
            .expect("overlay")
            .iter()
            .find(|bag| bag.constant == "PRIMAL_BAG")
            .expect("Primal Bag is present");
        assert!(bag.playable);
        assert_eq!(bag.bag, "oteryn:item.tibia.i39546");
        assert_eq!(bag.rewards.len(), 14);
        let mut cumulative = 0;
        for reward in &bag.rewards {
            let plan = reward_bag_conversion(native, &item(&bag.bag), cumulative)
                .expect("Primal Bag reward is mintable in active content");
            assert_eq!(plan.recipe, "oteryn:reward-bag.primal_bag");
            assert_eq!(plan.outputs, vec![(item(&reward.item), 1)]);
            cumulative += reward.weight;
        }
        assert_eq!(cumulative, 54_000);
    }

    #[test]
    fn w19_blue_present_reaches_existing_reward_bag_conversion() {
        use crate::content::native_gameplay::test_support::{
            activated_with_item_keys, activated_with_loot_tables,
        };
        let world = activated_with_item_keys(None)
            .active()
            .expect("active")
            .identity()
            .world_id();
        let (controller, _) = activated_with_loot_tables(world, None);
        let active = controller.active().expect("active");
        let native = active.native_gameplay().expect("native");
        let plan = reward_bag_conversion(native, &item("oteryn:item.tibia.i6570"), 0)
            .expect("W19 blue present is handled by the existing reward-bag owner");
        assert_eq!(plan.recipe, "oteryn:reward-bag.event_6570");
        assert_eq!(plan.outputs.len(), 1);
    }

    #[test]
    fn w19_music_box_mints_soft_boots_with_the_existing_full_active_time_budget() {
        use crate::content::native_gameplay::test_support::{
            activated_with_item_keys, activated_with_loot_tables,
        };
        use crate::domain::timed_item::LaneWriteKind;
        use crate::domain::timed_item_host::{HostedItem, TimedItemDefinition, TimedItemHost};
        let world = activated_with_item_keys(None)
            .active()
            .expect("active")
            .identity()
            .world_id();
        let (controller, _) = activated_with_loot_tables(world, None);
        let native = controller
            .active()
            .expect("active")
            .native_gameplay()
            .expect("native");
        let source = item("oteryn:item.tibia.i12045");
        let boot = (0..4096)
            .find_map(|draw| {
                let plan = reward_bag_conversion(native, &source, draw)?;
                plan.outputs
                    .into_iter()
                    .find(|(definition, count)| {
                        definition.production_key == "oteryn:item.tibia.i6529" && *count == 1
                    })
                    .map(|(definition, _)| definition)
            })
            .expect("the source music-box soft-boots branch is mintable");
        // The Lua table says 3549, but Crystal's Item::CreateItem factory
        // normalizes it to 6529 before delivery. The prior 3549 expectation omitted
        // that production call and would have admitted the wrong custody form.
        assert_eq!(
            super::super::timed_item_content::equipped_definition(native, &boot)
                .expect("paired worn form")
                .production_key,
            "oteryn:item.tibia.i3549"
        );
        let (facts, target) = super::super::timed_item_content::facts(native, &boot)
            .expect("existing authoritative active-time equipment owner");
        assert_eq!(facts.full.remaining_ms(), Some(14_400_000));
        assert_eq!(facts.full.charges(), None);
        assert_eq!(
            target
                .as_ref()
                .map(|target| target.definition.production_key.as_str()),
            Some("oteryn:item.tibia.i6530")
        );
        let definition = TimedItemDefinition {
            definition: boot,
            full: facts.full,
            decay_target: target.map(|target| target.definition),
        };
        // A newly minted item has no timed row. The existing owner treats that
        // state as the admitted full budget; a persisted row resumes its remainder.
        let mut host = TimedItemHost::new(1000).expect("clock");
        host.host_live(
            vec![HostedItem {
                item_instance_id: [7; 16],
                definition: definition.clone(),
                row: None,
            }],
            0,
        )
        .expect("first equip");
        host.advance(1000);
        let writes = host.due_writes(1000, || [8; 16], |_| None);
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0].write.expected_revision, 0);
        let values = match writes[0].write.kind {
            LaneWriteKind::Checkpoint { values } => Some(values),
            _ => None,
        }
        .expect("first timed write must store the remainder");
        assert_eq!(values.remaining_ms(), Some(14_399_000));
        let mut restarted = TimedItemHost::new(1000).expect("restart clock");
        restarted
            .host_live(
                vec![HostedItem {
                    item_instance_id: [7; 16],
                    definition,
                    row: Some((1, Some(values))),
                }],
                1000,
            )
            .expect("reequip");
        restarted.advance(1000);
        let resumed = restarted.due_writes(2000, || [9; 16], |_| None);
        assert_eq!(resumed.len(), 1);
        assert_eq!(resumed[0].write.expected_revision, 1);
        assert!(
            matches!(resumed[0].write.kind, LaneWriteKind::Checkpoint { values }
            if values.remaining_ms() == Some(14_398_000))
        );
    }

    #[test]
    fn event_catalogue_resolves_captured_selection_shapes_and_quantities() {
        use crate::content::native_gameplay::test_support::{
            activated_with_item_keys, activated_with_loot_tables,
        };
        let world = activated_with_item_keys(None)
            .active()
            .expect("active")
            .identity()
            .world_id();
        let (controller, _) = activated_with_loot_tables(world, None);
        let active = controller.active().expect("active");
        let native = active.native_gameplay().expect("native");
        let source: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../tools/content-schema/native-gameplay/event-item-actions.json"
        ))
        .expect("captured actions");
        for rule in source["rules"].as_array().expect("rules") {
            let key = format!("oteryn:item.tibia.i{}", rule["item_id"]);
            let rewards = rule["rewards"].as_array().expect("rewards");
            let mut reached = std::collections::BTreeSet::new();
            for draw in 0..512 {
                let plan = reward_bag_conversion(native, &item(&key), draw)
                    .expect("captured outcome resolves in active content");
                // W19 follows Item::CreateItem and splits nonstackable quantities
                // into individual instances. Compare captured quantities per gift,
                // and independently require each physical instance to be legal.
                let mut outputs = std::collections::BTreeMap::<u64, u32>::new();
                for (definition, count) in &plan.outputs {
                    let id: u32 = definition
                        .production_key
                        .strip_prefix("oteryn:item.tibia.i")
                        .expect("key")
                        .parse()
                        .expect("id");
                    let policy = native.item_policy_for_source_id(id).expect("output policy");
                    if policy.record().admission.stack_class
                        == crate::content::native_gameplay::NativeItemStackClass::NonStackable
                    {
                        assert_eq!(*count, 1, "{key}: {id}");
                    }
                    *outputs.entry(u64::from(id)).or_default() += count;
                }
                let count = match rule["selection"].as_str().expect("selection") {
                    "all" => rewards.len(),
                    "without_replacement" => 7,
                    _ => 1,
                };
                assert_eq!(outputs.len(), count, "{key}");
                let mut unique = std::collections::BTreeSet::new();
                for (&id, count) in &outputs {
                    assert!(
                        rewards.iter().any(|reward| {
                            // Crystal's factory grants base soft boots (6529)
                            // for the captured activated form (3549).
                            let expected = match reward["item_id"].as_u64().expect("reward id") {
                                3549 => 6529,
                                id => id,
                            };
                            expected == id
                                && reward["minimum"].as_u64().expect("minimum") <= u64::from(*count)
                                && u64::from(*count) <= reward["maximum"].as_u64().expect("maximum")
                        }),
                        "{key}: {id} x {count}"
                    );
                    unique.insert(id);
                    reached.insert(id);
                }
                if rule["selection"] == "without_replacement" {
                    assert_eq!(unique.len(), 7);
                }
            }
            assert!(!reached.is_empty());
        }
    }

    #[test]
    fn active_content_mints_every_legacy_reward_bag_output() {
        use crate::content::native_gameplay::test_support::{
            activated_with_item_keys, activated_with_loot_tables,
        };
        let world = activated_with_item_keys(None)
            .active()
            .expect("active")
            .identity()
            .world_id();
        let (controller, _) = activated_with_loot_tables(world, None);
        let active = controller.active().expect("active");
        let native = active.native_gameplay().expect("native");
        let bags = reward_bags().expect("overlay");
        // The 27 legacy bags remain intact; QENGINE2 adds the Gnomish supply
        // package from Crystal random_items.lua:49-80 (9f5a72c64).
        assert_eq!(bags.iter().filter(|b| b.playable).count(), 28);
        assert_eq!(
            bags.iter()
                .filter(|b| b.playable && b.constant != "QENGINE_GNOMISH_SUPPLY")
                .count(),
            27
        );
        for bag in bags.iter().filter(|b| {
            b.playable
                && !b.constant.starts_with("EVENT_")
                && b.constant != "QENGINE_GNOMISH_SUPPLY"
        }) {
            let mut reached = std::collections::BTreeSet::new();
            let mut cumulative = 0;
            for reward in &bag.rewards {
                let plan = reward_bag_conversion(native, &item(&bag.bag), cumulative)
                    .expect("admitted reward");
                assert_eq!(plan.outputs.len(), 1);
                assert_eq!(
                    (plan.outputs[0].0.production_key.as_str(), plan.outputs[0].1),
                    (reward.item.as_str(), 1)
                );
                reached.insert(reward.item.as_str());
                cumulative += reward.weight;
            }
            assert_eq!(reached.len(), bag.rewards.len());
        }
        // The new supply package grants source quantities, rather than the
        // legacy one-item outcomes. Exercise every admitted outcome exactly.
        let supply = bags
            .iter()
            .find(|bag| bag.constant == "QENGINE_GNOMISH_SUPPLY")
            .expect("Gnomish supply package");
        assert!(supply.playable);
        assert_eq!(supply.bag, "oteryn:item.tibia.i15698");
        let expected: std::collections::BTreeMap<_, _> = [
            (3723, 20),
            (16103, 1),
            (16143, 15),
            (15793, 15),
            (16167, 1),
            (236, 2),
            (237, 2),
            (266, 4),
            (268, 4),
            (238, 1),
            (239, 1),
            (7443, 1),
            (7439, 1),
            (3035, 5),
            (5911, 1),
            (7440, 1),
            (16165, 1),
            (16257, 1),
            (16254, 1),
            (3043, 1),
            (3039, 1),
            (16242, 1),
            (3037, 1),
            (3041, 1),
            (3038, 1),
        ]
        .into_iter()
        .map(|(id, count)| (format!("oteryn:item.tibia.i{id}"), count))
        .collect();
        assert_eq!(supply.rewards.len(), 25);
        assert_eq!(supply.rewards.iter().map(|r| r.weight).sum::<u64>(), 10001);
        let mut reached = std::collections::BTreeSet::new();
        for draw in 0..100_000 {
            let plan = reward_bag_conversion(native, &item(&supply.bag), draw)
                .expect("source-bound Gnomish supply outcome");
            assert_eq!(plan.outputs.len(), 1);
            let (output, count) = &plan.outputs[0];
            assert_eq!(Some(count), expected.get(&output.production_key));
            reached.insert(output.production_key.clone());
            if reached.len() == expected.len() {
                break;
            }
        }
        assert_eq!(reached, expected.into_keys().collect());
        for bag in bags.iter().filter(|b| !b.playable) {
            assert!(reward_bag_conversion(native, &item(&bag.bag), 0).is_none());
        }
        assert!(reward_bag_conversion(native, &item("oteryn:item.tibia.i3600"), 0).is_none());
    }
}
