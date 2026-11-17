//! View model of the assign-spell dialog: "Target Options" per slot and the known-spells filter.
use oteryn_client_runtime::spell_catalog::SpellCatalogEntry;

/// Heading of the target option group.
pub const TARGET_OPTIONS_LABEL: &str = "Target Options:";
/// Label of the spell list filter.
pub const SHOW_ONLY_KNOWN_LABEL: &str = "Show only known spells";

/// How a directional spell is aimed; stored per action slot by the client.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TargetOption {
    TowardsTarget,
    #[default]
    FacingDirection,
    TowardsCursor,
}
impl TargetOption {
    /// Dialog order.
    pub const ALL: [Self; 3] = [
        Self::TowardsTarget,
        Self::FacingDirection,
        Self::TowardsCursor,
    ];
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::TowardsTarget => tr!("Cast towards target"),
            Self::FacingDirection => tr!("Cast in facing direction"),
            Self::TowardsCursor => tr!("Cast towards cursor position"),
        }
    }
}

/// Spells the dialog lists; with `known_only` the unlearned ones are hidden.
#[must_use]
pub fn dialog_spells(entries: &[SpellCatalogEntry], known_only: bool) -> Vec<&SpellCatalogEntry> {
    entries
        .iter()
        .filter(|entry| entry.rune_item.is_none() && (!known_only || entry.learned))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn entry(name: &str, learned: bool, rune_item: Option<u32>) -> SpellCatalogEntry {
        SpellCatalogEntry {
            cast_index: 1,
            name: name.into(),
            words: "utani".into(),
            level: 1,
            mana: 1,
            mana_percent: false,
            soul: 0,
            learned,
            available: learned,
            cooldown_groups: vec![],
            rune_item,
        }
    }
    #[test]
    fn target_options_use_the_test_client_strings() {
        assert_eq!(TARGET_OPTIONS_LABEL, "Target Options:");
        assert_eq!(
            TargetOption::ALL.map(TargetOption::label),
            [
                "Cast towards target",
                "Cast in facing direction",
                "Cast towards cursor position"
            ]
        );
        assert_eq!(TargetOption::default(), TargetOption::FacingDirection);
    }
    #[test]
    fn show_only_known_spells_hides_unlearned_and_runes() {
        let entries = [
            entry("Known", true, None),
            entry("Unknown", false, None),
            entry("Rune", true, Some(2260)),
        ];
        assert_eq!(dialog_spells(&entries, false).len(), 2);
        let known = dialog_spells(&entries, true);
        assert_eq!(known.len(), 1);
        assert_eq!(known[0].name, "Known");
    }
}
