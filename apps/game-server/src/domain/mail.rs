//! MAIL-0 §4/§5 text and address values. These establish no Item/content authority.

use super::character_name::CharacterName;

pub const LETTER_TEXT_SCALARS_MAX: usize = 1_999;
pub const LABEL_TEXT_SCALARS_MAX: usize = 79;

/// Accepted mail text class, not proof that an ItemInstance is writable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MailTextClass {
    Letter,
    Label,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MailTextRefusal {
    TooLong,
    ControlCharacter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MailAddressRefusal {
    NoAddress,
    UnknownRecipient,
}

/// Player content. Diagnostics must never expose its body, address or writer.
#[derive(Clone, PartialEq, Eq)]
pub struct MailText(String);

impl std::fmt::Debug for MailText {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("MailText(..)")
    }
}

impl MailText {
    /// UTF-8 is guaranteed by `str`; Unicode scalar count, not grapheme count.
    /// Empty text is valid: the eventual fenced writer deletes the text row.
    /// No normalization, truncation or control-character stripping is performed.
    pub fn parse(class: MailTextClass, text: &str) -> Result<Self, MailTextRefusal> {
        let maximum = match class {
            MailTextClass::Letter => LETTER_TEXT_SCALARS_MAX,
            MailTextClass::Label => LABEL_TEXT_SCALARS_MAX,
        };
        if text.len() > maximum * 4 || text.chars().count() > maximum {
            return Err(MailTextRefusal::TooLong);
        }
        if text
            .chars()
            .any(|character| character != '\n' && character.is_control())
        {
            return Err(MailTextRefusal::ControlCharacter);
        }
        Ok(Self(text.to_owned()))
    }

    /// For the reachable-item view or eventual text store, never audit/log output.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Missing/empty first line is NO_ADDRESS; invalid names reveal no recipient.
    /// Only the first LF-delimited line is trimmed; the town/body is ignored.
    pub fn address(text: Option<&Self>) -> Result<CharacterName, MailAddressRefusal> {
        let first_line = text
            .map(|text| text.0.split('\n').next().unwrap_or_default().trim())
            .unwrap_or_default();
        if first_line.is_empty() {
            return Err(MailAddressRefusal::NoAddress);
        }
        CharacterName::parse(first_line).map_err(|_| MailAddressRefusal::UnknownRecipient)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mail_scalar_and_four_byte_boundaries_are_exact() {
        for (class, limit) in [(MailTextClass::Letter, 1_999), (MailTextClass::Label, 79)] {
            for scalar in ['a', '🦀'] {
                assert!(MailText::parse(class, &scalar.to_string().repeat(limit)).is_ok());
                assert_eq!(
                    MailText::parse(class, &scalar.to_string().repeat(limit + 1)),
                    Err(MailTextRefusal::TooLong)
                );
            }
        }
        // A combining mark is a second scalar, regardless of visual graphemes.
        assert!(
            MailText::parse(MailTextClass::Label, &format!("{}e\u{301}", "a".repeat(77))).is_ok()
        );
        assert_eq!(
            MailText::parse(MailTextClass::Label, &format!("{}e\u{301}", "a".repeat(78))),
            Err(MailTextRefusal::TooLong)
        );
    }

    #[test]
    fn mail_control_characters_are_refused_without_sanitization() {
        assert!(MailText::parse(MailTextClass::Letter, "\n").is_ok());
        for control in ['\0', '\t', '\r', '\u{7f}', '\u{85}', '\u{9f}'] {
            assert_eq!(
                MailText::parse(MailTextClass::Letter, &format!("A{control}B")),
                Err(MailTextRefusal::ControlCharacter)
            );
        }
    }

    #[test]
    fn mail_empty_text_and_address_have_distinct_meaning() {
        assert_eq!(MailText::address(None), Err(MailAddressRefusal::NoAddress));
        for body in ["", "  ", "\nAldric", "   \nAldric"] {
            let text = MailText::parse(MailTextClass::Letter, body);
            assert_eq!(
                text.as_ref().map(|text| MailText::address(Some(text))),
                Ok(Err(MailAddressRefusal::NoAddress))
            );
        }
    }

    #[test]
    fn mail_address_reuses_canonical_name_key_and_ignores_town() -> Result<(), MailTextRefusal> {
        let text = MailText::parse(MailTextClass::Letter, "  Al Dric  \nnot a town 🦀\nbody")?;
        assert_eq!(
            MailText::address(Some(&text)).map(|name| name.comparison_key()),
            Ok("aldric".into())
        );
        for body in ["A", "Aldric1", "Al  Dric", "Aldríc", "Al\u{200b}dric"] {
            let text = MailText::parse(MailTextClass::Letter, body)?;
            assert_eq!(
                MailText::address(Some(&text)),
                Err(MailAddressRefusal::UnknownRecipient)
            );
        }
        Ok(())
    }

    #[test]
    fn mail_text_debug_is_private_and_preserves_the_original_body() -> Result<(), MailTextRefusal> {
        let text = MailText::parse(MailTextClass::Letter, "Aldric\nprivate body 🦀")?;
        assert_eq!(format!("{text:?}"), "MailText(..)");
        assert_eq!(text.as_str(), "Aldric\nprivate body 🦀");
        Ok(())
    }
}
