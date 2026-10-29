//! Native USE of a carried coin. The retained source distinguishes this from NPC exchange.
use super::connection::{UseCommand, UseOutcome};
use super::item_ref_admission::NativeItemWeights;
use super::{ComposedFreshAdmission, item_view::ItemKey};
use crate::domain::currency::Coin;
use crate::durability::coin_exchange::{CoinConverter, CoinExchangeRequest};
use crate::foundation::{CommandId, CommandRef, ExactActorRef};
use oteryn_gameplay_economy::coin_exchange::CoinExchange;
use oteryn_protocol_oteryn::world_object::UseDisposition;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn use_gold_converter(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        converter: ItemKey,
        coin_item: ItemKey,
    ) -> Option<UseOutcome> {
        let (ItemKey::Instance(converter), ItemKey::Instance(coin_item)) = (converter, coin_item)
        else {
            return None;
        };
        let fence = command.item_fence?;
        let native = self.active_generation?.native_gameplay()?;
        let inventory = self
            .root
            .read_item_use_inventory(self.character, self.holder, fence)
            .await
            .ok()?;
        let entries = inventory
            .backpack
            .iter()
            .flat_map(|bag| &bag.entries)
            .chain(inventory.nested.iter().map(|(_, e)| e));
        let mut converter_key = None;
        let mut target = None;
        for entry in entries {
            if entry.item.item_instance_id == converter {
                converter_key = Some(entry.item.definition.production_key.as_str());
            }
            if entry.item.item_instance_id == coin_item {
                target = Some((
                    Coin::from_production_key(&entry.item.definition.production_key)?,
                    entry.item.quantity,
                ));
            }
        }
        let tool = match converter_key? {
            "oteryn:item.tibia.i23722" => CoinConverter::Gold(converter),
            "oteryn:item.tibia.i28526" => CoinConverter::Magic(converter),
            _ => return None,
        };
        let (coin, quantity) = target?;
        let exchange = match (coin, quantity, tool) {
            (Coin::Gold, 100, _) => CoinExchange::GoldToPlatinum,
            (Coin::Platinum, 100, _) => CoinExchange::PlatinumToCrystal,
            (Coin::Platinum, _, CoinConverter::Gold(_)) if quantity > 0 => {
                CoinExchange::PlatinumToGold
            }
            (Coin::Crystal, _, CoinConverter::Gold(_)) if quantity > 0 => {
                CoinExchange::CrystalToPlatinum
            }
            _ => {
                return Some(UseOutcome {
                    disposition: UseDisposition::NotPossible,
                    ..UseOutcome::rejected()
                });
            }
        };
        let runtime = self.runtime.lock().await;
        {
            let states = self.states.spell_states.lock().await;
            if states.is_dead(actor) || !states.spell_idle(&runtime, actor, command.game_session_id)
            {
                return Some(UseOutcome::rejected());
            }
        }
        let permit = self.spell_lane_permit().await?;
        let content = NativeItemWeights(native);
        let coins =
            super::npc_command::bank_coin_definitions(self, &runtime).map(|(a, b, c)| [a, b, c]);
        let request = CoinExchangeRequest {
            exchange,
            amount: 1,
            npc_identity: coin_item,
            npc_generation: 1,
            npc_key: "item-use".into(),
            catalogue_revision: 1,
            conversation_revision: 1,
        };
        let reference = CommandRef::new(
            command.game_session_id,
            CommandId::new(command.command_id).ok()?,
        );
        match self
            .root
            .exchange_carried_coins(
                &permit,
                self.character,
                self.holder,
                fence,
                reference,
                Some(request),
                Some(coin_item),
                Some(&content),
                coins,
                Some(tool),
            )
            .await
        {
            Ok(Some(receipt)) => Some(UseOutcome {
                disposition: if receipt.changed {
                    UseDisposition::Committed
                } else {
                    UseDisposition::NotPossible
                },
                ..UseOutcome::rejected()
            }),
            _ => None,
        }
    }

    pub(super) async fn use_coin(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        item: ItemKey,
    ) -> Option<Option<UseOutcome>> {
        let ItemKey::Instance(id) = item else {
            return None;
        };
        let fence = command.item_fence?;
        let native = self.active_generation?.native_gameplay()?;
        let command_id = CommandId::new(command.command_id).ok()?;
        let reference = CommandRef::new(command.game_session_id, command_id);
        let permit = self.spell_lane_permit().await?;
        let content = NativeItemWeights(native);
        let outcome = |changed| {
            Some(Some(UseOutcome {
                disposition: if changed {
                    UseDisposition::Committed
                } else {
                    UseDisposition::NotPossible
                },
                ..UseOutcome::rejected()
            }))
        };
        let coins = {
            let runtime = self.runtime.lock().await;
            super::npc_command::bank_coin_definitions(self, &runtime)
                .map(|(gold, platinum, crystal)| [gold, platinum, crystal])
        };
        // Read the retained command before inspecting the possibly retired input stack.
        match self
            .root
            .exchange_carried_coins(
                &permit,
                self.character,
                self.holder,
                fence,
                reference,
                None,
                Some(id),
                Some(&content),
                coins,
                None,
            )
            .await
        {
            Ok(Some(receipt)) => return outcome(receipt.changed),
            Ok(None) => {}
            Err(_) => return Some(None),
        }
        // A carried non-coin is not ours, whatever its fence; its own owner refuses it.
        if let Ok(Some(backpack)) = self
            .root
            .read_character_backpack(self.character, fence.character_id)
            .await
            && backpack.entries.iter().any(|entry| {
                entry.item.item_instance_id == id
                    && Coin::from_production_key(&entry.item.definition.production_key).is_none()
            })
        {
            return None;
        }
        let inventory = match self
            .root
            .read_item_use_inventory(self.character, self.holder, fence)
            .await
        {
            Ok(inventory) => inventory,
            Err(_) => return Some(None),
        };
        let entry = inventory
            .backpack
            .iter()
            .flat_map(|bag| &bag.entries)
            .chain(inventory.nested.iter().map(|(_, entry)| entry))
            .find(|e| e.item.item_instance_id == id)?;
        let coin = Coin::from_production_key(&entry.item.definition.production_key)?;
        let exchange = match (coin, entry.item.quantity) {
            (Coin::Gold, _) => CoinExchange::GoldToPlatinum,
            (Coin::Platinum, 100) => CoinExchange::PlatinumToCrystal,
            (Coin::Platinum, _) => CoinExchange::PlatinumToGold,
            (Coin::Crystal, _) => CoinExchange::CrystalToPlatinum,
        };
        let runtime = self.runtime.lock().await;
        {
            let states = self.states.spell_states.lock().await;
            if states.is_dead(actor) || !states.spell_idle(&runtime, actor, command.game_session_id)
            {
                return outcome(false);
            }
        }
        if !self
            .states
            .player_trades
            .physical(command.game_session_id, Some(id))
        {
            return outcome(false);
        }
        let coins = super::npc_command::bank_coin_definitions(self, &runtime)
            .map(|(gold, platinum, crystal)| [gold, platinum, crystal]);
        // These legacy columns are inert for item origin; 0336 constrains their sentinel shape.
        let request = CoinExchangeRequest {
            exchange,
            amount: 1,
            npc_identity: id,
            npc_generation: 1,
            npc_key: "item-use".into(),
            catalogue_revision: 1,
            conversation_revision: 1,
        };
        match self
            .root
            .exchange_carried_coins(
                &permit,
                self.character,
                self.holder,
                fence,
                reference,
                Some(request),
                Some(id),
                Some(&content),
                coins,
                None,
            )
            .await
        {
            Ok(Some(receipt)) => outcome(receipt.changed),
            _ => Some(None),
        }
    }
}
