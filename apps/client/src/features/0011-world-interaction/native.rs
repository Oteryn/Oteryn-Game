//! Popup lifetime and HWND access stay on the shell thread.
#![allow(
    unsafe_code,
    reason = "Win32 popup is created and destroyed on the UI thread"
)]
use crate::play::{PlayLink, PlayView};
use oteryn_client_ui::world_interaction::menu::{ContextAction, Menu, context_entries};
use std::ptr::null;
use windows_sys::Win32::{Foundation::POINT, UI::WindowsAndMessaging::*};
pub fn open_menu(view: &PlayView, link: &PlayLink, menu: &Menu) {
    open(view, link, Some(menu), None, None);
}

pub fn open_context(view: &PlayView, link: &PlayLink, tile: oteryn_renderer::TileCoord) {
    let menu = view.world_context(tile, link);
    let npc = link
        .npc_dialogue_panel()
        .selected
        .then(|| view.npc_at(tile))
        .flatten();
    let player = view.party_target(tile).filter(|player| {
        matches!(view.world_context_target(tile),
            Some(oteryn_client_runtime::world_interaction::Target::Creature(current))
                if current == *player)
    });
    open(view, link, menu.as_ref(), npc, player);
}

fn open(
    view: &PlayView,
    link: &PlayLink,
    menu: Option<&Menu>,
    npc: Option<oteryn_session::EntityRef>,
    player: Option<oteryn_session::EntityRef>,
) {
    let entries = context_entries(
        menu,
        npc.is_some(),
        player.is_some() && link.player_trade_panel().selected,
        player.is_some() && link.party_panel().selected,
    );
    if entries.is_empty() && !crate::play::feature_0098_quick_loot::view::available(link, menu, player) {
        return;
    }
    // SAFETY: called from the native shell input phase; all handles remain on this thread.
    unsafe {
        let popup = CreatePopupMenu();
        if popup.is_null() {
            return;
        }
        for (index, (label, _)) in entries.iter().enumerate() {
            let wide: Vec<u16> = label.encode_utf16().chain(Some(0)).collect();
            AppendMenuW(popup, MF_STRING, index + 1, wide.as_ptr());
        }
        crate::play::feature_0098_quick_loot::view::decorate(
            link, view.preferences().gameplay.inspect_context_menu, popup, menu, player,
        );
        let mut point = POINT { x: 0, y: 0 };
        let owner = GetForegroundWindow();
        if !owner.is_null() && GetCursorPos(&mut point) != 0 {
            let selected = TrackPopupMenu(
                popup,
                TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON,
                point.x,
                point.y,
                0,
                owner,
                null(),
            );
            crate::play::feature_0098_quick_loot::view::choose(link, menu, player, selected);
            if let Ok(index) = usize::try_from(selected)
                && let Some((_, action)) = index.checked_sub(1).and_then(|index| entries.get(index))
            {
                match *action {
                    ContextAction::World(action) => {
                        if let Some(menu) = menu {
                            view.world_context_choose(link, menu, action);
                        }
                    }
                    ContextAction::Talk | ContextAction::NpcTrade => {
                        if let Some(npc) = npc {
                            link.talk_to_npc(
                                npc,
                                if *action == ContextAction::Talk {
                                    "hi"
                                } else {
                                    "trade"
                                },
                            );
                        }
                    }
                    ContextAction::PlayerTrade => {
                        if let Some(player) = player
                            && link.player_trade_choose_partner(player)
                        {
                            crate::play::feature_0060_player_trade::view::open_window(link);
                        }
                    }
                    ContextAction::Invite => {
                        if let Some(player) = player {
                            link.party_request(oteryn_client_ui::party::Intent::Invite(player));
                            link.open_party();
                        }
                    }
                    ContextAction::Party => link.open_party(),
                }
            }
        }
        DestroyMenu(popup);
    }
}
