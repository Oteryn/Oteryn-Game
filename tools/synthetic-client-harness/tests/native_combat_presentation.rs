//! C19: server presentation events use the production mailbox and visible scene batch.
use oteryn_client::play::{PlayView, play_channel};
use oteryn_renderer::TileCoord;
use oteryn_session::{
    ActorPosition, AppliedDelta, EntityRef, PresentationColor, PresentationEvent,
    PresentationEventKind, PresentationSource, PresentationValue, PresentationValueKind,
    SessionEvent,
};

#[test]
fn damage_heal_and_hit_effect_reach_the_play_scene() -> Result<(), Box<dyn std::error::Error>> {
    let (link, _commands, outbox) = play_channel();
    let mut view = PlayView::new(TileCoord::new(100, 200), 7, &[])?;
    let base = view.scene().sprites().len();
    let position = ActorPosition {
        x: 101,
        y: 200,
        floor: 7,
    };
    let target = EntityRef {
        identity: [1; 16],
        generation: 1,
    };
    let kinds = [
        PresentationEventKind::ValueText {
            kind: PresentationValueKind::DamageReceived,
            position,
            target,
            attacker: None,
            primary: PresentationValue {
                value: 125,
                color: PresentationColor::Red,
            },
            secondary: Some(PresentationValue {
                value: 20,
                color: PresentationColor::Blue,
            }),
        },
        PresentationEventKind::ValueText {
            kind: PresentationValueKind::Healed,
            position,
            target,
            attacker: None,
            primary: PresentationValue {
                value: 50,
                color: PresentationColor::LightGreen,
            },
            secondary: None,
        },
        PresentationEventKind::MagicEffect {
            position,
            effect_id: 1,
            source: PresentationSource::Own,
        },
        // The donor mapping renders DRAWBLOOD (1) red, FIREWORK_YELLOW (29) yellow.
        PresentationEventKind::MagicEffect {
            position,
            effect_id: 29,
            source: PresentationSource::Own,
        },
    ];
    outbox.push(vec![SessionEvent::Presentation(AppliedDelta {
        server_sequence: 42,
        base_revision: 0,
        new_revision: 1,
        value: kinds
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
    assert!(view.scene().sprites().len() > base + 4);
    let atlas = view.scene().atlas();
    let mut colors = std::collections::BTreeSet::new();
    let mut blood_edges = 0;
    let mut yellow_sparks = 0;
    for quad in &view.scene().sprites().instances()[base..] {
        let x = (quad.uv[0] * atlas.width() as f32) as usize;
        let y = (quad.uv[1] * atlas.height() as f32) as usize;
        let offset = (y * atlas.width() as usize + x) * 4;
        let color = &atlas.rgba()[offset..offset + 3];
        colors.insert(color);
        blood_edges +=
            usize::from(color == [255, 0, 0] && quad.size.iter().any(|size| *size > 3.0));
        yellow_sparks += usize::from(color == [255, 255, 0] && quad.size == [3.0, 3.0]);
    }
    assert_eq!(blood_edges, 4, "DRAWBLOOD keeps its red border");
    assert_eq!(yellow_sparks, 8, "yellow firework keeps all eight sparks");
    for expected in [
        &[255, 0, 0][..],
        &[0, 0, 255],
        &[102, 255, 102],
        &[255, 255, 0],
    ] {
        assert!(
            colors.contains(expected),
            "expected {expected:?}, got {colors:?}"
        );
    }
    Ok(())
}
