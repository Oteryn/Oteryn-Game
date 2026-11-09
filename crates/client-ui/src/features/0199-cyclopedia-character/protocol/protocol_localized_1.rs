use super::*;
pub(super) fn decode_tab(
    b: &[u8],
    requested: cyclopedia_character::CharacterTab,
    limit: u8,
) -> Option<String> {
    use cyclopedia_character::{
        CharacterTab as Tab,
        combat::{CombatStat, CombatStatUnit, CombatStats},
    };
    if b.len() > cyclopedia_character::MAX_TAB_RESULT_BYTES || b.first() != Some(&(requested as u8))
    {
        return None;
    }
    let mut r = Reader(&b[1..]);
    let mut rows = vec![format!("{requested:?}")];
    match requested {
        Tab::GeneralStats => {
            rows.push(lfmt!(
                "Experience: {}\nLevel: {}\nMagic level: {}",
                r.u64()?,
                r.u32()?,
                r.u16()?
            ));
            for label in [
                tr!("Fist"),
                tr!("Club"),
                tr!("Sword"),
                tr!("Axe"),
                tr!("Distance"),
                tr!("Shielding"),
                tr!("Fishing"),
            ] {
                rows.push(format!("{label}: {}", r.u16()?));
            }
            rows.push(lfmt!(
                "{}\nOffline training: {} minutes",
                stamina::display(r.u16()?),
                r.u16()?
            ));
            let known = r.u8()?;
            let (health, mana) = (r.u32()?, r.u32()?);
            rows.push(match known {
                0 if health == 0 && mana == 0 => {
                    tr!("Health: unavailable\nMana: unavailable").into()
                }
                1 => lfmt!(
                    "Health: {health} (last saved)\nMana: {mana} (last saved)",
                    health = health,
                    mana = mana
                ),
                _ => return None,
            });
        }
        Tab::RecentDeaths | Tab::RecentPvpKills => {
            let count = r.count(limit)?;
            for _ in 0..count {
                let at = i64::from_le_bytes(r.bytes()?);
                let status = if requested == Tab::RecentPvpKills {
                    match r.u8()? {
                        1 => tr!("Unjustified: "),
                        2 => tr!("Guild war: "),
                        _ => return None,
                    }
                } else {
                    ""
                };
                rows.push(format!("{at}: {status}{}", r.text()?));
            }
        }
        Tab::Achievements | Tab::Appearances => {
            let total = r.u16()?;
            let count = r.count(limit)?;
            if count > total {
                return None;
            }
            rows.push(lfmt!("Total: {total}", total = total));
            for _ in 0..count {
                rows.push(r.text()?.into());
            }
        }
        Tab::StoreSummary => {
            let count = r.u8()?;
            rows.push(tr!("Blessings / death protection").into());
            rows.push(lfmt!("Owned blessings: {count}", count = count));
            if count == 0 {
                rows.push(tr!("No blessings owned").into());
            }
            rows.push(
                tr!("Loss percentages are unavailable; ownership does not imply full protection.")
                    .into(),
            );
            for _ in 0..count {
                rows.push(lfmt!("Blessing: {}", r.text()?));
            }
            rows.push(lfmt!(
                "Permanent prey slots: {}\nPrey wildcards: {}\nHirelings: {}",
                r.u8()?,
                r.u8()?,
                r.u8()?
            ));
        }
        Tab::Inspection => {
            let count = r.u8()?;
            for _ in 0..count {
                let slot = r.u8()?;
                let len = usize::from(r.u16()?);
                if len > 512 {
                    return None;
                }
                let definition = r.string(len)?;
                rows.push(lfmt!(
                    "Slot {slot}: {definition} × {}",
                    r.u32()?,
                    definition = definition,
                    slot = slot
                ));
            }
        }
        Tab::Offence | Tab::Defence | Tab::Misc => {
            let stats = CombatStats::decode(b).ok()?;
            rows.push(lfmt!(
                "Observed at: {} ms\nCharacter revision: {}\nEquipment revision: {}",
                stats.observed_at_unix_ms,
                stats.character_revision,
                stats.equipment_revision
            ));
            for key in CombatStat::for_tab(requested) {
                let value = match stats.get(*key) {
                    None => "unavailable".into(),
                    Some(v) => match key.unit() {
                        CombatStatUnit::PartsPerMillion => {
                            let magnitude = v.unsigned_abs();
                            format!(
                                "{}{}.{:04}%",
                                if v < 0 { "-" } else { "" },
                                magnitude / 10_000,
                                magnitude % 10_000
                            )
                        }
                        CombatStatUnit::Seconds => format!("{v} seconds"),
                        _ => v.to_string(),
                    },
                };
                rows.push(format!("{key:?}: {value}"));
            }
            return Some(rows.join("\n"));
        }
    }
    r.0.is_empty().then(|| rows.join("\n"))
}
