//! Keep both accepted increment wire forms and their receipt serialization.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum Increment {
    Amount(i64),
    Flag(bool),
}

impl super::Storage {
    pub fn increment_delta(&self) -> Option<i64> {
        match self.increment {
            Some(Increment::Amount(delta)) => Some(delta),
            Some(Increment::Flag(true)) => Some(self.set),
            None | Some(Increment::Flag(false)) => None,
        }
    }
}

pub(super) fn read<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Increment>, D::Error> {
    Ok(Option::<Increment>::deserialize(deserializer)?
        .filter(|value| !matches!(value, Increment::Flag(false))))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn both_wire_forms_keep_their_exact_serialization() -> Result<(), serde_json::Error> {
        for wire in ["1", "-2", "true"] {
            let value: Increment = serde_json::from_str(wire)?;
            assert_eq!(serde_json::to_string(&value)?, wire);
        }
        assert!(serde_json::from_str::<Increment>("\"true\"").is_err());
        assert!(serde_json::from_str::<Increment>("1.5").is_err());
        Ok(())
    }
}
