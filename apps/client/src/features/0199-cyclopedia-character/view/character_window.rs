// HWND operations stay on the creating thread, following the daily reward panel.
#![allow(unsafe_code)]
use super::controls::{control, read_text, wide};
use super::*;
use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use std::ptr::{null, null_mut};
use std::sync::{Arc, Weak};
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::{
    Controls::{ICC_PROGRESS_CLASS, INITCOMMONCONTROLSEX, InitCommonControlsEx},
    Input::KeyboardAndMouse::EnableWindow,
    WindowsAndMessaging::*,
};
type WeakMailbox = Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>;

fn run(weak: WeakMailbox, commands: UnboundedSender<PlayCommand>) {
    unsafe {
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide(tr!("Character cyclopedia")).as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            740,
            740,
            null_mut(),
            null_mut(),
            null_mut(),
            null(),
        );
        if hwnd.is_null() {
            close(&weak);
            return;
        }
        let mut actions = Box::<Actions>::default();
        SetWindowLongPtrW(
            hwnd,
            GWLP_USERDATA,
            (&mut *actions as *mut Actions) as isize,
        );
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, proc as *const () as isize);
        let character_widgets::Widgets {
            badge_id,
            edits,
            output,
            heading,
            history,
            detail,
            achievement_browser,
            status_label,
            cards,
            stamina_bar,
            training_bar,
        } = character_widgets::create(hwnd);
        let mut tables = super::character_paint::Tables::default();
        let mut message: MSG = std::mem::zeroed();
        loop {
            while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            if IsWindow(hwnd) == 0 {
                break;
            }
            let Some(shared) = weak.upgrade() else { break };
            let mut mailbox = lock(&shared);
            if mailbox.cyclopedia_character_shutdown {
                break;
            }
            let mut analytics = false;
            let mut browser = None;
            for action in std::mem::take(&mut actions.0) {
                if action == 150 {
                    let badge = read_text(badge_id);
                    if let Some(command) =
                        submit(&mut mailbox.cyclopedia_character, 150, [&badge, "", ""])
                        && commands.send(PlayCommand::Feature(command)).is_err()
                    {
                        mailbox.cyclopedia_character.pending = false;
                        mailbox.cyclopedia_character.feedback = Some("The session has ended");
                    }
                    continue;
                }
                if let Some(selected) =
                    super::browsers::Browser::for_action(action, &mailbox.cyclopedia_character)
                {
                    browser = Some(selected);
                    continue;
                }
                if action == 117 {
                    analytics = true;
                    continue;
                }
                let values = edits.map(read_text);
                if let Some(command) = submit(
                    &mut mailbox.cyclopedia_character,
                    action,
                    values.each_ref().map(String::as_str),
                ) && commands.send(PlayCommand::Feature(command)).is_err()
                {
                    mailbox.cyclopedia_character.pending = false;
                    mailbox.cyclopedia_character.feedback = Some(tr!("The session has ended"));
                }
                SetWindowTextW(
                    edits[0],
                    wide(&mailbox.cyclopedia_character.offset.to_string()).as_ptr(),
                );
            }
            SetWindowTextW(
                output,
                wide(&display(&mailbox.cyclopedia_character)).as_ptr(),
            );
            let p = &mut mailbox.cyclopedia_character;
            if !p.pending
                && p.misc_resources_due
                && let Some(command) = request(p, Intent::MiscResources)
                && commands.send(PlayCommand::Feature(command)).is_err()
            {
                p.pending = false;
                p.feedback = Some(tr!("The session has ended"));
            }
            let panel = &mailbox.cyclopedia_character;
            EnableWindow(
                achievement_browser,
                i32::from(panel.selected && !panel.pending),
            );
            ShowWindow(
                achievement_browser,
                if panel.tab
                    == oteryn_client_runtime::cyclopedia_character::CharacterTab::Achievements
                {
                    SW_SHOW
                } else {
                    SW_HIDE
                },
            );
            let status = if panel.pending {
                tr!("Waiting for the server")
            } else {
                panel.feedback.unwrap_or("")
            };
            SetWindowTextW(status_label, wide(status).as_ptr());
            let badges = panel.command_kind == 387;
            let table = !badges && panel.history_tab == Some(panel.tab);
            let pvp = panel.tab
                == oteryn_client_runtime::cyclopedia_character::CharacterTab::RecentPvpKills;
            for id in 114..=116 {
                ShowWindow(GetDlgItem(hwnd, id), if pvp { SW_SHOW } else { SW_HIDE });
            }
            let overview = !badges
                && panel.general.is_some()
                && panel.tab
                    == oteryn_client_runtime::cyclopedia_character::CharacterTab::GeneralStats;
            let combat = !badges && panel.combat.as_ref().is_some_and(|s| s.tab() == panel.tab);
            let store = !badges
                && panel.store.is_some()
                && panel.tab
                    == oteryn_client_runtime::cyclopedia_character::CharacterTab::StoreSummary;
            let misc = !badges
                && panel.tab == oteryn_client_runtime::cyclopedia_character::CharacterTab::Misc;
            super::misc_layout::layout(&cards, [stamina_bar, training_bar], heading, detail, misc);
            character_widgets::render_cards(
                panel,
                &cards,
                [stamina_bar, training_bar],
                output,
                [table, overview, combat, store, misc],
            );
            for handle in [heading, detail] {
                ShowWindow(handle, if table || combat { SW_SHOW } else { SW_HIDE });
            }
            ShowWindow(
                history,
                if table || combat || store {
                    SW_SHOW
                } else {
                    SW_HIDE
                },
            );
            drop(mailbox);
            if let Some(browser) = browser {
                browser.open(weak.clone(), commands.clone());
            }
            if analytics {
                crate::play::feature_0010_analyser::view::open_shared(&shared, &commands);
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        DestroyWindow(hwnd);
        close(&weak);
        drop(actions);
    }
}

#[path = "character_widgets.rs"]
mod character_widgets;

#[path = "character_events.rs"]
mod character_events;
use character_events::{Actions, close, proc};

pub(in crate::play) use character_events::open_window;
