fn run(
    weak: Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>,
    commands: UnboundedSender<PlayCommand>,
) {
    // SAFETY: all HWND operations and message dispatch happen on their creating thread.
    unsafe {
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide("Quest Log — select a quest, then a mission").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            1080,
            580,
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
        let QuestControls {
            quests,
            list_search,
            list_sort,
            missions,
            description,
            refresh,
            track,
        } = create_questcontrols(hwnd);
        if [
            quests,
            missions,
            description,
            refresh,
            track,
            list_search,
            list_sort,
        ]
        .iter()
        .any(|h| h.is_null())
        {
            DestroyWindow(hwnd);
            close(&weak);
            return;
        }
        let auto_add = control(
            hwnd,
            "BUTTON",
            "Auto-add: off",
            AUTO_ADD,
            750,
            10,
            290,
            28,
            WS_TABSTOP,
        );
        let auto_remove = control(
            hwnd,
            "BUTTON",
            "Auto-remove: off",
            AUTO_REMOVE,
            750,
            45,
            290,
            28,
            WS_TABSTOP,
        );
        let filter = control(
            hwnd,
            "BUTTON",
            "Tracker: all quests",
            FILTER,
            750,
            80,
            290,
            28,
            WS_TABSTOP,
        );
        control(
            hwnd,
            "STATIC",
            "Search tracked missions",
            109,
            750,
            115,
            290,
            22,
            0,
        );
        let search = control(
            hwnd,
            "EDIT",
            "",
            110,
            750,
            140,
            290,
            28,
            WS_BORDER | WS_TABSTOP,
        );
        SendMessageW(
            search,
            windows_sys::Win32::UI::Controls::EM_SETLIMITTEXT,
            128,
            0,
        );
        let tracker = control(
            hwnd,
            "EDIT",
            "",
            111,
            750,
            180,
            290,
            350,
            ES_MULTILINE as u32 | ES_READONLY as u32 | WS_VSCROLL | WS_BORDER,
        );
        let mut held = None;
        let mut shown_quests = Vec::new();
        let mut options_initialized = false;
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
            let Some(mailbox) = weak.upgrade() else {
                break;
            };
            let mut mailbox = lock(&mailbox);
            if mailbox.quest_shutdown {
                break;
            }
            if !options_initialized {
                SetWindowTextW(list_search, wide(&mailbox.quest.list_search).as_ptr());
                SendMessageW(
                    list_sort,
                    CB_SETCURSEL,
                    mailbox.quest.list_sort.selection(),
                    0,
                );
                options_initialized = true;
            }
            for id in std::mem::take(&mut actions.0) {
                let query = match id {
                    AUTO_ADD => {
                        let p = &mut mailbox.quest;
                        p.tracker.auto_add = !p.tracker.auto_add;
                        None
                    }
                    AUTO_REMOVE => {
                        mailbox.quest.tracker.auto_remove = !mailbox.quest.tracker.auto_remove;
                        None
                    }
                    FILTER => {
                        use oteryn_client_ui::quest_log::TrackerFilter as F;
                        mailbox.quest.tracker.filter = match mailbox.quest.tracker.filter {
                            F::All => F::Active,
                            F::Active => F::Completed,
                            F::Completed => F::All,
                        };
                        None
                    }
                    REFRESH => Some(QuestLogQuery::List),
                    QUESTS => {
                        let row = SendMessageW(quests, LB_GETCURSEL, 0, 0);
                        usize::try_from(row)
                            .ok()
                            .and_then(|i| quest_row_query(&mailbox.quest, &shown_quests, i))
                    }
                    MISSIONS => {
                        let row = SendMessageW(missions, LB_GETCURSEL, 0, 0);
                        mailbox.quest.mission = match &mailbox.quest.state.log.view {
                            Some(QuestLogView::Line(line)) => usize::try_from(row)
                                .ok()
                                .and_then(|i| line.missions.get(i))
                                .map(|m| m.mission_index),
                            _ => None,
                        };
                        None
                    }
                    TRACK => tracking_query(&mailbox.quest),
                    _ => None,
                };
                if let Some(query) = query
                    && !mailbox.quest_pending
                    && let Ok(payload) = request(&mut mailbox.quest, &query)
                {
                    mailbox.quest_pending = commands.send(PlayCommand::QuestLog(payload)).is_ok();
                }
            }
            let mut list_text = [0u16; 129];
            let len = GetWindowTextW(list_search, list_text.as_mut_ptr(), 129).max(0) as usize;
            mailbox.quest.list_search = String::from_utf16_lossy(&list_text[..len]);
            mailbox.quest.list_sort =
                QuestSort::from_selection(SendMessageW(list_sort, CB_GETCURSEL, 0, 0));
            let mut search_text = [0u16; 129];
            let len = GetWindowTextW(search, search_text.as_mut_ptr(), 129);
            mailbox.quest.tracker.search =
                String::from_utf16_lossy(&search_text[..len.max(0) as usize]);
            if !mailbox.quest_pending
                && let Some(query) = oteryn_client_ui::quest_log::tracker_query(
                    &mut mailbox.quest,
                    std::time::Instant::now(),
                )
                && let Ok(payload) = request(&mut mailbox.quest, &query)
            {
                mailbox.quest_pending = commands.send(PlayCommand::QuestLog(payload)).is_ok();
            }
            let panel = &mailbox.quest;
            SetWindowTextW(
                auto_add,
                wide(if panel.tracker.auto_add {
                    "Auto-add: on"
                } else {
                    "Auto-add: off"
                })
                .as_ptr(),
            );
            SetWindowTextW(
                auto_remove,
                wide(if panel.tracker.auto_remove {
                    "Auto-remove: on"
                } else {
                    "Auto-remove: off"
                })
                .as_ptr(),
            );
            SetWindowTextW(
                filter,
                wide(&format!("Tracker: {:?}", panel.tracker.filter)).as_ptr(),
            );
            if held.as_ref() != Some(panel) {
                let text = oteryn_client_ui::quest_log::tracker_lines(panel)
                    .iter()
                    .map(|line| {
                        format!(
                            "Quest {}{}\r\n{}",
                            line.quest_index,
                            if line.completed { " (completed)" } else { "" },
                            line.missions
                                .iter()
                                .map(|m| format!(
                                    "{}{}: {}",
                                    m.name,
                                    if m.done { " (done)" } else { "" },
                                    m.text
                                ))
                                .collect::<Vec<_>>()
                                .join("\r\n")
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\r\n\r\n");
                SetWindowTextW(tracker, wide(&text).as_ptr());
                if held.as_ref().is_none_or(|old: &QuestPanel| {
                    old.state.list != panel.state.list
                        || old.list_search != panel.list_search
                        || old.list_sort != panel.list_sort
                }) {
                    let selected = usize::try_from(SendMessageW(quests, LB_GETCURSEL, 0, 0))
                        .ok()
                        .and_then(|i| shown_quests.get(i))
                        .copied();
                    SendMessageW(quests, LB_RESETCONTENT, 0, 0);
                    let rows = quest_rows(panel);
                    shown_quests = rows.iter().map(|q| q.quest_index).collect();
                    for q in rows {
                        let label = format!(
                            "Quest {}{}",
                            q.quest_index,
                            if q.completed { " (completed)" } else { "" }
                        );
                        SendMessageW(quests, LB_ADDSTRING, 0, wide(&label).as_ptr() as isize);
                    }
                    if let Some(row) =
                        selected.and_then(|id| shown_quests.iter().position(|q| *q == id))
                    {
                        SendMessageW(quests, LB_SETCURSEL, row, 0);
                    }
                }
                if held
                    .as_ref()
                    .is_none_or(|old| old.state.log.view != panel.state.log.view)
                {
                    SendMessageW(missions, LB_RESETCONTENT, 0, 0);
                    if let Some(QuestLogView::Line(line)) = &panel.state.log.view {
                        let selected = shown_quests.iter().position(|id| *id == line.quest_index);
                        SendMessageW(
                            quests,
                            LB_SETCURSEL,
                            selected.map_or(usize::MAX, |row| row),
                            0,
                        );
                        for m in &line.missions {
                            SendMessageW(
                                missions,
                                LB_ADDSTRING,
                                0,
                                wide(&format!(
                                    "{}{}",
                                    m.name,
                                    if m.done { " (completed)" } else { "" }
                                ))
                                .as_ptr() as isize,
                            );
                        }
                    }
                }
                SetWindowTextW(description, wide(&panel_text(panel)).as_ptr());
                let tracked = matches!(&panel.state.log.view, Some(QuestLogView::Line(line)) if panel.state.log.tracked.iter().any(|q| q.quest_index == line.quest_index));
                SetWindowTextW(
                    track,
                    wide(if tracked {
                        "Stop tracking quest"
                    } else {
                        "Track quest"
                    })
                    .as_ptr(),
                );
                held = Some(panel.clone());
            }
            drop(mailbox);
            std::thread::sleep(Duration::from_millis(30));
        }
        if IsWindow(hwnd) != 0 {
            DestroyWindow(hwnd);
        }
        // No procedure can run after destruction; the userdata box can now be dropped.
        close(&weak);
    }
}
