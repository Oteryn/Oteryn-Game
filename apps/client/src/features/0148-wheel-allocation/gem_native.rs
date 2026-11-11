//! Original geometric Atelier and accessible detail controls; callbacks borrow the live mailbox.
#![allow(
    unsafe_code,
    reason = "Win32 handles and renderer live on their creating thread"
)]
use crate::{
    combat_input::PlayCommand,
    play::{
        feature_0148_wheel_allocation::input::gem_presentation::{self as callbacks, Action},
        mailbox::{SharedMailbox, lock},
    },
};
use oteryn_client_ui::wheel_allocation::{Intent, WheelPanel, gem_label, gems, vessel_name};
use oteryn_foundation::ProcessGeneration;
use oteryn_renderer::WindowsRenderer;
use std::{
    num::NonZeroIsize,
    ptr::{null, null_mut},
    sync::Arc,
};
use tokio::sync::mpsc::UnboundedSender;
use windows_sys::Win32::{
    Foundation::*,
    UI::{Input::KeyboardAndMouse::EnableWindow, WindowsAndMessaging::*},
};
use winit::raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, Win32WindowHandle, WindowHandle, WindowsDisplayHandle,
};
#[path = "gem_window.rs"]
mod window;
use window::{Events, Target, control, procedure, row, wide};
#[path = "gem_launch.rs"]
mod launch;
pub(in crate::play) use launch::open;
fn run(
    weak: std::sync::Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>,
    sender: UnboundedSender<PlayCommand>,
    epoch: u64,
) {
    unsafe {
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide("Gem Atelier — gems and shared modifier grades").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            1140,
            820,
            null_mut(),
            null_mut(),
            null_mut(),
            null(),
        );
        if !hwnd.is_null() {
            let mut events = Box::<Events>::default();
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, (&mut *events as *mut Events) as isize);
            SetWindowLongPtrW(hwnd, GWLP_WNDPROC, procedure as *const () as isize);
            let Some(layout::Controls {
                surface,
                list,
                mods,
                body,
                quality,
                vessel,
                buttons,
            }) = layout::create(hwnd)
            else {
                DestroyWindow(hwnd);
                if let Some(shared) = weak.upgrade() {
                    let mut m = lock(&shared);
                    if m.wheel.gems.window_epoch == epoch {
                        m.wheel.gems.open = false;
                        m.wheel.gems.review = None;
                        m.wheel.gems.feedback =
                            Some("Could not create Gem Atelier controls".into());
                    }
                }
                return;
            };
            let generation = ProcessGeneration::new(1);
            let mut renderer = NonZeroIsize::new(surface as isize)
                .and_then(|h| WindowsRenderer::new(Target(h), generation, 320, 64).ok());
            let mut drawn = None;
            let mut held: Option<WheelPanel> = None;
            let mut painted_revision = None;
            let mut painted_nonce = None;
            let mut gem_ids = Vec::new();
            let mut mod_ids = Vec::new();
            loop {
                let mut message: MSG = std::mem::zeroed();
                while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                    if IsDialogMessageW(hwnd, &message) == 0 {
                        TranslateMessage(&message);
                        DispatchMessageW(&message);
                    }
                }
                if events.close || IsWindow(hwnd) == 0 || sender.is_closed() {
                    break;
                }
                let Some(shared) = weak.upgrade() else {
                    break;
                };
                let mut m = lock(&shared);
                if m.wheel_shutdown
                    || !m.wheel.selected
                    || !m.wheel.gems.open
                    || m.wheel.gems.window_epoch != epoch
                {
                    break;
                }
                for (id, n) in std::mem::take(&mut events.actions) {
                    let action = match (id, n) {
                        (10, n) if n == LBN_SELCHANGE as usize => row(list, false).and_then(|i| {
                            Some(Action::Gem {
                                revision: painted_revision?,
                                id: *gem_ids.get(i)?,
                            })
                        }),
                        (11, n) if n == LBN_SELCHANGE as usize => row(mods, false).and_then(|i| {
                            let (kind, id) = *mod_ids.get(i)?;
                            Some(Action::Modifier {
                                nonce: painted_nonce?,
                                kind,
                                id,
                            })
                        }),
                        (20, n) if n == CBN_SELCHANGE as usize => row(quality, true)
                            .and_then(|i| u8::try_from(i).ok())
                            .map(Action::Quality),
                        (21, n) if n == CBN_SELCHANGE as usize => row(vessel, true)
                            .and_then(|i| u8::try_from(i).ok())
                            .map(Action::Vessel),
                        (100, n) if n == BN_CLICKED as usize => Some(Action::Stage(Intent::Reveal)),
                        (101, n) if n == BN_CLICKED as usize => {
                            Some(Action::Stage(Intent::Dismantle))
                        }
                        (102, n) if n == BN_CLICKED as usize => {
                            Some(Action::Stage(Intent::Reshape))
                        }
                        (103, n) if n == BN_CLICKED as usize => {
                            Some(Action::Stage(Intent::GradeUp))
                        }
                        (104, n) if n == BN_CLICKED as usize => Some(Action::Lock),
                        (105, n) if n == BN_CLICKED as usize => Some(Action::Socket),
                        (106, n) if n == BN_CLICKED as usize => Some(Action::Clear),
                        (107, n) if n == BN_CLICKED as usize => Some(Action::Save),
                        (108, n) if n == BN_CLICKED as usize => Some(Action::Refresh),
                        (109, n) if n == BN_CLICKED as usize => Some(Action::Confirm),
                        (110, n) if n == BN_CLICKED as usize => Some(Action::Cancel),
                        (111, n) if n == BN_CLICKED as usize => Some(Action::Retry),
                        _ => None,
                    };
                    if let Some(action) = action {
                        callbacks::activate(&mut m, &sender, action);
                    }
                }
                if m.wheel
                    .gems
                    .review
                    .as_ref()
                    .is_some_and(|r| r.expires <= std::time::Instant::now())
                {
                    m.wheel.gems.review = None;
                    m.wheel.gems.feedback =
                        Some("Review expired. Choose the operation again".into());
                }
                if m.wheel.gems.selected
                    && m.wheel
                        .confirmed_atelier
                        .as_ref()
                        .is_some_and(|a| Some(a.revision) != m.wheel.gems.last_revision)
                    && m.wheel.gems.review.is_none()
                {
                    callbacks::query(&mut m, &sender);
                }
                let panel = m.wheel.clone();
                drop(m);
                if held.as_ref() != Some(&panel) {
                    let revision = panel.confirmed_atelier.as_ref().map(|a| a.revision);
                    if revision != painted_revision {
                        SendMessageW(list, LB_RESETCONTENT, 0, 0);
                        gem_ids.clear();
                        if let Some(a) = &panel.confirmed_atelier {
                            for g in &a.gems {
                                gem_ids.push(g.id);
                                SendMessageW(
                                    list,
                                    LB_ADDSTRING,
                                    0,
                                    wide(&gem_label(g)).as_ptr() as isize,
                                );
                            }
                        }
                        painted_revision = revision;
                    }
                    let nonce = gems::current(&panel).map(|r| r.request.nonce);
                    if nonce != painted_nonce {
                        SendMessageW(mods, LB_RESETCONTENT, 0, 0);
                        mod_ids.clear();
                        if let Some(r) = gems::current(&panel) {
                            for row in &r.modifiers {
                                mod_ids.push((row.kind, row.id));
                                SendMessageW(
                                    mods,
                                    LB_ADDSTRING,
                                    0,
                                    wide(&row.name).as_ptr() as isize,
                                );
                            }
                        }
                        painted_nonce = nonce;
                    }
                    SendMessageW(
                        list,
                        LB_SETCURSEL,
                        gem_ids
                            .iter()
                            .position(|id| Some(*id) == panel.gem)
                            .unwrap_or(usize::MAX),
                        0,
                    );
                    SendMessageW(
                        mods,
                        LB_SETCURSEL,
                        mod_ids
                            .iter()
                            .position(|id| Some(*id) == panel.modifier)
                            .unwrap_or(usize::MAX),
                        0,
                    );
                    SendMessageW(quality, CB_SETCURSEL, usize::from(panel.quality), 0);
                    SendMessageW(vessel, CB_SETCURSEL, usize::from(panel.vessel_slot), 0);
                    let mut detail = gems::view::details(&panel);
                    if renderer.is_none() {
                        detail.push_str("\nGem glyph renderer unavailable; the current text details remain available.");
                    }
                    SetWindowTextW(body, wide(&detail.replace('\n', "\r\n")).as_ptr());
                    let editing = panel.ready()
                        && panel.gems.pending.is_none()
                        && panel.gems.review.is_none();
                    for h in [list, mods, quality, vessel] {
                        EnableWindow(h, i32::from(editing));
                    }
                    for (index, h) in buttons.iter().enumerate() {
                        EnableWindow(
                            *h,
                            i32::from(if index == 11 {
                                panel.retry.is_some() && panel.pending.is_none()
                            } else if index >= 9 {
                                panel.gems.review.is_some() && panel.pending.is_none()
                            } else {
                                editing
                            }),
                        );
                    }
                    drawn = None;
                    if let Ok((atlas, batch)) = super::gem_surface::frame(&panel)
                        && let Some(r) = &mut renderer
                    {
                        r.set_atlas(atlas);
                        drawn = Some(batch);
                    }
                    held = Some(panel);
                }
                if let (Some(r), Some(batch)) = (&mut renderer, &drawn) {
                    let _ =
                        r.render_batches(generation, batch, &oteryn_renderer::SpriteBatch::new());
                }
                MsgWaitForMultipleObjects(0, null(), 0, 50, QS_ALLINPUT);
            }
            if let Some(r) = &mut renderer {
                let _ = r.close(generation);
            }
            drop(renderer);
            DestroyWindow(hwnd);
        }
    }
    if let Some(shared) = weak.upgrade() {
        let mut m = lock(&shared);
        if m.wheel.gems.window_epoch == epoch {
            m.wheel.gems.open = false;
            m.wheel.gems.review = None;
        }
    }
}

#[path = "gem_layout.rs"]
mod layout;
