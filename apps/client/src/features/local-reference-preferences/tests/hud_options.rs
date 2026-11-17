use super::*;

fn hud_actor(
    id: u8,
    kind: oteryn_session::EntityKind,
    x: i32,
) -> oteryn_session::WorldSpatialEntity {
    oteryn_session::WorldSpatialEntity {
        entity: EntityRef {
            identity: [id; 16],
            generation: 1,
        },
        kind,
        display_name: None,
        position: ActorPosition {
            x,
            y: 200,
            floor: 0,
        },
        detail: oteryn_session::EntityDetail::Actor {
            direction: StepDirection::South,
            appearance_ref: 1,
            health_percent: 50,
        },
    }
}

#[test]
fn own_health_and_mana_visibility_are_independent() -> Result<(), Box<dyn std::error::Error>> {
    let mut play = view()?;
    play.vitals = Some(oteryn_session::ActorVitals {
        health: 50,
        max_health: 100,
        mana: 20,
        max_mana: 40,
        ..Default::default()
    });
    play.rebuild()?;
    let both = play.scene().sprites().len();
    let mut preferences = play.preferences().clone();
    preferences.hud_health = false;
    play.set_preferences(preferences)?;
    assert_eq!(play.scene().sprites().len(), both - 2);
    let mut preferences = play.preferences().clone();
    preferences.hud_mana = false;
    play.set_preferences(preferences)?;
    assert_eq!(play.scene().sprites().len(), both - 4);
    Ok(())
}

#[test]
fn hud_marks_and_npc_badges_are_independent_and_generation_fenced()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::party_skull::PartyIcon;
    use oteryn_session::{EntityKind, VisibleAppearance};
    let own = hud_actor(1, EntityKind::Player, 100);
    let other = hud_actor(2, EntityKind::Player, 102);
    let mut play = view()?.with_entities(vec![hud_actor(3, EntityKind::Npc, 101), other])?;
    play.set_own_identity([1; 16]);
    play.set_presentations(vec![VisibleAppearance {
        entity: own,
        look: None,
        light: None,
        forge_stacks: None,
    }])?;
    play.party_icons = vec![
        PartyIcon {
            entity: own.entity,
            shield: 3,
            skull: 4,
        },
        PartyIcon {
            entity: other.entity,
            shield: 3,
            skull: 4,
        },
    ];
    let mut preferences = play.preferences().clone();
    preferences.hud_marks_own = false;
    preferences.hud_marks_other = false;
    preferences.hud_npc_icons = false;
    preferences.creature_bars = false;
    play.set_preferences(preferences.clone())?;
    let baseline = play.scene().sprites().len();
    preferences.hud_npc_icons = true;
    play.set_preferences(preferences.clone())?;
    let npc = play.scene().sprites().len();
    assert!(npc > baseline);
    preferences.hud_marks_other = true;
    play.set_preferences(preferences.clone())?;
    let marked = play.scene().sprites().len();
    assert!(marked > npc);
    preferences.hud_marks_own = true;
    play.set_preferences(preferences)?;
    assert!(play.scene().sprites().len() > marked);
    play.party_icons[1].entity.generation += 1;
    play.rebuild()?;
    assert_eq!(play.scene().sprites().len(), marked);
    play.party_icons[0].entity.generation += 1;
    play.rebuild()?;
    assert_eq!(play.scene().sprites().len(), npc);
    Ok(())
}

#[test]
fn hud_arc_opacity_reaches_renderer_upload_without_dimming_other_quads()
-> Result<(), Box<dyn std::error::Error>> {
    let mut play = view()?;
    play.vitals = Some(oteryn_session::ActorVitals {
        health: 50,
        max_health: 100,
        mana: 20,
        max_mana: 40,
        ..Default::default()
    });
    let mut preferences = play.preferences().clone();
    preferences.health_circle = true;
    preferences.arc_opacity = 25;
    play.set_preferences(preferences)?;
    let sprites = play.scene().sprites();
    let bytes = sprites.lighting.instance_bytes(sprites.instances());
    let stride = oteryn_renderer::lighting::LIT_INSTANCE_BYTES;
    for index in sprites.len() - 32..sprites.len() {
        let alpha =
            f32::from_ne_bytes(bytes[(index + 1) * stride - 4..(index + 1) * stride].try_into()?);
        assert_eq!(alpha, 0.25);
    }
    assert_eq!(
        f32::from_ne_bytes(bytes[stride - 4..stride].try_into()?),
        1.0
    );
    let mut invalid = play.preferences().clone();
    invalid.arc_opacity = 101;
    assert!(play.set_preferences(invalid).is_err());
    let mut disabled = play.preferences().clone();
    disabled.health_circle = false;
    play.set_preferences(disabled)?;
    assert!(play.scene().sprites().lighting.opacity.is_empty());
    Ok(())
}
