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

fn bundle_root() -> std::io::Result<(tempfile::TempDir, GraphicsPins)> {
    let fixture = tempfile::tempdir()?;
    let root = fixture.path();
    let files = root.join("content/assets/files");
    std::fs::create_dir_all(&files)?;
    Ok((fixture, fixture::write(&files)?))
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
    let (fixture, pins) = bundle_root()?;
    let root = fixture.path();
    let mut view = PlayView::new(TileCoord::new(0, 0), 0, &[])?;
    view.scene.prime_tibia_graphics(root.to_path_buf(), pins);
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
    Ok(())
}

#[test]
fn spell_words_draw_the_tibia_game_font_without_a_length_cap() -> TestResult {
    let (fixture, pins) = bundle_root()?;
    let root = fixture.path();
    // Wider than the former 64-character cap, narrower than the view.
    let words = format!("{}utevo lux", "exura gran mas res ".repeat(3));
    assert!(words.len() > 64, "longer than the former cap");
    let (link, _commands, outbox) = play_channel();
    let mut view = PlayView::new(TileCoord::new(0, 0), 0, &[])?;
    view.scene.prime_tibia_graphics(root.to_path_buf(), pins);
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
    Ok(())
}

#[test]
fn skull_and_party_shield_marks_draw_the_tibia_flag_artwork() -> TestResult {
    let (fixture, pins) = bundle_root()?;
    let root = fixture.path();
    let mut view = PlayView::new(TileCoord::new(0, 0), 0, &[])?;
    view.scene.prime_tibia_graphics(root.to_path_buf(), pins);
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
    Ok(())
}

fn guild_member() -> oteryn_session::WorldSpatialEntity {
    oteryn_session::WorldSpatialEntity {
        motion: None,
        entity: EntityRef {
            identity: [6; 16],
            generation: 2,
        },
        kind: oteryn_session::EntityKind::Player,
        position: ActorPosition { x: 3, y: 3, floor: 0 },
        display_name: None,
        detail: oteryn_session::EntityDetail::Actor {
            direction: oteryn_session::StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
        },
    }
}

#[test]
fn server_guild_emblems_draw_the_tibia_flag_artwork_beside_the_party_marks() -> TestResult {
    use oteryn_session::guild_emblem::{Emblem, GuildEmblem};
    let (fixture, pins) = bundle_root()?;
    let root = fixture.path();
    let (link, _commands, outbox) = play_channel();
    let mut view = PlayView::new(TileCoord::new(0, 0), 0, &[])?;
    view.scene.prime_tibia_graphics(root.to_path_buf(), pins);
    let entity = guild_member();
    view.entities = vec![entity];
    outbox.push(vec![SessionEvent::GuildEmblems(AppliedDelta {
        server_sequence: 1,
        base_revision: 0,
        new_revision: 1,
        value: vec![GuildEmblem {
            entity: entity.entity,
            emblem: Emblem::Enemy,
        }],
    })]);
    view.tick(&link).map_err(|error| format!("{error:?}"))?;
    view.party_icons = vec![oteryn_client_runtime::party_skull::PartyIcon {
        entity: entity.entity,
        shield: 3,
        skull: 0,
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
    let at = |colour: Vec<u8>| {
        icon_quads
            .iter()
            .find(|(_, pixel)| *pixel == colour)
            .map(|(position, _)| *position)
    };
    let shield_at = at(fixture::icon_color(false, 2).to_vec()).ok_or("shield icon")?;
    // Emblem 2 is the red enemy icon, the second of guild.png.
    let emblem_at = at(fixture::guild_color(1).to_vec()).ok_or("guild emblem icon")?;
    assert_eq!(emblem_at[1], shield_at[1]);
    assert!(emblem_at[0] - shield_at[0] >= ICON_PX as f32, "beside the marks");
    for other in [0, 2, 3, 4] {
        assert!(at(fixture::guild_color(other).to_vec()).is_none());
    }
    Ok(())
}

#[test]
fn guild_emblems_fall_back_to_builtin_glyphs_without_the_bundle() -> TestResult {
    use oteryn_session::guild_emblem::{Emblem, GuildEmblem};
    let entity = guild_member();
    let quads = |emblems: Vec<GuildEmblem>| -> Result<usize, Box<dyn std::error::Error>> {
        let mut view = PlayView::new(TileCoord::new(0, 0), 0, &[])?;
        view.entities = vec![entity];
        view.guild_emblems = emblems;
        view.set_presentations(vec![oteryn_session::VisibleAppearance {
            entity,
            look: None,
            light: None,
            forge_stacks: None,
        }])?;
        view.project_presentation(Instant::now())?;
        Ok(view.scene().sprites().len())
    };
    let none = quads(Vec::new())?;
    for emblem in [Emblem::Ally, Emblem::Enemy, Emblem::Neutral, Emblem::Member, Emblem::Other] {
        assert!(
            quads(vec![GuildEmblem {
                entity: entity.entity,
                emblem,
            }])? > none,
            "{emblem:?} glyph"
        );
    }
    Ok(())
}
