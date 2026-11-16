//! Value numbers, spell words and party/PvP marks use Tibia's font and flag artwork.
use crate::play::{PlayView, play_channel};
use oteryn_client_assets::tibia_graphics::fixture::{self, ADVANCE, GLYPH_HEIGHT, GLYPH_WIDTH};
use oteryn_client_assets::tibia_graphics::{GraphicsPins, ICON_PX};
use oteryn_renderer::{QuadInstance, TileCoord};
use oteryn_session::{
    ActorPosition, AppliedDelta, EntityRef, PresentationColor, PresentationEvent,
    PresentationEventKind, PresentationValue, PresentationValueKind, SessionEvent,
};
use std::collections::BTreeSet;
use std::time::Instant;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn bundle_root(label: &str) -> std::io::Result<(std::path::PathBuf, GraphicsPins)> {
    let root = std::env::temp_dir().join(format!(
        "oteryn-client-overlays-{label}-{}",
        std::process::id()
    ));
    let files = root.join("content/assets/files");
    std::fs::create_dir_all(&files)?;
    Ok((root, fixture::write(&files)?))
}

/// Glyph pixels of `text` laid out with the font's advances, relative to the first pixel.
fn expected_pixels(text: &str) -> BTreeSet<(i32, i32)> {
    let mut pixels = BTreeSet::new();
    for (index, character) in text.chars().enumerate() {
        for y in 0..usize::from(GLYPH_HEIGHT) {
            for x in 0..usize::from(GLYPH_WIDTH) {
                if fixture::glyph_index(character, x, y) != 0 {
                    pixels.insert((
                        index as i32 * i32::from(ADVANCE) + x as i32,
                        y as i32,
                    ));
                }
            }
        }
    }
    pixels
}

fn drawn_pixels(quads: &[QuadInstance]) -> BTreeSet<(i32, i32)> {
    let left = quads.iter().map(|quad| quad.position[0]).fold(f32::MAX, f32::min);
    let top = quads.iter().map(|quad| quad.position[1]).fold(f32::MAX, f32::min);
    quads
        .iter()
        .map(|quad| {
            (
                (quad.position[0] - left) as i32,
                (quad.position[1] - top) as i32,
            )
        })
        .collect()
}

fn normalised(pixels: BTreeSet<(i32, i32)>) -> BTreeSet<(i32, i32)> {
    let left = pixels.iter().map(|pixel| pixel.0).min().unwrap_or(0);
    let top = pixels.iter().map(|pixel| pixel.1).min().unwrap_or(0);
    pixels.into_iter().map(|(x, y)| (x - left, y - top)).collect()
}

#[test]
fn value_numbers_draw_the_tibia_game_font_outlined_in_their_colour() -> TestResult {
    let (root, pins) = bundle_root("numbers")?;
    let mut view = PlayView::new(TileCoord::new(0, 0), 0, &[])?;
    view.scene.prime_tibia_graphics(root.clone(), pins);
    let base = view.scene().sprites().len();
    let now = Instant::now();
    view.receive_presentation(
        &[PresentationEventKind::ValueText {
            kind: PresentationValueKind::Healed,
            position: ActorPosition { x: 2, y: 2, floor: 0 },
            target: EntityRef {
                identity: [1; 16],
                generation: 1,
            },
            attacker: None,
            primary: PresentationValue {
                value: 70,
                color: PresentationColor::LightGreen,
            },
            secondary: None,
        }],
        now,
    );
    view.project_presentation(now)?;
    let quads = &view.scene().sprites().instances()[base..];
    assert!(quads.iter().all(|quad| quad.size == [1.0, 1.0]));
    assert_eq!(drawn_pixels(quads), normalised(expected_pixels("70")));
    let colours: BTreeSet<_> = quads.iter().map(|quad| quad.uv[0].to_bits()).collect();
    assert_eq!(colours.len(), 2, "black outline and the green fill");
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn spell_words_draw_the_tibia_game_font_without_a_length_cap() -> TestResult {
    let (root, pins) = bundle_root("words")?;
    // Wider than the former 64-character cap, narrower than the view.
    let words = format!("{}utevo lux", "exura gran mas res ".repeat(3));
    assert!(words.len() > 64, "longer than the former cap");
    let (link, _commands, outbox) = play_channel();
    let mut view = PlayView::new(TileCoord::new(0, 0), 0, &[])?;
    view.scene.prime_tibia_graphics(root.clone(), pins);
    outbox.spell_catalog_initialize(
        true,
        Some(
            oteryn_session::spell_catalog::encode_catalog(&[
                oteryn_session::spell_catalog::SpellCatalogEntry {
                    cast_index: 19,
                    name: "Healing".into(),
                    words: words.clone(),
                    level: 8,
                    mana: 20,
                    mana_percent: false,
                    soul: 0,
                    learned: true,
                    available: true,
                    cooldown_groups: vec![1],
                    rune_item: None,
                },
            ])
            .map_err(|error| format!("{error:?}"))?,
        ),
    )?;
    let base = view.scene().sprites().len();
    outbox.push(vec![SessionEvent::Presentation(AppliedDelta {
        server_sequence: 1,
        base_revision: 0,
        new_revision: 1,
        value: vec![PresentationEvent {
            decision_ordinal: 1,
            emission_ordinal: 1,
            kind: PresentationEventKind::SpellWords {
                caster: EntityRef {
                    identity: [1; 16],
                    generation: 1,
                },
                position: ActorPosition { x: 0, y: 0, floor: 0 },
                spell: 19,
            },
        }],
    })]);
    view.tick(&link).map_err(|error| format!("{error:?}"))?;
    let now = view.visuals.first().ok_or("spell words cue")?.received;
    view.project_presentation(now)?;
    let quads: Vec<_> = view.scene().sprites().instances()[base..]
        .iter()
        .copied()
        .filter(|quad| quad.size == [1.0, 1.0])
        .collect();
    assert_eq!(drawn_pixels(&quads), normalised(expected_pixels(&words)));
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn skull_and_party_shield_marks_draw_the_tibia_flag_artwork() -> TestResult {
    let (root, pins) = bundle_root("flags")?;
    let mut view = PlayView::new(TileCoord::new(0, 0), 0, &[])?;
    view.scene.prime_tibia_graphics(root.clone(), pins);
    let entity = oteryn_session::WorldSpatialEntity {
        motion: None,
        entity: EntityRef {
            identity: [5; 16],
            generation: 1,
        },
        kind: oteryn_session::EntityKind::Player,
        position: ActorPosition { x: 3, y: 3, floor: 0 },
        display_name: None,
        detail: oteryn_session::EntityDetail::Actor {
            direction: oteryn_session::StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
        },
    };
    view.entities = vec![entity];
    view.party_icons = vec![oteryn_client_runtime::party_skull::PartyIcon {
        entity: entity.entity,
        shield: 3,
        skull: 4,
    }];
    view.set_presentations(vec![oteryn_session::VisibleAppearance {
        entity,
        look: None,
        light: None,
        forge_stacks: None,
    }])?;
    view.project_presentation(Instant::now())?;
    let atlas = view.scene().atlas();
    let width = atlas.width() as usize;
    let icon_quads: Vec<_> = view
        .scene()
        .sprites()
        .instances()
        .iter()
        .filter(|quad| quad.size == [32.0, 32.0])
        .filter_map(|quad| {
            let left = (quad.uv[0] * atlas.width() as f32).round() as usize;
            let top = (quad.uv[1] * atlas.height() as f32).round() as usize;
            let at = ((top + 1) * width + left + 1) * 4;
            Some((quad.position, atlas.rgba().get(at..at + 4)?.to_vec()))
        })
        .collect();
    let shield = fixture::icon_color(false, 2).to_vec();
    let skull = fixture::icon_color(true, 3).to_vec();
    let at = |colour: &Vec<u8>| {
        icon_quads
            .iter()
            .find(|(_, pixel)| pixel == colour)
            .map(|(position, _)| *position)
    };
    let (shield_at, skull_at) = (at(&shield).ok_or("shield icon")?, at(&skull).ok_or("skull icon")?);
    assert_eq!(skull_at[1], shield_at[1]);
    assert!(skull_at[0] - shield_at[0] >= ICON_PX as f32, "side by side");
    std::fs::remove_dir_all(root)?;
    Ok(())
}
