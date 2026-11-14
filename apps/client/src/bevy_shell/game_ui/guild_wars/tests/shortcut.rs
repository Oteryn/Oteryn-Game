use super::*;
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};

pub(super) fn owner_chord() -> Result<NormalizedInputEvent, Error> {
    Ok(NormalizedInputEvent::Key {
        code: KeyCode::new(26)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(Modifier::Control).with(Modifier::Alt),
        repeat: false,
    })
}

#[test]
fn guild_wars_default_chord_routes_through_controls() -> Result<(), Error> {
    let (mut app, mut commands, _) = fixture()?;
    click(&mut app, Kind::Close)?;
    app.update();
    let chord = owner_chord()?;
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(std::slice::from_ref(&chord), true);
    assert!(!panel(&app)?.open);
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(std::slice::from_ref(&chord), false);
    assert!(panel(&app)?.open);
    assert!(commands.try_recv().is_err());
    Ok(())
}
