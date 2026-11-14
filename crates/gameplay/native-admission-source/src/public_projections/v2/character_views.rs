//! Accepted G-4 revision 3 character views. Catalogue, ownership and committed-cut
//! evidence belongs to the source reader; these checks enforce the wire bounds.
use super::{id, key, number};
use serde::{Deserialize, Serialize};

pub fn revision_key(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.as_bytes()[0].is_ascii_alphanumeric()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
}
fn character(world: &str, revision: &str) -> bool {
    id(world) && number(revision, false, u64::MAX)
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Skills {
    pub world_id: String,
    pub character_revision: String,
    pub level: String,
    pub total_experience: String,
    pub magic_level: u16,
    pub fist_level: u16,
    pub club_level: u16,
    pub sword_level: u16,
    pub axe_level: u16,
    pub distance_level: u16,
    pub shielding_level: u16,
    pub fishing_level: u16,
}
impl Skills {
    pub(super) fn valid(&self) -> bool {
        character(&self.world_id, &self.character_revision)
            && number(&self.level, false, u32::MAX.into())
            && number(&self.total_experience, true, i64::MAX as u64)
            && self.magic_level <= 1000
            && [
                self.fist_level,
                self.club_level,
                self.sword_level,
                self.axe_level,
                self.distance_level,
                self.shielding_level,
                self.fishing_level,
            ]
            .iter()
            .all(|n| (10..=1000).contains(n))
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Earned {
    pub key: String,
    pub earned_at: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Achievements {
    pub world_id: String,
    pub character_revision: String,
    pub catalogue_revision: String,
    pub earned: Vec<Earned>,
}
impl Achievements {
    pub(super) fn valid(&self) -> bool {
        character(&self.world_id, &self.character_revision)
            && revision_key(&self.catalogue_revision)
            && self.earned.len() <= 1024
            && self.earned.iter().all(|e| {
                e.key
                    .strip_prefix("oteryn:achievement/")
                    .is_some_and(|suffix| {
                        !suffix.is_empty()
                            && suffix.len() <= 141
                            && e.key.len() <= 160
                            && suffix
                                .bytes()
                                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
                    })
                    && number(&e.earned_at, false, i64::MAX as u64)
            })
            && self.earned.windows(2).all(|p| p[0].key < p[1].key)
    }
}
/// Declaration order is the accepted EquipmentSlot wire order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Slot {
    Head,
    Necklace,
    Container,
    Armor,
    RightHand,
    LeftHand,
    Legs,
    Feet,
    Ring,
    Ammo,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Equipped {
    pub slot: Slot,
    pub item_key: String,
    pub item_revision: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Equipment {
    pub world_id: String,
    pub character_revision: String,
    pub slots: Vec<Equipped>,
}
impl Equipment {
    pub(super) fn valid(&self) -> bool {
        character(&self.world_id, &self.character_revision)
            && self.slots.len() <= 10
            && self
                .slots
                .iter()
                .all(|s| key(&s.item_key) && revision_key(&s.item_revision))
            && self.slots.windows(2).all(|p| p[0].slot < p[1].slot)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mount {
    pub mount_key: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Outfit {
    pub world_id: String,
    pub character_revision: String,
    pub content_revision: String,
    pub outfit_key: String,
    pub colours: [u16; 4],
    pub addons: u8,
    pub mount: Vec<Mount>,
}
impl Outfit {
    pub(super) fn valid(&self) -> bool {
        character(&self.world_id, &self.character_revision)
            && revision_key(&self.content_revision)
            && key(&self.outfit_key)
            && self.mount.len() <= 1
            && self.mount.iter().all(|m| key(&m.mount_key))
    }
}
