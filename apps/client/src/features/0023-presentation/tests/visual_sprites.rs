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
