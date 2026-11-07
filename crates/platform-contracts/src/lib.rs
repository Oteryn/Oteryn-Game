//! Non-secret Platform values that intentionally contain no gameplay route or credential.

use oteryn_foundation::{BoundError, BoundedText};

macro_rules! bounded_reference {
    ($name:ident, $max:expr) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(BoundedText);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, BoundError> {
                BoundedText::new(value, $max).map(Self)
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }
        }
    };
}

bounded_reference!(ClientAccountContextId, 96);
bounded_reference!(DirectoryWorldRef, 96);
bounded_reference!(DirectoryCharacterRef, 96);
bounded_reference!(DirectoryChannelRef, 96);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClientDirectoryEpoch(u64);

impl ClientDirectoryEpoch {
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryChannel {
    pub channel_ref: DirectoryChannelRef,
    pub display_name: BoundedText,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryCharacter {
    pub character_ref: DirectoryCharacterRef,
    pub display_name: BoundedText,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryWorld {
    pub world_ref: DirectoryWorldRef,
    pub display_name: BoundedText,
    pub channels: Vec<DirectoryChannel>,
    pub characters: Vec<DirectoryCharacter>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectorySnapshot {
    pub epoch: ClientDirectoryEpoch,
    pub worlds: Vec<DirectoryWorld>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameplayAvailability {
    PreNativeProtocol,
}

impl GameplayAvailability {
    #[must_use]
    pub const fn is_available(self) -> bool {
        false
    }

    #[must_use]
    pub const fn player_message(self) -> &'static str {
        "Native gameplay is not available in this pre-protocol build."
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pre_native_state_is_fail_closed() {
        let availability = GameplayAvailability::PreNativeProtocol;
        assert!(!availability.is_available());
        assert!(availability.player_message().contains("not available"));
    }
}
