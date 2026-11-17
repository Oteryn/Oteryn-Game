/// Whether Win32 message metadata identifies touch/pen-promoted mouse input.
pub const fn is_touch_promoted(extra: u32) -> bool {
    extra & 0xffff_ff00 == 0xff51_5700
}

#[cfg(test)]
mod tests {
    use super::is_touch_promoted;

    #[test]
    fn touch_and_pen_promoted_messages_match_with_any_low_byte() {
        for flags in 0..=255 {
            assert!(is_touch_promoted(0xff51_5700 | flags));
        }
    }

    #[test]
    fn real_mouse_and_unrelated_metadata_are_not_touch_promoted() {
        for extra in [
            0,
            1,
            0x80,
            0xff51_5600,
            0xff51_5800,
            0xfe51_5700,
            0xffff_ffff,
        ] {
            assert!(!is_touch_promoted(extra));
        }
    }
}
