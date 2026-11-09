//! Additional entries in the existing combined context menu; no second popup.
#![allow(
    unsafe_code,
    reason = "Called on the shell thread with its live popup HMENU"
)]
use crate::play::PlayLink;
use oteryn_client_ui::world_interaction::menu::{Menu, Target};
use windows_sys::Win32::UI::WindowsAndMessaging::{AppendMenuW, HMENU, MF_STRING};
fn target(
    menu: Option<&Menu>,
    player: Option<oteryn_session::EntityRef>,
) -> Option<oteryn_client_ui::inspection::Request> {
    use oteryn_client_ui::inspection::Request;
    player.map(Request::Player).or_else(|| match &menu?.target {
        Target::Item(item) => Some(Request::Item(*item)),
        _ => None,
    })
}
pub(in crate::play) fn decorate(
    link: &PlayLink,
    inspect: bool,
    popup: HMENU,
    menu: Option<&Menu>,
    player: Option<oteryn_session::EntityRef>,
) {
    let state = link.discovery_panel();
    for (id, label, show) in [
        (
            60001,
            tr!("Inspect"),
            inspect && state.inspection.selected && target(menu, player).is_some(),
        ),
        (
            60002,
            tr!("Ask inspection permission"),
            inspect && state.inspection.selected && player.is_some(),
        ),
        (60003, tr!("Quick loot nearby"), state.loot.selected),
    ] {
        if show {
            let wide: Vec<u16> = label.encode_utf16().chain(Some(0)).collect();
            unsafe {
                AppendMenuW(popup, MF_STRING, id, wide.as_ptr());
            }
        }
    }
}
pub(in crate::play) fn choose(
    link: &PlayLink,
    menu: Option<&Menu>,
    player: Option<oteryn_session::EntityRef>,
    action: i32,
) {
    use oteryn_client_ui::{inspection::Request, quick_loot::Action};
    match action {
        60001 => {
            if let Some(request) = target(menu, player) {
                link.inspect(request);
                link.discovery_open(132);
            }
        }
        60002 => {
            if let Some(target) = player {
                link.inspect(Request::Permission {
                    action: 1,
                    target: Some(target),
                });
                link.discovery_open(132);
            }
        }
        60003 => {
            link.quick_loot_action(Action::LootAll);
            link.discovery_open(98);
        }
        _ => {}
    }
}

pub(in crate::play) fn available(
    link: &PlayLink,
    menu: Option<&Menu>,
    player: Option<oteryn_session::EntityRef>,
) -> bool {
    let state = link.discovery_panel();
    state.loot.selected || (state.inspection.selected && target(menu, player).is_some())
}
