//! Window-thread selector loop.
use super::*;
pub(super) fn run(
    weak: std::sync::Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>,
    commands: tokio::sync::mpsc::UnboundedSender<crate::combat_input::PlayCommand>,
) {
    // SAFETY: all HWNDs and action storage belong exclusively to this thread.
    unsafe {
        let h = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide("Charms").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            820,
            690,
            null_mut(),
            null_mut(),
            null_mut(),
            null(),
        );
        if !h.is_null() {
            let mut actions = Box::<Actions>::default();
            SetWindowLongPtrW(h, GWLP_USERDATA, (&mut *actions as *mut Actions) as isize);
            SetWindowLongPtrW(h, GWLP_WNDPROC, procedure as *const () as isize);
            let balance = control(
                h,
                "STATIC",
                "Loading confirmed balances",
                90,
                [10, 10, 780, 55],
                0,
            );
            control(h, "STATIC", "Charm", 91, [10, 75, 120, 25], 0);
            let charms = control(
                h,
                "COMBOBOX",
                "",
                92,
                [10, 105, 780, 300],
                CBS_DROPDOWNLIST as u32 | WS_VSCROLL | WS_TABSTOP,
            );
            control(h, "STATIC", "Eligible creature", 93, [10, 145, 300, 25], 0);
            let races = control(
                h,
                "COMBOBOX",
                "",
                94,
                [10, 175, 780, 350],
                CBS_DROPDOWNLIST as u32 | WS_VSCROLL | WS_TABSTOP,
            );
            let detail = control(
                h,
                "EDIT",
                "",
                95,
                [10, 220, 780, 280],
                WS_BORDER | WS_VSCROLL | ES_MULTILINE as u32 | ES_READONLY as u32,
            );
            let feedback = control(h, "STATIC", "", 96, [10, 570, 780, 65], 0);
            for (id, label, x) in [
                (1, "Refresh", 10),
                (2, "Unlock stage", 150),
                (3, "Assign", 290),
                (4, "Remove", 430),
                (5, "Reset all", 570),
            ] {
                control(h, "BUTTON", label, id, [x, 520, 130, 30], WS_TABSTOP);
            }
            let mut charm_rows = Vec::new();
            let mut race_rows = Vec::new();
            let mut initialized = false;
            loop {
                let mut msg: MSG = std::mem::zeroed();
                while PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) != 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
                if IsWindow(h) == 0 || commands.is_closed() {
                    break;
                }
                let selected_charm = SendMessageW(charms, CB_GETCURSEL, 0, 0);
                let selected_race = SendMessageW(races, CB_GETCURSEL, 0, 0);
                let mut requested = actions.0.drain(..).collect::<Vec<_>>();
                requested.retain(|id| !matches!(id,4|5) || MessageBoxW(h,wide(if *id==5 {"Remove all charm assignments? The server charges the applicable gold fees."} else {"Remove this charm assignment? The server charges the applicable gold fee."}).as_ptr(),wide("Confirm charm removal").as_ptr(),MB_YESNO|MB_ICONQUESTION)==IDYES);
                let Some(shared) = weak.upgrade() else {
                    break;
                };
                let p = {
                    let mut m = lock(&shared);
                    let p = &mut m.properties.charm_progression;
                    if !p.selected || !p.selector.open {
                        break;
                    }
                    let before = p.fields[0].clone();
                    if let Ok(i) = usize::try_from(selected_charm)
                        && let Some((id, _)) = charm_rows.get(i)
                    {
                        p.select_confirmed(*id, None);
                    }
                    if p.fields[0] == before
                        && let Ok(i) = usize::try_from(selected_race)
                        && let Some((race, _)) = race_rows.get(i)
                        && let Ok(charm) = p.fields[0].parse()
                    {
                        p.select_confirmed(charm, Some(*race));
                    }
                    let mut intent = requested.first().and_then(|id| match id {
                        1 => Some(Intent::Query),
                        2 => Some(Intent::Unlock),
                        3 => Some(Intent::Assign),
                        4 => Some(Intent::Remove),
                        5 => Some(Intent::ResetAll),
                        _ => None,
                    });
                    if !initialized && !p.pending {
                        intent = Some(Intent::Query);
                        initialized = true;
                    }
                    if intent == Some(Intent::Query) {
                        p.selector.attempted = false;
                        p.selector.next = None;
                        p.selector.race_failed = false;
                        p.selector.races.clear();
                    }
                    if let Some(intent) = intent {
                        super::super::super::input::selector::submit(&mut m, &commands, intent);
                    }
                    let p = &mut m.properties.charm_progression;
                    if p.reply.is_some() && !p.pending {
                        let after = if !p.selector.attempted {
                            Some(0)
                        } else {
                            p.selector.next
                        };
                        if let Some(after) = after {
                            super::super::super::input::selector::request(&mut m, &commands, after);
                        } else {
                            super::super::super::input::selector::request_races(&mut m, &commands);
                        }
                    }
                    m.properties.charm_progression.clone()
                };
                let choices = p.charm_choices();
                if choices != charm_rows {
                    populate(charms, &choices);
                    charm_rows = choices;
                }
                let choices = p.assignment_choices();
                if choices != race_rows {
                    populate(races, &choices);
                    race_rows = choices;
                }
                if let Some(i) = charm_rows
                    .iter()
                    .position(|(id, _)| id.to_string() == p.fields[0])
                {
                    SendMessageW(charms, CB_SETCURSEL, i, 0);
                } else if !charm_rows.is_empty() {
                    SendMessageW(charms, CB_SETCURSEL, 0, 0);
                }
                if let Some(i) = race_rows
                    .iter()
                    .position(|(id, _)| p.selector.selected_race == Some(*id))
                {
                    SendMessageW(races, CB_SETCURSEL, i, 0);
                } else {
                    SendMessageW(races, CB_SETCURSEL, usize::MAX, 0);
                }
                SetWindowTextW(balance, wide(&p.charm_balance_text()).as_ptr());
                SetWindowTextW(detail, wide(&p.selected_charm_text()).as_ptr());
                SetWindowTextW(
                    feedback,
                    wide(&format!(
                        "{}{}",
                        if p.pending { "Loading… " } else { "" },
                        p.feedback
                    ))
                    .as_ptr(),
                );
                std::thread::sleep(std::time::Duration::from_millis(30));
            }
            DestroyWindow(h);
        }
    }
    if let Some(shared) = weak.upgrade() {
        let mut m = lock(&shared);
        m.properties.charm_progression.selector.open = false;
        m.properties.charm_progression.open = false;
    }
}
