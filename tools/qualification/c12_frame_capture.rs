//! C12's #3521 envelopes describe the original frames fed to the native session decoder.

use oteryn_protocol_oteryn::{
    FrameLength, GameSessionId, MessageType, decode_command_result, decode_framed_envelope,
    decode_server_accepted, decode_state_delta,
};

pub const FRAME_LIMIT: usize = 1024 * 1024;

/// External admission input, carried beside the transcript without modifying any frame.
#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct ItemDictionaryPin {
    pub generation: [u8; 32],
    pub count: u32,
}

impl ItemDictionaryPin {
    pub fn sorted_refs(&self) -> Result<Vec<u32>, &'static str> {
        if !(1..=100_000).contains(&self.count) {
            return Err("C12 Item dictionary count outside bounds");
        }
        Ok((1..=self.count).collect())
    }
}

#[derive(Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Envelope {
    command_id: u64,
    game_session_id: String,
    payload: Vec<u8>,
    server_sequence: u64,
    status: u32,
}

/// Keep command frames only. Admission material and liveness traffic never reach disk.
pub fn command_frames(
    mut wire: &[u8],
) -> Result<Vec<&[u8]>, Box<dyn std::error::Error + Send + Sync>> {
    let mut commands = Vec::new();
    while !wire.is_empty() {
        let prefix = wire.get(..4).ok_or("truncated C12 outgoing frame prefix")?;
        let size = FrameLength::from_prefix(prefix)?.get() as usize + 4;
        let frame = wire.get(..size).ok_or("truncated C12 outgoing frame")?;
        if decode_framed_envelope(frame)?.message_type() == MessageType::ClientCommand {
            commands.push(frame);
        }
        wire = &wire[size..];
    }
    if commands.is_empty() {
        return Err("captured C12 command stream was empty".into());
    }
    Ok(commands)
}

pub fn envelopes(
    mut wire: &[u8],
    session: GameSessionId,
) -> Result<Vec<Envelope>, Box<dyn std::error::Error + Send + Sync>> {
    if wire.is_empty() || wire.len() > FRAME_LIMIT {
        return Err("C12 wire capture size outside bounds".into());
    }
    let hex = format!("{:032x}", u128::from_be_bytes(*session.as_bytes()));
    let uuid = format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..],
    );
    let mut result = Vec::new();
    while !wire.is_empty() {
        let prefix = wire.get(..4).ok_or("C12 truncated frame prefix")?;
        let size = FrameLength::from_prefix(prefix)?.get() as usize;
        let (frame, rest) = wire
            .split_at_checked(4 + size)
            .ok_or("C12 truncated original frame")?;
        let frame = decode_framed_envelope(frame)?;
        if result.is_empty() {
            if frame.message_type() != MessageType::ServerAccepted
                || decode_server_accepted(frame.payload())?.game_session_id != session
            {
                return Err("C12 original admission does not match capture session".into());
            }
        } else if frame.message_type() == MessageType::ServerAccepted {
            return Err("C12 wire contains a second admission".into());
        }
        // Snapshot and push frames carry no CommandResult ID or status on the wire.
        let (command_id, status, payload) = match frame.message_type() {
            MessageType::CommandResult => {
                let command = decode_command_result(frame.payload())?;
                (command.command_id, command.status as u32, command.payload)
            }
            MessageType::StateDelta => (0, 0, decode_state_delta(frame.payload())?.payload),
            _ => (0, 0, frame.payload()),
        };
        result.push(Envelope {
            command_id,
            game_session_id: uuid.clone(),
            payload: payload.to_vec(),
            server_sequence: frame.server_sequence(),
            status,
        });
        wire = rest;
    }
    Ok(result)
}

use oteryn_protocol_oteryn::item_view::{CharacterInventory, EquipmentSlot};
use oteryn_protocol_oteryn::presentation::{
    PresentationColor, PresentationEvent, PresentationEventKind, PresentationValueKind,
};
use oteryn_protocol_oteryn::world_spatial::ActorPosition;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::num::NonZeroU32;

pub fn equipped(inventory: &CharacterInventory, definition: NonZeroU32) -> bool {
    item_units(inventory, definition) == 1
        && inventory.equipment.iter().any(|entry| {
            entry.slot == EquipmentSlot::RightHand
                && entry.item.item_definition_ref == definition
                && entry.item.count.get() == 1
        })
        && !inventory
            .equipment
            .iter()
            .any(|entry| entry.slot == EquipmentSlot::LeftHand)
}

pub fn carried(inventory: &CharacterInventory, definition: NonZeroU32) -> bool {
    item_units(inventory, definition) == 1
        && inventory
            .entries
            .iter()
            .any(|entry| entry.item_definition_ref == definition && entry.count.get() == 1)
}

pub fn item_units(inventory: &CharacterInventory, definition: NonZeroU32) -> u64 {
    inventory
        .entries
        .iter()
        .chain(inventory.main_backpack.iter())
        .chain(inventory.equipment.iter().map(|entry| &entry.item))
        .filter(|entry| entry.item_definition_ref == definition)
        .map(|entry| u64::from(entry.count.get()))
        .sum()
}

pub fn exact_target(origin: ActorPosition, target: ActorPosition, expected: ActorPosition) -> bool {
    origin == expected
        && target.floor == origin.floor
        && target.x == origin.x
        && target.y.checked_add(1) == Some(origin.y)
}

pub fn spent_harmony(before: u32, after: u32) -> bool {
    before == 1 && after == 0
}

pub fn exact_saved_mana(before_cast: u32, before_logout: u32, saved: u32, reloaded: u32) -> bool {
    before_logout < before_cast && before_logout == saved && saved == reloaded
}

pub fn mana_before_cast(
    events: impl IntoIterator<Item = (u64, u32)>,
    start: u32,
    max_mana: u32,
    cast_sequence: u64,
) -> Option<u32> {
    let mut mana = start;
    for (sequence, update) in events {
        if sequence >= cast_sequence || update == mana {
            continue;
        }
        // A Monk outside a protection zone can gain one regeneration credit
        // while this single command is in flight. A debit is never regeneration.
        if mana != start || update <= mana || update > start.saturating_add(2).min(max_mana) {
            return None;
        }
        mana = update;
    }
    Some(mana)
}

pub fn attributed_hits(
    events: &[PresentationEvent],
    caster: EntityRef,
    victim: EntityRef,
    spell: u32,
    hits: &mut Vec<u64>,
) {
    for event in events {
        if let PresentationEventKind::ValueText {
            kind: PresentationValueKind::DamageDealt,
            target,
            attacker,
            primary,
            secondary,
            ..
        } = event.kind
        {
            let has_spell_cue = events.iter().any(|part| part.decision_ordinal == event.decision_ordinal
                && matches!(part.kind, PresentationEventKind::SpellWords { caster: source, spell: cue, .. }
                    if source == caster && cue == spell));
            // The isolated target's counterattack is the only unrelated damage in this window.
            if target == caster && attacker == Some(victim) && !has_spell_cue {
                continue;
            }
            let attributed = target == victim
                && attacker == Some(caster)
                && primary.color == PresentationColor::Red
                && secondary.is_none()
                && has_spell_cue;
            // Preserve malformed or unattributed popups as failed observations.
            hits.push(if attributed { primary.value } else { 0 });
        }
    }
}

pub fn exact_hit(hits: &[u64], expected: u64) -> bool {
    expected > 0 && hits == [expected]
}

#[cfg(test)]
mod tests {
    #[test]
    fn c12_rejects_an_early_mana_debit() {
        use super::mana_before_cast;
        assert_eq!(mana_before_cast([(3, 790)], 800, 810, 5), None);
        assert_eq!(
            mana_before_cast([(3, 802), (5, 752)], 800, 810, 5),
            Some(802)
        );
        assert_eq!(mana_before_cast([(3, 802), (4, 804)], 800, 810, 5), None);
    }

    use super::*;
    use oteryn_protocol_oteryn::item_view::{EquippedItem, ItemEntry};
    use std::num::NonZeroU64;

    #[test]
    fn c12_equipment_rejects_a_retained_shield() {
        let item = ItemEntry {
            appearance_id: 0,
            handle: NonZeroU64::MIN,
            item_definition_ref: NonZeroU32::MIN,
            count: NonZeroU32::MIN,
            sub_type: 0,
        };
        let mut inventory = CharacterInventory {
            equipment: vec![EquippedItem {
                slot: EquipmentSlot::RightHand,
                item,
            }],
            ..CharacterInventory::default()
        };
        assert!(equipped(&inventory, item.item_definition_ref));
        assert!(!carried(&inventory, item.item_definition_ref));
        inventory.equipment.push(EquippedItem {
            slot: EquipmentSlot::LeftHand,
            item: ItemEntry {
                item_definition_ref: NonZeroU32::MIN.saturating_add(1),
                ..item
            },
        });
        assert!(!equipped(&inventory, item.item_definition_ref));
        inventory.equipment.clear();
        inventory.entries.push(item);
        assert!(carried(&inventory, item.item_definition_ref));
        assert!(!equipped(&inventory, item.item_definition_ref));
    }

    #[test]
    fn c12_rejects_harmony_noop_and_stale_mana() {
        assert!(spent_harmony(1, 0));
        assert!(!spent_harmony(0, 0));
        assert!(!spent_harmony(1, 1));
        assert!(exact_saved_mana(810, 769, 769, 769));
        assert!(!exact_saved_mana(810, 769, 770, 770));
        assert!(!exact_saved_mana(810, 769, 769, 770));
        assert!(!exact_saved_mana(810, 810, 810, 810));
    }

    #[test]
    fn c12_rejects_displacement_after_attack() {
        let origin = ActorPosition {
            x: 100,
            y: 100,
            floor: 7,
        };
        let target = ActorPosition { y: 99, ..origin };
        assert!(exact_target(origin, target, origin));
        assert!(!exact_target(
            origin,
            ActorPosition { x: 101, ..target },
            origin
        ));
        assert!(!exact_target(
            ActorPosition { x: 101, ..origin },
            target,
            origin
        ));
        assert!(!exact_target(
            origin,
            ActorPosition { floor: 8, ..target },
            origin
        ));
    }

    #[test]
    fn c12_late_duplicate_invalidates_the_initial_exact_hit() {
        use oteryn_protocol_oteryn::presentation::PresentationValue;
        let caster = EntityRef {
            identity: [1; 16],
            generation: 1,
        };
        let target = EntityRef {
            identity: [2; 16],
            generation: 1,
        };
        let position = ActorPosition {
            x: 100,
            y: 100,
            floor: 7,
        };
        let first = [
            PresentationEvent {
                decision_ordinal: 5,
                emission_ordinal: 1,
                kind: PresentationEventKind::SpellWords {
                    caster,
                    position,
                    spell: 42,
                },
            },
            PresentationEvent {
                decision_ordinal: 5,
                emission_ordinal: 2,
                kind: PresentationEventKind::ValueText {
                    kind: PresentationValueKind::DamageDealt,
                    position,
                    target,
                    attacker: Some(caster),
                    primary: PresentationValue {
                        color: PresentationColor::Red,
                        value: 33,
                    },
                    secondary: None,
                },
            },
        ];
        let mut hits = Vec::new();
        assert!(!exact_hit(&hits, 33));
        attributed_hits(&first, caster, target, 42, &mut hits);
        assert!(exact_hit(&hits, 33));
        // Decision ordinals are local to a batch. A duplicate decision can arrive in a later batch.
        let late = first.map(|mut event| {
            event.decision_ordinal = 1;
            event
        });
        attributed_hits(&late, caster, target, 42, &mut hits);
        assert!(!exact_hit(&hits, 33));
        assert!(!exact_hit(&[11, 22], 33));
        let mut hits = Vec::new();
        attributed_hits(&first, caster, target, 42, &mut hits);
        attributed_hits(&late[1..], caster, target, 42, &mut hits);
        assert_eq!(hits, [33, 0]);
        assert!(!exact_hit(&hits, 33));
        let mut wrong_target = late[1];
        if let PresentationEventKind::ValueText { target, .. } = &mut wrong_target.kind {
            *target = EntityRef {
                identity: [3; 16],
                generation: 1,
            };
        }
        let mut hits = Vec::new();
        attributed_hits(&first, caster, target, 42, &mut hits);
        attributed_hits(&[wrong_target], caster, target, 42, &mut hits);
        assert_eq!(hits, [33, 0]);
        assert!(!exact_hit(&hits, 33));
    }
}
