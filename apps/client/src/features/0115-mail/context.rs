//! Popup lifetime and HWND access stay on the shell thread.
#![allow(
    unsafe_code,
    reason = "Win32 popup is created and destroyed on the UI thread"
)]
use crate::play::{PlayLink, PlayView};
use oteryn_client_runtime::world_interaction::Target;
use oteryn_client_ui::mail::context::{self, Action as Property, Entry};
use oteryn_client_ui::world_interaction::menu::{ContextAction, Menu};
use std::ptr::null;
use windows_sys::Win32::{Foundation::POINT, UI::WindowsAndMessaging::*};
impl PlayView {
    pub fn properties_context_menu(&self, link: &PlayLink, tile: oteryn_renderer::TileCoord) {
        let view = self;
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
        open(view, link, menu.as_ref(), npc, player, tile);
    }
}

fn open(
    view: &PlayView,
    link: &PlayLink,
    menu: Option<&Menu>,
    npc: Option<oteryn_session::EntityRef>,
    player: Option<oteryn_session::EntityRef>,
    tile: oteryn_renderer::TileCoord,
) {
    let target = view.world_context_target(tile);
    let handle = match target {
        Some(Target::Item(handle)) => Some(handle.get()),
        _ => None,
    };
    let entries = context::combined(
        menu,
        [
            npc.is_some(),
            player.is_some() && link.player_trade_panel().selected,
            player.is_some() && link.party_panel().selected,
        ],
        handle,
        [
            link.mail_panel().selected,
            link.training_panel().selected,
            link.house_podium_panel().selected,
            link.guild_bank_panel().selected,
        ],
    );
    if entries.is_empty() {
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
            if let Ok(index) = usize::try_from(selected)
                && let Some((_, action)) = index.checked_sub(1).and_then(|index| entries.get(index))
            {
                if let Entry::Property(action) = *action {
                    if view.world_context_target(tile) == target {
                        match action {
                            Property::Letter(handle) => {
                                link.mail_edit(0, handle.to_string());
                                link.mail_open();
                            }
                            Property::Mailbox(handle) => {
                                link.mail_edit(1, handle.to_string());
                                link.mail_open();
                            }
                            Property::Offline(handle) => {
                                link.training_edit(0, handle.to_string());
                                link.training_open();
                            }
                            Property::Weapon(handle) => {
                                link.training_edit(1, handle.to_string());
                                link.training_open();
                            }
                            Property::Dummy(handle) => {
                                link.training_edit(2, handle.to_string());
                                link.training_open();
                            }
                            Property::Podium => link.house_podium_open(),
                            Property::GuildBank => link.guild_bank_open(),
                        }
                    }
                    DestroyMenu(popup);
                    return;
                }
                let Entry::Existing(action) = *action else {
                    DestroyMenu(popup);
                    return;
                };
                match action {
                    ContextAction::World(action) => {
                        if let Some(menu) = menu {
                            view.world_context_choose(link, menu, action);
                        }
                    }
                    ContextAction::Talk | ContextAction::NpcTrade => {
                        if let Some(npc) = npc {
                            link.talk_to_npc(
                                npc,
                                if action == ContextAction::Talk {
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
