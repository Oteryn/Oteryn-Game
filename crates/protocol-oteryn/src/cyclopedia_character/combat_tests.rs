use super::*;
use crate::cyclopedia_character::{MAX_TAB_RESULT_BYTES, TabView, decode_tab_request};

#[test]
fn p12c_combat_tabs_preserve_unknown_and_known_zero() -> Result<(), TabWireError> {
    for tab in [
        CharacterTab::Offence,
        CharacterTab::Defence,
        CharacterTab::Misc,
    ] {
        assert_eq!(decode_tab_request(&[tab as u8, 0, 0, 1])?.tab, tab);
        let mut value = CombatStats::new(tab, 123, 7, 3)?;
        let key = CombatStat::for_tab(tab)[0];
        assert_eq!(value.get(key), None);
        let unknown = TabView::Combat(value.clone()).encode()?;
        assert_eq!(CombatStats::decode(&unknown)?, value);
        value.set(key, 0)?;
        let zero = TabView::Combat(value.clone()).encode()?;
        assert_ne!(zero, unknown);
        assert_eq!(CombatStats::decode(&zero)?, value);
        assert_eq!(value.get(key), Some(0));
    }
    Ok(())
}

#[test]
fn p12c_combat_codec_refuses_bad_fields_tags_lengths_and_revisions() -> Result<(), TabWireError> {
    let value = CombatStats::new(CharacterTab::Offence, 0, 1, 1)?;
    let bytes = TabView::Combat(value).encode()?;
    for length in 0..bytes.len() {
        assert_eq!(CombatStats::decode(&bytes[..length]), Err(TabWireError));
    }
    for (at, replacement) in [(0, 1), (9, 0), (17, 0), (25, 0), (26, 64), (27, 2)] {
        let mut corrupt = bytes.clone();
        corrupt[at] = replacement;
        assert_eq!(CombatStats::decode(&corrupt), Err(TabWireError));
    }
    let mut trailing = bytes;
    trailing.push(0);
    assert_eq!(CombatStats::decode(&trailing), Err(TabWireError));
    assert_eq!(
        CombatStats::new(CharacterTab::Inspection, 1, 1, 1),
        Err(TabWireError)
    );
    Ok(())
}

#[test]
fn p12c_combat_fields_are_unique_bounded_and_respect_units() -> Result<(), TabWireError> {
    let mut codes = Vec::new();
    for tab in [
        CharacterTab::Offence,
        CharacterTab::Defence,
        CharacterTab::Misc,
    ] {
        let mut value = CombatStats::new(tab, u64::MAX, 1, 1)?;
        for key in CombatStat::for_tab(tab) {
            assert_eq!(key.tab(), tab);
            assert!(!codes.contains(&(*key as u8)));
            codes.push(*key as u8);
            value.set(*key, i64::MAX)?;
        }
        let encoded = TabView::Combat(value.clone()).encode()?;
        assert!(encoded.len() < MAX_TAB_RESULT_BYTES);
        assert_eq!(CombatStats::decode(&encoded)?, value);
    }
    let mut defence = CombatStats::new(CharacterTab::Defence, 0, 1, 1)?;
    defence.set(CombatStat::AbsorbFire, -125_000)?;
    assert_eq!(defence.get(CombatStat::AbsorbFire), Some(-125_000));
    assert_eq!(defence.set(CombatStat::Armor, -1), Err(TabWireError));
    assert_eq!(
        defence.set(CombatStat::CriticalChance, 0),
        Err(TabWireError)
    );
    assert_eq!(
        CombatStat::CriticalChance.unit(),
        CombatStatUnit::PartsPerMillion
    );
    assert_eq!(
        CombatStat::WealthDuplexSeconds.unit(),
        CombatStatUnit::Seconds
    );
    Ok(())
}
