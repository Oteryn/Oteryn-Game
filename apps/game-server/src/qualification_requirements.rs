//! Shared qualification row declarations. Spell access comes from the served book.
use std::collections::BTreeMap;

#[derive(Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct RowRequirements {
    pub(crate) level: Option<u32>,
    pub(crate) vocation: Option<String>,
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

    pub(crate) fn build<'a>(
        &'a self,
        book: &crate::spell::SpellBook,
    ) -> Result<(&'a str, u32, bool), &'static str> {
        let vocation = self.vocation.as_deref().unwrap_or("none");
        let required = crate::spell::Vocation::from_key(vocation).ok_or("unknown row vocation")?;
        let mut level = self.level.unwrap_or(1);
        let mut premium = false;
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
        Ok((vocation, level, premium))
    }
}
