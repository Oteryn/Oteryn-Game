//! Dawnport's four Global vocation-trial movement actions.
//!
//! Crystal `data-global/scripts/movements/others/dawnport_vocation_trial.lua:26-145`
//! defines these action ids, outfits and item grants; lines 323-350 apply them on step-in and
//! line 334 refuses level 20 and above. Global has four classic vocations, so Crystal's later
//! monk extension (lines 146-172) is deliberately not part of this table.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrialOutfit {
    pub female: u16,
    pub male: u16,
    pub colours: [u8; 4],
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrialItem {
    pub id: u16,
    pub amount: u16,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DawnportTrial {
    pub action_id: u16,
    pub vocation: &'static str,
    pub outfit: TrialOutfit,
    pub items: &'static [TrialItem],
    pub spells: &'static [&'static str],
}

const SORCERER_ITEMS: &[TrialItem] = &[
    TrialItem {
        id: 21348,
        amount: 1,
    },
    TrialItem {
        id: 21400,
        amount: 1,
    },
    TrialItem {
        id: 7876,
        amount: 2,
    },
    TrialItem {
        id: 268,
        amount: 10,
    },
    TrialItem {
        id: 21352,
        amount: 2,
    },
    TrialItem {
        id: 21351,
        amount: 2,
    },
    TrialItem {
        id: 3577,
        amount: 1,
    },
];
const DRUID_ITEMS: &[TrialItem] = &[
    TrialItem {
        id: 21350,
        amount: 1,
    },
    TrialItem {
        id: 21400,
        amount: 1,
    },
    TrialItem {
        id: 7876,
        amount: 2,
    },
    TrialItem {
        id: 268,
        amount: 10,
    },
    TrialItem {
        id: 21352,
        amount: 2,
    },
    TrialItem {
        id: 21351,
        amount: 2,
    },
    TrialItem {
        id: 3577,
        amount: 1,
    },
];
const PALADIN_ITEMS: &[TrialItem] = &[
    TrialItem {
        id: 3350,
        amount: 1,
    },
    TrialItem {
        id: 35562,
        amount: 1,
    },
    TrialItem {
        id: 21470,
        amount: 100,
    },
    TrialItem {
        id: 7876,
        amount: 7,
    },
    TrialItem { id: 268, amount: 5 },
    TrialItem {
        id: 21352,
        amount: 1,
    },
    TrialItem {
        id: 21351,
        amount: 1,
    },
    TrialItem {
        id: 3577,
        amount: 1,
    },
];
const KNIGHT_ITEMS: &[TrialItem] = &[
    TrialItem {
        id: 3267,
        amount: 1,
    },
    TrialItem {
        id: 3412,
        amount: 1,
    },
    TrialItem {
        id: 7876,
        amount: 10,
    },
    TrialItem { id: 268, amount: 2 },
    TrialItem {
        id: 3577,
        amount: 1,
    },
];

pub const TRIALS: [DawnportTrial; 4] = [
    DawnportTrial {
        action_id: 25005,
        vocation: "sorcerer",
        outfit: TrialOutfit {
            female: 138,
            male: 130,
            colours: [95, 109, 112, 128],
        },
        items: SORCERER_ITEMS,
        spells: &["Magic Patch", "Buzz", "Scorch"],
    },
    DawnportTrial {
        action_id: 25006,
        vocation: "druid",
        outfit: TrialOutfit {
            female: 138,
            male: 130,
            colours: [95, 123, 9, 118],
        },
        items: DRUID_ITEMS,
        spells: &["Magic Patch", "Chill Out", "Mud Attack"],
    },
    DawnportTrial {
        action_id: 25007,
        vocation: "paladin",
        outfit: TrialOutfit {
            female: 137,
            male: 129,
            colours: [95, 117, 98, 78],
        },
        items: PALADIN_ITEMS,
        spells: &["Magic Patch", "Arrow Call", "Lesser Ethereal Spear"],
    },
    DawnportTrial {
        action_id: 25008,
        vocation: "knight",
        outfit: TrialOutfit {
            female: 139,
            male: 131,
            colours: [95, 38, 94, 115],
        },
        items: KNIGHT_ITEMS,
        spells: &["Bruise Bane", "Lesser Front Sweep"],
    },
];

pub fn trial_for_step(action_id: u16, level: u32, current: &str) -> Option<&'static DawnportTrial> {
    (level < 20).then_some(())?;
    TRIALS
        .iter()
        .find(|trial| trial.action_id == action_id && current != trial.vocation)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn each_global_trial_tile_resolves_its_complete_loadout() {
        for (action, vocation, items, spells) in [
            (25005, "sorcerer", 7, 3),
            (25006, "druid", 7, 3),
            (25007, "paladin", 8, 3),
            (25008, "knight", 5, 2),
        ] {
            let trial = trial_for_step(action, 19, "none").unwrap();
            assert_eq!(
                (trial.vocation, trial.items.len(), trial.spells.len()),
                (vocation, items, spells)
            );
        }
    }
    #[test]
    fn level_twenty_and_same_vocation_are_refused_but_another_tile_switches_back() {
        assert!(trial_for_step(25005, 20, "knight").is_none());
        assert!(trial_for_step(25005, 19, "sorcerer").is_none());
        assert_eq!(
            trial_for_step(25008, 19, "sorcerer").map(|t| t.vocation),
            Some("knight")
        );
        assert_eq!(
            trial_for_step(25005, 19, "knight").map(|t| t.vocation),
            Some("sorcerer")
        );
    }
}
