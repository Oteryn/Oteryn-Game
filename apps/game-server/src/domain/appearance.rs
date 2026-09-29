//! Pure Character appearance selection rules (`OTERYN_GAME_CHARACTER_APPEARANCE_OWNER_DECISION`
//! §4.2-§4.4, owner decisions D47, D49 and D61).
//!
//! Content definitions and the account's `AccountUnlock` facts are supplied by the caller; the
//! durable selection, its fence and the change command are separate children (APP-1, APP-3).

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Sex {
    Female,
    Male,
}

/// Head, body, legs and feet palette indices.
pub type Colours = [u16; 4];

/// The selected appearance (§4.2). `addons` holds addon bits of the outfit; bit `i` is addon
/// `i + 1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppearanceSelection<K> {
    pub outfit_key: K,
    pub colours: Colours,
    pub addons: u8,
    pub mount_key: Option<K>,
}

/// The look a world shows for a character. No mount is shown while mount activation is deferred
/// (§4.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisplayedLook<'a, K> {
    pub outfit_key: &'a K,
    pub colours: Colours,
    pub addons: u8,
}

/// The account's earned-unlock fact for one cosmetic (#707 decision §4.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnlockFact {
    Absent,
    /// Unlocked, and the active definition is compatible with the unlock's recorded provenance.
    Compatible,
    /// Unlocked under a provenance the active definition is not compatible with (D49).
    Incompatible,
}

/// The active content of this world.
pub trait AppearanceContent<K> {
    fn palette_size(&self) -> u16;
    /// The outfit's addon count, or `None` when the key is not in the active content.
    fn outfit_addon_count(&self, outfit: &K) -> Option<u8>;
    fn outfit_is_starter(&self, outfit: &K, sex: Sex) -> bool;
    fn addon_is_starter(&self, outfit: &K, sex: Sex, addon: u8) -> bool;
    fn mount_exists(&self, mount: &K) -> bool;
}

/// The account's `AccountUnlock` set.
pub trait AccountUnlocks<K> {
    fn outfit(&self, outfit: &K) -> UnlockFact;
    fn addon(&self, outfit: &K, addon: u8) -> UnlockFact;
    fn mount(&self, mount: &K) -> UnlockFact;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppearanceRejection {
    UnknownOutfit,
    OutfitNotAuthorized,
    AddonOutOfRange { addon: u8 },
    AddonNotAuthorized { addon: u8 },
    ColourOutOfPalette { slot: usize },
    UnknownMount,
    MountNotAuthorized,
}

/// §4.3: a selection is allowed only if the outfit, every addon and the mount are each authorized
/// by starter content or a compatible earned unlock, and every colour is in the palette. The Store
/// path stays unavailable until the gap register §32 decision, so it never authorizes. Premium
/// flags are not checked in V1 (D61).
///
/// # Errors
///
/// Returns the first failed rule, in the order outfit, addons, colours, mount.
pub fn validate_selection<K>(
    selection: &AppearanceSelection<K>,
    sex: Sex,
    content: &impl AppearanceContent<K>,
    unlocks: &impl AccountUnlocks<K>,
) -> Result<(), AppearanceRejection> {
    let outfit = &selection.outfit_key;
    let addon_count = content
        .outfit_addon_count(outfit)
        .ok_or(AppearanceRejection::UnknownOutfit)?;
    if !(content.outfit_is_starter(outfit, sex) || unlocks.outfit(outfit) == UnlockFact::Compatible)
    {
        return Err(AppearanceRejection::OutfitNotAuthorized);
    }
    for addon in 1..=8_u8 {
        if selection.addons & (1 << (addon - 1)) == 0 {
            continue;
        }
        if addon > addon_count {
            return Err(AppearanceRejection::AddonOutOfRange { addon });
        }
        if !(content.addon_is_starter(outfit, sex, addon)
            || unlocks.addon(outfit, addon) == UnlockFact::Compatible)
        {
            return Err(AppearanceRejection::AddonNotAuthorized { addon });
        }
    }
    let palette_size = content.palette_size();
    if let Some(slot) = selection
        .colours
        .iter()
        .position(|colour| *colour >= palette_size)
    {
        return Err(AppearanceRejection::ColourOutOfPalette { slot });
    }
    if let Some(mount) = &selection.mount_key {
        if !content.mount_exists(mount) {
            return Err(AppearanceRejection::UnknownMount);
        }
        if unlocks.mount(mount) != UnlockFact::Compatible {
            return Err(AppearanceRejection::MountNotAuthorized);
        }
    }
    Ok(())
}

/// §4.4: a world shows the stored selection if it is valid there, otherwise the content's fallback
/// look for the sex. The stored selection is only borrowed, so it is never changed or
/// reinterpreted (D49).
pub fn displayed_look<'a, K>(
    stored: &'a AppearanceSelection<K>,
    fallback: &'a AppearanceSelection<K>,
    sex: Sex,
    content: &impl AppearanceContent<K>,
    unlocks: &impl AccountUnlocks<K>,
) -> DisplayedLook<'a, K> {
    let shown = if validate_selection(stored, sex, content, unlocks).is_ok() {
        stored
    } else {
        fallback
    };
    DisplayedLook {
        outfit_key: &shown.outfit_key,
        colours: shown.colours,
        addons: shown.addons,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    const CITIZEN: &str = "outfit.citizen";
    const HUNTER: &str = "outfit.hunter";
    const HORSE: &str = "mount.horse";
    const LOST: &str = "outfit.removed";

    struct Content;

    impl AppearanceContent<&'static str> for Content {
        fn palette_size(&self) -> u16 {
            133
        }
        fn outfit_addon_count(&self, outfit: &&'static str) -> Option<u8> {
            match *outfit {
                CITIZEN | HUNTER => Some(2),
                _ => None,
            }
        }
        fn outfit_is_starter(&self, outfit: &&'static str, sex: Sex) -> bool {
            *outfit == CITIZEN && sex == Sex::Male
        }
        fn addon_is_starter(&self, _outfit: &&'static str, _sex: Sex, _addon: u8) -> bool {
            false
        }
        fn mount_exists(&self, mount: &&'static str) -> bool {
            *mount == HORSE
        }
    }

    #[derive(Default)]
    struct Unlocks {
        outfits: BTreeMap<&'static str, UnlockFact>,
        addons: BTreeMap<(&'static str, u8), UnlockFact>,
        mounts: BTreeMap<&'static str, UnlockFact>,
    }

    impl AccountUnlocks<&'static str> for Unlocks {
        fn outfit(&self, outfit: &&'static str) -> UnlockFact {
            self.outfits
                .get(outfit)
                .copied()
                .unwrap_or(UnlockFact::Absent)
        }
        fn addon(&self, outfit: &&'static str, addon: u8) -> UnlockFact {
            self.addons
                .get(&(*outfit, addon))
                .copied()
                .unwrap_or(UnlockFact::Absent)
        }
        fn mount(&self, mount: &&'static str) -> UnlockFact {
            self.mounts
                .get(mount)
                .copied()
                .unwrap_or(UnlockFact::Absent)
        }
    }

    fn selection(outfit: &'static str) -> AppearanceSelection<&'static str> {
        AppearanceSelection {
            outfit_key: outfit,
            colours: [0, 44, 132, 7],
            addons: 0,
            mount_key: None,
        }
    }

    fn hunter_unlocked() -> Unlocks {
        let mut unlocks = Unlocks::default();
        unlocks.outfits.insert(HUNTER, UnlockFact::Compatible);
        unlocks
    }

    #[test]
    fn a_starter_outfit_needs_no_unlock_but_only_for_its_sex() {
        let unlocks = Unlocks::default();
        assert_eq!(
            validate_selection(&selection(CITIZEN), Sex::Male, &Content, &unlocks),
            Ok(())
        );
        assert_eq!(
            validate_selection(&selection(CITIZEN), Sex::Female, &Content, &unlocks),
            Err(AppearanceRejection::OutfitNotAuthorized)
        );
    }

    #[test]
    fn an_earned_outfit_needs_a_compatible_unlock() {
        assert_eq!(
            validate_selection(
                &selection(HUNTER),
                Sex::Female,
                &Content,
                &hunter_unlocked()
            ),
            Ok(())
        );
        let mut incompatible = Unlocks::default();
        incompatible
            .outfits
            .insert(HUNTER, UnlockFact::Incompatible);
        for unlocks in [Unlocks::default(), incompatible] {
            assert_eq!(
                validate_selection(&selection(HUNTER), Sex::Female, &Content, &unlocks),
                Err(AppearanceRejection::OutfitNotAuthorized)
            );
        }
        assert_eq!(
            validate_selection(&selection(LOST), Sex::Female, &Content, &hunter_unlocked()),
            Err(AppearanceRejection::UnknownOutfit)
        );
    }

    #[test]
    fn addons_must_be_in_range_and_each_authorized() {
        let mut unlocks = hunter_unlocked();
        unlocks.addons.insert((HUNTER, 1), UnlockFact::Compatible);
        let mut choice = selection(HUNTER);
        choice.addons = 0b01;
        assert_eq!(
            validate_selection(&choice, Sex::Male, &Content, &unlocks),
            Ok(())
        );
        choice.addons = 0b11;
        assert_eq!(
            validate_selection(&choice, Sex::Male, &Content, &unlocks),
            Err(AppearanceRejection::AddonNotAuthorized { addon: 2 })
        );
        unlocks.addons.insert((HUNTER, 2), UnlockFact::Incompatible);
        assert_eq!(
            validate_selection(&choice, Sex::Male, &Content, &unlocks),
            Err(AppearanceRejection::AddonNotAuthorized { addon: 2 })
        );
        choice.addons = 0b100;
        assert_eq!(
            validate_selection(&choice, Sex::Male, &Content, &unlocks),
            Err(AppearanceRejection::AddonOutOfRange { addon: 3 })
        );
        choice.addons = 0b1000_0000;
        assert_eq!(
            validate_selection(&choice, Sex::Male, &Content, &unlocks),
            Err(AppearanceRejection::AddonOutOfRange { addon: 8 })
        );
    }

    #[test]
    fn every_colour_must_be_within_the_palette() {
        let mut choice = selection(CITIZEN);
        choice.colours = [132, 132, 132, 132];
        assert_eq!(
            validate_selection(&choice, Sex::Male, &Content, &Unlocks::default()),
            Ok(())
        );
        choice.colours[2] = 133;
        assert_eq!(
            validate_selection(&choice, Sex::Male, &Content, &Unlocks::default()),
            Err(AppearanceRejection::ColourOutOfPalette { slot: 2 })
        );
    }

    #[test]
    fn a_mount_must_exist_and_be_unlocked() {
        let mut choice = selection(CITIZEN);
        choice.mount_key = Some(HORSE);
        let mut unlocks = Unlocks::default();
        assert_eq!(
            validate_selection(&choice, Sex::Male, &Content, &unlocks),
            Err(AppearanceRejection::MountNotAuthorized)
        );
        unlocks.mounts.insert(HORSE, UnlockFact::Compatible);
        assert_eq!(
            validate_selection(&choice, Sex::Male, &Content, &unlocks),
            Ok(())
        );
        choice.mount_key = Some(LOST);
        assert_eq!(
            validate_selection(&choice, Sex::Male, &Content, &unlocks),
            Err(AppearanceRejection::UnknownMount)
        );
    }

    #[test]
    fn an_invalid_stored_selection_shows_the_fallback_and_is_kept() {
        let fallback = selection(CITIZEN);
        let mut stored = selection(HUNTER);
        stored.colours = [1, 2, 3, 4];
        stored.mount_key = Some(HORSE);
        let before = stored.clone();
        let mut unlocks = hunter_unlocked();
        unlocks.mounts.insert(HORSE, UnlockFact::Compatible);

        let shown = displayed_look(&stored, &fallback, Sex::Male, &Content, &unlocks);
        assert_eq!(shown.outfit_key, &HUNTER);
        assert_eq!(shown.colours, [1, 2, 3, 4]);

        unlocks.outfits.insert(HUNTER, UnlockFact::Incompatible);
        let shown = displayed_look(&stored, &fallback, Sex::Male, &Content, &unlocks);
        assert_eq!(shown.outfit_key, &CITIZEN);
        assert_eq!(shown.colours, fallback.colours);
        assert_eq!(shown.addons, 0);
        assert_eq!(stored, before);
    }
}
