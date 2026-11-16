use crate::play::{PlayView, play_channel};
use oteryn_renderer::TileCoord;
use oteryn_session::{
    ActorPosition, AppliedDelta, PresentationEvent, PresentationEventKind, PresentationSource,
    SessionEvent,
};
use std::time::Duration;

#[test]
fn t012_server_effects_and_missiles_reach_animated_scene_and_respect_options()
-> Result<(), Box<dyn std::error::Error>> {
    let mut view = PlayView::new(TileCoord::new(0, 0), 0, &[])?;
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    view.scene.prime_visual_assets(root);
    let (link, _commands, outbox) = play_channel();
    let at = ActorPosition {
        x: 0,
        y: 0,
        floor: 0,
    };
    let baseline = view.scene().sprites().len();
    let effect = PresentationEventKind::MagicEffect {
        position: at,
        effect_id: 1,
        source: PresentationSource::Own,
    };
    let missile = PresentationEventKind::Missile {
        from: at,
        to: ActorPosition { x: 8, ..at },
        missile_id: 1,
        source: PresentationSource::Creatures,
    };
    outbox.push(vec![SessionEvent::Presentation(AppliedDelta {
        server_sequence: 1,
        base_revision: 0,
        new_revision: 1,
        value: [effect, missile]
            .into_iter()
            .enumerate()
            .map(|(i, kind)| PresentationEvent {
                decision_ordinal: 1,
                emission_ordinal: i as u32 + 1,
                kind,
            })
            .collect(),
    })]);
    assert!(view.tick(&link).is_ok());
    let now = view.visuals.first().ok_or("server cue missing")?.received;
    view.project_presentation(now)?;
    let first = view.scene().sprites().instances();
    assert_eq!(
        first.len(),
        baseline + 5,
        "four cells of pinned effect 1 plus one missile cell"
    );
    let effect_uv = first[baseline].uv;
    let missile_position = first[first.len() - 1].position;
    assert_ne!(effect_uv[0], effect_uv[2], "a sprite spans a full UV cell");
    view.project_presentation(now + Duration::from_millis(110))?;
    let next = view.scene().sprites().instances();
    assert_ne!(
        next[baseline].uv, effect_uv,
        "server effect advances to next catalog phase"
    );
    assert_ne!(
        next[next.len() - 1].position,
        missile_position,
        "missile interpolates server endpoints"
    );
    assert_eq!(
        view.scene().sprites().lighting.emissive_from,
        Some(baseline)
    );
    let mut preferences = view.preferences().clone();
    preferences.show_effects = false;
    preferences.show_lights = false;
    view.set_preferences(preferences)?;
    assert_eq!(view.scene().sprites().len(), baseline);
    assert_eq!(view.scene().sprites().lighting.ambient, [255; 3]);
    preferences = view.preferences().clone();
    preferences.show_effects = true;
    view.set_preferences(preferences)?;
    view.project_presentation(now + Duration::from_millis(601))?;
    assert_eq!(
        view.scene().sprites().len(),
        baseline,
        "both visuals expire without looping"
    );
    Ok(())
}

#[test]
fn t012_spell_words_use_server_catalog_only_after_server_success()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_session::spell_catalog::SpellCatalogEntry;
    let mut view = PlayView::new(TileCoord::new(0, 0), 0, &[])?;
    let (link, _commands, outbox) = play_channel();
    outbox.spell_catalog_initialize(
        true,
        Some(
            oteryn_session::spell_catalog::encode_catalog(&[SpellCatalogEntry {
                cast_index: 19,
                name: "Healing".into(),
                words: "exura".into(),
                level: 8,
                mana: 20,
                mana_percent: false,
                soul: 0,
                learned: true,
                available: true,
                cooldown_groups: vec![1],
                rune_item: None,
            }])
            .map_err(|error| format!("{error:?}"))?,
        ),
    )?;
    let baseline = view.scene().sprites().len();
    assert!(view.tick(&link).is_ok());
    assert!(
        view.spell_words.is_empty(),
        "catalog alone does not emit a cast"
    );
    outbox.push(vec![SessionEvent::Presentation(AppliedDelta {
        server_sequence: 1,
        base_revision: 0,
        new_revision: 1,
        value: vec![PresentationEvent {
            decision_ordinal: 1,
            emission_ordinal: 1,
            kind: PresentationEventKind::SpellWords {
                caster: oteryn_session::EntityRef {
                    identity: [1; 16],
                    generation: 1,
                },
                position: ActorPosition {
                    x: 0,
                    y: 0,
                    floor: 0,
                },
                spell: 19,
            },
        }],
    })]);
    assert!(view.tick(&link).is_ok());
    assert_eq!(view.spell_words.get(&19).map(String::as_str), Some("exura"));
    assert!(view.scene().sprites().len() > baseline);
    outbox.spell_catalog_initialize(false, None)?;
    assert!(view.tick(&link).is_ok());
    assert!(
        view.spell_words.is_empty(),
        "never guess words when server catalog is unavailable"
    );
    assert_eq!(view.scene().sprites().len(), baseline);
    Ok(())
}

#[test]
fn missiles_apply_source_opacity_to_sprite_and_fallback() -> Result<(), Box<dyn std::error::Error>> {
    for sprites in [false, true] {
        let mut view = PlayView::new(TileCoord::new(0, 0), 0, &[])?;
        if sprites {
            view.scene.prime_visual_assets(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."));
        }
        let baseline = view.scene().sprites().len();
        let at = ActorPosition { x: 0, y: 0, floor: 0 };
        let now = std::time::Instant::now();
        for source in [PresentationSource::Own, PresentationSource::Others, PresentationSource::Creatures] {
            view.visuals.clear();
            view.receive_presentation(&[PresentationEventKind::Missile {
                from: at, to: ActorPosition { x: 1, ..at }, missile_id: 1, source,
            }], now);
            let mut preferences = view.preferences().clone();
            preferences.effect_opacity = [25; 3];
            view.set_preferences(preferences.clone())?;
            view.project_presentation(now)?;
            let batch = view.scene().sprites();
            assert!(batch.len() > baseline);
            assert!(batch.lighting.opacity.iter().any(|(range, alpha)| range.contains(&baseline) && *alpha == 25));
            preferences.effect_opacity = [0; 3];
            view.set_preferences(preferences)?;
            view.project_presentation(now)?;
            assert_eq!(view.scene().sprites().len(), baseline);
        }
    }
    Ok(())
}

#[test]
fn shallow_missile_uses_cardinal_artwork_through_presentation_events()
-> Result<(), Box<dyn std::error::Error>> {
    let mut view = PlayView::new(TileCoord::new(0, 0), 0, &[])?;
    view.scene.prime_visual_assets(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."));
    let (link, _, outbox) = play_channel();
    let mut uvs = Vec::new();
    for (i, (x, y)) in [(8, 0), (8, 1), (0, 8), (1, 8), (8, 8)].into_iter().enumerate() {
        view.visuals.clear();
        outbox.push(vec![SessionEvent::Presentation(AppliedDelta {
            server_sequence: i as u64 + 1, base_revision: i as u64, new_revision: i as u64 + 1,
            value: vec![PresentationEvent { decision_ordinal: i as u32 + 1, emission_ordinal: 1,
                kind: PresentationEventKind::Missile { from: ActorPosition {x: 0,y: 0,floor: 0},
                    to: ActorPosition {x,y,floor: 0}, missile_id: 1, source: PresentationSource::Own } }],
        })]);
        view.tick(&link).map_err(|e| format!("{e:?}"))?;
        let now = view.visuals.first().ok_or("cue")?.received;
        view.project_presentation(now)?;
        uvs.push(view.scene().sprites().instances().last().ok_or("missile")?.uv);
    }
    assert_eq!(uvs[0], uvs[1]);
    assert_eq!(uvs[2], uvs[3]);
    assert_ne!(uvs[0], uvs[4]);
    Ok(())
}

#[test]
fn luminous_server_effects_and_missiles_feed_the_live_light_pass()
-> Result<(), Box<dyn std::error::Error>> {
    let mut view = PlayView::new(TileCoord::new(0,0), 0, &[])?;
    view.scene.prime_visual_assets(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."));
    let (link, _, outbox) = play_channel();
    let at = ActorPosition { x: 0, y: 0, floor: 0 };
    outbox.push(vec![SessionEvent::Presentation(AppliedDelta {
        server_sequence: 1, base_revision: 0, new_revision: 1,
        value: [PresentationEventKind::MagicEffect { position: at, effect_id: 5, source: PresentationSource::Own },
            PresentationEventKind::Missile { from: at, to: ActorPosition { x: 8, ..at }, missile_id: 4, source: PresentationSource::Own }]
            .into_iter().enumerate().map(|(i,kind)| PresentationEvent { decision_ordinal: 1, emission_ordinal: i as u32 + 1, kind }).collect(),
    })]);
    view.tick(&link).map_err(|e| format!("{e:?}"))?;
    let now = view.visuals.first().ok_or("cue")?.received;
    view.project_presentation(now)?;
    assert_eq!(view.scene().sprites().lighting.sources.len(), 2);
    let before = view.scene().sprites().lighting.sources[1].position;
    view.project_presentation(now + Duration::from_millis(100))?;
    assert_ne!(view.scene().sprites().lighting.sources[1].position, before);
    view.project_presentation(now + Duration::from_secs(3))?;
    assert!(view.scene().sprites().lighting.sources.is_empty());
    Ok(())
}

#[test]
fn received_lower_floor_effect_is_projected_before_foreground_terrain()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_session::world_map::*;
    let mut view = PlayView::new(TileCoord::new(100,200), -6, &[])?;
    let own = ActorPosition { x: 100, y: 200, floor: -6 };
    let map = WorldMapViewSnapshot { header: MapViewHeader { origin: own, content_generation: [1;32], bundle_digest: [2;32], reset_epoch: 1, first_visible_floor: -6 },
        tiles: [-7,-6].into_iter().map(|floor| MapTile { position: ActorPosition { floor, ..own }, items: vec![], ground_speed: 150, more: false }).collect() };
    view.scene.prime_visual_assets(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."));
    view.set_live_map(&map)?;
    let (link, _, outbox) = play_channel();
    outbox.push(vec![SessionEvent::Presentation(AppliedDelta { server_sequence: 1, base_revision: 0, new_revision: 1,
        value: vec![PresentationEvent { decision_ordinal: 1, emission_ordinal: 1,
            kind: PresentationEventKind::MagicEffect { position: ActorPosition { floor: 7, ..own }, effect_id: 1, source: PresentationSource::Global } }] })]);
    view.tick(&link).map_err(|e| format!("{e:?}"))?;
    let now = view.visuals.first().ok_or("cue")?.received;
    view.project_presentation(now)?;
    let baseline = view.scene().sprites().len();
    #[cfg(bevy_shell)]
    {
        let ranges = view.scene().floor_quad_ranges();
        let lower = ranges.iter().find(|(_,floor)| *floor == -7).ok_or("lower floor")?;
        let foreground = ranges.iter().find(|(_,floor)| *floor == -6).ok_or("foreground")?;
        // Pinned effect 1 has one cell; the floor also contains one terrain cell.
        assert_eq!(lower.0.len(), 2);
        assert!(lower.0.end <= foreground.0.start);
    }
    view.project_presentation(now + Duration::from_secs(1))?;
    assert_eq!(view.scene().sprites().len() + 1, baseline);
    Ok(())
}

#[test]
fn received_lower_floor_actor_is_painted_in_its_floor_and_removed_with_observation()
-> Result<(),Box<dyn std::error::Error>> {
    use oteryn_session::world_map::*;
    let mut view = PlayView::new(TileCoord::new(100,200),-6,&[])?;
    let own = ActorPosition {x:100,y:200,floor:-6};
    let map = WorldMapViewSnapshot {header:MapViewHeader {origin:own,content_generation:[1;32],bundle_digest:[2;32],reset_epoch:1,first_visible_floor:-6},
        tiles:[-7,-6].into_iter().map(|floor|MapTile {position:ActorPosition {floor,..own},items:vec![],ground_speed:150,more:false}).collect()};
    view.set_live_map(&map)?;
    let before = view.scene().sprites().len();
    let (link,_,outbox) = play_channel();
    let mut actor = oteryn_session::VisibleAppearance {
        entity:oteryn_session::WorldSpatialEntity {motion: None, entity:oteryn_session::EntityRef {identity:[4;16],generation:1},kind:oteryn_session::EntityKind::Creature,
            position:ActorPosition {floor:-7,..own},display_name:None,
            detail:oteryn_session::EntityDetail::Actor {direction:oteryn_session::StepDirection::South,appearance_ref:1,health_percent:100}},
        look:None,light:None,forge_stacks:None,
    };
    outbox.appearances(vec![actor.clone()]);
    view.tick(&link).map_err(|e|format!("{e:?}"))?;
    assert_eq!(view.scene().sprites().len(),before+1);
    #[cfg(bevy_shell)]
    assert_eq!(view.scene().floor_quad_ranges().iter().find(|(_,floor)| *floor == -7).ok_or("lower floor")?.0.len(),2);
    actor.entity.position.floor = -5; // No terrain observation for this floor: do not fabricate it.
    outbox.appearances(vec![actor]);
    view.tick(&link).map_err(|e|format!("{e:?}"))?;
    assert_eq!(view.scene().sprites().len(),before);
    outbox.appearances(Vec::new());
    view.tick(&link).map_err(|e|format!("{e:?}"))?;
    assert_eq!(view.scene().sprites().len(),before);
    Ok(())
}
