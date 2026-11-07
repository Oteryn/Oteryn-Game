//! Pure house-podium admission rules. Persistence and transport must supply the
//! current house/session fences; this module deliberately grants no authority.
//!
//! The podiums are the catalogue's items (Crystal 201189f8 `data/items/items.xml`):
//! renown 35973/35974 (67245-67252), vigour 38707 (71414-71416) and tenacity
//! 42367/42368 (75107-75114). Renown shows an outfit or a mount
//! (`game.cpp:4603-4636`); vigour shows a boss and tenacity a bestiary creature
//! (`game.cpp:11149`, `getMonsterTypeByRaceId(race, itemId == ITEM_PODIUM_OF_VIGOUR)`).

pub const RENOWN_ITEMS: [&str; 2] = ["oteryn:item.tibia.i35973", "oteryn:item.tibia.i35974"];
pub const VIGOUR_ITEM: &str = "oteryn:item.tibia.i38707";
pub const TENACITY_ITEMS: [&str; 2] = ["oteryn:item.tibia.i42367", "oteryn:item.tibia.i42368"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PodiumKind {
    Renown,
    Vigour,
    Tenacity,
}

impl PodiumKind {
    pub fn of(item: &str) -> Option<Self> {
        if RENOWN_ITEMS.contains(&item) {
            Some(Self::Renown)
        } else if item == VIGOUR_ITEM {
            Some(Self::Vigour)
        } else if TENACITY_ITEMS.contains(&item) {
            Some(Self::Tenacity)
        } else {
            None
        }
    }

    pub fn from_ordinal(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::Renown),
            2 => Some(Self::Vigour),
            3 => Some(Self::Tenacity),
            _ => None,
        }
    }

    /// Wire and storage ordinal (migration 0561).
    pub fn ordinal(self) -> u8 {
        match self {
            Self::Renown => 1,
            Self::Vigour => 2,
            Self::Tenacity => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayKind {
    Outfit,
    Mount,
    Creature,
}

impl DisplayKind {
    pub fn from_ordinal(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::Outfit),
            2 => Some(Self::Mount),
            3 => Some(Self::Creature),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    WrongItem,
    WrongDisplayKind,
    UnknownContent,
    InvalidVariant,
}

/// Content-owned lookup used instead of guessing Tibia/Canary numeric ids.
pub trait PodiumContent {
    fn has_outfit(&self, key: &str) -> bool;
    fn has_mount(&self, key: &str) -> bool;
    /// A creature the podium may show; `boss` is true for the podium of vigour.
    fn has_creature(&self, key: &str, boss: bool) -> bool;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Display<'a> {
    pub kind: DisplayKind,
    pub key: &'a str,
    pub direction: u8,
}

pub fn admit<C: PodiumContent>(
    item: &str,
    display: Display<'_>,
    content: &C,
) -> Result<PodiumKind, Refusal> {
    admit_kind(
        PodiumKind::of(item).ok_or(Refusal::WrongItem)?,
        display,
        content,
    )
}

/// [`admit`] for a podium whose kind the store already resolved from its item.
pub fn admit_kind<C: PodiumContent>(
    podium: PodiumKind,
    display: Display<'_>,
    content: &C,
) -> Result<PodiumKind, Refusal> {
    if display.direction > 3 {
        return Err(Refusal::InvalidVariant);
    }
    let known = match (podium, display.kind) {
        (PodiumKind::Renown, DisplayKind::Outfit) => content.has_outfit(display.key),
        (PodiumKind::Renown, DisplayKind::Mount) => content.has_mount(display.key),
        (PodiumKind::Vigour, DisplayKind::Creature) => content.has_creature(display.key, true),
        (PodiumKind::Tenacity, DisplayKind::Creature) => content.has_creature(display.key, false),
        _ => return Err(Refusal::WrongDisplayKind),
    };
    known.then_some(podium).ok_or(Refusal::UnknownContent)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Known;
    impl PodiumContent for Known {
        fn has_outfit(&self, k: &str) -> bool {
            k == "oteryn:content.outfit.citizen"
        }
        fn has_mount(&self, k: &str) -> bool {
            k == "oteryn:content.mount.widow_queen"
        }
        fn has_creature(&self, k: &str, boss: bool) -> bool {
            if boss {
                k == "oteryn:creature.ferumbras"
            } else {
                k == "oteryn:creature.rat"
            }
        }
    }
    #[test]
    fn podium_matrix_and_content_are_closed() {
        let d = |kind, key| Display {
            kind,
            key,
            direction: 2,
        };
        let renown = RENOWN_ITEMS[1];
        let tenacity = TENACITY_ITEMS[0];
        let outfit = d(DisplayKind::Outfit, "oteryn:content.outfit.citizen");
        let mount = d(DisplayKind::Mount, "oteryn:content.mount.widow_queen");
        let boss = d(DisplayKind::Creature, "oteryn:creature.ferumbras");
        let rat = d(DisplayKind::Creature, "oteryn:creature.rat");
        assert_eq!(admit(renown, outfit, &Known), Ok(PodiumKind::Renown));
        assert_eq!(admit(renown, mount, &Known), Ok(PodiumKind::Renown));
        assert_eq!(admit(VIGOUR_ITEM, boss, &Known), Ok(PodiumKind::Vigour));
        assert_eq!(admit(tenacity, rat, &Known), Ok(PodiumKind::Tenacity));
        assert_eq!(
            admit(VIGOUR_ITEM, rat, &Known),
            Err(Refusal::UnknownContent)
        );
        assert_eq!(admit(tenacity, boss, &Known), Err(Refusal::UnknownContent));
        assert_eq!(
            admit(VIGOUR_ITEM, mount, &Known),
            Err(Refusal::WrongDisplayKind)
        );
        assert_eq!(admit(renown, rat, &Known), Err(Refusal::WrongDisplayKind));
        assert_eq!(
            admit(tenacity, outfit, &Known),
            Err(Refusal::WrongDisplayKind)
        );
        assert_eq!(
            admit(
                renown,
                d(DisplayKind::Outfit, "oteryn:content.outfit.invented"),
                &Known
            ),
            Err(Refusal::UnknownContent)
        );
        assert_eq!(
            admit("oteryn:content.item.podium_of_renown", outfit, &Known),
            Err(Refusal::WrongItem)
        );
        assert_eq!(
            admit(
                renown,
                Display {
                    direction: 4,
                    ..outfit
                },
                &Known
            ),
            Err(Refusal::InvalidVariant)
        );
        assert_eq!(
            [RENOWN_ITEMS[0], VIGOUR_ITEM, TENACITY_ITEMS[1]]
                .map(|i| PodiumKind::of(i).map(PodiumKind::ordinal)),
            [Some(1), Some(2), Some(3)]
        );
        assert_eq!(PodiumKind::from_ordinal(2), Some(PodiumKind::Vigour));
        assert_eq!(PodiumKind::from_ordinal(0), None);
        assert_eq!(DisplayKind::from_ordinal(3), Some(DisplayKind::Creature));
        assert_eq!(DisplayKind::from_ordinal(4), None);
    }
}
