use super::*;

pub(super) unsafe fn window(
    hwnd: HWND,
    weak: &Weak<Mutex<Mailbox>>,
    commands: &tokio::sync::mpsc::UnboundedSender<crate::combat_input::PlayCommand>,
    panel: u32,
) {
    unsafe {
        let mut actions = Box::<Actions>::default();
        SetWindowLongPtrW(
            hwnd,
            GWLP_USERDATA,
            (&mut *actions as *mut Actions) as isize,
        );
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, proc as *const () as isize);
        let text = control(
            hwnd,
            "EDIT",
            "",
            100,
            (10, 10, 810, 230),
            WS_BORDER | WS_VSCROLL | ES_MULTILINE as u32 | ES_READONLY as u32,
        );
        let rows = control(
            hwnd,
            "LISTBOX",
            "",
            101,
            (10, 250, 810, 100),
            WS_BORDER | WS_VSCROLL | WS_TABSTOP,
        );
        let (buttons, fields) = specs(panel);
        let mut inputs = Vec::new();
        for (i, (label, initial)) in fields.iter().enumerate() {
            let x = 10 + (i % 3) as i32 * 270;
            let y = 360 + (i / 3) as i32 * 60;
            control(hwnd, "STATIC", label, 200 + i, (x, y, 260, 25), 0);
            let input = control(
                hwnd,
                "EDIT",
                initial,
                300 + i,
                (x, y + 25, 250, 25),
                WS_BORDER | WS_TABSTOP,
            );
            SendMessageW(input, EM_SETLIMITTEXT, 36, 0);
            inputs.push(input);
        }
        let mut controls = Vec::new();
        for (i, label) in buttons.iter().enumerate() {
            controls.push(control(
                hwnd,
                "BUTTON",
                label,
                i + 1,
                (
                    10 + (i % 3) as i32 * 270,
                    (if fields.len() > 12 {
                        680
                    } else if fields.len() > 9 {
                        620
                    } else {
                        550
                    }) + (i / 3) as i32 * 42,
                    260,
                    36,
                ),
                WS_TABSTOP,
            ));
        }
        if panel == 285 {
            control(
                hwnd,
                "STATIC",
                "Store tabs: Catalogue / Purchase history",
                400,
                (10, 360, 800, 30),
                0,
            );
        }
        let mut held = String::new();
        let mut held_rows = Vec::<String>::new();
        while IsWindow(hwnd) != 0 && !commands.is_closed() {
            let mut message: MSG = std::mem::zeroed();
            while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            if IsWindow(hwnd) == 0 {
                break;
            }
            let Some(shared) = weak.upgrade() else {
                break;
            };
            let ended = {
                let mailbox = lock(&shared);
                mailbox.quest_shutdown
            };
            if ended {
                break;
            }
            // This temporary facade borrows the live session; running PlayLink::drop
            // here would shut the session down after the first panel repaint.
            let mut link = std::mem::ManuallyDrop::new(PlayLink {
                mailbox: shared,
                commands: commands.clone(),
            });
            for action in std::mem::take(&mut actions.0) {
                let values: Vec<String> = inputs
                    .iter()
                    .map(|input| {
                        let mut value = [0u16; 37];
                        let len = GetWindowTextW(*input, value.as_mut_ptr(), 37);
                        String::from_utf16_lossy(&value[..len.max(0) as usize])
                    })
                    .collect();
                let row = usize::try_from(SendMessageW(rows, LB_GETCURSEL, 0, 0))
                    .unwrap_or(if panel == 98 { usize::MAX } else { 0 });
                if panel == 285
                    && (action == 3
                        || ((action == 1 || action == 2)
                            && link.store_panel().page
                                == oteryn_client_ui::store::StorePage::Catalogue))
                {
                    let command = {
                        let mut mailbox = lock(&link.mailbox);
                        if mailbox.store.category.is_empty() {
                            mailbox.store.category = "featured".into();
                        }
                        if action != 2 {
                            mailbox.store.catalogue = None;
                        }
                        oteryn_client_ui::store::catalogue(&mut mailbox.store)
                    };
                    if let Some(command) = command {
                        link.discovery_send(command);
                    }
                } else if panel == 285 && action == 5 {
                    if let [offer, from, until] = values.as_slice() {
                        link.purchase_history_filter(offer, from, until);
                    }
                } else if panel == 285 && action == 4 {
                    lock(&link.mailbox).store.page = oteryn_client_ui::store::StorePage::History;
                    link.purchase_history(false);
                } else {
                    link.discovery_action(panel, action, &values, row);
                }
            }
            let state = link.discovery_panel();
            let store = link.store_panel();
            let (display, names, available) = match panel {
                98 => (
                    oteryn_client_ui::quick_loot::panel_text(&state.loot),
                    if state.loot.managed {
                        state
                            .loot
                            .destinations
                            .iter()
                            .map(|action| format!("{action:?}"))
                            .collect()
                    } else {
                        state
                            .loot
                            .skipped
                            .iter()
                            .map(|id| format!("Skip item {id}"))
                            .chain(
                                state
                                    .loot
                                    .accepted
                                    .iter()
                                    .map(|id| format!("Accept item {id}")),
                            )
                            .collect()
                    },
                    state.loot.selected && state.loot.pending.is_none(),
                ),
                132 => (
                    oteryn_client_ui::inspection::panel_text(&state.inspection),
                    state
                        .inspection
                        .reply
                        .as_ref()
                        .map(|r| {
                            r.requests
                                .iter()
                                .enumerate()
                                .map(|(i, _)| format!("Permission request {}", i + 1))
                                .collect()
                        })
                        .unwrap_or_default(),
                    state.inspection.selected && !state.inspection.pending,
                ),
                232 => {
                    let names = state.teams.visible_names();
                    (
                        format!(
                            "{}\r\n{}",
                            oteryn_client_ui::team_finder::panel_text(&state.teams),
                            oteryn_client_ui::team_finder_leader::view::panel_text(&state.leader)
                        ),
                        names,
                        state.teams.selected && state.teams.pending.is_none(),
                    )
                }
                _ => (
                    if store.page == oteryn_client_ui::store::StorePage::Catalogue {
                        oteryn_client_ui::store::panel_text(&store)
                    } else {
                        oteryn_client_ui::purchase_history::panel_text(&state.history)
                    },
                    Vec::new(),
                    state.history.selected && !state.history.pending,
                ),
            };
            if display != held {
                SetWindowTextW(text, wide(&display).as_ptr());
                held = display;
            }
            if names != held_rows {
                SendMessageW(rows, LB_RESETCONTENT, 0, 0);
                for name in &names {
                    SendMessageW(rows, LB_ADDSTRING, 0, wide(name).as_ptr() as isize);
                }
                if !names.is_empty() {
                    SendMessageW(rows, LB_SETCURSEL, 0, 0);
                }
                held_rows = names;
            }
            for (i, button) in controls.iter().enumerate() {
                let enable = if panel == 98 && (1..5).contains(&i) {
                    available && !state.loot.managed
                } else if panel == 98 && i == 7 {
                    available && state.loot.managed
                } else if panel == 232 && (8..10).contains(&i) {
                    state.leader.selected && !state.leader.pending
                } else if panel == 285 && i >= 2 {
                    store.selected || state.history.selected
                } else if panel == 285
                    && i == 0
                    && store.page == oteryn_client_ui::store::StorePage::Catalogue
                {
                    store.selected && !store.pending
                } else if panel == 285 && i == 1 {
                    match store.page {
                        oteryn_client_ui::store::StorePage::Catalogue => {
                            store.selected
                                && !store.pending
                                && store
                                    .catalogue
                                    .as_ref()
                                    .is_some_and(|p| !p.next_page_cursor.is_empty())
                        }
                        _ => {
                            available
                                && state
                                    .history
                                    .page
                                    .as_ref()
                                    .is_some_and(|p| !p.next_page_cursor.is_empty())
                        }
                    }
                } else {
                    available
                };
                EnableWindow(*button, i32::from(enable));
            }
            // Release both cloned handles without invoking the owning facade's destructor.
            std::ptr::drop_in_place(&mut link.mailbox);
            std::ptr::drop_in_place(&mut link.commands);
            MsgWaitForMultipleObjects(0, null(), 0, 50, QS_ALLINPUT);
        }
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
        if IsWindow(hwnd) != 0 {
            DestroyWindow(hwnd);
        }
    }
}
