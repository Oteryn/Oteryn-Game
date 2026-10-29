//! Native USE of a carried coin. The retained source distinguishes this from NPC exchange.
use super::connection::{UseCommand, UseOutcome};
use super::item_ref_admission::NativeItemWeights;
use super::{ComposedFreshAdmission, item_view::ItemKey};
use crate::domain::currency::Coin;
use crate::domain::timed_item::TimedValues;
use crate::durability::coin_exchange::{
    CoinConverter, CoinExchangeRequest, MAGIC_CONVERTER_ACTIVE, MAGIC_CONVERTER_INACTIVE,
};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_timed_state::TimedDefinitionFacts;
use crate::durability::item_use::ItemUseInventory;
use crate::foundation::{CommandId, CommandRef, ExactActorRef, GameSessionId};
use oteryn_gameplay_economy::coin_exchange::CoinExchange;
use oteryn_protocol_oteryn::world_object::UseDisposition;
use sha2::{Digest, Sha256};

const GOLD_CONVERTER: &str = "oteryn:item.tibia.i23722";
const TEMPORARY_GOLD_CONVERTER: &str = "oteryn:item.tibia.i25719";
const GOLD_COIN: &str = "oteryn:item.tibia.i3031";
const PLATINUM_COIN: &str = "oteryn:item.tibia.i3035";

/// Fingerprint of the carried tree; a refused upgrade is retried only after it changes.
fn inventory_revision(inventory: &ItemUseInventory) -> [u8; 16] {
    let mut hash = Sha256::new();
    let main = inventory.backpack.iter().flat_map(|bag| &bag.entries);
    for entry in main.chain(inventory.nested.iter().map(|(_, entry)| entry)) {
        hash.update(entry.item.item_instance_id);
        hash.update(entry.item.quantity.to_be_bytes());
    }
    let digest: [u8; 32] = hash.finalize().into();
    let mut key = [0; 16];
    key.copy_from_slice(&digest[..16]);
    key
}

/// Canary items.xml gives every coin converter form charges="500"; the content definitions
/// and the admitted native profiles record the same count. The converters carry no temporal
/// or transform semantics, so their charge-only facts are formed here and the durable owner
/// pins them in the timed definitions on first write.
const CONVERTER_CHARGES: u32 = 500;

fn converter_facts(definition: &TypedDefinitionRef) -> Option<TimedDefinitionFacts> {
    let converter = matches!(
        definition.production_key.as_str(),
        GOLD_CONVERTER
            | TEMPORARY_GOLD_CONVERTER
            | MAGIC_CONVERTER_INACTIVE
            | MAGIC_CONVERTER_ACTIVE
    );
    (converter && definition.family == "Item").then_some(())?;
    Some(TimedDefinitionFacts {
        definition: definition.clone(),
        full: TimedValues::new(Some(CONVERTER_CHARGES), None).ok()?,
        lit_continuous: false,
    })
}

fn not_possible() -> UseOutcome {
    UseOutcome {
        disposition: UseDisposition::NotPossible,
        ..UseOutcome::rejected()
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Plain use of a coin converter. A gold converter needs a coin target and spends
    /// nothing here; a magic converter switches on or off (magic_gold_converter.lua L10-21).
    pub(super) async fn use_converter_item(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        item: ItemKey,
    ) -> Option<Option<UseOutcome>> {
        let ItemKey::Instance(id) = item else {
            return None;
        };
        let fence = command.item_fence?;
        let backpack = self
            .root
            .read_character_backpack(self.character, fence.character_id)
            .await
            .ok()??;
        let entry = backpack
            .entries
            .iter()
            .find(|entry| entry.item.item_instance_id == id)?;
        match entry.item.definition.production_key.as_str() {
            GOLD_CONVERTER | TEMPORARY_GOLD_CONVERTER => Some(Some(not_possible())),
            MAGIC_CONVERTER_INACTIVE | MAGIC_CONVERTER_ACTIVE => {
                Some(self.toggle_magic_converter(actor, command, id).await)
            }
            _ => None,
        }
    }

    async fn toggle_magic_converter(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        converter: [u8; 16],
    ) -> Option<UseOutcome> {
        let fence = command.item_fence?;
        let native = self.active_generation?.native_gameplay()?;
        let index = native.item_index()?;
        let pinned = |key: &str| {
            let definition = TypedDefinitionRef {
                family: "Item".into(),
                production_key: key.into(),
                revision_ref: index.revision(key)?.into(),
            };
            converter_facts(&definition)
        };
        let (inactive, active) = (
            pinned(MAGIC_CONVERTER_INACTIVE)?,
            pinned(MAGIC_CONVERTER_ACTIVE)?,
        );
        {
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            if states.is_dead(actor) || !states.spell_idle(&runtime, actor, command.game_session_id)
            {
                return Some(UseOutcome::rejected());
            }
        }
        if !self
            .states
            .player_trades
            .physical(command.game_session_id, Some(converter))
        {
            return Some(UseOutcome::rejected());
        }
        let permit = self.spell_lane_permit().await?;
        let transaction = super::npc_trade_command::derived(
            b"oteryn:magic-converter-toggle:v1",
            command.game_session_id,
            command.command_id,
        );
        match self
            .root
            .toggle_magic_converter(
                &permit,
                self.character,
                self.holder,
                fence,
                transaction,
                converter,
                &inactive,
                &active,
            )
            .await
        {
            Ok(Some(on)) => {
                let mut active = self.states.timed_items.magic_converters.lock().await;
                if on {
                    active.entry(command.game_session_id).or_insert_with(|| {
                        super::timed_item::MagicConverterClock {
                            nonce: transaction,
                            serial: 0,
                            refused: None,
                        }
                    });
                } else {
                    active.remove(&command.game_session_id);
                }
                Some(UseOutcome {
                    disposition: UseDisposition::Committed,
                    ..UseOutcome::rejected()
                })
            }
            Ok(None) => Some(not_possible()),
            Err(_) => None,
        }
    }

    /// Owner tick for one switched-on session: upgrade the first carried 100-stack of gold or
    /// platinum and spend one charge (magic_gold_converter.lua L23-55). The caller holds the
    /// spell lane. Returns false when the session no longer carries an active converter.
    pub(super) async fn tick_magic_converter(&self, session: GameSessionId) -> bool {
        let Ok(Some(fence)) = self.current_quest_fence(session).await else {
            return true;
        };
        let Some(native) = self.active_generation.and_then(|a| a.native_gameplay()) else {
            return true;
        };
        let fence = fence.item_fence();
        let Ok(inventory) = self
            .root
            .read_item_use_inventory(self.character, self.holder, fence)
            .await
        else {
            return true;
        };
        let main = inventory.backpack.iter().flat_map(|bag| &bag.entries);
        let Some(converter) = main
            .clone()
            .find(|entry| entry.item.definition.production_key == MAGIC_CONVERTER_ACTIVE)
        else {
            return false;
        };
        let Some(facts) = converter_facts(&converter.item.definition) else {
            return false;
        };
        let source = main
            .chain(inventory.nested.iter().map(|(_, entry)| entry))
            .find_map(|entry| {
                let exchange = match entry.item.definition.production_key.as_str() {
                    GOLD_COIN => CoinExchange::GoldToPlatinum,
                    PLATINUM_COIN => CoinExchange::PlatinumToCrystal,
                    _ => return None,
                };
                (entry.item.quantity == 100).then_some((entry.item.item_instance_id, exchange))
            });
        let Some((source, exchange)) = source else {
            return true;
        };
        let (nonce, serial) = {
            let mut clocks = self.states.timed_items.magic_converters.lock().await;
            let Some(clock) = clocks.get_mut(&session) else {
                return true;
            };
            if clock.refused == Some((source, inventory_revision(&inventory))) {
                return true;
            }
            (clock.nonce, clock.serial)
        };
        let mut hash = Sha256::new();
        hash.update(b"oteryn:magic-converter-clock:v1");
        hash.update(session.as_bytes());
        hash.update(self.holder.fact().registration_revision().to_be_bytes());
        hash.update(fence.connection_generation.get().to_be_bytes());
        hash.update(nonce);
        hash.update(serial.to_be_bytes());
        let digest: [u8; 32] = hash.finalize().into();
        let mut raw = [0; 8];
        raw.copy_from_slice(&digest[..8]);
        // The high bit keeps owner ticks apart from client command sequences.
        let Ok(command_id) = CommandId::new(u64::from_be_bytes(raw) | 1 << 63) else {
            return true;
        };
        let coins = {
            let runtime = self.runtime.lock().await;
            super::npc_command::bank_coin_definitions(self, &runtime).map(|(a, b, c)| [a, b, c])
        };
        let Some(permit) = self.spell_lane_permit().await else {
            return true;
        };
        let request = CoinExchangeRequest {
            exchange,
            amount: 1,
            npc_identity: source,
            npc_generation: 1,
            npc_key: "item-use".into(),
            catalogue_revision: 1,
            conversation_revision: 1,
        };
        let result = self
            .root
            .exchange_carried_coins(
                &permit,
                self.character,
                self.holder,
                fence,
                CommandRef::new(session, command_id),
                Some(request),
                Some(source),
                Some(&NativeItemWeights(native)),
                coins,
                Some((CoinConverter::Magic(converter.item.item_instance_id), facts)),
            )
            .await;
        // An ambiguous result keeps its serial, so the next tick resumes the same receipt.
        if let Ok(Some(receipt)) = result {
            let mut clocks = self.states.timed_items.magic_converters.lock().await;
            if let Some(clock) = clocks.get_mut(&session) {
                clock.serial = clock.serial.wrapping_add(1);
                clock.refused =
                    (!receipt.changed).then_some((source, inventory_revision(&inventory)));
            }
        }
        true
    }
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
        let mut converter_definition = None;
        let mut target = None;
        for entry in entries {
            if entry.item.item_instance_id == converter {
                converter_definition = Some(entry.item.definition.clone());
            }
            if entry.item.item_instance_id == coin_item {
                target = Some((
                    Coin::from_production_key(&entry.item.definition.production_key)?,
                    entry.item.quantity,
                ));
            }
        }
        let magic = matches!(
            converter_definition
                .as_ref()
                .map(|d| d.production_key.as_str()),
            Some(MAGIC_CONVERTER_INACTIVE | MAGIC_CONVERTER_ACTIVE)
        );
        if !magic {
            // Read the retained command before inspecting the possibly retired coin or
            // (spent temporary) converter; facts are read only by a still-pending receipt.
            let definition = match converter_definition.clone() {
                Some(definition) => definition,
                None => TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: GOLD_CONVERTER.into(),
                    revision_ref: native.item_index()?.revision(GOLD_CONVERTER)?.into(),
                },
            };
            let facts = converter_facts(&definition)?;
            let permit = self.spell_lane_permit().await?;
            let coins = {
                let runtime = self.runtime.lock().await;
                super::npc_command::bank_coin_definitions(self, &runtime).map(|(a, b, c)| [a, b, c])
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
                    None,
                    Some(coin_item),
                    Some(&NativeItemWeights(native)),
                    coins,
                    Some((CoinConverter::Gold(converter), facts)),
                )
                .await
            {
                Ok(Some(receipt)) => {
                    return Some(UseOutcome {
                        disposition: if receipt.changed {
                            UseDisposition::Committed
                        } else {
                            UseDisposition::NotPossible
                        },
                        ..UseOutcome::rejected()
                    });
                }
                Ok(None) => {}
                Err(_) => return None,
            }
        }
        let converter_definition = converter_definition?;
        // Canary magic_gold_converter.lua L10-21: using it on anything only switches it.
        let tool = match converter_definition.production_key.as_str() {
            GOLD_CONVERTER | TEMPORARY_GOLD_CONVERTER => CoinConverter::Gold(converter),
            MAGIC_CONVERTER_INACTIVE | MAGIC_CONVERTER_ACTIVE => {
                return self.toggle_magic_converter(actor, command, converter).await;
            }
            _ => return None,
        };
        let facts = converter_facts(&converter_definition)?;
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
                Some((tool, facts)),
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

#[cfg(test)]
mod tests {
    use super::*;
    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const KEYS: [&str; 4] = [
        GOLD_CONVERTER,
        TEMPORARY_GOLD_CONVERTER,
        MAGIC_CONVERTER_INACTIVE,
        MAGIC_CONVERTER_ACTIVE,
    ];

    fn charges(semantics: &serde_json::Value) -> Option<u64> {
        semantics["charges"]["value"]["count"]["value"].as_u64()
    }

    #[test]
    fn converter_facts_match_content_charges() -> TestResult {
        let root = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/items/definitions"
        );
        let mut seen = 0;
        for path in std::fs::read_dir(root)? {
            let path = path?.path();
            let name = path.file_name().ok_or("name")?.to_string_lossy();
            if !name.starts_with("items-") {
                continue;
            }
            let document: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
            for record in document["records"].as_array().ok_or("records")? {
                let definition = &record["definition"];
                let key = definition["identity"]["key"].as_str().ok_or("key")?;
                if !KEYS.contains(&key) {
                    continue;
                }
                seen += 1;
                assert_eq!(
                    charges(&definition["semantics"]),
                    Some(u64::from(CONVERTER_CHARGES)),
                    "{key}"
                );
                let typed = TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: key.into(),
                    revision_ref: definition["identity"]["revision"]
                        .as_str()
                        .ok_or("revision")?
                        .into(),
                };
                let facts = converter_facts(&typed).ok_or("facts")?;
                assert_eq!(facts.full.charges(), Some(CONVERTER_CHARGES));
                assert_eq!(facts.full.remaining_ms(), None);
            }
        }
        assert_eq!(seen, KEYS.len());
        // The admitted native profiles carry the same budget the carried planner reads.
        let native: serde_json::Value = serde_json::from_slice(&std::fs::read(format!(
            "{root}/spell-native-profiles.json"
        ))?)?;
        let mut profiled = 0;
        for record in native["records"].as_array().ok_or("records")? {
            let key = record["production_definition"]["production_key"]
                .as_str()
                .ok_or("key")?;
            if KEYS.contains(&key) {
                profiled += 1;
                assert_eq!(
                    charges(&record["semantics"]),
                    Some(u64::from(CONVERTER_CHARGES)),
                    "{key}"
                );
            }
        }
        assert_eq!(profiled, KEYS.len());
        let coin = TypedDefinitionRef {
            family: "Item".into(),
            production_key: GOLD_COIN.into(),
            revision_ref: "definition-r1".into(),
        };
        assert!(converter_facts(&coin).is_none());
        Ok(())
    }
}
