use super::*;

#[test]
fn hud_arc_opacity_reaches_renderer_upload_without_dimming_other_quads() -> Result<(), Box<dyn std::error::Error>> {
    let mut play = view()?;
    play.vitals = Some(oteryn_session::ActorVitals { health: 50, max_health: 100, mana: 20, max_mana: 40, ..Default::default() });
    let mut preferences = play.preferences().clone();
    preferences.health_circle = true;
    preferences.arc_opacity = 25;
    play.set_preferences(preferences)?;
    let sprites = play.scene().sprites();
    let bytes = sprites.lighting.instance_bytes(sprites.instances());
    let stride = oteryn_renderer::lighting::LIT_INSTANCE_BYTES;
    for index in sprites.len()-32..sprites.len() {
        let alpha = f32::from_ne_bytes(bytes[(index+1)*stride-4..(index+1)*stride].try_into()?);
        assert_eq!(alpha, 0.25);
    }
    assert_eq!(f32::from_ne_bytes(bytes[stride-4..stride].try_into()?), 1.0);
    let mut invalid = play.preferences().clone();
    invalid.arc_opacity = 101;
    assert!(play.set_preferences(invalid).is_err());
    let mut disabled = play.preferences().clone();
    disabled.health_circle = false;
    play.set_preferences(disabled)?;
    assert!(play.scene().sprites().lighting.opacity.is_empty());
    Ok(())
}
