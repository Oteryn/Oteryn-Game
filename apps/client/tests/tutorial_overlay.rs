use oteryn_client::play::PlayView;
use oteryn_input_actions::{
    ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent, PointerCoordinate,
    PointerDelta, PointerMotion, PointerPosition,
};
use oteryn_renderer::TileCoord;
#[test]
fn tutorial_toggle_projects_context_marker_and_mouse_trail()
-> Result<(), Box<dyn std::error::Error>> {
    let mut view = PlayView::new(TileCoord::new(100, 100), 7, &[])?;
    let before = view.scene().sprites().len();
    let toggle = NormalizedInputEvent::Key {
        code: KeyCode::new(17)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(Modifier::Control)
            .with(Modifier::Alt)
            .with(Modifier::Shift),
        repeat: false,
    };
    assert!(view.tutorial_input(std::slice::from_ref(&toggle))?);
    let pointer = NormalizedInputEvent::PointerMoved {
        position: PointerPosition::new(PointerCoordinate::new(150)?, PointerCoordinate::new(150)?),
        motion: PointerMotion::new(PointerDelta::new(1)?, PointerDelta::new(1)?),
    };
    view.tutorial_input(&[pointer])?;
    assert!(view.scene().sprites().len() > before);
    assert!(view.tutorial_input(&[toggle])?);
    assert_eq!(view.scene().sprites().len(), before);
    Ok(())
}
