//! REWARD-CHEST-WIRE-1: USE on a carried reward bag → CRAFT-1's fenced consume/mint owner.
//! Outer None delegates an ordinary item; inner None is an unknown outcome.
use super::connection::{UseCommand, UseOutcome};
use super::item_ref_admission::NativeItemWeights;
use super::{ComposedFreshAdmission, item_view::ItemKey};
use crate::durability::item_craft::ItemCraftRequest;
use crate::foundation::{CommandId, CommandRef, ExactActorRef};
use oteryn_gameplay_items::reward_bag::reward_bags;
use oteryn_protocol_oteryn::world_object::UseDisposition;

/// The receipt binds the command to its source instance, which outlives view handles.
const INTENT_TAG: &[u8] = b"oteryn:reward-bag-use:v1";

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn use_reward_bag(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        item: ItemKey,
    ) -> Option<Option<UseOutcome>> {
        let ItemKey::Instance(id) = item else {
            return None;
        };
        let native = self.active_generation?.native_gameplay()?;
        let fence = command.item_fence?;
        let backpack = self
            .root
            .read_character_backpack(self.character, fence.character_id)
            .await
            .ok()??;
        let carried = backpack
            .entries
            .iter()
            .find(|entry| entry.item.item_instance_id == id);
        // A carried non-bag is not ours; a missing source may be this command's consumed bag.
        if carried.is_some_and(|entry| {
            !reward_bags().is_some_and(|bags| {
                bags.iter()
                    .any(|bag| bag.playable && bag.bag == entry.item.definition.production_key)
            })
        }) {
            return None;
        }
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
            station: None,
            intent: [INTENT_TAG, id.as_slice()].concat(),
        };
        let content = NativeItemWeights(native);
        // Replay of an exact receipt precedes every live check, including the consumed bag.
        let replay = self
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
            .await;
        match replay {
            Ok(Some(result)) => return outcome(result),
            Ok(None) if carried.is_none() => return None,
            Ok(None) => {}
            Err(_) => return Some(None),
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
    fn active_content_mints_every_covet_reward_and_closes_other_bags() {
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
        let covet = bags.iter().find(|b| b.playable).expect("playable bag");
        let mut reached = std::collections::BTreeSet::new();
        let mut cumulative = 0;
        for reward in &covet.rewards {
            let plan = reward_bag_conversion(native, &item(&covet.bag), cumulative)
                .expect("admitted reward");
            assert_eq!(plan.outputs.len(), 1);
            assert_eq!(
                (plan.outputs[0].0.production_key.as_str(), plan.outputs[0].1),
                (reward.item.as_str(), 1)
            );
            reached.insert(reward.item.as_str());
            cumulative += reward.weight;
        }
        assert_eq!(reached.len(), covet.rewards.len());
        for bag in bags.iter().filter(|b| !b.playable) {
            assert!(reward_bag_conversion(native, &item(&bag.bag), 0).is_none());
        }
        assert!(reward_bag_conversion(native, &item("oteryn:item.tibia.i3600"), 0).is_none());
    }
}
