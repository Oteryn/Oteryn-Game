//! Character display name and its uniqueness key, naming policy revision 1 (CHAR-NAME-1, owner
//! answers of 2026-09-30 recorded in `CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md` §6.1).
//!
//! A name is 2..=29 ASCII letters in words joined by single spaces, with no leading or trailing
//! space (tibia.com manual §starting 2.2.1: at most 29 characters, no digits, no special
//! characters). The comparison key is the name in ASCII lower case with the spaces removed, and it
//! is unique in one global namespace across every World. Migration 0021 enforces the same grammar
//! and key in `game_character_is_name` and `game_character_name_key`.

/// Naming policy revision these rules implement.
pub const NAMING_POLICY_REVISION: u16 = 1;
pub const MIN_NAME_CHARS: usize = 2;
pub const MAX_NAME_CHARS: usize = 29;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidCharacterName;

/// A display name valid under naming policy revision 1.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct CharacterName(String);

// A name identifies a player, so it is never written to diagnostics.
impl std::fmt::Debug for CharacterName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CharacterName(..)")
    }
}

impl CharacterName {
    pub fn parse(value: &str) -> Result<Self, InvalidCharacterName> {
        let bytes = value.as_bytes();
        if !(MIN_NAME_CHARS..=MAX_NAME_CHARS).contains(&bytes.len())
            || !bytes.iter().all(|b| b.is_ascii_alphabetic() || *b == b' ')
            || value.split(' ').any(str::is_empty)
        {
            return Err(InvalidCharacterName);
        }
        Ok(Self(value.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Uniqueness key: ASCII lower case without spaces, so `Al Dric` and `aldric` collide.
    #[must_use]
    pub fn comparison_key(&self) -> String {
        self.0
            .bytes()
            .filter(|b| *b != b' ')
            .map(|b| char::from(b.to_ascii_lowercase()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_one_accepts_letters_and_single_inner_spaces() {
        for name in ["Ab", "Aldric", "Sir Aldric Of Thais", &"a".repeat(29)] {
            assert_eq!(CharacterName::parse(name).map(|n| n.0), Ok(name.to_owned()));
        }
    }

    #[test]
    fn policy_one_rejects_everything_else() {
        for name in [
            "",
            "A",
            &"a".repeat(30),
            " Aldric",
            "Aldric ",
            "Al  Dric",
            "Aldric1",
            "Al'dric",
            "Al-dric",
            "Aldríc",
            "Al\tdric",
            "Al\u{200b}dric",
        ] {
            assert_eq!(CharacterName::parse(name), Err(InvalidCharacterName));
        }
    }

    #[test]
    fn comparison_key_folds_case_and_spaces() -> Result<(), InvalidCharacterName> {
        let key = CharacterName::parse("Al Dric")?.comparison_key();
        assert_eq!(key, "aldric");
        assert_eq!(CharacterName::parse("ALDRIC")?.comparison_key(), key);
        assert_ne!(CharacterName::parse("Aldrik")?.comparison_key(), key);
        assert!(!format!("{:?}", CharacterName::parse("Aldric")?).contains("Aldric"));
        Ok(())
    }
}
