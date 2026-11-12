fn run(weak: WeakMailbox, commands: UnboundedSender<PlayCommand>) {
    // SAFETY: windows, controls, userdata and message dispatch remain on this thread.
    unsafe {
        let mut actions = Box::<Actions>::default();
        let hwnd = window(null_mut(), "Prey", 936, 446, &mut actions);
        if hwnd.is_null() {
            close(&weak);
            return;
        }
        let description = control(hwnd, "STATIC", "", 900, [12, 12, 770, 28], 0);
        let refresh = control(
            hwnd,
            "BUTTON",
            "Refresh",
            REFRESH,
            [796, 10, 110, 28],
            WS_TABSTOP,
        );
        let slots: Vec<_> = (0..3).map(|index| slot_controls(hwnd, index)).collect();
        if description.is_null()
            || refresh.is_null()
            || slots.iter().any(|slot| {
                slot.title.is_null()
                    || slot.creature.is_null()
                    || slot
                        .creatures
                        .iter()
                        .chain(slot.tiers.iter())
                        .chain(slot.buttons.iter())
                        .any(|h| h.is_null())
            })
        {
            DestroyWindow(hwnd);
            close(&weak);
            return;
        }
        let mut offers: Option<Offers> = None;
        let mut held: Option<PreyPanel> = None;
        let mut message: MSG = std::mem::zeroed();
        loop {
            while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                let dialog = offers
                    .as_ref()
                    .filter(|offers| IsWindow(offers.window) != 0)
                    .map_or(hwnd, |offers| offers.window);
                if IsDialogMessageW(dialog, &message) == 0 {
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
            if mailbox.prey_shutdown {
                break;
            }
            if offers
                .as_ref()
                .is_some_and(|offers| IsWindow(offers.window) == 0)
            {
                offers = None;
                mailbox.prey.offered_slot = None;
            }
            let mut confirmation = None;
            for action in std::mem::take(&mut actions.0) {
                let intent = if action == REFRESH {
                    Some(Intent::Refresh)
                } else if (OFFER..OFFER + prey::MAX_OFFERED_PAGE).contains(&action) {
                    confirmation = mailbox.prey.confirm_offer(action - OFFER);
                    None
                } else if action == FIRST || action == NEXT {
                    mailbox.prey.offered_slot.map(|slot| Intent::OfferedToday {
                        slot,
                        after: if action == FIRST {
                            String::new()
                        } else {
                            mailbox.prey.offered_next.clone()
                        },
                    })
                } else if (1000..1300).contains(&action) {
                    let slot = ((action - 1000) / 100 + 1) as u8;
                    let control = (action - 1000) % 100;
                    match control {
                        0..=63 => {
                            mailbox.prey.choose_candidate(slot, control as u8 + 1);
                            None
                        }
                        70..=74 => Some(Intent::SetTier {
                            slot,
                            tier: (control - 69) as u8,
                        }),
                        80 => Some(Intent::ListReroll { slot }),
                        81 => Some(Intent::BonusReroll { slot }),
                        82 => Some(Intent::Select { slot }),
                        83 => Some(Intent::OfferedToday {
                            slot,
                            after: String::new(),
                        }),
                        84..=86 => Some(Intent::SetOption {
                            slot,
                            option: (control - 84) as u8,
                        }),
                        87 => {
                            let mut text = [0_u16; 257];
                            let count = GetWindowTextW(
                                slots[usize::from(slot - 1)].creature,
                                text.as_mut_ptr(),
                                text.len() as i32,
                            );
                            String::from_utf16(&text[..count as usize])
                                .ok()
                                .map(|creature| Intent::SelectAny { slot, creature })
                        }
                        _ => None,
                    }
                } else {
                    None
                };
                if let Some(intent) = intent {
                    super::super::input::enqueue(&mut mailbox, &commands, intent);
                }
            }
            if mailbox.prey.offered_slot.is_some() && offers.is_none() {
                let created = offered_window(hwnd, &mut actions);
                if created.window.is_null()
                    || created.rows.iter().any(|h| h.is_null())
                    || created.first.is_null()
                    || created.next.is_null()
                {
                    if !created.window.is_null() {
                        DestroyWindow(created.window);
                    }
                    mailbox.prey.offered_slot = None;
                    mailbox.prey.feedback = Some("Could not open the offered-creature list.");
                } else {
                    offers = Some(created);
                }
            }
            if mailbox.prey.offered_slot.is_none()
                && let Some(old) = offers.take()
            {
                DestroyWindow(old.window);
            }
            let panel = &mailbox.prey;
            if held.as_ref() != Some(panel) {
                SetWindowTextW(description, wide(&panel_text(panel)).as_ptr());
                EnableWindow(
                    refresh,
                    i32::from(panel.selected && panel.pending.is_none()),
                );
                render_slots(hwnd, &slots, panel);
                if let Some(offers) = &offers {
                    render_offers(offers, panel);
                }
                held = Some(panel.clone());
            }
            drop(mailbox);
            if let Some(confirmation) = confirmation {
                let answer = MessageBoxW(
                    hwnd,
                    wide(&format!(
                        "Select {} again in slot {} for {} gold?",
                        creature_label(&confirmation.creature),
                        confirmation.slot,
                        confirmation.price_gold
                    ))
                    .as_ptr(),
                    wide("Confirm Prey purchase").as_ptr(),
                    MB_YESNO | MB_ICONQUESTION | MB_DEFBUTTON2,
                );
                if answer == IDYES {
                    super::super::input::enqueue(
                        &mut lock(&shared),
                        &commands,
                        Intent::ConfirmPickAgain(confirmation),
                    );
                } else {
                    lock(&shared).prey.confirmation = None;
                }
            }
            std::thread::sleep(Duration::from_millis(30));
        }
        if let Some(offers) = offers
            && IsWindow(offers.window) != 0
        {
            DestroyWindow(offers.window);
        }
        if IsWindow(hwnd) != 0 {
            DestroyWindow(hwnd);
        }
        close(&weak);
    }
}
