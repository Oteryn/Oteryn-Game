//! Native battle-list window loop. Handles stay on this thread.
use super::*;
pub(super) fn run(weak: Weak<Mutex<Mailbox>>, commands: UnboundedSender<PlayCommand>) {
    // SAFETY: all window operations, dispatch and destruction stay on this thread.
    unsafe {
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide("Battle — Alt+B").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            760,
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
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, window_proc as *const () as isize);
        let headings: Vec<_> = (0..4)
            .map(|i| control(hwnd, "STATIC", "", 0, (10 + i * 180, 10, 170, 25), 0))
            .collect();
        let lists: Vec<_> = (0..4)
            .map(|i| {
                control(
                    hwnd,
                    "LISTBOX",
                    "",
                    200 + i as usize,
                    (10 + i * 180, 40, 170, 260),
                    LBS_NOTIFY as u32 | WS_BORDER | WS_VSCROLL | WS_HSCROLL,
                )
            })
            .collect();
        let stop = control(hwnd, "BUTTON", "Stop attack", 101, (10, 320, 130, 30), 0);
        for &list in &lists {
            SendMessageW(list, LB_SETHORIZONTALEXTENT, 2048, 0);
        }
        let offensive = control(hwnd, "BUTTON", "Offensive", 102, (150, 320, 110, 30), 0);
        let balanced = control(hwnd, "BUTTON", "Balanced", 103, (270, 320, 110, 30), 0);
        let defensive = control(hwnd, "BUTTON", "Defensive", 104, (390, 320, 110, 30), 0);
        let stand = control(hwnd, "BUTTON", "Stand", 107, (510, 320, 100, 30), 0);
        let chase = control(hwnd, "BUTTON", "Chase", 108, (620, 320, 100, 30), 0);
        let text = control(hwnd, "STATIC", "", 109, (10, 360, 720, 60), 0);
        let editor_label = control(
            hwnd,
            "STATIC",
            "List / name / name filter / maximum HP %",
            0,
            (10, 425, 720, 25),
            0,
        );
        let active = control(
            hwnd,
            "COMBOBOX",
            "",
            300,
            (10, 455, 170, 150),
            CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
        );
        let title = control(hwnd, "EDIT", "", 301, (190, 455, 170, 26), WS_BORDER);
        let search = control(hwnd, "EDIT", "", 302, (370, 455, 170, 26), WS_BORDER);
        let maximum = control(
            hwnd,
            "EDIT",
            "100",
            306,
            (550, 455, 170, 26),
            WS_BORDER | ES_NUMBER as u32,
        );
        let players = control(
            hwnd,
            "BUTTON",
            "Players",
            303,
            (10, 495, 160, 26),
            BS_AUTOCHECKBOX as u32,
        );
        let creatures = control(
            hwnd,
            "BUTTON",
            "Creatures",
            304,
            (190, 495, 160, 26),
            BS_AUTOCHECKBOX as u32,
        );
        let sort = control(
            hwnd,
            "COMBOBOX",
            "",
            305,
            (370, 495, 350, 150),
            CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
        );
        for label in [
            "Name",
            "Health: low first",
            "Health: high first",
            "Identity",
        ] {
            SendMessageW(sort, CB_ADDSTRING, 0, wide(label).as_ptr() as isize);
        }
        let apply = control(hwnd, "BUTTON", "Save list", 307, (10, 540, 220, 30), 0);
        let add = control(hwnd, "BUTTON", "Add list", 308, (250, 540, 220, 30), 0);
        let remove = control(hwnd, "BUTTON", "Remove list", 309, (490, 540, 230, 30), 0);
        let controls: Vec<_> = lists
            .iter()
            .copied()
            .chain([stop, offensive, balanced, defensive, stand, chase])
            .collect();
        if controls
            .iter()
            .chain(headings.iter())
            .chain(
                [
                    text,
                    editor_label,
                    active,
                    title,
                    search,
                    maximum,
                    players,
                    creatures,
                    sort,
                    apply,
                    add,
                    remove,
                ]
                .iter(),
            )
            .any(|h| h.is_null())
        {
            DestroyWindow(hwnd);
            close(&weak);
            return;
        }
        let mut held: Option<BattlePanel> = None;
        let mut held_lists = None;
        let mut displayed: Vec<Vec<oteryn_session::WorldSpatialEntity>> = Vec::new();
        let mut message: MSG = std::mem::zeroed();
        loop {
            while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                if IsDialogMessageW(hwnd, &message) == 0 {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
            if IsWindow(hwnd) == 0 {
                break;
            }
            let Some(shared) = weak.upgrade() else {
                break;
            };
            let mut mailbox = lock(&shared);
            if mailbox.battle.shutdown || !mailbox.battle.open {
                break;
            }
            for id in std::mem::take(&mut actions.0) {
                if id == 300 || (307..=309).contains(&id) {
                    let state = &mut mailbox.battle_lists;
                    if id == 300 {
                        state.select(
                            (SendMessageW(active, CB_GETCURSEL, 0, 0).max(0) as usize)
                                .min(state.preferences.len() - 1),
                        );
                        held_lists = None;
                        continue;
                    }
                    let mut candidate = state.preferences.clone();
                    let mut selected = state.active.min(candidate.len() - 1);
                    match id {
                        307 => {
                            let Ok(max_health) = read_text(maximum).parse() else {
                                mailbox.battle.feedback =
                                    Some("Enter a maximum HP percentage from 0 to 100");
                                continue;
                            };
                            candidate[selected] = BattleListPreferences {
                                name: read_text(title).trim().into(),
                                search: read_text(search).trim().into(),
                                players: SendMessageW(players, BM_GETCHECK, 0, 0) == 1,
                                creatures: SendMessageW(creatures, BM_GETCHECK, 0, 0) == 1,
                                max_health,
                                sort: match SendMessageW(sort, CB_GETCURSEL, 0, 0) {
                                    1 => BattleSort::HealthAscending,
                                    2 => BattleSort::HealthDescending,
                                    3 => BattleSort::Identity,
                                    _ => BattleSort::Name,
                                },
                            };
                        }
                        308 if candidate.len() < 4 => {
                            let next = (1..=8).map(|i| format!("List {i}")).find(|name| {
                                !candidate.iter().any(|c| c.name.eq_ignore_ascii_case(name))
                            });
                            if let Some(name) = next {
                                candidate.push(BattleListPreferences {
                                    name,
                                    ..Default::default()
                                });
                                selected = candidate.len() - 1;
                            }
                        }
                        309 if candidate.len() > 1 => {
                            candidate.remove(selected);
                            selected = selected.min(candidate.len() - 1);
                        }
                        _ => continue,
                    }
                    if validate_lists(&candidate) {
                        state.preferences = candidate;
                        state.active = selected;
                        state.dirty = true;
                        state.edit = crate::play::feature_0017_attack::view::next_edit();
                        mailbox.battle.feedback = Some("List updated");
                    } else {
                        mailbox.battle.feedback = Some(
                            "Use unique list names (1–32 bytes), a filter up to 64 bytes and HP from 0 to 100",
                        );
                    }
                    continue;
                }
                let panel = &mut mailbox.battle;
                let action = match id {
                    200..=203 => {
                        let list_index = id - 200;
                        let index = SendMessageW(lists[list_index], LB_GETCURSEL, 0, 0);
                        let Some(row) = usize::try_from(index)
                            .ok()
                            .and_then(|i| displayed.get(list_index)?.get(i))
                        else {
                            continue;
                        };
                        Action::Attack(row.entity)
                    }
                    101 => Action::Stop,
                    102 => Action::Fight(FightMode::Offensive),
                    103 => Action::Fight(FightMode::Balanced),
                    104 => Action::Fight(FightMode::Defensive),
                    107 => Action::Chase(ChaseMode::Stand),
                    108 => Action::Chase(ChaseMode::Chase),
                    _ => continue,
                };
                if let Some(command) = battle::request(panel, action)
                    && commands.send(PlayCommand::Battle(command)).is_err()
                {
                    panel.pending = false;
                    panel.pending_command = None;
                }
            }
            let list_state = mailbox.battle_lists.clone();
            let panel = &mailbox.battle;
            if held.as_ref() != Some(panel) || held_lists.as_ref() != Some(&list_state) {
                displayed.clear();
                SendMessageW(active, CB_RESETCONTENT, 0, 0);
                for i in 0..4 {
                    let config = list_state.preferences.get(i);
                    ShowWindow(
                        headings[i],
                        if config.is_some() { SW_SHOW } else { SW_HIDE },
                    );
                    ShowWindow(lists[i], if config.is_some() { SW_SHOW } else { SW_HIDE });
                    let Some(config) = config else {
                        continue;
                    };
                    SetWindowTextW(headings[i], wide(&config.name).as_ptr());
                    SendMessageW(
                        active,
                        CB_ADDSTRING,
                        0,
                        wide(&config.name).as_ptr() as isize,
                    );
                    let rows = config.rows(&panel.rows);
                    SendMessageW(lists[i], LB_RESETCONTENT, 0, 0);
                    for row in &rows {
                        SendMessageW(
                            lists[i],
                            LB_ADDSTRING,
                            0,
                            wide(&row_label(row)).as_ptr() as isize,
                        );
                    }
                    let selected = panel
                        .combat
                        .and_then(|s| s.target)
                        .and_then(|target| rows.iter().position(|r| r.entity == target));
                    SendMessageW(lists[i], LB_SETCURSEL, selected.unwrap_or(usize::MAX), 0);
                    displayed.push(rows);
                }
                let selected = list_state.active.min(list_state.preferences.len() - 1);
                SendMessageW(active, CB_SETCURSEL, selected, 0);
                if held_lists.as_ref() != Some(&list_state) {
                    let config = &list_state.preferences[selected];
                    SetWindowTextW(title, wide(&config.name).as_ptr());
                    SetWindowTextW(search, wide(&config.search).as_ptr());
                    SetWindowTextW(maximum, wide(&config.max_health.to_string()).as_ptr());
                    SendMessageW(players, BM_SETCHECK, usize::from(config.players), 0);
                    SendMessageW(creatures, BM_SETCHECK, usize::from(config.creatures), 0);
                    SendMessageW(sort, CB_SETCURSEL, config.sort as usize, 0);
                }
                let status = panel.combat.map_or_else(
                    || "Combat capability unavailable".into(),
                    |state| {
                        format!(
                            "Fight: {:?}   Chase: {:?}\r\n{}",
                            state.modes.fight_mode,
                            state.modes.chase,
                            panel
                                .feedback
                                .unwrap_or("Select a creature or player to attack")
                        )
                    },
                );
                SetWindowTextW(text, wide(&status).as_ptr());
                for &control in &controls {
                    windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow(
                        control,
                        i32::from(panel.combat.is_some() && !panel.pending),
                    );
                }
                held = Some(panel.clone());
                held_lists = Some(list_state);
            }
            drop(mailbox);
            std::thread::sleep(Duration::from_millis(30));
        }
        if IsWindow(hwnd) != 0 {
            DestroyWindow(hwnd);
        }
        close(&weak);
    }
}
