//! S-SPELLBOOK: bounded own-actor catalog. Snapshot and delta both replace the complete list.
pub use crate::charm_wire::CyclopediaWireError as CatalogError;
use crate::charm_wire::{
    push_message_field, push_varint_field, read_bytes, read_uint32, read_varint,
};

pub const CAPABILITY_ACTOR_SPELL_CATALOG_V1: u32 = 287;
pub const STATE_DOMAIN_ACTOR_SPELL_CATALOG: u32 = 100;
pub const MAX_ENTRIES: usize = 1024;
pub const MAX_TEXT_BYTES: usize = 128;
pub const MAX_CATALOG_BYTES: usize = 262_144;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellCatalogEntry {
    pub cast_index: u32,
    pub name: String,
    /// Empty for rune use (which has no spoken words).
    pub words: String,
    pub level: u32,
    pub mana: u32,
    pub mana_percent: bool,
    pub soul: u32,
    pub learned: bool,
    /// Level, learning, vocation, premium and rune magic-level eligibility. This does not
    /// promise a legal target, equipment, sufficient pools or an expired cooldown.
    pub available: bool,
    /// S9 ordinals, matching presentation ActorCooldownKind::Group.
    pub cooldown_groups: Vec<u32>,
    pub rune_item: Option<u32>,
}

fn validate(entries: &[SpellCatalogEntry]) -> Result<(), CatalogError> {
    if entries.len() > MAX_ENTRIES {
        return Err(CatalogError::LimitExceeded);
    }
    let mut previous = 0;
    for entry in entries {
        if entry.name.len() > MAX_TEXT_BYTES
            || entry.words.len() > MAX_TEXT_BYTES
            || entry.cooldown_groups.len() > 16
        {
            return Err(CatalogError::LimitExceeded);
        }
        if entry.cast_index <= previous
            || entry.cast_index >= (1 << 28)
            || entry.name.is_empty()
            || entry.rune_item == Some(0)
            || (entry.rune_item.is_some() && !entry.words.is_empty())
            || (entry.rune_item.is_none() && entry.words.is_empty())
            || (entry.mana_percent && entry.mana > 100)
            || entry
                .cooldown_groups
                .iter()
                .any(|id| !(1..=16).contains(id))
            || entry
                .cooldown_groups
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
        {
            return Err(CatalogError::Malformed);
        }
        previous = entry.cast_index;
    }
    Ok(())
}

pub fn encode_catalog(entries: &[SpellCatalogEntry]) -> Result<Vec<u8>, CatalogError> {
    validate(entries)?;
    let mut output = Vec::new();
    for entry in entries {
        let mut bytes = Vec::new();
        push_varint_field(&mut bytes, 1, u64::from(entry.cast_index));
        push_message_field(&mut bytes, 2, entry.name.as_bytes());
        push_message_field(&mut bytes, 3, entry.words.as_bytes());
        for (field, value) in [
            (4, entry.level),
            (5, entry.mana),
            (6, u32::from(entry.mana_percent)),
            (7, entry.soul),
            (8, u32::from(entry.learned)),
            (9, u32::from(entry.available)),
        ] {
            push_varint_field(&mut bytes, field, u64::from(value));
        }
        for id in &entry.cooldown_groups {
            push_varint_field(&mut bytes, 10, u64::from(*id));
        }
        if let Some(item) = entry.rune_item {
            push_varint_field(&mut bytes, 11, u64::from(item));
        }
        push_message_field(&mut output, 1, &bytes);
        if output.len() > MAX_CATALOG_BYTES {
            return Err(CatalogError::LimitExceeded);
        }
    }
    Ok(output)
}

pub fn decode_catalog(input: &[u8]) -> Result<Vec<SpellCatalogEntry>, CatalogError> {
    if input.len() > MAX_CATALOG_BYTES {
        return Err(CatalogError::LimitExceeded);
    }
    let mut cursor = 0;
    let mut entries = Vec::new();
    while cursor < input.len() {
        if entries.len() == MAX_ENTRIES {
            return Err(CatalogError::LimitExceeded);
        }
        if read_varint(input, &mut cursor)? != 10 {
            return Err(CatalogError::Malformed);
        }
        let bytes = read_bytes(input, &mut cursor)?;
        let mut offset = 0;
        let mut seen = 0_u16;
        let mut fields = [0; 12];
        let mut name = String::new();
        let mut words = String::new();
        let mut groups = Vec::new();
        while offset < bytes.len() {
            let tag = read_varint(bytes, &mut offset)?;
            let field = usize::try_from(tag >> 3).map_err(|_| CatalogError::Malformed)?;
            if !(1..=11).contains(&field) || (field != 10 && seen & (1 << field) != 0) {
                return Err(CatalogError::Malformed);
            }
            seen |= 1 << field;
            if field == 2 || field == 3 {
                if tag & 7 != 2 {
                    return Err(CatalogError::Malformed);
                }
                let text = read_bytes(bytes, &mut offset)?;
                if text.len() > MAX_TEXT_BYTES {
                    return Err(CatalogError::LimitExceeded);
                }
                let text = std::str::from_utf8(text)
                    .map_err(|_| CatalogError::Malformed)?
                    .to_owned();
                if field == 2 {
                    name = text;
                } else {
                    words = text;
                }
            } else {
                if tag & 7 != 0 {
                    return Err(CatalogError::Malformed);
                }
                let value = read_uint32(bytes, &mut offset)?;
                if field == 10 {
                    if groups.len() == 16 {
                        return Err(CatalogError::LimitExceeded);
                    }
                    groups.push(value);
                } else {
                    fields[field] = value;
                }
            }
        }
        let boolean = |value| match value {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(CatalogError::Malformed),
        };
        entries.push(SpellCatalogEntry {
            cast_index: fields[1],
            name,
            words,
            level: fields[4],
            mana: fields[5],
            mana_percent: boolean(fields[6])?,
            soul: fields[7],
            learned: boolean(fields[8])?,
            available: boolean(fields[9])?,
            cooldown_groups: groups,
            rune_item: (seen & (1 << 11) != 0).then_some(fields[11]),
        });
    }
    validate(&entries)?;
    Ok(entries)
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "bounded wire fixtures in tests")]
mod tests {
    use super::*;

    fn entry() -> SpellCatalogEntry {
        SpellCatalogEntry {
            cast_index: 7,
            name: "Light Healing".into(),
            words: "exura".into(),
            level: 8,
            mana: 20,
            mana_percent: false,
            soul: 0,
            learned: true,
            available: true,
            cooldown_groups: vec![1, 2],
            rune_item: None,
        }
    }

    #[test]
    fn spell_catalog_independent_fixture_and_malformed_fields() {
        let fixture = [
            10, 24, 8, 7, 18, 1, b'A', 26, 1, b'B', 32, 8, 40, 20, 48, 0, 56, 0, 64, 1, 72, 1, 80,
            1, 80, 2,
        ];
        let value = SpellCatalogEntry {
            name: "A".into(),
            words: "B".into(),
            ..entry()
        };
        assert_eq!(
            encode_catalog(std::slice::from_ref(&value)),
            Ok(fixture.to_vec())
        );
        assert_eq!(decode_catalog(&fixture), Ok(vec![value]));
        for suffix in [
            &[8, 7][..],
            &[96, 1][..],
            &[88, 0][..],
            &[80, 2][..],
            &[80, 17][..],
        ] {
            let mut malformed = fixture.to_vec();
            malformed[1] += suffix.len() as u8;
            malformed.extend_from_slice(suffix);
            assert!(decode_catalog(&malformed).is_err());
        }
        for (index, value) in [(6, 255), (15, 2), (19, 2), (21, 2)] {
            let mut malformed = fixture;
            malformed[index] = value;
            assert!(decode_catalog(&malformed).is_err());
        }
        for end in 1..fixture.len() {
            assert!(decode_catalog(&fixture[..end]).is_err());
        }
    }

    #[test]
    fn spell_catalog_round_trip_and_bounds() {
        let mut catalog = vec![entry()];
        let encoded = encode_catalog(&catalog).expect("encode");
        assert_eq!(decode_catalog(&encoded), Ok(catalog.clone()));
        catalog[0].rune_item = Some(3155);
        catalog[0].words.clear();
        catalog[0].mana_percent = true;
        catalog[0].mana = 100;
        assert_eq!(
            decode_catalog(&encode_catalog(&catalog).expect("rune")),
            Ok(catalog.clone())
        );
        catalog[0].name = "n".repeat(MAX_TEXT_BYTES + 1);
        assert_eq!(encode_catalog(&catalog), Err(CatalogError::LimitExceeded));
        assert_eq!(
            decode_catalog(&vec![0; MAX_CATALOG_BYTES + 1]),
            Err(CatalogError::LimitExceeded)
        );
        let over_count: Vec<_> = (1..=MAX_ENTRIES + 1)
            .map(|index| SpellCatalogEntry {
                cast_index: index as u32,
                ..entry()
            })
            .collect();
        assert_eq!(
            encode_catalog(&over_count),
            Err(CatalogError::LimitExceeded)
        );
        let mut over_count_bytes = Vec::new();
        for entry in &over_count {
            over_count_bytes
                .extend(encode_catalog(std::slice::from_ref(entry)).expect("single entry"));
        }
        assert_eq!(
            decode_catalog(&over_count_bytes),
            Err(CatalogError::LimitExceeded)
        );
        // Independently encoded entry containing a 129-byte name: outer length 134,
        // cast_index 1, field 2 length 129. The text bound fires before semantic checks.
        let mut over_text = vec![10, 134, 1, 8, 1, 18, 129, 1];
        over_text.extend_from_slice(&[b'A'; MAX_TEXT_BYTES + 1]);
        assert_eq!(decode_catalog(&over_text), Err(CatalogError::LimitExceeded));
        for bytes in [&[8, 1][..], &[10, 1, 0][..], &[10, 2, 8, 0][..]] {
            assert!(decode_catalog(bytes).is_err());
        }
        let mut invalid = entry();
        invalid.cast_index = 0;
        assert!(encode_catalog(&[invalid]).is_err());
        let mut invalid = entry();
        invalid.cooldown_groups = vec![17];
        assert!(encode_catalog(&[invalid]).is_err());
        let mut invalid = entry();
        invalid.mana_percent = true;
        invalid.mana = 101;
        assert!(encode_catalog(&[invalid]).is_err());
        assert!(encode_catalog(&[entry(), entry()]).is_err());
        let mut maximum = entry();
        maximum.name = "n".repeat(MAX_TEXT_BYTES);
        maximum.words = "w".repeat(MAX_TEXT_BYTES);
        maximum.cooldown_groups = (1..=16).collect();
        maximum.cast_index = (1 << 28) - 1;
        assert_eq!(
            decode_catalog(&encode_catalog(std::slice::from_ref(&maximum)).expect("maximum")),
            Ok(vec![maximum])
        );
        let many: Vec<_> = (1..=MAX_ENTRIES)
            .map(|index| SpellCatalogEntry {
                cast_index: index as u32,
                ..entry()
            })
            .collect();
        assert_eq!(
            decode_catalog(&encode_catalog(&many).expect("entry bound")),
            Ok(many)
        );
        let mut over_bytes: Vec<_> = (1..=MAX_ENTRIES)
            .map(|index| SpellCatalogEntry {
                cast_index: index as u32,
                name: "n".repeat(MAX_TEXT_BYTES),
                words: "w".repeat(MAX_TEXT_BYTES),
                ..entry()
            })
            .collect();
        assert_eq!(
            encode_catalog(&over_bytes),
            Err(CatalogError::LimitExceeded)
        );
        over_bytes[0].name.clear();
        assert_eq!(encode_catalog(&over_bytes), Err(CatalogError::Malformed));
    }
}
