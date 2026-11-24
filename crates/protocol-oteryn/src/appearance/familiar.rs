//! Negotiated familiar choices and server confirmation, separate from APP-3's strict payloads.
use serde::{Deserialize, Serialize};

pub const CAPABILITY_FAMILIAR_APPEARANCE_V1: u32 = 579;
pub const QUERY: u32 = 6324;
pub const CHANGE: u32 = 6325;
pub const MAX_CHANGE_BYTES: usize = 320;
pub const MAX_VIEW_BYTES: usize = 65_536;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Choice {
    pub key: String,
    pub look_type: u32,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct View {
    pub root_revision: u64,
    pub choices: Vec<Choice>,
    pub selected: Option<Choice>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Change {
    pub root_revision: u64,
    pub key: String,
}

impl Change {
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > MAX_CHANGE_BYTES {
            return None;
        }
        let change: Self = serde_json::from_slice(bytes).ok()?;
        (change.root_revision != 0 && valid_key(&change.key)).then_some(change)
    }
}

impl View {
    pub fn encode(&self) -> Option<Vec<u8>> {
        if self.root_revision == 0
            || self.choices.len() > 256
            || !self
                .choices
                .windows(2)
                .all(|pair| pair[0].key < pair[1].key)
            || self
                .choices
                .iter()
                .any(|choice| choice.look_type == 0 || !valid_key(&choice.key))
            || self
                .selected
                .as_ref()
                .is_some_and(|choice| !self.choices.contains(choice))
        {
            return None;
        }
        let bytes = serde_json::to_vec(self).ok()?;
        (bytes.len() <= MAX_VIEW_BYTES).then_some(bytes)
    }
}

fn valid_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 256
        && key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b":/._-".contains(&b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn familiar_appearance_accepts_keys_only_and_confirms_a_granted_look()
    -> Result<(), Box<dyn std::error::Error>> {
        let key = "oteryn:creature.knight-familiar";
        let change = Change {
            root_revision: 4,
            key: key.into(),
        };
        assert_eq!(Change::decode(&serde_json::to_vec(&change)?), Some(change));
        assert!(
            Change::decode(br#"{"root_revision":4,"key":"oteryn:familiar","look_type":1}"#)
                .is_none()
        );
        let mut view = View {
            root_revision: 5,
            choices: vec![Choice {
                key: key.into(),
                look_type: 18,
            }],
            selected: Some(Choice {
                key: key.into(),
                look_type: 18,
            }),
        };
        let encoded = view.encode().ok_or("valid familiar catalogue")?;
        assert_eq!(serde_json::from_slice::<View>(&encoded)?, view);
        view.choices.clear();
        assert!(view.encode().is_none());
        Ok(())
    }
}
