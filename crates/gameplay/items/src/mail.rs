//! MAIL-0 §§3–8 pure posting rules, independent of mailbox or database authority.
//!
//! BAGS-DEPOT-1's Inbox trees and MAIL-CONTENT-1's qualified stamp/mailbox
//! bindings are absent on main. This is a checked value projection only: a
//! future owner must resolve complete current facts under its native locks,
//! re-plan, and atomically TRANSFER + PRESERVE_INSTANCE TRANSFORM with audit.
//! It must not mint replacement items or treat this plan as permission to write.

use crate::item_transfer::{GAMEITEM01_CONTAINER_ENTRIES_MAX, InventoryItem, stack_maximum};
use oteryn_gameplay_domain::domain::character_name::CharacterName;
use oteryn_gameplay_domain::domain::item_definition::{ItemDefinitionFacts, ItemStackClass};
use oteryn_gameplay_domain::domain::mail::{
    MailAddressRefusal, MailText, MailTextClass, MailTextRefusal,
};
use std::collections::{BTreeMap, BTreeSet};

/// Canary items.xml i3503/i3504; MAIL0-RL-03 retains this direct capacity.
pub const PARCEL_DIRECT_CHILDREN_MAX: usize = 10;
/// MAIL0-RL-03 / BAGS-0 §3: root at backpack depth 1, descendants at most 8.
pub const PARCEL_HEIGHT_MAX: usize = 7;
/// MAIL0-RL-03: 500 carried items, minus the main backpack itself.
pub const PARCEL_ITEMS_MAX: usize = 499;
/// MAIL0-RL-05, an intentional Oteryn limit, counting all parcel descendants.
pub const MAIL_INBOX_ITEMS_MAX: u64 = 50_000;

pub const PARCEL_KEY: &str = "oteryn:item.tibia.i3503";
pub const STAMPED_PARCEL_KEY: &str = "oteryn:item.tibia.i3504";
pub const LETTER_KEY: &str = "oteryn:item.tibia.i3505";
pub const STAMPED_LETTER_KEY: &str = "oteryn:item.tibia.i3506";
pub const LABEL_KEY: &str = "oteryn:item.tibia.i3507";

/// One complete source-tree member. Root has no parent; every ordinal is
/// positive and unique within its parent. Text is already privacy-redacted.
#[derive(Debug, Clone, Copy)]
pub struct MailTreeItem<'a> {
    pub item: &'a InventoryItem,
    pub facts: &'a ItemDefinitionFacts,
    pub parent: Option<[u8; 16]>,
    pub ordinal: u64,
    pub text: Option<&'a MailText>,
}

/// Expected data, never proof of custody, junior status, Content or capacity.
/// The source must be a direct main-backpack entry; the durable owner must
/// independently establish that fact and the absence of an offered trade.
#[derive(Debug, Clone, Copy)]
pub struct MailPostingInput<'a> {
    pub root: [u8; 16],
    pub items: &'a [MailTreeItem<'a>],
    pub root_has_store_owner: bool,
    pub sender_junior: bool,
    pub recipient_junior: bool,
    pub inbox_items: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MailRuleRefusal {
    NotMailable,
    NoAddress,
    UnknownRecipient,
    RecipientCannotReceiveParcels,
    RecipientInboxFull,
    MalformedTree,
    DefinitionMismatch,
    InvalidQuantity,
    WeightUnknown,
    ArithmeticOverflow,
    InvalidText(MailTextRefusal),
}

/// Immutable expected outcome DATA. Town is deliberately absent: MAIL-0 §5
/// and Canary Mailbox::getReceiver ignore all lines after the recipient.
/// The stamp key names the expected target, not an admitted content revision.
/// Weight includes each item/stack once; there is no separate postal weight cap
/// (Canary Mailbox::canSend/queryAdd). Item custody and quantities are unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MailPostingPlan {
    pub recipient: CharacterName,
    pub root: [u8; 16],
    pub stamped_production_key: &'static str,
    pub sorted_item_ids: Vec<[u8; 16]>,
    pub weight_centi_oz: u64,
    pub inbox_items_after: u64,
}

impl TryFrom<MailPostingInput<'_>> for MailPostingPlan {
    type Error = MailRuleRefusal;

    fn try_from(input: MailPostingInput<'_>) -> Result<Self, Self::Error> {
        use MailRuleRefusal as E;
        if input.items.is_empty() || input.items.len() > PARCEL_ITEMS_MAX {
            return Err(E::MalformedTree);
        }
        let mut nodes = BTreeMap::new();
        let mut slots = BTreeSet::new();
        let mut children = BTreeMap::<[u8; 16], usize>::new();
        let mut weight_centi_oz = 0_u64;
        for node in input.items {
            if nodes.insert(node.item.item_instance_id, node).is_some() || node.ordinal == 0 {
                return Err(E::MalformedTree);
            }
            if node.item.definition != node.facts.definition
                || node.facts.definition.family != "Item"
            {
                return Err(E::DefinitionMismatch);
            }
            let (maximum, _) = stack_maximum(node.facts.stack).map_err(|_| E::InvalidQuantity)?;
            if node.item.quantity == 0 || node.item.quantity > maximum {
                return Err(E::InvalidQuantity);
            }
            let weight = node.facts.weight.ok_or(E::WeightUnknown)?;
            weight_centi_oz = u64::from(weight)
                .checked_mul(u64::from(node.item.quantity))
                .and_then(|weight| weight_centi_oz.checked_add(weight))
                .ok_or(E::ArithmeticOverflow)?;
            if let Some(parent) = node.parent {
                if !slots.insert((parent, node.ordinal)) {
                    return Err(E::MalformedTree);
                }
                *children.entry(parent).or_default() += 1;
            }
            // MailText can have been parsed as Letter even on a label. Recheck
            // its actual class so that the 1,999 limit cannot bypass 79.
            let class = match node.item.definition.production_key.as_str() {
                LETTER_KEY | STAMPED_LETTER_KEY => Some(MailTextClass::Letter),
                LABEL_KEY => Some(MailTextClass::Label),
                _ => None,
            };
            if let (Some(class), Some(text)) = (class, node.text) {
                MailText::parse(class, text.as_str()).map_err(E::InvalidText)?;
            }
        }
        let root = nodes.get(&input.root).ok_or(E::MalformedTree)?;
        if root.parent.is_some() {
            return Err(E::MalformedTree);
        }
        if input.root_has_store_owner
            || root.item.quantity != 1
            || root.facts.stack != ItemStackClass::NonStackable
        {
            return Err(E::NotMailable);
        }
        let parcel = match root.item.definition.production_key.as_str() {
            PARCEL_KEY
                if root.facts.container_capacity == Some(PARCEL_DIRECT_CHILDREN_MAX as u32) =>
            {
                true
            }
            LETTER_KEY if root.facts.container_capacity.is_none() && input.items.len() == 1 => {
                false
            }
            _ => return Err(E::NotMailable),
        };
        if parcel && input.sender_junior {
            return Err(E::NotMailable);
        }
        for node in input.items {
            if node.item.item_instance_id == input.root {
                continue;
            }
            let mut cursor = node;
            let mut depth = 0;
            // Bounded even for a cycle. Every chain must reach this one root.
            while cursor.item.item_instance_id != input.root {
                depth += 1;
                if depth > PARCEL_HEIGHT_MAX {
                    return Err(E::MalformedTree);
                }
                cursor = nodes
                    .get(&cursor.parent.ok_or(E::MalformedTree)?)
                    .copied()
                    .ok_or(E::MalformedTree)?;
            }
        }
        for (parent, count) in children {
            let parent = nodes.get(&parent).ok_or(E::MalformedTree)?;
            let capacity = parent.facts.container_capacity.ok_or(E::MalformedTree)?;
            if !(1..=GAMEITEM01_CONTAINER_ENTRIES_MAX).contains(&capacity)
                || count > capacity as usize
                || parent.facts.stack != ItemStackClass::NonStackable
            {
                return Err(E::MalformedTree);
            }
        }
        let text = if parcel {
            // Only direct labels, in entry order; an empty text is skipped, but
            // a nonempty text with an empty/invalid first line is not skipped.
            input
                .items
                .iter()
                .filter(|node| {
                    node.parent == Some(input.root)
                        && node.item.definition.production_key == LABEL_KEY
                        && node.text.is_some_and(|text| !text.as_str().is_empty())
                })
                .min_by_key(|node| node.ordinal)
                .and_then(|node| node.text)
        } else {
            root.text
        };
        let recipient = MailText::address(text).map_err(|error| match error {
            MailAddressRefusal::NoAddress => E::NoAddress,
            MailAddressRefusal::UnknownRecipient => E::UnknownRecipient,
        })?;
        if parcel && input.recipient_junior {
            return Err(E::RecipientCannotReceiveParcels);
        }
        let inbox_items_after = input
            .inbox_items
            .checked_add(input.items.len() as u64)
            .filter(|total| *total <= MAIL_INBOX_ITEMS_MAX)
            .ok_or(E::RecipientInboxFull)?;
        Ok(Self {
            recipient,
            root: input.root,
            stamped_production_key: if parcel {
                STAMPED_PARCEL_KEY
            } else {
                STAMPED_LETTER_KEY
            },
            sorted_item_ids: nodes.into_keys().collect(),
            weight_centi_oz,
            inbox_items_after,
        })
    }
}

#[cfg(test)]
#[path = "mail_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "mail_property_tests.rs"]
mod property_tests;
