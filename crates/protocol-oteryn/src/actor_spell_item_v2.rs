//! Unallocated additive candidate for an exact rune ItemInstance. Capability
//! and command numbers belong to the owning registry/Hello negotiation.
use crate::actor_spell::{
    ActorSpellError, SpellCastIntent, decode_spell_cast_intent, encode_spell_cast_intent,
};

pub const CAPABILITY_NAME: &str = "ACTOR_SPELL_ITEM_INSTANCE_V2";
pub const COMMAND_NAME: &str = "WORLD_ACTOR_SPELL_CAST_ITEM_INSTANCE_V2";
pub const MAX_ITEM_INTENT_BYTES: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemSpellCastIntent {
    /// Exact optional target Item; current custody is independently resolved.
    pub target_item: Option<TargetItemInstance>,
    pub intent: SpellCastIntent,
    pub rune_item_instance: [u8; 16],
    pub expected_state_revision: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetItemInstance {
    pub item_instance: [u8; 16],
    pub expected_state_revision: u64,
}
fn valid(value: &ItemSpellCastIntent) -> bool {
    value.rune_item_instance[6] >> 4 == 7
        && value.rune_item_instance[8] >> 6 == 2
        && value.expected_state_revision > 0
        && value.target_item.is_none_or(|target| {
            target.item_instance[6] >> 4 == 7
                && target.item_instance[8] >> 6 == 2
                && target.expected_state_revision > 0
        })
}
fn varint(out: &mut Vec<u8>, mut value: u64) {
    while value >= 128 {
        out.push(value as u8 | 128);
        value >>= 7;
    }
    out.push(value as u8);
}
fn read(input: &[u8], cursor: &mut usize) -> Result<u64, ActorSpellError> {
    let mut value = 0;
    for shift in (0..70).step_by(7) {
        let byte = *input.get(*cursor).ok_or(ActorSpellError::Malformed)?;
        *cursor += 1;
        if shift == 63 && byte > 1 {
            return Err(ActorSpellError::Malformed);
        }
        value |= u64::from(byte & 127) << shift;
        if byte & 128 == 0 {
            return Ok(value);
        }
    }
    Err(ActorSpellError::Malformed)
}
fn bytes<'a>(input: &'a [u8], cursor: &mut usize) -> Result<&'a [u8], ActorSpellError> {
    let length = usize::try_from(read(input, cursor)?).map_err(|_| ActorSpellError::Malformed)?;
    let end = cursor
        .checked_add(length)
        .ok_or(ActorSpellError::Malformed)?;
    let result = input.get(*cursor..end).ok_or(ActorSpellError::Malformed)?;
    *cursor = end;
    Ok(result)
}
pub fn encode_item_spell_cast_intent(
    value: &ItemSpellCastIntent,
) -> Result<Vec<u8>, ActorSpellError> {
    if !valid(value) {
        return Err(ActorSpellError::Malformed);
    }
    let nested = encode_spell_cast_intent(&value.intent);
    let mut out = vec![8, 2, 18];
    varint(&mut out, nested.len() as u64);
    out.extend(nested);
    out.extend([26, 16]);
    out.extend(value.rune_item_instance);
    out.push(32);
    varint(&mut out, value.expected_state_revision);
    if let Some(target) = value.target_item {
        out.extend([42, 16]);
        out.extend(target.item_instance);
        out.push(48);
        varint(&mut out, target.expected_state_revision);
    }
    if out.len() > MAX_ITEM_INTENT_BYTES {
        return Err(ActorSpellError::LimitExceeded);
    }
    Ok(out)
}
pub fn decode_item_spell_cast_intent(input: &[u8]) -> Result<ItemSpellCastIntent, ActorSpellError> {
    if input.len() > MAX_ITEM_INTENT_BYTES {
        return Err(ActorSpellError::LimitExceeded);
    }
    let mut cursor = 0;
    if read(input, &mut cursor)? != 8
        || read(input, &mut cursor)? != 2
        || read(input, &mut cursor)? != 18
    {
        return Err(ActorSpellError::Malformed);
    }
    let intent = decode_spell_cast_intent(bytes(input, &mut cursor)?)?;
    if read(input, &mut cursor)? != 26 {
        return Err(ActorSpellError::Malformed);
    }
    let rune_item_instance = bytes(input, &mut cursor)?
        .try_into()
        .map_err(|_| ActorSpellError::Malformed)?;
    if read(input, &mut cursor)? != 32 {
        return Err(ActorSpellError::Malformed);
    }
    let expected_state_revision = read(input, &mut cursor)?;
    let target_item = if cursor == input.len() {
        None
    } else {
        if read(input, &mut cursor)? != 42 {
            return Err(ActorSpellError::Malformed);
        }
        let item_instance = bytes(input, &mut cursor)?
            .try_into()
            .map_err(|_| ActorSpellError::Malformed)?;
        if read(input, &mut cursor)? != 48 {
            return Err(ActorSpellError::Malformed);
        }
        Some(TargetItemInstance {
            item_instance,
            expected_state_revision: read(input, &mut cursor)?,
        })
    };
    let value = ItemSpellCastIntent {
        target_item,
        intent,
        rune_item_instance,
        expected_state_revision,
    };
    if cursor != input.len()
        || !valid(&value)
        || encode_item_spell_cast_intent(&value)?.as_slice() != input
    {
        return Err(ActorSpellError::Malformed);
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn value() -> ItemSpellCastIntent {
        ItemSpellCastIntent {
            target_item: None,
            intent: SpellCastIntent {
                spell: std::num::NonZeroU32::new(1).unwrap(),
                target: crate::actor_spell::SpellTarget::None,
                aim_at_target: false,
            },
            rune_item_instance: [1, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 16],
            expected_state_revision: 17,
        }
    }
    #[test]
    fn exact_instance_and_full_revision_round_trip() {
        let mut value = value();
        value.expected_state_revision = u64::MAX;
        assert_eq!(
            decode_item_spell_cast_intent(&encode_item_spell_cast_intent(&value).unwrap()).unwrap(),
            value
        );
    }
    #[test]
    fn non_rfc_identity_and_zero_revision_refused() {
        let mut altered = value();
        altered.rune_item_instance[6] = 0x40;
        assert!(encode_item_spell_cast_intent(&altered).is_err());
        altered = value();
        altered.rune_item_instance[8] = 0;
        assert!(encode_item_spell_cast_intent(&altered).is_err());
        altered = value();
        altered.expected_state_revision = 0;
        assert!(encode_item_spell_cast_intent(&altered).is_err());
    }
    #[test]
    fn alternate_encoding_duplicate_or_truncated_source_refused() {
        let encoded = encode_item_spell_cast_intent(&value()).unwrap();
        for end in 0..encoded.len() {
            assert!(decode_item_spell_cast_intent(&encoded[..end]).is_err());
        }
        let mut changed = encoded.clone();
        changed.extend([32, 17]);
        assert!(decode_item_spell_cast_intent(&changed).is_err());
        changed = encoded.clone();
        changed.splice(1..2, [0x82, 0]);
        assert!(decode_item_spell_cast_intent(&changed).is_err());
        changed = encoded;
        changed[1] = 1;
        assert!(decode_item_spell_cast_intent(&changed).is_err());
    }
}

#[cfg(test)]
mod target_tests {
    use super::*;
    #[test]
    fn target_is_full_revision_bound_and_strictly_optional() {
        let id = [1, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 16];
        let mut value = ItemSpellCastIntent {
            intent: SpellCastIntent {
                spell: std::num::NonZeroU32::new(1).unwrap(),
                target: crate::actor_spell::SpellTarget::None,
                aim_at_target: false,
            },
            rune_item_instance: id,
            expected_state_revision: 1,
            target_item: Some(TargetItemInstance {
                item_instance: id,
                expected_state_revision: u64::MAX,
            }),
        };
        let bytes = encode_item_spell_cast_intent(&value).unwrap();
        assert_eq!(decode_item_spell_cast_intent(&bytes).unwrap(), value);
        let mut duplicate = bytes.clone();
        duplicate.extend_from_slice(&bytes[bytes.len() - 10..]);
        assert!(decode_item_spell_cast_intent(&duplicate).is_err());
        value.target_item.as_mut().unwrap().expected_state_revision = 0;
        assert!(encode_item_spell_cast_intent(&value).is_err());
        value.target_item = None;
        assert!(encode_item_spell_cast_intent(&value).unwrap().len() < bytes.len());
    }
}
