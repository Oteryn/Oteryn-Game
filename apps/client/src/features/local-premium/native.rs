use super::*;
use crate::account_services::window::Window;

pub(super) fn present(view: &View, cancel: CancellationToken) -> Action {
    form(view, cancel).unwrap_or(Action::Back)
}
fn form(view: &View, cancel: CancellationToken) -> Option<Action> {
    let window = Window::new(tr!("Oteryn — Premium overview"), 800, 620, cancel)?;
    window.button(tr!("Refresh Premium status"), 1, [15, 15, 290, 32])?;
    window.button(tr!("Back"), 99, [610, 15, 160, 32])?;
    let output = window.output(&view.status_text(), [15, 65, 755, 490])?;
    let mut previous = String::new();
    loop {
        match window.event() {
            1 => return Some(Action::Refresh),
            99 => return Some(Action::Back),
            _ => {}
        }
        let text = view.status_text();
        if text != previous {
            output.set(&text);
            previous = text;
        }
    }
}
