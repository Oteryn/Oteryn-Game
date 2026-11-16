//! Creature names use Tibia's game font from the official graphics bundle in the asset root.
use super::*;
use oteryn_client_assets::tibia_graphics::GraphicsPins;
use oteryn_client_assets::tibia_graphics::fixture::{self, ADVANCE, GLYPH_HEIGHT, GLYPH_WIDTH};

fn bundle_root(label: &str) -> std::io::Result<(std::path::PathBuf, GraphicsPins)> {
    let root = std::env::temp_dir().join(format!(
        "oteryn-client-names-{label}-{}",
        std::process::id()
    ));
    let files = root.join("content/assets/files");
    std::fs::create_dir_all(&files)?;
    Ok((root, fixture::write(&files)?))
}

fn creature(name: &str) -> Result<VisibleAppearance, BatchError> {
    Ok(VisibleAppearance {
        entity: oteryn_session::WorldSpatialEntity {
            motion: None,
            entity: oteryn_session::EntityRef {
                identity: [8; 16],
                generation: 1,
            },
            kind: oteryn_session::EntityKind::Creature,
            position: oteryn_session::ActorPosition {
                x: 101,
                y: 200,
                floor: 0,
            },
            display_name: Some(
                oteryn_session::EntityDisplayName::new(name)
                    .map_err(|_| BatchError::InvalidView)?,
            ),
            detail: EntityDetail::Actor {
                direction: oteryn_session::StepDirection::South,
                appearance_ref: 1,
                health_percent: 100,
            },
        },
        look: None,
        light: None,
        forge_stacks: None,
    })
}

#[test]
fn creature_names_draw_the_tibia_game_font_from_the_asset_bundle()
-> Result<(), Box<dyn std::error::Error>> {
    let (root, pins) = bundle_root("font")?;
    let tile = TileCoord::new(100, 200);
    let mut scene =
        Scene::centered_on(Arc::new(World::builtin()?), None, tile, StepDir::South, &[])?;
    scene.cache.graphics_pins = Some(pins);
    scene.set_presentations_from(&[creature("Rat")?], 0, Some(root.clone()))?;
    let quads = scene.sprites.instances();
    let name = &quads[quads.len() - 3..];
    let atlas = scene.atlas();
    let width = atlas.width() as usize;
    for (index, (quad, character)) in name.iter().zip("Rat".chars()).enumerate() {
        assert_eq!(quad.size, [32.0, 32.0], "one bundle glyph per cell");
        assert_eq!(
            quad.position[0] - name[0].position[0],
            index as f32 * f32::from(ADVANCE),
            "the font's own advance"
        );
        let left = (quad.uv[0] * atlas.width() as f32).round() as usize;
        let top = (quad.uv[1] * atlas.height() as f32).round() as usize;
        for y in 0..usize::from(GLYPH_HEIGHT) {
            for x in 0..usize::from(GLYPH_WIDTH) {
                let at = ((top + y) * width + left + x) * 4;
                // Full health paints the fill green; the outline stays black.
                let expected = match fixture::glyph_index(character, x, y) {
                    0 => [0; 4],
                    1 => [0, 0, 0, 255],
                    _ => [0, 188, 0, 255],
                };
                assert_eq!(atlas.rgba()[at..at + 4], expected, "{character} {x},{y}");
            }
        }
    }
    // A character the bundle font lacks keeps the builtin glyph.
    scene.set_presentations_from(&[creature("Rat!")?], 0, Some(root.clone()))?;
    let quads = scene.sprites.instances();
    let last = quads[quads.len() - 1];
    assert_eq!(last.size, [8.0, 12.0]);
    assert_eq!(
        last.position[0] - quads[quads.len() - 2].position[0],
        f32::from(ADVANCE) + 1.0,
        "the builtin glyph follows the last bundle glyph's advance"
    );
    std::fs::remove_dir_all(root)?;
    Ok(())
}
