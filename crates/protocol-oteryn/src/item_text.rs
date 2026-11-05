//! Read-only carried authored book windows returned by USE under ITEM_TEXT_V1.

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn item_text_use_roundtrip_preserves_utf8_and_refuses_bounds_and_trailing_fields() {
        let view = ItemTextView { handle: 7, text: "A quest book\né".into(), revision: 1 };
        let bytes = encode_item_text_result(&view).expect("view");
        assert_eq!(decode_item_text_result(&bytes), Ok(view.clone()));
        let mut trailing = bytes.clone();
        trailing.extend([0x20, 1]);
        assert!(decode_item_text_result(&trailing).is_err());
        for text in [String::new(), "a".repeat(MAX_ITEM_TEXT_BYTES + 1), "é".repeat(4001), "a\0b".into()] {
            let invalid = ItemTextView { text, ..view.clone() };
            assert!(encode_item_text_result(&invalid).is_err());
        }
        assert!(encode_item_text_result(&ItemTextView { handle: 0, ..view.clone() }).is_err());
        assert!(encode_item_text_result(&ItemTextView { revision: 0, ..view }).is_err());
    }
}
