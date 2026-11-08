//! Source entry identities, not client-side quest eligibility or completion state.
use super::BossEncounterPanel;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndgameChain {
    SoulWar,
    RottenBlood,
}

impl EndgameChain {
    fn bosses(self) -> &'static [&'static str] {
        // Crystal 201189f Soul War entrances start Megalomania in its purple phase.
        match self {
            Self::SoulWar => &[
                "oteryn:creature.goshnar_s_cruelty",
                "oteryn:creature.goshnar_s_greed",
                "oteryn:creature.goshnar_s_hatred",
                "oteryn:creature.goshnar_s_malice",
                "oteryn:creature.goshnar_s_spite",
                "oteryn:creature.goshnar_s_megalomania_purple",
            ],
            Self::RottenBlood => &[
                "oteryn:creature.ichgahal",
                "oteryn:creature.murcion",
                "oteryn:creature.vemiath",
                "oteryn:creature.chagorz",
                "oteryn:creature.bakragore",
            ],
        }
    }
}

/// Choose a boss to submit with Enter. Selection never sends a command or skips
/// server prerequisites; cycling is useful when revisiting a previous encounter.
pub fn select_next_boss(panel: &mut BossEncounterPanel, chain: EndgameChain) -> bool {
    if !panel.selected || panel.pending {
        return false;
    }
    let bosses = chain.bosses();
    let next = bosses
        .iter()
        .position(|key| *key == panel.fields[0])
        .map_or(0, |index| (index + 1) % bosses.len());
    panel.fields[0] = bosses[next].into();
    panel.feedback = Some(
        "Boss selected. Enter requests access; the server checks quest progress and returns you after the encounter.",
    );
    true
}
