use super::{
    ContentError, ContentLockBinding, PackageManifestBinding, ProductionAtom, ProductionKey,
};
use crate::foundation::WorldId;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use std::collections::BTreeSet;

pub const REFERENCE_PLAYABLE_CONTENT_PROFILE_ID: &str = "REFERENCE_PLAYABLE_CONTENT_PROFILE/v1";
pub const REFERENCE_PLAYABLE_CAPABILITY_PROFILE: &str = "content:reference-playable-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DefinitionFamily {
    Terrain,
    Presentation,
    LocalObject,
    Creature,
    Item,
    Loot,
    Ability,
    Effect,
    Formula,
    Behavior,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DefinitionRevisionRef(ProductionAtom);

impl DefinitionRevisionRef {
    pub fn new(value: &str) -> Result<Self, ContentError> {
        Ok(Self(ProductionAtom::new(
            "reference-playable definition revision",
            value,
        )?))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TypedDefinitionRef {
    family: DefinitionFamily,
    key: ProductionKey,
    revision: DefinitionRevisionRef,
}

impl TypedDefinitionRef {
    pub fn new(
        family: DefinitionFamily,
        key: ProductionKey,
        revision: DefinitionRevisionRef,
    ) -> Self {
        Self {
            family,
            key,
            revision,
        }
    }

    pub const fn family(&self) -> DefinitionFamily {
        self.family
    }

    pub fn key(&self) -> &ProductionKey {
        &self.key
    }

    pub fn revision(&self) -> &DefinitionRevisionRef {
        &self.revision
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceEffectFamily {
    Damage,
    Heal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceAbilityDefinition {
    /// Authored structural references. Order and repeated entries are preserved, but this field
    /// does not define execution order or multi-hit behavior.
    pub effects: Vec<TypedDefinitionRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceEffectDefinition {
    pub family: ReferenceEffectFamily,
    /// Opaque authored formula endpoint. Formula evaluation semantics are outside this profile.
    pub formula: TypedDefinitionRef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReferenceFormulaDefinition;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceItemPhysicalClass {
    Unknown,
    Physical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceItemStackClass {
    Unknown,
    NonStackable,
    StackCapable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReferenceItemDestination {
    CharacterInventory,
}

pub const REFERENCE_ITEM_MAX_NAME_BYTES: usize = 46;
pub const REFERENCE_ITEM_MAX_DESCRIPTION_BYTES: usize = 200;
pub const REFERENCE_ITEM_MAX_EQUIPMENT_PATTERNS: usize = 2;
pub const REFERENCE_ITEM_MAX_ADDITIONAL_SLOTS: usize = 9;
pub const REFERENCE_ITEM_MAX_EXCLUSIVE_GROUPS: usize = 2;
pub const REFERENCE_ITEM_MAX_BASE_VOCATIONS: usize = 5;
pub const REFERENCE_ITEM_MAX_WEAPON_ELEMENTS: usize = 5;
pub const REFERENCE_ITEM_MAX_RESISTANCES: usize = 12;
pub const REFERENCE_ITEM_MAX_MODIFIERS: usize = 37;
pub const REFERENCE_ITEM_MAX_IMBUEMENT_FAMILIES: usize = 20;
pub const REFERENCE_ITEM_MAX_IMBUEMENT_SLOTS: u8 = 3;
pub const REFERENCE_ITEM_REGISTRY_SIZE: u32 = 38_157;
pub const REFERENCE_ITEM_EXPLICIT_UNSUPPORTED_V1: [&str; 7] = [
    "presentation.appearance_binding",
    "presentation.aliases",
    "presentation.tags",
    "equipment.compatibility_rule",
    "modifier.augment_binding",
    "trade_restrictions.account_binding_policy",
    "trade_restrictions.character_binding_policy",
];

/// Truth-bearing immutable Item field. `Known(false)` and `Known(0)` are deliberately
/// different from `Unknown`, `NotApplicable` and `Conflict`.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(tag = "state", content = "value", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReferenceItemField<T> {
    #[default]
    Unknown,
    NotApplicable,
    Conflict,
    Known(T),
}

#[derive(Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum ReferenceItemFieldState {
    Unknown,
    NotApplicable,
    Conflict,
    Known,
}

#[derive(Default)]
enum ReferenceItemFieldValue<T> {
    #[default]
    Missing,
    Present(T),
}

fn deserialize_reference_item_field_value<'de, D, T>(
    deserializer: D,
) -> Result<ReferenceItemFieldValue<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(ReferenceItemFieldValue::Present)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, bound(deserialize = "T: Deserialize<'de>"))]
struct ReferenceItemFieldEnvelope<T> {
    state: ReferenceItemFieldState,
    #[serde(default, deserialize_with = "deserialize_reference_item_field_value")]
    value: ReferenceItemFieldValue<T>,
}

impl<'de, T> Deserialize<'de> for ReferenceItemField<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let envelope = ReferenceItemFieldEnvelope::<T>::deserialize(deserializer)?;
        match (envelope.state, envelope.value) {
            (ReferenceItemFieldState::Unknown, ReferenceItemFieldValue::Missing) => {
                Ok(Self::Unknown)
            }
            (ReferenceItemFieldState::NotApplicable, ReferenceItemFieldValue::Missing) => {
                Ok(Self::NotApplicable)
            }
            (ReferenceItemFieldState::Conflict, ReferenceItemFieldValue::Missing) => {
                Ok(Self::Conflict)
            }
            (ReferenceItemFieldState::Known, ReferenceItemFieldValue::Present(value)) => {
                Ok(Self::Known(value))
            }
            (ReferenceItemFieldState::Known, ReferenceItemFieldValue::Missing) => {
                Err(de::Error::missing_field("value"))
            }
            (_, ReferenceItemFieldValue::Present(_)) => Err(de::Error::custom(
                "Reference Item non-KNOWN field state cannot carry a value",
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReferenceItemGroupKey(ProductionKey);

impl ReferenceItemGroupKey {
    pub fn new(value: &str) -> Result<Self, ContentError> {
        Ok(Self(ProductionKey::new(value)?))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Serialize for ReferenceItemGroupKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ReferenceItemGroupKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(&value).map_err(serde::de::Error::custom)
    }
}

macro_rules! item_enum {
    ($name:ident { $($variant:ident = $value:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
        #[repr(u8)]
        #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
        pub enum $name { $($variant = $value),+ }

        impl $name {
            pub const fn wire(self) -> u8 { self as u8 }

            pub fn from_wire(value: u8) -> Result<Self, ContentError> {
                match value {
                    $($value => Ok(Self::$variant),)+
                    _ => Err(ContentError::InvalidArtifact(concat!("unknown ", stringify!($name)))),
                }
            }
        }
    };
}

item_enum!(ReferenceItemType {
    Bed = 1, Carpet = 2, Container = 3, Depot = 4, Door = 5, Dummy = 6,
    Key = 7, Ladder = 8, MagicField = 9, Mailbox = 10, RewardChest = 11,
    Rune = 12, Teleport = 13, TrashHolder = 14,
});
item_enum!(ReferenceEquipmentSlot {
    Head = 1, Torso = 2, Legs = 3, Feet = 4, Weapon = 5, Shield = 6,
    Amulet = 7, Ring = 8, Container = 9, Extra = 10,
});
item_enum!(ReferenceBaseVocation {
    Druid = 1, Knight = 2, Monk = 3, Paladin = 4, Sorcerer = 5,
});
item_enum!(ReferenceWeaponType {
    Ammunition = 1, Axe = 2, Club = 3, Distance = 4, Fist = 5,
    Shield = 6, Spellbook = 7, Sword = 8, Wand = 9,
});
item_enum!(ReferenceAmmoType { Arrow = 1, Bolt = 2 });
item_enum!(ReferenceFluidType {
    Beer = 1, Blood = 2, Lemonade = 3, Mud = 4, Rum = 5, Slime = 6,
    Water = 7, Wine = 8,
});
item_enum!(ReferenceTemporalMode {
    DurableAbsoluteDeadline = 1, AuthoritativeActiveTimeBudget = 2,
});
item_enum!(ReferenceWeaponElement {
    Death = 1, Earth = 2, Energy = 3, Fire = 4, Ice = 5,
});
item_enum!(ReferenceModifierElement {
    Death = 1, Earth = 2, Energy = 3, Fire = 4, Holy = 5, Ice = 6,
});
item_enum!(ReferenceResistanceKind {
    Death = 1, Drown = 2, Earth = 3, Energy = 4, Fire = 5, Holy = 6,
    Ice = 7, LifeDrain = 8, ManaDrain = 9, Physical = 10, Poison = 11,
    FireField = 12,
});
item_enum!(ReferenceImbuementFamily {
    CriticalHit = 1, ElementalDamage = 2, ProtectionDeath = 3,
    ProtectionEarth = 4, ProtectionEnergy = 5, ProtectionFire = 6,
    ProtectionHoly = 7, ProtectionIce = 8, IncreaseCapacity = 9,
    IncreaseSpeed = 10, LifeLeech = 11, ManaLeech = 12,
    ParalysisRemoval = 13, SkillAxe = 14, SkillClub = 15,
    SkillDistance = 16, SkillFist = 17, SkillMagicLevel = 18,
    SkillShielding = 19, SkillSword = 20,
});
item_enum!(ReferenceTransformKind {
    Rotate = 1, Wrap = 2, Use = 3, Equip = 4, Deequip = 5,
    Male = 6, Female = 7, Destroy = 8, Decay = 9, WriteOnce = 10,
});
item_enum!(ReferenceSkillModifierKind {
    ElementalBond = 1, Invisibility = 2, ManaShield = 3, Mantra = 4,
    CleavePercent = 5, CriticalHitChance = 6, CriticalHitDamage = 7,
    DeathMagicLevelPoints = 8, EarthMagicLevelPoints = 9,
    EnergyMagicLevelPoints = 10, FireMagicLevelPoints = 11,
    HealingMagicLevelPoints = 12, HealthGain = 13, HealthTicks = 14,
    HolyMagicLevelPoints = 15, IceMagicLevelPoints = 16,
    LifeLeechAmount = 17, LifeLeechChance = 18, MagicLevelPoints = 19,
    MagicShieldCapacityFlat = 20, MagicShieldCapacityPercent = 21,
    ManaGain = 22, ManaLeechAmount = 23, ManaLeechChance = 24,
    ManaTicks = 25, PerfectShotDamage = 26, PerfectShotRange = 27,
    ReflectDamage = 28, SkillAxe = 29, SkillClub = 30, SkillDistance = 31,
    SkillFist = 32, SkillShield = 33, SkillSword = 34, Speed = 35,
    SuppressDrown = 36, SuppressDrunk = 37,
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ReferenceSignedPoints(pub i32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ReferenceCells(pub u16);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ReferenceMilliseconds(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceRationalPercent {
    pub numerator: i64,
    pub denominator: u64,
}

impl ReferenceRationalPercent {
    pub fn new(numerator: i64, denominator: u64) -> Result<Self, ContentError> {
        let value = Self {
            numerator,
            denominator,
        };
        value.validate()?;
        Ok(value)
    }

    pub(crate) fn validate(self) -> Result<(), ContentError> {
        if self.denominator == 0 || gcd_u64(self.numerator.unsigned_abs(), self.denominator) != 1 {
            return Err(ContentError::InvalidArtifact(
                "Reference Item rational percent is not canonical",
            ));
        }
        Ok(())
    }
}

fn gcd_u64(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceItemTarget {
    pub key: String,
    pub revision: String,
}

impl ReferenceItemTarget {
    pub fn new(key: &str, revision: &str) -> Result<Self, ContentError> {
        ProductionKey::new(key)?;
        DefinitionRevisionRef::new(revision)?;
        Ok(Self {
            key: key.to_owned(),
            revision: revision.to_owned(),
        })
    }

    pub(crate) fn typed_ref(&self) -> Result<TypedDefinitionRef, ContentError> {
        Ok(TypedDefinitionRef::new(
            DefinitionFamily::Item,
            ProductionKey::new(&self.key)?,
            DefinitionRevisionRef::new(&self.revision)?,
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceItemPresentation {
    pub name: ReferenceItemField<String>,
    pub description: ReferenceItemField<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceItemClassification {
    pub item_type: ReferenceItemField<ReferenceItemType>,
    pub capabilities: ReferenceItemField<[ReferenceItemField<bool>; 24]>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceItemPhysical {
    pub weight: ReferenceItemField<u32>,
    pub movable: ReferenceItemField<bool>,
    pub pickupable: ReferenceItemField<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceItemStack {
    pub stackable: ReferenceItemField<bool>,
    pub stack_max: ReferenceItemField<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceEquipmentPattern {
    pub pattern_id: u8,
    pub primary_slot: ReferenceItemField<ReferenceEquipmentSlot>,
    pub additional_reserved_slots: ReferenceItemField<Vec<ReferenceEquipmentSlot>>,
    pub mutually_exclusive_groups: ReferenceItemField<Vec<ReferenceItemGroupKey>>,
    pub vocations: ReferenceItemField<Vec<ReferenceBaseVocation>>,
    pub level: ReferenceItemField<u16>,
    /// `Known` is rejected in v1 because no compatibility grammar is accepted.
    pub compatibility_rule: ReferenceItemField<()>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceItemEquipment {
    pub patterns: ReferenceItemField<Vec<ReferenceEquipmentPattern>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceElementalAttack {
    pub element: ReferenceWeaponElement,
    pub points: ReferenceItemField<ReferenceSignedPoints>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceItemWeapon {
    pub weapon_type: ReferenceItemField<ReferenceWeaponType>,
    pub attack: ReferenceItemField<ReferenceSignedPoints>,
    pub defense: ReferenceItemField<ReferenceSignedPoints>,
    pub extra_defense: ReferenceItemField<ReferenceSignedPoints>,
    pub range: ReferenceItemField<ReferenceCells>,
    pub hit_chance: ReferenceItemField<ReferenceRationalPercent>,
    pub max_hit_chance: ReferenceItemField<ReferenceRationalPercent>,
    pub ammunition: ReferenceItemField<ReferenceAmmoType>,
    pub elemental: ReferenceItemField<Vec<ReferenceElementalAttack>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceResistance {
    pub kind: ReferenceResistanceKind,
    pub percent: ReferenceItemField<ReferenceRationalPercent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceItemProtection {
    pub armor: ReferenceItemField<ReferenceSignedPoints>,
    pub resistances: ReferenceItemField<Vec<ReferenceResistance>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReferenceModifierParameter {
    Boolean(bool),
    SignedPoints(ReferenceSignedPoints),
    Cells(ReferenceCells),
    Milliseconds(ReferenceMilliseconds),
    RationalPercent(ReferenceRationalPercent),
    Element(ReferenceModifierElement),
}

macro_rules! strict_modifier_parameter_kind {
    ($name:ident, $variant:ident, $wire:literal) => {
        #[derive(Deserialize)]
        enum $name {
            #[serde(rename = $wire)]
            $variant,
        }
    };
}

strict_modifier_parameter_kind!(ReferenceModifierBooleanKind, Boolean, "BOOLEAN");
strict_modifier_parameter_kind!(
    ReferenceModifierSignedPointsKind,
    SignedPoints,
    "SIGNED_POINTS"
);
strict_modifier_parameter_kind!(ReferenceModifierCellsKind, Cells, "CELLS");
strict_modifier_parameter_kind!(
    ReferenceModifierMillisecondsKind,
    Milliseconds,
    "MILLISECONDS"
);
strict_modifier_parameter_kind!(
    ReferenceModifierRationalPercentKind,
    RationalPercent,
    "RATIONAL_PERCENT"
);
strict_modifier_parameter_kind!(ReferenceModifierElementKind, Element, "ELEMENT");

#[derive(Deserialize)]
#[serde(
    deny_unknown_fields,
    bound(deserialize = "K: Deserialize<'de>, V: Deserialize<'de>")
)]
struct ReferenceModifierParameterCase<K, V> {
    #[serde(rename = "kind")]
    _kind: K,
    value: V,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ReferenceModifierParameterEnvelope {
    Boolean(ReferenceModifierParameterCase<ReferenceModifierBooleanKind, bool>),
    SignedPoints(
        ReferenceModifierParameterCase<ReferenceModifierSignedPointsKind, ReferenceSignedPoints>,
    ),
    Cells(ReferenceModifierParameterCase<ReferenceModifierCellsKind, ReferenceCells>),
    Milliseconds(
        ReferenceModifierParameterCase<ReferenceModifierMillisecondsKind, ReferenceMilliseconds>,
    ),
    RationalPercent(
        ReferenceModifierParameterCase<
            ReferenceModifierRationalPercentKind,
            ReferenceRationalPercent,
        >,
    ),
    Element(ReferenceModifierParameterCase<ReferenceModifierElementKind, ReferenceModifierElement>),
}

impl<'de> Deserialize<'de> for ReferenceModifierParameter {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(
            match ReferenceModifierParameterEnvelope::deserialize(deserializer)? {
                ReferenceModifierParameterEnvelope::Boolean(case) => Self::Boolean(case.value),
                ReferenceModifierParameterEnvelope::SignedPoints(case) => {
                    Self::SignedPoints(case.value)
                }
                ReferenceModifierParameterEnvelope::Cells(case) => Self::Cells(case.value),
                ReferenceModifierParameterEnvelope::Milliseconds(case) => {
                    Self::Milliseconds(case.value)
                }
                ReferenceModifierParameterEnvelope::RationalPercent(case) => {
                    Self::RationalPercent(case.value)
                }
                ReferenceModifierParameterEnvelope::Element(case) => Self::Element(case.value),
            },
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceModifierBinding {
    pub kind: ReferenceSkillModifierKind,
    /// Profile-local closed capacity ID; an accepted ruleset binding remains independently gated.
    pub target_domain: ReferenceItemField<u8>,
    /// Profile-local closed capacity ID; an accepted ruleset binding remains independently gated.
    pub evaluation_phase: ReferenceItemField<u8>,
    pub priority: ReferenceItemField<i16>,
    pub parameter: ReferenceItemField<ReferenceModifierParameter>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceItemSkillModifiers {
    pub modifiers: ReferenceItemField<Vec<ReferenceModifierBinding>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceItemCharges {
    pub count: ReferenceItemField<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceItemTemporal {
    pub consumption_mode: ReferenceItemField<ReferenceTemporalMode>,
    pub duration: ReferenceItemField<ReferenceMilliseconds>,
    pub stop_duration: ReferenceItemField<bool>,
    pub decay_target: ReferenceItemField<ReferenceItemTarget>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceItemContainer {
    pub capacity: ReferenceItemField<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ReferenceImbuementTier {
    Two,
    Three,
    Ten,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceImbuementAllowance {
    pub family: ReferenceImbuementFamily,
    pub tier: ReferenceImbuementTier,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceItemImbuement {
    pub slot_count: ReferenceItemField<u8>,
    pub allowed_family_tiers: ReferenceItemField<Vec<ReferenceImbuementAllowance>>,
    pub excluded_families: ReferenceItemField<Vec<ReferenceImbuementFamily>>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceTransformTarget {
    pub kind: ReferenceTransformKind,
    pub target: ReferenceItemField<ReferenceItemTarget>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceItemUseTransform {
    /// Exactly ten ordered entries when the outer group is known.
    pub targets: Vec<ReferenceTransformTarget>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceItemTradeRestrictions {
    pub tradeable: ReferenceItemField<bool>,
    pub marketable: ReferenceItemField<bool>,
    pub vocations: ReferenceItemField<Vec<ReferenceBaseVocation>>,
    /// `Known` is rejected in v1; immutable binding policy grammar is unaccepted.
    pub account_binding_policy: ReferenceItemField<()>,
    /// `Known` is rejected in v1; immutable binding policy grammar is unaccepted.
    pub character_binding_policy: ReferenceItemField<()>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceItemFluid {
    pub fluid_type: ReferenceItemField<ReferenceFluidType>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceItemReadableWriteable {
    pub readable: ReferenceItemField<bool>,
    pub writeable: ReferenceItemField<bool>,
    pub distance_read: ReferenceItemField<bool>,
    pub max_text_length: ReferenceItemField<u32>,
    pub write_once_target: ReferenceItemField<ReferenceItemTarget>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ReferenceItemSemantics {
    #[serde(default)]
    pub presentation: ReferenceItemField<ReferenceItemPresentation>,
    #[serde(default)]
    pub classification: ReferenceItemField<ReferenceItemClassification>,
    #[serde(default)]
    pub physical: ReferenceItemField<ReferenceItemPhysical>,
    #[serde(default)]
    pub stack: ReferenceItemField<ReferenceItemStack>,
    #[serde(default)]
    pub equipment: ReferenceItemField<ReferenceItemEquipment>,
    #[serde(default)]
    pub weapon: ReferenceItemField<ReferenceItemWeapon>,
    #[serde(default)]
    pub protection: ReferenceItemField<ReferenceItemProtection>,
    #[serde(default)]
    pub skill_modifiers: ReferenceItemField<ReferenceItemSkillModifiers>,
    #[serde(default)]
    pub charges: ReferenceItemField<ReferenceItemCharges>,
    #[serde(default)]
    pub temporal: ReferenceItemField<ReferenceItemTemporal>,
    #[serde(default)]
    pub container: ReferenceItemField<ReferenceItemContainer>,
    #[serde(default)]
    pub imbuement: ReferenceItemField<ReferenceItemImbuement>,
    #[serde(default)]
    pub use_transform: ReferenceItemField<ReferenceItemUseTransform>,
    #[serde(default)]
    pub trade_restrictions: ReferenceItemField<ReferenceItemTradeRestrictions>,
    #[serde(default)]
    pub fluid: ReferenceItemField<ReferenceItemFluid>,
    #[serde(default)]
    pub readable_writeable: ReferenceItemField<ReferenceItemReadableWriteable>,
}

impl ReferenceItemSemantics {
    pub fn is_all_unknown(&self) -> bool {
        matches!(self.presentation, ReferenceItemField::Unknown)
            && matches!(self.classification, ReferenceItemField::Unknown)
            && matches!(self.physical, ReferenceItemField::Unknown)
            && matches!(self.stack, ReferenceItemField::Unknown)
            && matches!(self.equipment, ReferenceItemField::Unknown)
            && matches!(self.weapon, ReferenceItemField::Unknown)
            && matches!(self.protection, ReferenceItemField::Unknown)
            && matches!(self.skill_modifiers, ReferenceItemField::Unknown)
            && matches!(self.charges, ReferenceItemField::Unknown)
            && matches!(self.temporal, ReferenceItemField::Unknown)
            && matches!(self.container, ReferenceItemField::Unknown)
            && matches!(self.imbuement, ReferenceItemField::Unknown)
            && matches!(self.use_transform, ReferenceItemField::Unknown)
            && matches!(self.trade_restrictions, ReferenceItemField::Unknown)
            && matches!(self.fluid, ReferenceItemField::Unknown)
            && matches!(self.readable_writeable, ReferenceItemField::Unknown)
    }

    pub fn client_projection(&self) -> Self {
        Self {
            presentation: self.presentation.clone(),
            classification: self.classification.clone(),
            physical: self.physical.clone(),
            stack: self.stack.clone(),
            equipment: self.equipment.clone(),
            weapon: self.weapon.clone(),
            protection: self.protection.clone(),
            skill_modifiers: self.skill_modifiers.clone(),
            charges: self.charges.clone(),
            container: self.container.clone(),
            imbuement: self.imbuement.clone(),
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceItemDefinition {
    pub physical_class: ReferenceItemPhysicalClass,
    pub materializable: bool,
    pub stack_class: ReferenceItemStackClass,
    pub legal_destinations: Vec<ReferenceItemDestination>,
    pub semantics: ReferenceItemSemantics,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceCreatureDefinition {
    pub presentation: TypedDefinitionRef,
    pub behavior: TypedDefinitionRef,
    pub loot: Option<TypedDefinitionRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceLootSelectionAlgorithm {
    IndependentBernoulliPpm,
    WeightedSingleSelection,
    GuaranteedEntries,
    NestedGroups,
}

pub const REFERENCE_LOOT_PROBABILITY_PPM_SCALE: u32 = 1_000_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceLootEntry {
    pub item: TypedDefinitionRef,
    pub min_count: u32,
    pub max_count: u32,
    pub probability_ppm: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceLootDefinition {
    pub algorithm: ReferenceLootSelectionAlgorithm,
    pub entries: Vec<ReferenceLootEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(
    clippy::large_enum_variant,
    reason = "preserve the established ReferenceDefinitionKind API while the bounded typed Item payload grows"
)]
pub enum ReferenceDefinitionKind {
    Generic,
    Ability(ReferenceAbilityDefinition),
    Effect(ReferenceEffectDefinition),
    Formula(ReferenceFormulaDefinition),
    Item(ReferenceItemDefinition),
    Creature(ReferenceCreatureDefinition),
    Loot(ReferenceLootDefinition),
    LocalObjectStates(Vec<ProductionKey>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientProjectionClass {
    ServerOnly,
    ClientSafe,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceDefinition {
    pub definition: TypedDefinitionRef,
    pub kind: ReferenceDefinitionKind,
    pub client_projection: ClientProjectionClass,
}

const REFERENCE_EVIDENCE_MANIFEST_JSON: &str =
    include_str!("../../../../docs/contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.json");
const REFERENCE_EVIDENCE_CASE_KEY_PREFIX: &str = "oteryn:reference.case.";

#[derive(Debug, Deserialize)]
struct AcceptedReferenceEvidenceManifest {
    schema_version: u64,
    manifest_revision: u64,
    status: String,
    cases: Vec<AcceptedReferenceEvidenceCase>,
}

#[derive(Debug, Deserialize)]
struct AcceptedReferenceEvidenceCase {
    case_id: String,
    domain: String,
    target: AcceptedReferenceEvidenceTarget,
    provenance: AcceptedReferenceEvidenceProvenance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReferenceTargetClaim {
    SpatialAddress,
    PresentationFootprint,
    CollisionFootprint,
    OrderedPlacementSequence,
}

// D1 requires per-target-sensitive evidence binding. The evidence manifest currently has no
// CONTENT_WORLD mechanic case that can authorize any of these claims. Keep this exact-case
// consumer binding empty until a separately accepted manifest case establishes the semantics;
// a PROVEN case from another domain must never become geometry/footprint/order authority.
// ENGINE_STATIC_CELL_CARRIER/v1 inputs have engineering provenance only and cannot populate
// these Reference target claim bindings or authorize Reference activation.
const REFERENCE_TARGET_CLAIM_CASE_BINDINGS: &[(&str, ReferenceTargetClaim)] = &[];

#[derive(Debug, Deserialize)]
struct AcceptedReferenceEvidenceTarget {
    evidence_class: String,
    sources: Vec<AcceptedReferenceEvidenceSource>,
}

#[derive(Debug, Deserialize)]
struct AcceptedReferenceEvidenceSource {
    source_type: String,
    provenance_state: String,
}

#[derive(Debug, Deserialize)]
struct AcceptedReferenceEvidenceProvenance {
    state: String,
    legal_review_state: String,
}

#[derive(Debug)]
struct ReferenceEvidenceAuthority {
    manifest: AcceptedReferenceEvidenceManifest,
}

impl ReferenceEvidenceAuthority {
    fn load() -> Result<Self, ContentError> {
        let manifest: AcceptedReferenceEvidenceManifest =
            serde_json::from_str(REFERENCE_EVIDENCE_MANIFEST_JSON).map_err(|_| {
                ContentError::InvalidArtifact(
                    "accepted Reference evidence manifest cannot be decoded",
                )
            })?;
        if manifest.schema_version != 1
            || manifest.manifest_revision == 0
            || manifest.status != "ACCEPTED"
        {
            return Err(ContentError::InvalidArtifact(
                "accepted Reference evidence manifest identity is invalid",
            ));
        }
        let mut case_ids = BTreeSet::new();
        for case in &manifest.cases {
            if case.case_id.is_empty() || !case_ids.insert(case.case_id.clone()) {
                return Err(ContentError::InvalidArtifact(
                    "accepted Reference evidence manifest case identity is invalid",
                ));
            }
        }
        Ok(Self { manifest })
    }

    fn manifest_revision_atom(&self) -> Result<ProductionAtom, ContentError> {
        ProductionAtom::new(
            "reference manifest revision",
            &format!("manifest-r{}", self.manifest.manifest_revision),
        )
    }

    fn require_case_bound_to_claim(
        &self,
        case: &AcceptedReferenceEvidenceCase,
        required_claim: ReferenceTargetClaim,
    ) -> Result<(), ContentError> {
        if case.domain != "CONTENT_WORLD" {
            return Err(ContentError::InvalidArtifact(
                "reference-playable evidence case domain does not match target-sensitive claim",
            ));
        }
        let bound_claim = REFERENCE_TARGET_CLAIM_CASE_BINDINGS
            .iter()
            .find_map(|(case_id, claim)| (case.case_id == *case_id).then_some(*claim));
        if bound_claim != Some(required_claim) {
            return Err(ContentError::InvalidArtifact(
                "reference-playable evidence case is not bound to target-sensitive claim",
            ));
        }
        Ok(())
    }

    fn resolve_case<'a>(
        &'a self,
        case_key: &ProductionKey,
    ) -> Result<&'a AcceptedReferenceEvidenceCase, ContentError> {
        let case_id = case_key
            .as_str()
            .strip_prefix(REFERENCE_EVIDENCE_CASE_KEY_PREFIX)
            .ok_or(ContentError::InvalidArtifact(
                "reference-playable evidence case key is invalid",
            ))?;
        self.manifest
            .cases
            .iter()
            .find(|case| case.case_id == case_id)
            .ok_or_else(|| ContentError::MissingReference {
                owner: case_key.as_str().to_owned(),
                target: "accepted Reference evidence case".to_owned(),
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceDisposition {
    Proven,
    Observed,
    Derived,
    Unknown,
    Conflict,
    DeclaredDifference,
    OtsHypothesisOnly,
    ObservedPostTarget,
}

impl EvidenceDisposition {
    pub const fn is_reference_promotable(self) -> bool {
        matches!(self, Self::Proven)
    }

    fn from_manifest(value: &str) -> Result<Self, ContentError> {
        match value {
            "PROVEN" => Ok(Self::Proven),
            "OBSERVED" => Ok(Self::Observed),
            "DERIVED" => Ok(Self::Derived),
            "UNKNOWN" => Ok(Self::Unknown),
            "CONFLICT" => Ok(Self::Conflict),
            "DECLARED_DIFFERENCE" => Ok(Self::DeclaredDifference),
            _ => Err(ContentError::InvalidArtifact(
                "accepted Reference evidence class is unsupported",
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceBindingRef {
    manifest_revision: ProductionAtom,
    case_key: ProductionKey,
    disposition: EvidenceDisposition,
}

impl EvidenceBindingRef {
    pub fn new(
        manifest_revision: ProductionAtom,
        case_key: ProductionKey,
        disposition: EvidenceDisposition,
    ) -> Self {
        Self {
            manifest_revision,
            case_key,
            disposition,
        }
    }

    pub fn from_accepted_case(case_key: ProductionKey) -> Result<Self, ContentError> {
        let authority = ReferenceEvidenceAuthority::load()?;
        let case = authority.resolve_case(&case_key)?;
        Ok(Self {
            manifest_revision: authority.manifest_revision_atom()?,
            case_key,
            disposition: EvidenceDisposition::from_manifest(&case.target.evidence_class)?,
        })
    }

    pub fn manifest_revision(&self) -> &ProductionAtom {
        &self.manifest_revision
    }

    pub fn case_key(&self) -> &ProductionKey {
        &self.case_key
    }

    pub const fn disposition(&self) -> EvidenceDisposition {
        self.disposition
    }

    fn require_reference_promotion(
        &self,
        authority: &ReferenceEvidenceAuthority,
        required_claim: ReferenceTargetClaim,
    ) -> Result<(), ContentError> {
        if self.manifest_revision != authority.manifest_revision_atom()? {
            return Err(ContentError::RevisionMismatch(
                "reference-playable evidence manifest revision",
            ));
        }

        let case = authority.resolve_case(&self.case_key)?;
        let actual_disposition = EvidenceDisposition::from_manifest(&case.target.evidence_class)?;
        if self.disposition != actual_disposition {
            return Err(ContentError::InvalidArtifact(
                "reference-playable evidence disposition does not match accepted manifest",
            ));
        }
        if !actual_disposition.is_reference_promotable() {
            return Err(ContentError::InvalidArtifact(
                "target-sensitive Reference claim lacks promotable evidence",
            ));
        }

        let all_sources_cleared = !case.target.sources.is_empty()
            && case
                .target
                .sources
                .iter()
                .all(|source| source.provenance_state == "CLEARED");
        let has_non_ots_source = case
            .target
            .sources
            .iter()
            .any(|source| source.source_type != "OTS_HYPOTHESIS_ONLY");
        if case.provenance.state != "CLEARED"
            || case.provenance.legal_review_state != "CLEARED"
            || !all_sources_cleared
            || !has_non_ots_source
        {
            return Err(ContentError::InvalidArtifact(
                "target-sensitive Reference claim lacks cleared provenance",
            ));
        }
        authority.require_case_bound_to_claim(case, required_claim)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PlacementKey(ProductionKey);

impl PlacementKey {
    pub fn new(value: &str) -> Result<Self, ContentError> {
        Ok(Self(ProductionKey::new(value)?))
    }

    pub fn as_production_key(&self) -> &ProductionKey {
        &self.0
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MapRevisionRef(ProductionAtom);

impl MapRevisionRef {
    pub fn new(value: &str) -> Result<Self, ContentError> {
        Ok(Self(ProductionAtom::new(
            "reference-playable map revision",
            value,
        )?))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CoordinateFrameRef(ProductionAtom);

impl CoordinateFrameRef {
    pub fn new(value: &str) -> Result<Self, ContentError> {
        Ok(Self(ProductionAtom::new(
            "reference-playable coordinate frame",
            value,
        )?))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LogicalCell {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpatialAddress {
    pub world_id: WorldId,
    pub coordinate_frame: CoordinateFrameRef,
    pub cell: LogicalCell,
    pub evidence: EvidenceBindingRef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FootprintCell {
    pub dx: i32,
    pub dy: i32,
    pub dz: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnresolvedTargetField {
    Unknown,
    Conflict,
    OtsHypothesisOnly,
    ObservedPostTarget,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FootprintRelation {
    Qualified {
        members: Vec<FootprintCell>,
        evidence: EvidenceBindingRef,
    },
    Unresolved(UnresolvedTargetField),
}

impl FootprintRelation {
    fn canonicalize_structure(&mut self) -> Result<(), ContentError> {
        if let Self::Qualified { members, .. } = self {
            members.sort();
            if members.windows(2).any(|pair| pair[0] == pair[1]) {
                return Err(ContentError::InvalidArtifact(
                    "reference-playable footprint contains duplicate member",
                ));
            }
        }
        Ok(())
    }

    fn require_resolved(&self) -> Result<(), ContentError> {
        if matches!(self, Self::Unresolved(_)) {
            return Err(ContentError::InvalidArtifact(
                "reference-playable footprint remains unresolved",
            ));
        }
        Ok(())
    }

    fn validate_for_reference(
        &self,
        authority: &ReferenceEvidenceAuthority,
        required_claim: ReferenceTargetClaim,
    ) -> Result<(), ContentError> {
        match self {
            Self::Qualified { evidence, .. } => {
                evidence.require_reference_promotion(authority, required_claim)
            }
            Self::Unresolved(_) => Err(ContentError::InvalidArtifact(
                "reference-playable footprint remains unresolved",
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacementRef {
    pub key: PlacementKey,
    pub map_revision: MapRevisionRef,
    pub definition: TypedDefinitionRef,
    pub address: SpatialAddress,
    pub presentation_footprint: FootprintRelation,
    pub collision_footprint: FootprintRelation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderedPlacementSet {
    pub field_key: ProductionKey,
    pub placement_keys: Vec<PlacementKey>,
    pub evidence: EvidenceBindingRef,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TransitionKey(ProductionKey);

impl TransitionKey {
    pub fn new(value: &str) -> Result<Self, ContentError> {
        Ok(Self(ProductionKey::new(value)?))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerCapabilityRequirement {
    pub capability_key: ProductionKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionBinding {
    pub key: TransitionKey,
    pub definition: TypedDefinitionRef,
    pub source_state: ProductionKey,
    pub normalized_intent_family: ProductionKey,
    pub target_state: ProductionKey,
    pub owner_capability: OwnerCapabilityRequirement,
    pub policy_guard_refs: Vec<ProductionKey>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferencePlayableContentSource {
    pub profile_revision: ProductionAtom,
    pub capability_profile: ProductionAtom,
    pub package_manifest: PackageManifestBinding,
    pub content_lock: ContentLockBinding,
    pub world_id: WorldId,
    pub coordinate_frame: CoordinateFrameRef,
    pub definitions: Vec<ReferenceDefinition>,
    pub placements: Vec<PlacementRef>,
    pub ordered_placements: Vec<OrderedPlacementSet>,
    pub transitions: Vec<TransitionBinding>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientSafeItemDefinition {
    pub physical_class: ReferenceItemPhysicalClass,
    pub stack_class: ReferenceItemStackClass,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientSafeCreatureDefinition {
    pub presentation: TypedDefinitionRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientSafeDefinitionKind {
    Generic,
    Effect(ReferenceEffectFamily),
    Item(ClientSafeItemDefinition),
    Creature(ClientSafeCreatureDefinition),
    LocalObjectStates(Vec<ProductionKey>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientSafeDefinitionRef {
    pub definition: TypedDefinitionRef,
    pub kind: ClientSafeDefinitionKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalReferencePlayableContent {
    pub profile_revision: ProductionAtom,
    pub capability_profile: ProductionAtom,
    pub package_manifest: PackageManifestBinding,
    pub content_lock: ContentLockBinding,
    pub world_id: WorldId,
    pub coordinate_frame: CoordinateFrameRef,
    pub definitions: Vec<ReferenceDefinition>,
    pub placements: Vec<PlacementRef>,
    pub ordered_placements: Vec<OrderedPlacementSet>,
    pub transitions: Vec<TransitionBinding>,
}

impl CanonicalReferencePlayableContent {
    pub fn client_safe_definitions(&self) -> Vec<ClientSafeDefinitionRef> {
        self.definitions
            .iter()
            .filter(|definition| definition.client_projection == ClientProjectionClass::ClientSafe)
            .filter_map(|definition| {
                let kind = match &definition.kind {
                    ReferenceDefinitionKind::Generic => ClientSafeDefinitionKind::Generic,
                    ReferenceDefinitionKind::Effect(effect) => {
                        ClientSafeDefinitionKind::Effect(effect.family)
                    }
                    ReferenceDefinitionKind::Item(item) => {
                        ClientSafeDefinitionKind::Item(ClientSafeItemDefinition {
                            physical_class: item.physical_class,
                            stack_class: item.stack_class,
                        })
                    }
                    ReferenceDefinitionKind::Creature(creature) => {
                        ClientSafeDefinitionKind::Creature(ClientSafeCreatureDefinition {
                            presentation: creature.presentation.clone(),
                        })
                    }
                    ReferenceDefinitionKind::Ability(_)
                    | ReferenceDefinitionKind::Formula(_)
                    | ReferenceDefinitionKind::Loot(_) => return None,
                    ReferenceDefinitionKind::LocalObjectStates(states) => {
                        ClientSafeDefinitionKind::LocalObjectStates(states.clone())
                    }
                };
                Some(ClientSafeDefinitionRef {
                    definition: definition.definition.clone(),
                    kind,
                })
            })
            .collect()
    }
}

fn validate_content_lock(
    package: &PackageManifestBinding,
    content_lock: &ContentLockBinding,
) -> Result<(), ContentError> {
    let expected_digest = package.package_provenance_digest()?;
    let mut matches = content_lock
        .entries
        .iter()
        .filter(|entry| entry.package_key == package.package_key);
    let root = matches.next().ok_or(ContentError::InvalidArtifact(
        "reference-playable Content Lock lacks root package",
    ))?;
    if matches.next().is_some() {
        return Err(ContentError::InvalidArtifact(
            "reference-playable Content Lock duplicates root package",
        ));
    }
    if root.floating
        || root.dependency
        || root.package_revision != package.package_revision
        || root.package_provenance_digest != expected_digest
    {
        return Err(ContentError::InvalidArtifact(
            "reference-playable Content Lock does not bind exact root package provenance",
        ));
    }
    Ok(())
}

pub(crate) fn validate_item_definition(item: &ReferenceItemDefinition) -> Result<(), ContentError> {
    validate_item_semantics(item)?;
    let mut destinations = BTreeSet::new();
    for destination in &item.legal_destinations {
        if !destinations.insert(*destination) {
            return Err(ContentError::InvalidArtifact(
                "reference-playable item duplicates legal destination capability",
            ));
        }
    }

    let identity_only = item.physical_class == ReferenceItemPhysicalClass::Unknown
        || item.stack_class == ReferenceItemStackClass::Unknown;
    if identity_only {
        if item.physical_class != ReferenceItemPhysicalClass::Unknown
            || item.stack_class != ReferenceItemStackClass::Unknown
            || item.materializable
            || !destinations.is_empty()
        {
            return Err(ContentError::InvalidArtifact(
                "reference-playable identity-only item must keep physical, stack, materialization and destination semantics unresolved",
            ));
        }
        return Ok(());
    }

    let inventory_legal = destinations.contains(&ReferenceItemDestination::CharacterInventory);
    if item.materializable && !inventory_legal {
        return Err(ContentError::InvalidArtifact(
            "reference-playable materializable item requires CharacterInventory destination capability",
        ));
    }
    if !item.materializable && inventory_legal {
        return Err(ContentError::InvalidArtifact(
            "reference-playable non-materializable item cannot declare CharacterInventory destination capability",
        ));
    }
    Ok(())
}

fn require_limit(resource: &'static str, actual: usize, limit: usize) -> Result<(), ContentError> {
    if actual > limit {
        return Err(ContentError::LimitExceeded {
            resource,
            actual,
            limit,
        });
    }
    Ok(())
}

fn require_sorted_unique<T: Ord>(resource: &'static str, values: &[T]) -> Result<(), ContentError> {
    if values.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(ContentError::InvalidArtifact(resource));
    }
    Ok(())
}

fn reject_known_unsupported(
    field: &ReferenceItemField<()>,
    resource: &'static str,
) -> Result<(), ContentError> {
    if matches!(field, ReferenceItemField::Known(())) {
        return Err(ContentError::InvalidArtifact(resource));
    }
    Ok(())
}

fn validate_item_target(target: &ReferenceItemTarget) -> Result<(), ContentError> {
    target.typed_ref().map(|_| ())
}

fn validate_rational_field(
    field: &ReferenceItemField<ReferenceRationalPercent>,
) -> Result<(), ContentError> {
    if let ReferenceItemField::Known(value) = field {
        value.validate()?;
    }
    Ok(())
}

fn validate_item_semantics(item: &ReferenceItemDefinition) -> Result<(), ContentError> {
    use ReferenceItemField::Known;
    let semantics = &item.semantics;
    if let Known(value) = &semantics.presentation {
        if let Known(name) = &value.name {
            require_limit(
                "Reference Item presentation name bytes",
                name.len(),
                REFERENCE_ITEM_MAX_NAME_BYTES,
            )?;
        }
        if let Known(description) = &value.description {
            require_limit(
                "Reference Item presentation description bytes",
                description.len(),
                REFERENCE_ITEM_MAX_DESCRIPTION_BYTES,
            )?;
        }
    }
    if let Known(value) = &semantics.stack {
        if let Known(true) = value.stackable
            && !matches!(value.stack_max, Known(maximum) if maximum >= 1)
        {
            return Err(ContentError::InvalidArtifact(
                "Reference Item stackable=true requires known nonzero stack maximum",
            ));
        }
        if let Known(stackable) = value.stackable {
            match (item.stack_class, stackable) {
                (ReferenceItemStackClass::NonStackable, true)
                | (ReferenceItemStackClass::StackCapable, false) => {
                    return Err(ContentError::InvalidArtifact(
                        "Reference Item retained and typed stack semantics conflict",
                    ));
                }
                _ => {}
            }
        }
    }
    if let Known(value) = &semantics.equipment
        && let Known(patterns) = &value.patterns
    {
        if patterns.is_empty() {
            return Err(ContentError::InvalidArtifact(
                "Reference Item known Equipment patterns cannot be empty",
            ));
        }
        require_limit(
            "Reference Item Equipment patterns",
            patterns.len(),
            REFERENCE_ITEM_MAX_EQUIPMENT_PATTERNS,
        )?;
        let mut previous_id = 0;
        for (index, pattern) in patterns.iter().enumerate() {
            if pattern.pattern_id == 0
                || usize::from(pattern.pattern_id) > REFERENCE_ITEM_MAX_EQUIPMENT_PATTERNS
                || pattern.pattern_id <= previous_id
            {
                return Err(ContentError::InvalidArtifact(
                    "Reference Item Equipment pattern ids are not canonical",
                ));
            }
            previous_id = pattern.pattern_id;
            if let Known(slots) = &pattern.additional_reserved_slots {
                require_limit(
                    "Reference Item Equipment additional slots",
                    slots.len(),
                    REFERENCE_ITEM_MAX_ADDITIONAL_SLOTS,
                )?;
                require_sorted_unique("Reference Item Equipment slot order", slots)?;
                if matches!(pattern.primary_slot, Known(primary) if slots.contains(&primary)) {
                    return Err(ContentError::InvalidArtifact(
                        "Reference Item Equipment primary slot repeats as reservation",
                    ));
                }
            }
            if let Known(groups) = &pattern.mutually_exclusive_groups {
                require_limit(
                    "Reference Item Equipment mutually exclusive groups",
                    groups.len(),
                    REFERENCE_ITEM_MAX_EXCLUSIVE_GROUPS,
                )?;
                require_sorted_unique("Reference Item Equipment group order", groups)?;
            }
            if let Known(vocations) = &pattern.vocations {
                require_limit(
                    "Reference Item Equipment base vocations",
                    vocations.len(),
                    REFERENCE_ITEM_MAX_BASE_VOCATIONS,
                )?;
                require_sorted_unique("Reference Item Equipment vocation order", vocations)?;
            }
            reject_known_unsupported(
                &pattern.compatibility_rule,
                "Reference Item Equipment compatibility grammar is unsupported in v1",
            )?;
            if patterns[..index].iter().any(|other| {
                other.primary_slot == pattern.primary_slot
                    && other.additional_reserved_slots == pattern.additional_reserved_slots
                    && other.mutually_exclusive_groups == pattern.mutually_exclusive_groups
                    && other.vocations == pattern.vocations
                    && other.level == pattern.level
                    && other.compatibility_rule == pattern.compatibility_rule
            }) {
                return Err(ContentError::InvalidArtifact(
                    "Reference Item duplicates an Equipment semantic pattern",
                ));
            }
        }
    }
    if let Known(value) = &semantics.weapon {
        validate_rational_field(&value.hit_chance)?;
        validate_rational_field(&value.max_hit_chance)?;
        if let Known(entries) = &value.elemental {
            require_limit(
                "Reference Item Weapon elements",
                entries.len(),
                REFERENCE_ITEM_MAX_WEAPON_ELEMENTS,
            )?;
            require_sorted_unique(
                "Reference Item Weapon element order",
                &entries
                    .iter()
                    .map(|entry| entry.element)
                    .collect::<Vec<_>>(),
            )?;
        }
    }
    if let Known(value) = &semantics.protection
        && let Known(entries) = &value.resistances
    {
        require_limit(
            "Reference Item resistances",
            entries.len(),
            REFERENCE_ITEM_MAX_RESISTANCES,
        )?;
        require_sorted_unique(
            "Reference Item resistance order",
            &entries.iter().map(|entry| entry.kind).collect::<Vec<_>>(),
        )?;
        for entry in entries {
            validate_rational_field(&entry.percent)?;
        }
    }
    if let Known(value) = &semantics.skill_modifiers
        && let Known(entries) = &value.modifiers
    {
        require_limit(
            "Reference Item SkillModifiers",
            entries.len(),
            REFERENCE_ITEM_MAX_MODIFIERS,
        )?;
        require_sorted_unique(
            "Reference Item SkillModifier order",
            &entries.iter().map(|entry| entry.kind).collect::<Vec<_>>(),
        )?;
        for entry in entries {
            for capacity in [&entry.target_domain, &entry.evaluation_phase] {
                if matches!(capacity, Known(value) if !(1..=37).contains(value)) {
                    return Err(ContentError::InvalidArtifact(
                        "Reference Item SkillModifier capacity id is outside the closed domain",
                    ));
                }
            }
            if let Known(parameter) = &entry.parameter {
                validate_modifier_parameter(entry.kind, parameter)?;
            }
        }
    }
    if let Known(value) = &semantics.temporal
        && let Known(target) = &value.decay_target
    {
        validate_item_target(target)?;
    }
    if let Known(value) = &semantics.imbuement {
        if matches!(value.slot_count, Known(slots) if slots > REFERENCE_ITEM_MAX_IMBUEMENT_SLOTS) {
            return Err(ContentError::InvalidArtifact(
                "Reference Item imbuement slot count",
            ));
        }
        if let Known(entries) = &value.allowed_family_tiers {
            require_limit(
                "Reference Item imbuement allowances",
                entries.len(),
                REFERENCE_ITEM_MAX_IMBUEMENT_FAMILIES,
            )?;
            require_sorted_unique(
                "Reference Item imbuement allowance order",
                &entries.iter().map(|entry| entry.family).collect::<Vec<_>>(),
            )?;
        }
        if let Known(entries) = &value.excluded_families {
            require_limit(
                "Reference Item excluded imbuement families",
                entries.len(),
                REFERENCE_ITEM_MAX_IMBUEMENT_FAMILIES,
            )?;
            require_sorted_unique("Reference Item excluded imbuement family order", entries)?;
        }
    }
    if let Known(value) = &semantics.use_transform {
        if value.targets.len() != 10 {
            return Err(ContentError::InvalidArtifact(
                "Reference Item UseTransform requires ten fixed target kinds",
            ));
        }
        require_sorted_unique(
            "Reference Item UseTransform kind order",
            &value
                .targets
                .iter()
                .map(|entry| entry.kind)
                .collect::<Vec<_>>(),
        )?;
        for entry in &value.targets {
            if let Known(target) = &entry.target {
                validate_item_target(target)?;
            }
        }
    }
    if let Known(value) = &semantics.trade_restrictions {
        if let Known(vocations) = &value.vocations {
            require_limit(
                "Reference Item trade vocations",
                vocations.len(),
                REFERENCE_ITEM_MAX_BASE_VOCATIONS,
            )?;
            require_sorted_unique("Reference Item trade vocation order", vocations)?;
        }
        reject_known_unsupported(
            &value.account_binding_policy,
            "Reference Item immutable account binding is unsupported in v1",
        )?;
        reject_known_unsupported(
            &value.character_binding_policy,
            "Reference Item immutable character binding is unsupported in v1",
        )?;
    }
    if let Known(value) = &semantics.readable_writeable
        && let Known(target) = &value.write_once_target
    {
        validate_item_target(target)?;
    }
    Ok(())
}

fn validate_modifier_parameter(
    kind: ReferenceSkillModifierKind,
    parameter: &ReferenceModifierParameter,
) -> Result<(), ContentError> {
    use ReferenceModifierParameter as Parameter;
    use ReferenceSkillModifierKind as Kind;
    let valid = match kind {
        Kind::Invisibility | Kind::ManaShield | Kind::SuppressDrown | Kind::SuppressDrunk => {
            matches!(parameter, Parameter::Boolean(_))
        }
        Kind::CleavePercent
        | Kind::CriticalHitChance
        | Kind::CriticalHitDamage
        | Kind::LifeLeechAmount
        | Kind::LifeLeechChance
        | Kind::MagicShieldCapacityPercent
        | Kind::ManaLeechAmount
        | Kind::ManaLeechChance => {
            if let Parameter::RationalPercent(value) = parameter {
                value.validate()?;
                true
            } else {
                false
            }
        }
        Kind::HealthTicks | Kind::ManaTicks => matches!(parameter, Parameter::Milliseconds(_)),
        Kind::PerfectShotRange => matches!(parameter, Parameter::Cells(_)),
        Kind::ElementalBond => matches!(parameter, Parameter::Element(_)),
        _ => matches!(parameter, Parameter::SignedPoints(_)),
    };
    if !valid {
        return Err(ContentError::InvalidArtifact(
            "Reference Item SkillModifier parameter has the wrong typed shape",
        ));
    }
    Ok(())
}

fn canonicalize_definition(definition: &mut ReferenceDefinition) {
    match &mut definition.kind {
        ReferenceDefinitionKind::Item(item) => item.legal_destinations.sort(),
        ReferenceDefinitionKind::Loot(loot) => loot.entries.sort_by(|left, right| {
            left.item
                .cmp(&right.item)
                .then_with(|| left.min_count.cmp(&right.min_count))
                .then_with(|| left.max_count.cmp(&right.max_count))
                .then_with(|| left.probability_ppm.cmp(&right.probability_ppm))
        }),
        _ => {}
    }
}

fn validate_loot_definition(
    definition: &ReferenceDefinition,
    loot: &ReferenceLootDefinition,
) -> Result<(), ContentError> {
    if definition.client_projection != ClientProjectionClass::ServerOnly {
        return Err(ContentError::InvalidArtifact(
            "reference-playable loot selection authority must remain server-only",
        ));
    }

    match loot.algorithm {
        ReferenceLootSelectionAlgorithm::WeightedSingleSelection => {
            return Err(ContentError::InvalidArtifact(
                "reference-playable weighted loot entries require separately accepted typed weight semantics",
            ));
        }
        ReferenceLootSelectionAlgorithm::NestedGroups => {
            return Err(ContentError::InvalidArtifact(
                "reference-playable nested loot groups require separately accepted typed group references",
            ));
        }
        ReferenceLootSelectionAlgorithm::IndependentBernoulliPpm
        | ReferenceLootSelectionAlgorithm::GuaranteedEntries => {}
    }

    for entry in &loot.entries {
        if entry.min_count == 0 || entry.max_count < entry.min_count {
            return Err(ContentError::InvalidArtifact(
                "reference-playable loot entry requires a positive ordered count range",
            ));
        }

        match loot.algorithm {
            ReferenceLootSelectionAlgorithm::IndependentBernoulliPpm => {
                let probability_ppm =
                    entry.probability_ppm.ok_or(ContentError::InvalidArtifact(
                        "reference-playable Bernoulli loot entry requires explicit probability_ppm",
                    ))?;
                if probability_ppm > REFERENCE_LOOT_PROBABILITY_PPM_SCALE {
                    return Err(ContentError::InvalidArtifact(
                        "reference-playable loot probability_ppm exceeds one million",
                    ));
                }
            }
            ReferenceLootSelectionAlgorithm::GuaranteedEntries => {
                if entry.probability_ppm.is_some() {
                    return Err(ContentError::InvalidArtifact(
                        "reference-playable guaranteed loot entry cannot also declare probability_ppm",
                    ));
                }
            }
            ReferenceLootSelectionAlgorithm::WeightedSingleSelection => {
                return Err(ContentError::InvalidArtifact(
                    "reference-playable weighted loot entries require separately accepted typed weight semantics",
                ));
            }
            ReferenceLootSelectionAlgorithm::NestedGroups => {
                return Err(ContentError::InvalidArtifact(
                    "reference-playable nested loot groups require separately accepted typed group references",
                ));
            }
        }
    }
    Ok(())
}

fn validate_definition_shape(definition: &ReferenceDefinition) -> Result<(), ContentError> {
    match (&definition.definition.family, &definition.kind) {
        (DefinitionFamily::Ability, ReferenceDefinitionKind::Ability(_)) => {
            if definition.client_projection != ClientProjectionClass::ServerOnly {
                return Err(ContentError::InvalidArtifact(
                    "reference-playable ability structure must remain server-only",
                ));
            }
            Ok(())
        }
        (DefinitionFamily::Effect, ReferenceDefinitionKind::Effect(_)) => Ok(()),
        (DefinitionFamily::Formula, ReferenceDefinitionKind::Formula(_)) => {
            if definition.client_projection != ClientProjectionClass::ServerOnly {
                return Err(ContentError::InvalidArtifact(
                    "reference-playable formula endpoint must remain server-only",
                ));
            }
            Ok(())
        }
        (DefinitionFamily::Item, ReferenceDefinitionKind::Item(item)) => {
            validate_item_definition(item)
        }
        (DefinitionFamily::Creature, ReferenceDefinitionKind::Creature(_)) => Ok(()),
        (DefinitionFamily::Loot, ReferenceDefinitionKind::Loot(loot)) => {
            validate_loot_definition(definition, loot)
        }
        (DefinitionFamily::LocalObject, ReferenceDefinitionKind::LocalObjectStates(states)) => {
            if states.is_empty() {
                return Err(ContentError::InvalidArtifact(
                    "reference-playable local object requires finite state vocabulary",
                ));
            }
            let mut unique = BTreeSet::new();
            for state in states {
                if !unique.insert(state.clone()) {
                    return Err(ContentError::DuplicateKey(state.as_str().to_owned()));
                }
            }
            Ok(())
        }
        (DefinitionFamily::Ability, _) => Err(ContentError::InvalidArtifact(
            "reference-playable ability requires typed effect references",
        )),
        (DefinitionFamily::Effect, _) => Err(ContentError::InvalidArtifact(
            "reference-playable effect definition requires typed family and formula reference",
        )),
        (DefinitionFamily::Formula, _) => Err(ContentError::InvalidArtifact(
            "reference-playable formula requires opaque typed endpoint",
        )),
        (DefinitionFamily::Item, _) => Err(ContentError::InvalidArtifact(
            "reference-playable item requires typed static item semantics",
        )),
        (DefinitionFamily::LocalObject, _) => Err(ContentError::InvalidArtifact(
            "reference-playable local object requires finite state vocabulary",
        )),
        (_, ReferenceDefinitionKind::Ability(_))
        | (_, ReferenceDefinitionKind::Effect(_))
        | (_, ReferenceDefinitionKind::Formula(_))
        | (_, ReferenceDefinitionKind::Item(_))
        | (_, ReferenceDefinitionKind::Creature(_))
        | (_, ReferenceDefinitionKind::Loot(_))
        | (_, ReferenceDefinitionKind::LocalObjectStates(_)) => Err(ContentError::InvalidArtifact(
            "reference-playable definition kind does not match definition family",
        )),
        (DefinitionFamily::Creature, ReferenceDefinitionKind::Generic) => {
            Err(ContentError::InvalidArtifact(
                "reference-playable creature requires typed static creature semantics",
            ))
        }
        (DefinitionFamily::Loot, ReferenceDefinitionKind::Generic) => {
            Err(ContentError::InvalidArtifact(
                "reference-playable loot requires typed loot table semantics",
            ))
        }
        (_, ReferenceDefinitionKind::Generic) => Ok(()),
    }
}

fn resolve_definition<'a>(
    definitions: &'a [ReferenceDefinition],
    reference: &TypedDefinitionRef,
) -> Result<&'a ReferenceDefinition, ContentError> {
    if let Some(definition) = definitions
        .iter()
        .find(|candidate| candidate.definition == *reference)
    {
        return Ok(definition);
    }
    if definitions.iter().any(|candidate| {
        candidate.definition.family == reference.family && candidate.definition.key == reference.key
    }) {
        return Err(ContentError::RevisionMismatch(
            "reference-playable definition revision",
        ));
    }
    Err(ContentError::MissingReference {
        owner: format!("{:?}:{}", reference.family, reference.key.as_str()),
        target: "exact typed definition".to_owned(),
    })
}

fn resolve_expected_definition<'a>(
    definitions: &'a [ReferenceDefinition],
    reference: &TypedDefinitionRef,
    expected_family: DefinitionFamily,
    wrong_family_message: &'static str,
) -> Result<&'a ReferenceDefinition, ContentError> {
    if reference.family != expected_family {
        return Err(ContentError::InvalidArtifact(wrong_family_message));
    }
    resolve_definition(definitions, reference)
}

fn validate_definition_references(
    definitions: &[ReferenceDefinition],
    definition: &ReferenceDefinition,
) -> Result<(), ContentError> {
    match &definition.kind {
        ReferenceDefinitionKind::Ability(ability) => {
            for effect in &ability.effects {
                resolve_expected_definition(
                    definitions,
                    effect,
                    DefinitionFamily::Effect,
                    "reference-playable ability effect reference must target Effect",
                )?;
            }
            Ok(())
        }
        ReferenceDefinitionKind::Effect(effect) => {
            resolve_expected_definition(
                definitions,
                &effect.formula,
                DefinitionFamily::Formula,
                "reference-playable effect formula reference must target Formula",
            )?;
            Ok(())
        }
        ReferenceDefinitionKind::Creature(creature) => {
            let presentation = resolve_expected_definition(
                definitions,
                &creature.presentation,
                DefinitionFamily::Presentation,
                "reference-playable creature presentation reference must target Presentation",
            )?;
            if definition.client_projection == ClientProjectionClass::ClientSafe
                && presentation.client_projection != ClientProjectionClass::ClientSafe
            {
                return Err(ContentError::InvalidArtifact(
                    "reference-playable client-safe creature requires client-safe presentation target",
                ));
            }
            resolve_expected_definition(
                definitions,
                &creature.behavior,
                DefinitionFamily::Behavior,
                "reference-playable creature behavior reference must target Behavior",
            )?;
            if let Some(loot) = &creature.loot {
                resolve_expected_definition(
                    definitions,
                    loot,
                    DefinitionFamily::Loot,
                    "reference-playable creature loot reference must target Loot",
                )?;
            }
            Ok(())
        }
        ReferenceDefinitionKind::Loot(loot) => {
            for entry in &loot.entries {
                resolve_expected_definition(
                    definitions,
                    &entry.item,
                    DefinitionFamily::Item,
                    "reference-playable loot entry must target Item",
                )?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn validate_placement(
    source: &ReferencePlayableContentSource,
    placement: &PlacementRef,
    authority: &ReferenceEvidenceAuthority,
) -> Result<(), ContentError> {
    resolve_definition(&source.definitions, &placement.definition)?;
    if placement.address.world_id != source.world_id {
        return Err(ContentError::InvalidArtifact(
            "reference-playable placement WorldId mismatch",
        ));
    }
    if placement.address.coordinate_frame != source.coordinate_frame {
        return Err(ContentError::InvalidArtifact(
            "reference-playable coordinate frame mismatch",
        ));
    }
    placement.presentation_footprint.require_resolved()?;
    placement.collision_footprint.require_resolved()?;
    placement
        .address
        .evidence
        .require_reference_promotion(authority, ReferenceTargetClaim::SpatialAddress)?;
    placement
        .presentation_footprint
        .validate_for_reference(authority, ReferenceTargetClaim::PresentationFootprint)?;
    placement
        .collision_footprint
        .validate_for_reference(authority, ReferenceTargetClaim::CollisionFootprint)?;
    Ok(())
}

fn validate_ordering_structure(
    placements: &[PlacementRef],
    ordered: &OrderedPlacementSet,
) -> Result<(), ContentError> {
    let mut seen = BTreeSet::new();
    for key in &ordered.placement_keys {
        if !seen.insert(key.clone()) {
            return Err(ContentError::DuplicateKey(key.as_str().to_owned()));
        }
        if !placements.iter().any(|placement| placement.key == *key) {
            return Err(ContentError::MissingReference {
                owner: ordered.field_key.as_str().to_owned(),
                target: key.as_str().to_owned(),
            });
        }
    }
    Ok(())
}

fn validate_ordering_evidence(
    ordered: &OrderedPlacementSet,
    authority: &ReferenceEvidenceAuthority,
) -> Result<(), ContentError> {
    ordered
        .evidence
        .require_reference_promotion(authority, ReferenceTargetClaim::OrderedPlacementSequence)
}

fn validate_transition(
    definitions: &[ReferenceDefinition],
    transition: &TransitionBinding,
) -> Result<(), ContentError> {
    let definition = resolve_definition(definitions, &transition.definition)?;
    if transition.definition.family != DefinitionFamily::LocalObject {
        return Err(ContentError::InvalidArtifact(
            "reference-playable transition must target local-object definition",
        ));
    }
    let ReferenceDefinitionKind::LocalObjectStates(states) = &definition.kind else {
        return Err(ContentError::InvalidArtifact(
            "reference-playable transition target lacks state vocabulary",
        ));
    };
    for state in [&transition.source_state, &transition.target_state] {
        if !states.contains(state) {
            return Err(ContentError::MissingReference {
                owner: transition.key.as_str().to_owned(),
                target: state.as_str().to_owned(),
            });
        }
    }
    Ok(())
}

pub fn link_reference_playable(
    mut source: ReferencePlayableContentSource,
) -> Result<CanonicalReferencePlayableContent, ContentError> {
    if source.profile_revision.as_str() != REFERENCE_PLAYABLE_CONTENT_PROFILE_ID {
        return Err(ContentError::RevisionMismatch(
            "reference-playable profile revision",
        ));
    }
    if source.capability_profile.as_str() != REFERENCE_PLAYABLE_CAPABILITY_PROFILE {
        return Err(ContentError::RevisionMismatch(
            "reference-playable capability profile",
        ));
    }
    validate_content_lock(&source.package_manifest, &source.content_lock)?;
    let evidence_authority = ReferenceEvidenceAuthority::load()?;

    for definition in &mut source.definitions {
        canonicalize_definition(definition);
    }

    let mut definition_keys = BTreeSet::new();
    for definition in &source.definitions {
        validate_definition_shape(definition)?;
        let identity = (
            definition.definition.family,
            definition.definition.key.clone(),
        );
        if !definition_keys.insert(identity) {
            return Err(ContentError::DuplicateKey(
                definition.definition.key.as_str().to_owned(),
            ));
        }
    }
    for definition in &source.definitions {
        validate_definition_references(&source.definitions, definition)?;
    }

    for placement in &mut source.placements {
        placement.presentation_footprint.canonicalize_structure()?;
        placement.collision_footprint.canonicalize_structure()?;
    }

    let mut placement_keys = BTreeSet::new();
    for placement in &source.placements {
        if !placement_keys.insert(placement.key.clone()) {
            return Err(ContentError::DuplicateKey(
                placement.key.as_str().to_owned(),
            ));
        }
    }
    let mut ordering_keys = BTreeSet::new();
    for ordered in &source.ordered_placements {
        if !ordering_keys.insert(ordered.field_key.clone()) {
            return Err(ContentError::DuplicateKey(
                ordered.field_key.as_str().to_owned(),
            ));
        }
        validate_ordering_structure(&source.placements, ordered)?;
    }

    for placement in &source.placements {
        validate_placement(&source, placement, &evidence_authority)?;
    }
    for ordered in &source.ordered_placements {
        validate_ordering_evidence(ordered, &evidence_authority)?;
    }

    let mut transition_keys = BTreeSet::new();
    for transition in &source.transitions {
        if !transition_keys.insert(transition.key.clone()) {
            return Err(ContentError::DuplicateKey(
                transition.key.as_str().to_owned(),
            ));
        }
        validate_transition(&source.definitions, transition)?;
    }

    source.definitions.sort_by(|left, right| {
        left.definition
            .family
            .cmp(&right.definition.family)
            .then_with(|| left.definition.key.cmp(&right.definition.key))
            .then_with(|| left.definition.revision.cmp(&right.definition.revision))
    });
    source
        .placements
        .sort_by(|left, right| left.key.cmp(&right.key));
    source
        .ordered_placements
        .sort_by(|left, right| left.field_key.cmp(&right.field_key));
    source
        .transitions
        .sort_by(|left, right| left.key.cmp(&right.key));

    Ok(CanonicalReferencePlayableContent {
        profile_revision: source.profile_revision,
        capability_profile: source.capability_profile,
        package_manifest: source.package_manifest,
        content_lock: source.content_lock,
        world_id: source.world_id,
        coordinate_frame: source.coordinate_frame,
        definitions: source.definitions,
        placements: source.placements,
        ordered_placements: source.ordered_placements,
        transitions: source.transitions,
    })
}

#[cfg(test)]
mod corrective_tests {
    use super::*;

    #[test]
    fn accepted_case_from_other_domain_cannot_bind_content_world_target_claim()
    -> Result<(), ContentError> {
        let authority = ReferenceEvidenceAuthority::load()?;
        let key = ProductionKey::new(
            "oteryn:reference.case.ability_combat.light_healing.self_heal_semantics.v1",
        )?;
        let case = authority.resolve_case(&key)?;

        assert!(matches!(
            authority.require_case_bound_to_claim(case, ReferenceTargetClaim::SpatialAddress),
            Err(ContentError::InvalidArtifact(
                "reference-playable evidence case domain does not match target-sensitive claim"
            ))
        ));
        Ok(())
    }

    #[test]
    fn footprint_member_order_is_canonicalized() -> Result<(), ContentError> {
        let evidence = EvidenceBindingRef::new(
            ProductionAtom::new("reference manifest revision", "manifest-r1")?,
            ProductionKey::new(
                "oteryn:reference.case.ability_combat.light_healing.self_heal_semantics.v1",
            )?,
            EvidenceDisposition::Unknown,
        );
        let low = FootprintCell {
            dx: 0,
            dy: 0,
            dz: 0,
        };
        let high = FootprintCell {
            dx: 1,
            dy: 0,
            dz: 0,
        };
        let mut relation = FootprintRelation::Qualified {
            members: vec![high, low],
            evidence,
        };

        relation.canonicalize_structure()?;

        let FootprintRelation::Qualified { members, .. } = relation else {
            unreachable!("qualified footprint changed variant");
        };
        assert_eq!(members, vec![low, high]);
        Ok(())
    }
}
