use super::*;
pub(super) fn run(
    weak: Shared,
    commands: tokio::sync::mpsc::UnboundedSender<PlayCommand>,
    backend: Backend,
) {
    // SAFETY: handles and action storage remain on this thread until after DestroyWindow.
    unsafe {
        let h = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide(tr!("Account achievements — Ctrl+Shift+A")).as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            800,
            640,
            null_mut(),
            null_mut(),
            null_mut(),
            null(),
        );
        if h.is_null() {
            close(&weak, backend);
            return;
        }
        let mut actions = Box::<Actions>::default();
        SetWindowLongPtrW(h, GWLP_USERDATA, (&mut *actions as *mut Actions) as isize);
        SetWindowLongPtrW(h, GWLP_WNDPROC, proc as *const () as isize);
        let search = ctl(
            h,
            "EDIT",
            "",
            10,
            [10, 10, 330, 26],
            WS_BORDER | ES_AUTOHSCROLL as u32,
        );
        SendMessageW(search, EM_SETLIMITTEXT, 128, 0);
        let filter = ctl(
            h,
            "COMBOBOX",
            "",
            11,
            [350, 10, 180, 150],
            CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
        );
        for label in [tr!("All achievements"), tr!("Regular"), tr!("Secret")] {
            SendMessageW(filter, CB_ADDSTRING, 0, wide(label).as_ptr() as isize);
        }
        SendMessageW(filter, CB_SETCURSEL, 0, 0);
        let sort = ctl(
            h,
            "COMBOBOX",
            "",
            12,
            [540, 10, 230, 150],
            CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
        );
        for label in [
            tr!("Name"),
            tr!("Points"),
            tr!("Grade"),
            tr!("Recently earned"),
        ] {
            SendMessageW(sort, CB_ADDSTRING, 0, wide(label).as_ptr() as isize);
        }
        SendMessageW(sort, CB_SETCURSEL, 0, 0);
        let header = ctl(h, "STATIC", "", 13, [10, 48, 760, 45], 0);
        let list = ctl(
            h,
            "LISTBOX",
            "",
            14,
            [10, 100, 760, 275],
            WS_BORDER | WS_VSCROLL | LBS_NOTIFY as u32,
        );
        let detail = ctl(
            h,
            "EDIT",
            "",
            15,
            [10, 385, 760, 140],
            WS_BORDER | WS_VSCROLL | ES_MULTILINE as u32 | ES_READONLY as u32,
        );
        let refresh = ctl(
            h,
            "BUTTON",
            tr!("Load / refresh account"),
            2,
            [10, 540, 240, 32],
            0,
        );
        let previous = ctl(h, "BUTTON", tr!("Previous page"), 3, [270, 540, 240, 32], 0);
        let next = ctl(h, "BUTTON", tr!("Next page"), 4, [530, 540, 240, 32], 0);
        if [
            search, filter, sort, header, list, detail, refresh, previous, next,
        ]
        .iter()
        .any(|h| h.is_null())
        {
            DestroyWindow(h);
            close(&weak, backend);
            return;
        }
        let mut held = None;
        let mut shown: Vec<oteryn_client_runtime::account_achievements::AccountAchievementRow> =
            Vec::new();
        let mut matching = 0;
        loop {
            let mut msg: MSG = std::mem::zeroed();
            while PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) != 0 {
                if IsDialogMessageW(h, &msg) == 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
            if IsWindow(h) == 0 || commands.is_closed() {
                break;
            }
            let Some(shared) = weak.upgrade() else { break };
            let search_value = text(search);
            let filter_value = match SendMessageW(filter, CB_GETCURSEL, 0, 0) {
                1 => AchievementFilter::Regular,
                2 => AchievementFilter::Secret,
                _ => AchievementFilter::All,
            };
            let sort_value = match SendMessageW(sort, CB_GETCURSEL, 0, 0) {
                1 => AchievementSort::Points,
                2 => AchievementSort::Grade,
                3 => AchievementSort::Earned,
                _ => AchievementSort::Name,
            };
            let selected = usize::try_from(SendMessageW(list, LB_GETCURSEL, 0, 0))
                .ok()
                .and_then(|i| shown.get(i));
            SetWindowTextW(
                detail,
                wide(&selected.map_or_else(
                    || tr!("Select an achievement to read its description").into(),
                    |r| {
                        lfmt!(
                            "{}\r\n{}\r\nEarned: {} (server Unix ms)",
                            r.name,
                            r.description,
                            r.earned_at
                        )
                    },
                ))
                .as_ptr(),
            );
            let mut m = lock(&shared);
            if !backend.valid(&m) {
                break;
            }
            let ready = backend.ready(&m);
            SetWindowTextW(h, wide(&backend.title(&m)).as_ptr());
            let p = backend.panel(&mut m);
            let mut intents = Vec::new();
            if p.catalog.search != search_value
                || p.catalog.filter != filter_value
                || p.catalog.sort != sort_value
            {
                p.catalog.page = 0;
            }
            p.catalog.search = search_value;
            p.catalog.filter = filter_value;
            p.catalog.sort = sort_value;
            for id in actions.0.drain(..) {
                match id {
                    2 => {
                        p.catalog.enabled = true;
                        intents.push(Intent::Refresh);
                    }
                    3 if !p.catalog.enabled => {
                        intents.push(Intent::Previous);
                    }
                    4 if !p.catalog.enabled => {
                        intents.push(Intent::Next);
                    }
                    3 => {
                        p.catalog.page = p.catalog.page.saturating_sub(1);
                    }
                    4 if p.catalog.page.saturating_add(1).saturating_mul(64) < matching => {
                        p.catalog.page += 1;
                    }
                    _ => {}
                }
            }
            let signature = (
                p.catalog.revision,
                p.catalog.filter,
                p.catalog.sort,
                p.catalog.search.clone(),
                p.catalog.page,
                p.pending,
                p.error,
                p.selected,
                p.catalog.collecting,
            );
            if held.as_ref() != Some(&signature) {
                let rows = if p.catalog.ready {
                    catalog_rows(&p.catalog)
                } else {
                    p.page
                        .as_ref()
                        .map_or_else(Vec::new, |page| page.rows.iter().collect())
                };
                matching = rows.len();
                shown = rows
                    .into_iter()
                    .skip(if p.catalog.ready {
                        p.catalog.page.saturating_mul(64)
                    } else {
                        0
                    })
                    .take(64)
                    .cloned()
                    .collect();
                SendMessageW(list, LB_RESETCONTENT, 0, 0);
                for row in &shown {
                    let label = lfmt!(
                        "{} — Grade {} — {} points{}",
                        row.name,
                        row.grade,
                        row.points,
                        if row.secret { tr!(" — Secret") } else { "" }
                    );
                    SendMessageW(list, LB_ADDSTRING, 0, wide(&label).as_ptr() as isize);
                }
                let status = if !p.selected {
                    tr!("Feature unavailable in this session")
                } else if p.catalog.collecting {
                    tr!("Loading the complete account list")
                } else {
                    p.error.unwrap_or("")
                };
                let points = if p.catalog.ready {
                    p.catalog.points
                } else {
                    p.page.as_ref().map_or(0, |page| page.total_points)
                };
                let earned = if p.catalog.ready {
                    p.catalog.rows.len()
                } else {
                    p.page.as_ref().map_or(0, |page| page.fact_count as usize)
                };
                let page = if p.catalog.ready {
                    (p.catalog.page as u64).saturating_add(1)
                } else {
                    p.page.as_ref().map_or(1, |page| u64::from(page.page) + 1)
                };
                let scope = if p.catalog.ready {
                    "matching"
                } else {
                    tr!("here; load all for filters")
                };
                SetWindowTextW(header, wide(&lfmt!("{points} points · {earned} earned · {matching} {scope} · Page {page}\r\n{status}",earned=earned,matching=matching,page=page,points=points,scope=scope,status=status)).as_ptr());
                held = Some(signature);
            }
            EnableWindow(
                refresh,
                i32::from(ready && p.selected && p.pending.is_none()),
            );
            EnableWindow(
                previous,
                i32::from(if p.catalog.ready {
                    p.catalog.page > 0
                } else {
                    !p.catalog.enabled && p.pending.is_none() && p.has_previous()
                }),
            );
            EnableWindow(
                next,
                i32::from(if p.catalog.ready {
                    (p.catalog.page + 1).saturating_mul(64) < matching
                } else {
                    !p.catalog.enabled && p.pending.is_none() && p.has_next()
                }),
            );
            for control in [search, filter, sort] {
                EnableWindow(control, i32::from(p.catalog.ready));
            }
            for intent in intents {
                if let Some(command) = backend.request(&mut m, intent)
                    && commands.send(command).is_err()
                {
                    backend.fail(&mut m);
                }
            }
            if let Some(command) = backend.next(&mut m)
                && commands.send(command).is_err()
            {
                backend.fail(&mut m);
            }
            drop(m);
            std::thread::sleep(std::time::Duration::from_millis(30));
        }
        if IsWindow(h) != 0 {
            DestroyWindow(h);
        }
        close(&weak, backend);
    }
}
