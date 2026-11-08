#[path = "protocol/stamina.rs"]
mod stamina;
use super::CyclopediaCharacterPanel;
use oteryn_client_runtime::{
    CommandStatus,
    cyclopedia_character::{self, TabRequest},
    raw_feature::{FeatureCommand, GatedCommandResult},
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Badges,
    Tab,
    SelectTab(cyclopedia_character::CharacterTab),
    Next,
    Previous,
}
pub fn request(p: &mut CyclopediaCharacterPanel, intent: Intent) -> Option<FeatureCommand> {
    if !p.selected {
        p.feedback = Some("Feature unavailable in this session");
        return None;
    }
    if p.pending {
        p.feedback = Some("Waiting for the server");
        return None;
    }
    match intent {
        Intent::SelectTab(tab) => {
            p.tab = tab;
            p.offset = 0;
        }
        Intent::Next => p.offset = p.offset.saturating_add(u16::from(p.limit)),
        Intent::Previous => p.offset = p.offset.saturating_sub(u16::from(p.limit)),
        _ => {}
    }
    let (kind, payload) = match intent {
        Intent::Badges => (387, Vec::new()),
        Intent::Tab | Intent::SelectTab(_) | Intent::Next | Intent::Previous => {
            let mut v = vec![p.tab as u8];
            v.extend_from_slice(&p.offset.to_le_bytes());
            v.push(p.limit);
            if let Some(target) = p.target {
                v.extend_from_slice(&target)
            }
            if cyclopedia_character::decode_tab_request(&v)
                != Ok(TabRequest {
                    tab: p.tab,
                    page: oteryn_client_runtime::character_info::PageRequest::new(
                        p.offset, p.limit,
                    )?,
                    target: p.target,
                })
            {
                p.feedback = Some("Enter a valid page and target");
                return None;
            }
            (388, v)
        }
    };
    p.requested_tab = (kind == 388).then_some(p.tab);
    p.pending = true;
    p.command_kind = kind;
    p.feedback = None;
    Some(FeatureCommand {
        capability: 199,
        kind,
        payload,
    })
}
pub fn apply_result(p: &mut CyclopediaCharacterPanel, r: &GatedCommandResult) -> bool {
    let pending = p.pending;
    p.pending = false;
    let tab = p.requested_tab.take();
    if !pending {
        return false;
    }
    if !matches!(
        r.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        p.feedback = Some("The server refused the character query");
        return false;
    }
    let text = if p.command_kind == 387 {
        decode_badges(&r.payload)
    } else {
        tab.and_then(|tab| decode_tab(&r.payload, tab, p.limit))
    };
    match text {
        Some(v) => {
            let general = if tab == Some(cyclopedia_character::CharacterTab::GeneralStats) {
                let Some(stats) = decode_general(&r.payload) else {
                    p.feedback = Some("The server returned invalid general statistics");
                    return false;
                };
                Some(stats)
            } else {
                None
            };
            p.combat = if tab == Some(cyclopedia_character::CharacterTab::Defence) {
                cyclopedia_character::combat::CombatStats::decode(&r.payload).ok()
            } else {
                None
            };
            p.store = if tab == Some(cyclopedia_character::CharacterTab::StoreSummary) {
                decode_store(&r.payload)
            } else {
                None
            };
            p.general = general;
            if let Some(tab) = tab.filter(|t| {
                matches!(
                    t,
                    cyclopedia_character::CharacterTab::RecentDeaths
                        | cyclopedia_character::CharacterTab::RecentPvpKills
                )
            }) {
                let Some(rows) = decode_history(&r.payload, tab, p.limit) else {
                    p.feedback = Some("The server returned invalid character history");
                    return false;
                };
                p.history = rows;
                p.history_tab = Some(tab);
            } else {
                p.history.clear();
                p.history_tab = None;
            }
            p.result = Some(v);
            p.feedback = None;
            true
        }
        None => {
            p.feedback = Some("The server returned an invalid character view");
            false
        }
    }
}
fn decode_badges(b: &[u8]) -> Option<String> {
    let (&count, ids) = b.split_first()?;
    (usize::from(count) == ids.len()
        && ids
            .iter()
            .all(|id| cyclopedia_character::Badge::from_id(*id).is_some()))
    .then(|| {
        format!(
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
fn decode_tab(
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
            rows.push(format!(
                "Experience: {}\nLevel: {}\nMagic level: {}",
                r.u64()?,
                r.u32()?,
                r.u16()?
            ));
            for label in [
                "Fist",
                "Club",
                "Sword",
                "Axe",
                "Distance",
                "Shielding",
                "Fishing",
            ] {
                rows.push(format!("{label}: {}", r.u16()?));
            }
            rows.push(format!(
                "{}\nOffline training: {} minutes",
                stamina::display(r.u16()?),
                r.u16()?
            ));
            let known = r.u8()?;
            let (health, mana) = (r.u32()?, r.u32()?);
            rows.push(match known {
                0 if health == 0 && mana == 0 => "Health: unavailable\nMana: unavailable".into(),
                1 => format!("Health: {health} (last saved)\nMana: {mana} (last saved)"),
                _ => return None,
            });
        }
        Tab::RecentDeaths | Tab::RecentPvpKills => {
            let count = r.count(limit)?;
            for _ in 0..count {
                let at = i64::from_le_bytes(r.bytes()?);
                let status = if requested == Tab::RecentPvpKills {
                    match r.u8()? {
                        1 => "Unjustified: ",
                        2 => "Guild war: ",
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
            rows.push(format!("Total: {total}"));
            for _ in 0..count {
                rows.push(r.text()?.into());
            }
        }
        Tab::StoreSummary => {
            let count = r.u8()?;
            for _ in 0..count {
                rows.push(format!("Blessing: {}", r.text()?));
            }
            rows.push(format!(
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
                rows.push(format!("Slot {slot}: {definition} × {}", r.u32()?));
            }
        }
        Tab::Offence | Tab::Defence | Tab::Misc => {
            let stats = CombatStats::decode(b).ok()?;
            rows.push(format!(
                "Observed at: {} ms\nCharacter revision: {}\nEquipment revision: {}",
                stats.observed_at_unix_ms, stats.character_revision, stats.equipment_revision
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

fn decode_history(
    b: &[u8],
    tab: cyclopedia_character::CharacterTab,
    limit: u8,
) -> Option<Vec<super::HistoryRow>> {
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
        rows.push(super::HistoryRow {
            at,
            status,
            description: r.text()?.into(),
        });
    }
    r.0.is_empty().then_some(rows)
}

fn decode_general(b: &[u8]) -> Option<cyclopedia_character::GeneralStats> {
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

fn decode_store(b: &[u8]) -> Option<cyclopedia_character::StoreSummary> {
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
