#[cfg(test)]
mod tests {
    use super::*;

    fn shifted(x: u32, y: u32) -> AppearanceMsg {
        AppearanceMsg {
            id: Some(1),
            frame_group: vec![FrameGroupMsg {
                sprite_info: Some(SpriteInfoMsg {
                    sprite_id: vec![1],
                    ..SpriteInfoMsg::default()
                }),
            }],
            flags: Some(FlagsMsg {
                shift: Some(ShiftMsg {
                    x: Some(x),
                    y: Some(y),
                }),
                ..FlagsMsg::default()
            }),
        }
    }

    #[test]
    fn an_oversized_displacement_makes_the_entry_unusable() {
        let kept = convert(1, shifted(8, MAX_DISPLACEMENT_PX));
        assert_eq!(
            kept.map(|kept| (kept.displacement_x, kept.displacement_y)),
            Some((8, MAX_DISPLACEMENT_PX))
        );
        assert!(convert(1, shifted(u32::MAX, 0)).is_none());
        assert!(convert(1, shifted(0, MAX_DISPLACEMENT_PX + 1)).is_none());
    }

    fn with_flags(width: u32, flags: FlagsMsg) -> Option<Appearance> {
        let message = AppearanceMsg {
            id: Some(1),
            frame_group: vec![FrameGroupMsg {
                sprite_info: Some(SpriteInfoMsg {
                    pattern_width: Some(width),
                    sprite_id: vec![1; width as usize],
                    ..SpriteInfoMsg::default()
                }),
            }],
            flags: Some(flags),
        };
        convert(1, message)
    }

    #[test]
    fn a_hangable_entry_follows_the_hook_of_its_tile() {
        let hook = |direction| FlagsMsg {
            hook: Some(HookMsg {
                direction: Some(direction),
            }),
            ..FlagsMsg::default()
        };
        let hooks: Vec<_> = [hook(1), hook(2), hook(7)]
            .into_iter()
            .map(|flags| with_flags(1, flags).map(|wall| wall.hook))
            .collect();
        assert_eq!(
            hooks,
            [Some(Hook::South), Some(Hook::East), Some(Hook::None)]
        );

        let hanging = FlagsMsg {
            hang: Some(true),
            ..FlagsMsg::default()
        };
        let on = |width, hook, x| {
            with_flags(width, hanging.clone()).map(|item| {
                pattern(
                    &item,
                    Placement {
                        hook,
                        ..Placement::at(x, 5, -7)
                    },
                )
            })
        };
        assert_eq!(on(3, Hook::None, 1), Some((0, 0, 0)));
        assert_eq!(on(3, Hook::South, 1), Some((1, 0, 0)));
        assert_eq!(on(3, Hook::East, 1), Some((2, 0, 0)));
        assert_eq!(on(2, Hook::East, 1), Some((0, 0, 0)));
        assert_eq!(on(1, Hook::South, 0), Some((0, 0, 0)));
    }
}
