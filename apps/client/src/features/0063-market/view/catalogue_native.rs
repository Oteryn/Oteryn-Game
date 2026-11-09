//! Named catalogue with server-side filters; the parent offer form stays on this thread.
use super::table::Table;
use super::*;
use crate::combat_input::PlayCommand;
use oteryn_client_ui::market::details::{Filter, Query};
use tokio::sync::mpsc::UnboundedSender;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
type WeakMailbox = Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>;
unsafe extern "system" fn proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_COMMAND && (w >> 16 == BN_CLICKED as usize) {
            let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            if !actions.is_null() && (*actions).0.len() < 16 {
                (*actions).0.push(w & 0xffff);
            }
            return 0;
        }
        DefWindowProcW(hwnd, msg, w, l)
    }
}
fn vocation(value: u8) -> &'static str {
    match value {
        1 => "Druid",
        2 => "Knight",
        3 => "Monk",
        4 => "Paladin",
        5 => "Sorcerer",
        6 => "No vocation",
        _ => "Unknown",
    }
}
unsafe fn add_combo(hwnd: HWND, labels: impl IntoIterator<Item = String>) {
    unsafe {
        SendMessageW(hwnd, CB_RESETCONTENT, 0, 0);
        for label in labels {
            SendMessageW(hwnd, CB_ADDSTRING, 0, wide(&label).as_ptr() as isize);
        }
        SendMessageW(hwnd, CB_SETCURSEL, 0, 0);
    }
}
unsafe fn filters(fields: &[HWND]) -> Result<Filter, &'static str> {
    unsafe {
        let category = SendMessageW(fields[1], CB_GETCURSEL, 0, 0);
        let vocation = SendMessageW(fields[2], CB_GETCURSEL, 0, 0);
        let class = SendMessageW(fields[5], CB_GETCURSEL, 0, 0);
        let number = |h| -> Result<Option<u16>, &'static str> {
            let s = read(h);
            if s.trim().is_empty() {
                Ok(None)
            } else {
                s.trim()
                    .parse()
                    .map(Some)
                    .map_err(|_| "Enter whole-number level bounds")
            }
        };
        let result = Filter {
            search: read(fields[0]).trim().into(),
            category: (category > 0).then(|| read(fields[1])),
            vocation: (vocation > 0).then_some(vocation as u8),
            minimum_level: number(fields[3])?,
            maximum_level: number(fields[4])?,
            forge_class: (class > 0).then_some(class as u8),
            depot_only: SendMessageW(fields[6], BM_GETCHECK, 0, 0) != 0,
        };
        if !result.valid() {
            return Err("Invalid filter bounds");
        }
        Ok(result)
    }
}
pub(super) unsafe fn open(
    parent: HWND,
    item_field: HWND,
    weak: &WeakMailbox,
    commands: &UnboundedSender<PlayCommand>,
) {
    unsafe {
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide("Market catalogue").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            1080,
            650,
            parent,
            null_mut(),
            null_mut(),
            null(),
        );
        if hwnd.is_null() {
            return;
        }
        EnableWindow(parent, 0);
        let mut actions = Box::<Actions>::default();
        SetWindowLongPtrW(
            hwnd,
            GWLP_USERDATA,
            (&mut *actions as *mut Actions) as isize,
        );
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, proc as *const () as isize);
        let specs = [
            ("EDIT", "", 1, [15, 35, 220, 27], WS_BORDER),
            (
                "COMBOBOX",
                "",
                2,
                [250, 35, 200, 320],
                CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
            ),
            (
                "COMBOBOX",
                "",
                3,
                [465, 35, 150, 240],
                CBS_DROPDOWNLIST as u32,
            ),
            (
                "EDIT",
                "",
                4,
                [630, 35, 95, 27],
                WS_BORDER | ES_NUMBER as u32,
            ),
            (
                "EDIT",
                "",
                5,
                [740, 35, 95, 27],
                WS_BORDER | ES_NUMBER as u32,
            ),
            (
                "COMBOBOX",
                "",
                6,
                [850, 35, 170, 200],
                CBS_DROPDOWNLIST as u32,
            ),
            (
                "BUTTON",
                "In this depot",
                7,
                [15, 80, 170, 27],
                BS_AUTOCHECKBOX as u32,
            ),
        ];
        let fields: Vec<_> = specs
            .into_iter()
            .map(|(class, text, id, b, style)| {
                control(hwnd, class, text, id, b, style | WS_TABSTOP)
            })
            .collect();
        for (label, x, width) in [
            ("Search name", 15, 220),
            ("Category", 250, 200),
            ("Vocation", 465, 150),
            ("Minimum level", 630, 100),
            ("Maximum level", 740, 100),
            ("Forge class", 850, 170),
        ] {
            control(hwnd, "STATIC", label, 0, [x, 10, width, 22], 0);
        }
        for (label, id, x) in [
            ("Apply filters", 8, 200),
            ("Previous", 9, 370),
            ("Next", 10, 540),
            ("Browse offers", 11, 710),
        ] {
            control(hwnd, "BUTTON", label, id, [x, 80, 150, 28], WS_TABSTOP);
        }
        let Some(table) = Table::create(
            hwnd,
            12,
            [15, 130, 1020, 350],
            &[
                ("Item", 70),
                ("Name", 280),
                ("Category", 160),
                ("Level", 65),
                ("Vocations", 220),
                ("Class", 65),
                ("Depot units", 100),
            ],
        ) else {
            DestroyWindow(hwnd);
            EnableWindow(parent, 1);
            return;
        };
        let status = control(hwnd, "STATIC", "", 13, [15, 500, 1020, 80], 0);
        if fields.iter().any(|h| h.is_null()) || status.is_null() {
            DestroyWindow(hwnd);
            EnableWindow(parent, 1);
            return;
        }
        add_combo(fields[1], ["All categories".into()]);
        add_combo(
            fields[2],
            std::iter::once("Any vocation".into()).chain((1..=6).map(|v| vocation(v).into())),
        );
        add_combo(
            fields[5],
            std::iter::once("Any class".into()).chain((1..=4).map(|v| format!("Class {v}"))),
        );
        if let Some(shared) = weak.upgrade() {
            let mut m = lock(&shared);
            if m.market.details.pending.is_none() {
                crate::play::feature_0063_market::protocol::details::enqueue(
                    &mut m,
                    commands,
                    Query::Catalogue {
                        filter: Filter::default(),
                        page: 0,
                    },
                );
            }
        }
        let mut held = None;
        let mut categories = Vec::new();
        let mut message: MSG = std::mem::zeroed();
        let mut done = false;
        while !done && IsWindow(hwnd) != 0 && IsWindow(parent) != 0 {
            while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                if IsDialogMessageW(hwnd, &message) == 0 {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
            let Some(shared) = weak.upgrade() else {
                break;
            };
            let mut m = lock(&shared);
            if m.market_shutdown || !m.market.open {
                break;
            }
            for id in std::mem::take(&mut actions.0) {
                match id {
                    8..=10 => {
                        let (filter, page) = if id == 8 {
                            match filters(&fields) {
                                Ok(f) => (f, 0),
                                Err(e) => {
                                    m.market.details.feedback = Some(e);
                                    continue;
                                }
                            }
                        } else {
                            if id == 10 && !m.market.details.has_next {
                                continue;
                            }
                            (
                                m.market.details.filter.clone(),
                                if id == 9 {
                                    m.market.details.page.saturating_sub(1)
                                } else {
                                    m.market.details.page.saturating_add(1)
                                },
                            )
                        };
                        crate::play::feature_0063_market::protocol::details::enqueue(
                            &mut m,
                            commands,
                            Query::Catalogue { filter, page },
                        );
                    }
                    11 => {
                        if let Some(row) = table
                            .selection()
                            .and_then(|i| m.market.details.items.get(i))
                        {
                            let definition = row.definition;
                            m.market.definition = definition;
                            SetWindowTextW(item_field, wide(&definition.to_string()).as_ptr());
                            done =
                                crate::play::feature_0063_market::protocol::details::enqueue_book(
                                    &mut m,
                                    commands,
                                    Intent::Browse,
                                );
                        }
                    }
                    _ => {}
                }
            }
            if categories != m.market.details.categories {
                let old = read(fields[1]);
                categories.clone_from(&m.market.details.categories);
                add_combo(
                    fields[1],
                    std::iter::once("All categories".into()).chain(categories.iter().cloned()),
                );
                if let Some(i) = categories.iter().position(|c| *c == old) {
                    SendMessageW(fields[1], CB_SETCURSEL, i + 1, 0);
                }
            }
            if held.as_ref() != Some(&m.market.details) {
                let rows: Vec<_> = m
                    .market
                    .details
                    .items
                    .iter()
                    .map(|r| {
                        vec![
                            r.definition.to_string(),
                            r.name.clone(),
                            r.categories.join(", "),
                            r.level.map_or_else(|| "—".into(), |v| v.to_string()),
                            r.vocations.as_ref().map_or_else(
                                || "—".into(),
                                |vs| {
                                    if vs.is_empty() {
                                        "All".into()
                                    } else {
                                        vs.iter()
                                            .map(|v| vocation(*v))
                                            .collect::<Vec<_>>()
                                            .join(", ")
                                    }
                                },
                            ),
                            r.forge_class.map_or_else(|| "—".into(), |v| v.to_string()),
                            r.depot_units.to_string(),
                        ]
                    })
                    .collect();
                table.rows(&rows);
                SetWindowTextW(
                    status,
                    wide(&format!(
                        "Page {} · {} items · {}{}",
                        m.market.details.page + 1,
                        rows.len(),
                        if m.market.details.pending.is_some() {
                            "Waiting for server… "
                        } else {
                            ""
                        },
                        m.market
                            .details
                            .feedback
                            .unwrap_or("Select a named item to browse its buy/sell offers")
                    ))
                    .as_ptr(),
                );
                held = Some(m.market.details.clone());
            }
            drop(m);
            std::thread::sleep(std::time::Duration::from_millis(30));
        }
        if IsWindow(hwnd) != 0 {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            DestroyWindow(hwnd);
        }
        if let Some(shared) = weak.upgrade() {
            let mut m = lock(&shared);
            if matches!(m.market.details.pending, Some(Query::Catalogue { .. })) {
                m.market.details.pending = None;
            }
        }
        if IsWindow(parent) != 0 {
            EnableWindow(parent, 1);
            SetForegroundWindow(parent);
        }
    }
}
