//! The shared damage element (`docs/contracts/protocol-oteryn/v1/damage_element_v1.proto`).
//!
//! The SPELL-PRESENT-0 §2 content damage types in content order, healing excluded. Every payload
//! naming a damage element uses this type; zero and unknown wire values have no element.

/// `DamageElement`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DamageElement {
    Physical = 1,
    Fire = 2,
    Earth = 3,
    Energy = 4,
    Ice = 5,
    Holy = 6,
    Death = 7,
    LifeDrain = 8,
    ManaDrain = 9,
    Drowning = 10,
    Untyped = 11,
}

impl DamageElement {
    /// The wire value, never 0.
    #[must_use]
    pub const fn wire(self) -> u32 {
        self as u32
    }

    /// The element of a wire value; `None` for 0 and unknown values, which callers refuse.
    #[must_use]
    pub const fn from_wire(value: u32) -> Option<Self> {
        Some(match value {
            1 => Self::Physical,
            2 => Self::Fire,
            3 => Self::Earth,
            4 => Self::Energy,
            5 => Self::Ice,
            6 => Self::Holy,
            7 => Self::Death,
            8 => Self::LifeDrain,
            9 => Self::ManaDrain,
            10 => Self::Drowning,
            11 => Self::Untyped,
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::DamageElement;

    const SCHEMA: &str =
        include_str!("../../../docs/contracts/protocol-oteryn/v1/damage_element_v1.proto");

    /// Every wire value from 1 to 11 round-trips and matches the proto; 0 and 12 have no element.
    #[test]
    fn the_elements_match_the_proto_and_refuse_zero_and_unknown() {
        let names = [
            "PHYSICAL",
            "FIRE",
            "EARTH",
            "ENERGY",
            "ICE",
            "HOLY",
            "DEATH",
            "LIFE_DRAIN",
            "MANA_DRAIN",
            "DROWNING",
            "UNTYPED",
        ];
        for (value, name) in (1..=11).zip(names) {
            let element = DamageElement::from_wire(value);
            assert_eq!(element.map(DamageElement::wire), Some(value), "{name}");
            assert!(SCHEMA.contains(&format!("DAMAGE_ELEMENT_{name} = {value};")));
        }
        assert!(SCHEMA.contains("DAMAGE_ELEMENT_UNSPECIFIED = 0;"));
        assert!(!SCHEMA.contains("HEALING"));
        assert_eq!(DamageElement::from_wire(0), None);
        assert_eq!(DamageElement::from_wire(12), None);
    }
}
