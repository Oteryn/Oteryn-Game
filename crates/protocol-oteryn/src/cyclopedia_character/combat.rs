//! P12c current combat facts (Crystal `protocolgame.cpp:5016-5325`).
//!
//! `None` means that the native mechanic has no current authoritative observation. It is
//! deliberately different from a known zero. Ratios use parts per million (1_000_000 = 100%).
//! This is protocol-oteryn's schema, not a copy of the donor's client-specific packet layout.

use super::{CharacterTab, TabWireError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatStatUnit {
    Points,
    PartsPerMillion,
    Identifier,
    Seconds,
}

macro_rules! combat_stats {
    ($($tab:ident => { $($name:ident = $id:literal, $unit:ident, $signed:literal;)* })*) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        #[repr(u8)]
        pub enum CombatStat { $($($name = $id,)*)* }

        impl CombatStat {
            pub const fn tab(self) -> CharacterTab {
                match self { $($(Self::$name => CharacterTab::$tab,)*)* }
            }

            pub const fn unit(self) -> CombatStatUnit {
                match self { $($(Self::$name => CombatStatUnit::$unit,)*)* }
            }

            pub const fn permits_negative(self) -> bool {
                match self { $($(Self::$name => $signed,)*)* }
            }

            pub fn for_tab(tab: CharacterTab) -> &'static [Self] {
                match tab {
                    $(CharacterTab::$tab => &[$(Self::$name,)*],)*
                    _ => &[],
                }
            }
        }
    };
}

combat_stats! {
    Offence => {
        CriticalChance = 1, PartsPerMillion, false;
        CriticalChanceBase = 2, PartsPerMillion, false;
        CriticalChanceEquipment = 3, PartsPerMillion, false;
        CriticalChanceImbuement = 4, PartsPerMillion, true;
        CriticalChanceWheel = 5, PartsPerMillion, false;
        CriticalChanceConcoction = 6, PartsPerMillion, false;
        CriticalDamage = 7, PartsPerMillion, false;
        CriticalDamageBase = 8, PartsPerMillion, false;
        CriticalDamageEquipment = 9, PartsPerMillion, false;
        CriticalDamageImbuement = 10, PartsPerMillion, true;
        CriticalDamageWheel = 11, PartsPerMillion, false;
        LifeLeech = 12, PartsPerMillion, false;
        LifeLeechEquipment = 13, PartsPerMillion, false;
        LifeLeechImbuement = 14, PartsPerMillion, true;
        LifeLeechWheel = 15, PartsPerMillion, false;
        ManaLeech = 16, PartsPerMillion, false;
        ManaLeechEquipment = 17, PartsPerMillion, false;
        ManaLeechImbuement = 18, PartsPerMillion, true;
        ManaLeechWheel = 19, PartsPerMillion, false;
        Onslaught = 20, PartsPerMillion, false;
        OnslaughtEquipment = 21, PartsPerMillion, false;
        OnslaughtAmplification = 22, PartsPerMillion, false;
        Cleave = 23, PartsPerMillion, false;
        PerfectShot1 = 24, Points, false;
        PerfectShot2 = 25, Points, false;
        PerfectShot3 = 26, Points, false;
        PerfectShot4 = 27, Points, false;
        PerfectShot5 = 28, Points, false;
        PerfectShot6 = 29, Points, false;
        PerfectShot7 = 30, Points, false;
        FlatDamageHealing = 31, Points, false;
        AttackTotal = 32, Points, false;
        AttackEquipment = 33, Points, false;
        AttackSkillId = 34, Identifier, false;
        AttackSkill = 35, Points, true;
        AttackFightMode = 36, Points, true;
        AttackElement = 37, Identifier, false;
        ConvertedDamage = 38, PartsPerMillion, false;
        ConvertedElement = 39, Identifier, false;
        Accuracy1 = 40, PartsPerMillion, false;
        Accuracy2 = 41, PartsPerMillion, false;
        Accuracy3 = 42, PartsPerMillion, false;
        Accuracy4 = 43, PartsPerMillion, false;
        Accuracy5 = 44, PartsPerMillion, false;
        Accuracy6 = 45, PartsPerMillion, false;
        Accuracy7 = 46, PartsPerMillion, false;
    }
    Defence => {
        Dodge = 64, PartsPerMillion, false;
        DodgeEquipment = 65, PartsPerMillion, false;
        DodgeAmplification = 66, PartsPerMillion, false;
        DodgeWheel = 67, PartsPerMillion, false;
        MagicShieldCapacity = 68, Points, false;
        MagicShieldCapacityFlat = 69, Points, false;
        MagicShieldCapacityBonus = 70, PartsPerMillion, false;
        PhysicalReflection = 71, Points, false;
        Armor = 72, Points, false;
        Mantra = 73, Points, false;
        DefenceTotal = 74, Points, false;
        DefenceEquipment = 75, Points, false;
        DefenceSkillId = 76, Identifier, false;
        ShieldingSkill = 77, Points, false;
        DefenceWheel = 78, Points, false;
        Mitigation = 79, PartsPerMillion, false;
        MitigationEquipment = 80, PartsPerMillion, false;
        MitigationSkill = 81, PartsPerMillion, false;
        MitigationWheel = 82, PartsPerMillion, false;
        AbsorbPhysical = 83, PartsPerMillion, true;
        AbsorbFire = 84, PartsPerMillion, true;
        AbsorbEarth = 85, PartsPerMillion, true;
        AbsorbEnergy = 86, PartsPerMillion, true;
        AbsorbIce = 87, PartsPerMillion, true;
        AbsorbHoly = 88, PartsPerMillion, true;
        AbsorbDeath = 89, PartsPerMillion, true;
    }
    Misc => {
        Momentum = 100, PartsPerMillion, false;
        MomentumEquipment = 101, PartsPerMillion, false;
        MomentumAmplification = 102, PartsPerMillion, false;
        MomentumWheel = 103, PartsPerMillion, false;
        Transcendence = 104, PartsPerMillion, false;
        TranscendenceEquipment = 105, PartsPerMillion, false;
        TranscendenceAmplification = 106, PartsPerMillion, false;
        Amplification = 107, PartsPerMillion, false;
        Blessings = 108, Points, false;
        MaximumBlessings = 109, Points, false;
        WealthDuplexSeconds = 110, Seconds, false;
        // Other concoctions do not have a native effect owner yet. Their absence is unknown,
        // rather than an assertion that the player's entire donor concoction list is empty.
        OtherConcoctions = 111, Points, false;
    }
}

/// One persisted observation of the existing owners. The timestamp and revisions are sent so
/// a permitted inspection of an offline Character never represents an old sample as live.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombatStats {
    pub observed_at_unix_ms: u64,
    pub character_revision: u64,
    pub equipment_revision: u64,
    tab: CharacterTab,
    values: Vec<Option<i64>>,
}

impl CombatStats {
    pub fn new(
        tab: CharacterTab,
        observed_at_unix_ms: u64,
        character_revision: u64,
        equipment_revision: u64,
    ) -> Result<Self, TabWireError> {
        let keys = CombatStat::for_tab(tab);
        if keys.is_empty() || character_revision == 0 || equipment_revision == 0 {
            return Err(TabWireError);
        }
        Ok(Self {
            observed_at_unix_ms,
            character_revision,
            equipment_revision,
            tab,
            values: vec![None; keys.len()],
        })
    }

    pub const fn tab(&self) -> CharacterTab {
        self.tab
    }

    pub fn get(&self, key: CombatStat) -> Option<i64> {
        CombatStat::for_tab(self.tab)
            .iter()
            .position(|candidate| *candidate == key)
            .and_then(|at| self.values[at])
    }

    pub fn set(&mut self, key: CombatStat, value: i64) -> Result<(), TabWireError> {
        if value < 0 && !key.permits_negative() {
            return Err(TabWireError);
        }
        let at = CombatStat::for_tab(self.tab)
            .iter()
            .position(|candidate| *candidate == key)
            .ok_or(TabWireError)?;
        self.values[at] = Some(value);
        Ok(())
    }

    pub(super) fn encode_body(&self, bytes: &mut Vec<u8>) -> Result<(), TabWireError> {
        if self.character_revision == 0 || self.equipment_revision == 0 {
            return Err(TabWireError);
        }
        bytes.extend_from_slice(&self.observed_at_unix_ms.to_le_bytes());
        bytes.extend_from_slice(&self.character_revision.to_le_bytes());
        bytes.extend_from_slice(&self.equipment_revision.to_le_bytes());
        bytes.push(u8::try_from(self.values.len()).map_err(|_| TabWireError)?);
        for (key, value) in CombatStat::for_tab(self.tab).iter().zip(&self.values) {
            bytes.push(*key as u8);
            bytes.push(u8::from(value.is_some()));
            if let Some(value) = value {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
        Ok(())
    }

    /// Strict reader shared by durable read-back and client consumers: exact fields, order,
    /// knowledge tags and length. Unknown data can never silently become a zero.
    pub fn decode(payload: &[u8]) -> Result<Self, TabWireError> {
        let tab = payload
            .first()
            .copied()
            .and_then(CharacterTab::from_id)
            .ok_or(TabWireError)?;
        let keys = CombatStat::for_tab(tab);
        if keys.is_empty() || payload.len() < 26 || usize::from(payload[25]) != keys.len() {
            return Err(TabWireError);
        }
        let number = |at| {
            payload
                .get(at..at + 8)
                .and_then(|bytes| bytes.try_into().ok())
                .map(u64::from_le_bytes)
                .ok_or(TabWireError)
        };
        let mut value = Self::new(tab, number(1)?, number(9)?, number(17)?)?;
        let mut at = 26;
        for key in keys {
            if payload.get(at) != Some(&(*key as u8)) {
                return Err(TabWireError);
            }
            let known = *payload.get(at + 1).ok_or(TabWireError)?;
            at += 2;
            match known {
                0 => (),
                1 => {
                    let bytes = payload
                        .get(at..at + 8)
                        .and_then(|bytes| bytes.try_into().ok())
                        .ok_or(TabWireError)?;
                    value.set(*key, i64::from_le_bytes(bytes))?;
                    at += 8;
                }
                _ => return Err(TabWireError),
            }
        }
        if at != payload.len() {
            return Err(TabWireError);
        }
        Ok(value)
    }
}

#[cfg(test)]
#[path = "combat_tests.rs"]
mod tests;
