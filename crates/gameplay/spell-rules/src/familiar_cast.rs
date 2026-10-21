//! Pure vocation selection for the familiar source cast and lifecycle adapters.

use oteryn_gameplay_content::content::spell_data::Vocation;

pub fn base_familiar_vocation(vocation: Vocation) -> &'static str {
    use Vocation::*;
    match vocation {
        Druid | ElderDruid => "druid",
        Sorcerer | MasterSorcerer => "sorcerer",
        Knight | EliteKnight => "knight",
        Paladin | RoyalPaladin => "paladin",
        Monk | ExaltedMonk => "monk",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn promoted_familiar_owners_use_the_same_source_vocation() {
        for (base, promoted, key) in [
            (Vocation::Druid, Vocation::ElderDruid, "druid"),
            (Vocation::Sorcerer, Vocation::MasterSorcerer, "sorcerer"),
            (Vocation::Knight, Vocation::EliteKnight, "knight"),
            (Vocation::Paladin, Vocation::RoyalPaladin, "paladin"),
            (Vocation::Monk, Vocation::ExaltedMonk, "monk"),
        ] {
            assert_eq!(base_familiar_vocation(base), key);
            assert_eq!(base_familiar_vocation(promoted), key);
        }
    }
}
