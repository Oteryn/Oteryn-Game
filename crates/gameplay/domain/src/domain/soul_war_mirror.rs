//! Mirror Image source rules. Crystal201189f mirror_image.lua:113–143;
//! accepted authoring baseline §13.2 Q5b retains a 1HP non-player floor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Apparition {
    Druid,
    Knight,
    Paladin,
    Sorcerer,
    Monk,
}
impl Apparition {
    pub const ALL: [Self; 5] = [
        Self::Druid,
        Self::Knight,
        Self::Paladin,
        Self::Sorcerer,
        Self::Monk,
    ];
    pub fn creature_key(self) -> &'static str {
        match self {
            Self::Druid => "oteryn:creature.druid_s_apparition",
            Self::Knight => "oteryn:creature.knight_s_apparition",
            Self::Paladin => "oteryn:creature.paladin_s_apparition",
            Self::Sorcerer => "oteryn:creature.sorcerer_s_apparition",
            Self::Monk => "oteryn:creature.monk_s_apparition",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectPlayerReaction {
    ReplaceAtFullHealth(Apparition),
    Disappear,
}
/// One uniform draw in 0..200 exactly represents 70% / four times 7.5%.
/// A vocationless player's 70% empty create removes the source image.
pub fn direct_player_reaction(
    vocation: Option<Apparition>,
    draw: u16,
) -> Option<DirectPlayerReaction> {
    if draw >= 200 {
        return None;
    }
    if draw < 140 {
        return Some(match vocation {
            Some(own) => DirectPlayerReaction::ReplaceAtFullHealth(own),
            None => DirectPlayerReaction::Disappear,
        });
    }
    let (width, excluded) = match vocation {
        Some(own) => (15, Some(own)),
        None => (12, None),
    };
    Apparition::ALL
        .into_iter()
        .filter(|target| Some(*target) != excluded)
        .nth(usize::from((draw - 140) / width))
        .map(DirectPlayerReaction::ReplaceAtFullHealth)
}
/// Only the player's own damage transforms it; summons/familiars cannot kill it.
pub fn non_player_damage(health: u32, requested: u32) -> u32 {
    requested.min(health.saturating_sub(1))
}
