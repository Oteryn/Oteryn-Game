//! Shared qualification row declarations. Spell access comes from the served book.
use std::collections::BTreeMap;

#[derive(Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct RowRequirements {
    pub(crate) before_first_unlock: bool,
    pub(crate) peer_exhausted_stamina: bool,
    pub(crate) peer_level: Option<u32>,
    pub(crate) premium: bool,
    pub(crate) level: Option<u32>,
    pub(crate) vocation: Option<String>,
    pub(crate) runes: Vec<u32>,
    pub(crate) quest_transitions: Vec<String>,
    pub(crate) spells: Vec<String>,
    pub(crate) items: BTreeMap<u32, u32>,
    pub(crate) equipment: Vec<u32>,
    #[serde(rename = "source")]
    _source: Option<String>,
}

impl RowRequirements {
    pub(crate) fn load(row: &str) -> Option<Self> {
        let mut rows: BTreeMap<String, Self> =
            serde_json::from_str(include_str!("qualification/row_requirements.json")).ok()?;
        rows.remove(row)
    }

    pub(crate) fn rune_magic_level(
        &self,
        book: &crate::spell::SpellBook,
    ) -> Result<u16, &'static str> {
        self.runes.iter().try_fold(0, |highest, id| {
            let spell = book.rune(*id).ok_or("row rune absent from served book")?;
            let crate::spell::Carrier::Rune { magic_level, .. } = spell.carrier else {
                return Err("row rune has a non-rune carrier");
            };
            let magic_level =
                u16::try_from(magic_level).map_err(|_| "row rune magic level overflows")?;
            Ok(highest.max(magic_level))
        })
    }

    pub(crate) fn build<'a>(
        &'a self,
        book: &crate::spell::SpellBook,
    ) -> Result<(&'a str, u32, bool), &'static str> {
        let vocation = self.vocation.as_deref().unwrap_or("none");
        let required = crate::spell::Vocation::from_key(vocation).ok_or("unknown row vocation")?;
        let mut level = self.level.unwrap_or(1);
        if self.before_first_unlock {
            level = book
                .spells
                .iter()
                .enumerate()
                .filter(|(index, spell)| {
                    !book.inactive_aliases.contains(index)
                        && !spell.learning_required
                        && spell.level > 1
                        && spell.vocations.contains(&required)
                })
                .map(|(_, spell)| spell.level - 1)
                .min()
                .ok_or("row has no authored spell unlock")?;
        }
        let mut premium = self.premium;
        for words in &self.spells {
            let spell = book
                .spoken(words)
                .ok_or("row spell absent from served book")?
                .spell;
            if !spell.vocations.contains(&required) {
                return Err("row vocation cannot cast the authored spell");
            }
            level = level.max(spell.level);
            premium |= spell.premium;
        }
        for id in &self.runes {
            let spell = book.rune(*id).ok_or("row rune absent from served book")?;
            if !spell.vocations.contains(&required) {
                return Err("row vocation cannot use the authored rune");
            }
            level = level.max(spell.level);
            premium |= spell.premium;
        }
        Ok((vocation, level, premium))
    }
}
