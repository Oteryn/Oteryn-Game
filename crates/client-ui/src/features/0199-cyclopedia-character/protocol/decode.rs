use super::stamina;
use oteryn_client_runtime::cyclopedia_character;
pub(super) fn decode_badges(b: &[u8]) -> Option<String> {
    let (&count, ids) = b.split_first()?;
    (usize::from(count) == ids.len()
        && ids
            .iter()
            .all(|id| cyclopedia_character::Badge::from_id(*id).is_some()))
    .then(|| {
        lfmt!(
            "Badges: {}",
            ids.iter()
                .filter_map(|id| cyclopedia_character::Badge::from_id(*id))
                .map(|v| v.name())
                .collect::<Vec<_>>()
                .join(", ")
        )
    })
}
// Crystal 201189f protocolgame.cpp:4341-5265 supplies the tab facts. The accepted
// native layout/unknown-field differences are recorded in cyclopedia-character-v1.md.
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
                    None => tr!("unavailable").into(),
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

struct Reader<'a>(&'a [u8]);
impl<'a> Reader<'a> {
    fn bytes<const N: usize>(&mut self) -> Option<[u8; N]> {
        let bytes = self.0.get(..N)?.try_into().ok()?;
        self.0 = &self.0[N..];
        Some(bytes)
    }
    fn u8(&mut self) -> Option<u8> {
        Some(u8::from_le_bytes(self.bytes()?))
    }
    fn u16(&mut self) -> Option<u16> {
        Some(u16::from_le_bytes(self.bytes()?))
    }
    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.bytes()?))
    }
    fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.bytes()?))
    }
    fn count(&mut self, limit: u8) -> Option<u16> {
        let count = self.u16()?;
        (count <= u16::from(limit.min(50))).then_some(count)
    }
    fn text(&mut self) -> Option<&'a str> {
        let len = usize::from(self.u8()?);
        self.string(len)
    }
    fn string(&mut self, len: usize) -> Option<&'a str> {
        let text = std::str::from_utf8(self.0.get(..len)?).ok()?;
        self.0 = &self.0[len..];
        Some(text)
    }
}

pub(super) fn decode_history(
    b: &[u8],
    tab: cyclopedia_character::CharacterTab,
    limit: u8,
) -> Option<Vec<super::super::HistoryRow>> {
    use cyclopedia_character::{CharacterTab, KillStatus};
    if b.first() != Some(&(tab as u8)) {
        return None;
    }
    let mut r = Reader(&b[1..]);
    let count = r.count(limit)?;
    let mut rows = Vec::with_capacity(usize::from(count));
    for _ in 0..count {
        let at = i64::from_le_bytes(r.bytes()?);
        let status = if tab == CharacterTab::RecentPvpKills {
            Some(match r.u8()? {
                1 => KillStatus::Unjustified,
                2 => KillStatus::GuildWar,
                _ => return None,
            })
        } else {
            None
        };
        rows.push(super::super::HistoryRow {
            at,
            status,
            description: r.text()?.into(),
        });
    }
    r.0.is_empty().then_some(rows)
}

pub(super) fn decode_general(b: &[u8]) -> Option<cyclopedia_character::GeneralStats> {
    if b.first() != Some(&(cyclopedia_character::CharacterTab::GeneralStats as u8)) {
        return None;
    }
    let mut r = Reader(&b[1..]);
    let experience = r.u64()?;
    let level = r.u32()?;
    let magic_level = r.u16()?;
    let mut skills = [0; 7];
    for skill in &mut skills {
        *skill = r.u16()?;
    }
    let stamina_minutes = r.u16()?;
    let offline_training_minutes = r.u16()?;
    let known = r.u8()?;
    let (health, mana) = (r.u32()?, r.u32()?);
    let last_vitals = match known {
        0 if health == 0 && mana == 0 => None,
        1 => Some((health, mana)),
        _ => return None,
    };
    r.0.is_empty()
        .then_some(cyclopedia_character::GeneralStats {
            experience,
            level,
            magic_level,
            skills,
            stamina_minutes,
            offline_training_minutes,
            last_vitals,
        })
}

pub(super) fn decode_store(b: &[u8]) -> Option<cyclopedia_character::StoreSummary> {
    if b.first() != Some(&(cyclopedia_character::CharacterTab::StoreSummary as u8)) {
        return None;
    }
    let mut r = Reader(&b[1..]);
    let count = r.u8()?;
    let mut blessings = Vec::with_capacity(usize::from(count));
    for _ in 0..count {
        blessings.push(r.text()?.into());
    }
    let permanent_prey_slots = r.u8()?;
    let prey_wildcards = r.u8()?;
    let hirelings = r.u8()?;
    r.0.is_empty()
        .then_some(cyclopedia_character::StoreSummary {
            blessings,
            permanent_prey_slots,
            prey_wildcards,
            hirelings,
        })
}
