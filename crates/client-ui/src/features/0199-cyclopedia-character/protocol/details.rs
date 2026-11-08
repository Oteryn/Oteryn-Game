use super::*;

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
