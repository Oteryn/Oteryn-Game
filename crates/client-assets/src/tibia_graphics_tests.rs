use super::fixture::{self, ADVANCE, GLYPH_HEIGHT, GLYPH_WIDTH, X_OFFSET, Y_OFFSET};
use super::*;

fn scratch(label: &str) -> std::io::Result<std::path::PathBuf> {
    let dir = std::env::temp_dir().join(format!(
        "oteryn-tibia-graphics-{label}-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

#[test]
fn bundle_font_and_flags_decode_from_their_pinned_entries() -> Result<(), Box<dyn std::error::Error>>
{
    let dir = scratch("decode")?;
    let pins = fixture::write(&dir)?;
    let graphics = TibiaGraphics::load_pinned(&dir, &pins)?;
    let font = graphics.font();
    assert_eq!(font.line_height(), 12);
    let glyph = font.glyph('7').ok_or("digit glyph")?;
    assert_eq!(
        (
            glyph.width,
            glyph.height,
            glyph.x_offset,
            glyph.y_offset,
            glyph.advance
        ),
        (GLYPH_WIDTH, GLYPH_HEIGHT, X_OFFSET, Y_OFFSET, ADVANCE)
    );
    for y in 0..usize::from(GLYPH_HEIGHT) {
        for x in 0..usize::from(GLYPH_WIDTH) {
            let expected = match fixture::glyph_index('7', x, y) {
                0 => None,
                1 => Some([0, 0, 0, 255]),
                2 => Some([192, 192, 192, 255]),
                _ => Some([255, 255, 255, 255]),
            };
            assert_eq!(glyph.pixel(x, y), expected, "pixel {x},{y}");
        }
    }
    assert_eq!(
        Glyph::tint([192, 192, 192, 255], [10, 200, 30]),
        [10, 200, 30, 255]
    );
    assert_eq!(Glyph::tint([0, 0, 0, 255], [10, 200, 30]), [0, 0, 0, 255]);
    assert!(font.glyph('!').is_none());
    for value in 1..=SKULL_ICONS as u8 {
        let icon = graphics.skull(value).ok_or("skull")?;
        assert_eq!(icon.len(), ICON_PX * ICON_PX * 4);
        assert_eq!(icon[..4], [0; 4], "clear corner");
        assert_eq!(
            icon[4..8],
            fixture::icon_color(true, usize::from(value) - 1)
        );
    }
    for value in 1..=SHIELD_ICONS as u8 {
        let icon = graphics.shield(value).ok_or("shield")?;
        assert_eq!(
            icon[4..8],
            fixture::icon_color(false, usize::from(value) - 1)
        );
    }
    for value in 1..=GUILD_ICONS as u8 {
        let icon = graphics.guild(value).ok_or("guild emblem")?;
        assert_eq!(icon.len(), ICON_PX * ICON_PX * 4);
        assert_eq!(icon[4..8], fixture::guild_color(usize::from(value) - 1));
    }
    assert!(graphics.guild(0).is_none() && graphics.guild(6).is_none());
    assert!(graphics.skull(0).is_none() && graphics.skull(7).is_none());
    assert!(graphics.shield(0).is_none() && graphics.shield(12).is_none());
    std::fs::remove_dir_all(dir)?;
    Ok(())
}

#[test]
fn unpinned_missing_or_foreign_bundles_are_refused() -> Result<(), Box<dyn std::error::Error>> {
    let dir = scratch("refuse")?;
    assert!(matches!(
        TibiaGraphics::load(&dir),
        Err(AssetError::Io { .. })
    ));
    let pins = fixture::write(&dir)?;
    // The generated entries are not the official ones.
    assert!(matches!(
        TibiaGraphics::load(&dir),
        Err(AssetError::HashMismatch { .. })
    ));
    let mut wrong = pins.clone();
    wrong.shields = pins.skulls.clone();
    assert_eq!(
        TibiaGraphics::load_pinned(&dir, &wrong),
        Err(AssetError::HashMismatch {
            file: SHIELDS.into()
        })
    );
    std::fs::write(dir.join(GRAPHICS_FILE), b"qres\0\0\0\x09")?;
    assert!(matches!(
        TibiaGraphics::load_pinned(&dir, &pins),
        Err(AssetError::Malformed { .. })
    ));
    let (mut bytes, _) = fixture::bundle()?;
    bytes.truncate(bytes.len() / 2);
    std::fs::write(dir.join(GRAPHICS_FILE), bytes)?;
    assert!(TibiaGraphics::load_pinned(&dir, &pins).is_err());
    std::fs::remove_dir_all(dir)?;
    Ok(())
}
