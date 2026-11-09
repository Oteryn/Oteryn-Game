//! The read-only status panels share native window ownership and a one-second refresh.
#![allow(
    unsafe_code,
    reason = "Win32 controls stay on their creating UI thread"
)]
use super::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use crate::combat_input::PlayCommand;
use oteryn_client_runtime::raw_feature::FeatureCommand;
use std::ptr::{null, null_mut};
use std::sync::{Arc, Weak};
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

pub(super) trait StatusPanel: Clone + PartialEq + Send + 'static {
    fn skill_widget() -> bool {
        false
    }
    fn gauges(&self) -> Vec<(&'static str, u16, bool)> {
        Vec::new()
    }
    fn toggle_skill(&mut self, _index: usize) {}
    fn open(&self) -> bool;
    fn set_open(&mut self, value: bool);
    fn ready(&self) -> bool;
    fn text(&self) -> String;
    fn request(&mut self) -> Option<FeatureCommand>;
    fn ended(&mut self);
}
macro_rules! panel {
    ($module:ident,$panel:ident,$cap:expr,$cmd:expr) => {
        impl StatusPanel for oteryn_client_ui::$module::$panel {
            fn open(&self) -> bool {
                self.open
            }
            fn set_open(&mut self, value: bool) {
                self.open = value;
            }
            fn ready(&self) -> bool {
                self.selected && !self.pending
            }
            fn text(&self) -> String {
                oteryn_client_ui::$module::panel_text(self)
            }
            fn request(&mut self) -> Option<FeatureCommand> {
                oteryn_client_ui::$module::request(self).map(|payload| FeatureCommand {
                    capability: $cap,
                    kind: $cmd,
                    payload,
                })
            }
            fn ended(&mut self) {
                self.pending = false;
                self.error = Some("Session ended");
            }
        }
    };
}
panel!(condition_status, ConditionStatusPanel, 276, 492);
panel!(mechanics_status, MechanicsStatusPanel, 384, 669);
panel!(rest_status, RestStatusPanel, 277, 493);
panel!(battle_instinct_status, BattleInstinctStatusPanel, 278, 494);
impl StatusPanel for oteryn_client_ui::skill_progress::SkillProgressPanel {
    fn skill_widget() -> bool {
        true
    }
    fn gauges(&self) -> Vec<(&'static str, u16, bool)> {
        self.gauges()
    }
    fn toggle_skill(&mut self, index: usize) {
        self.toggle_skill(index);
    }
    fn open(&self) -> bool {
        self.open
    }
    fn set_open(&mut self, value: bool) {
        self.open = value;
    }
    fn ready(&self) -> bool {
        self.selected && !self.pending
    }
    fn text(&self) -> String {
        oteryn_client_ui::skill_progress::panel_text(self)
    }
    fn request(&mut self) -> Option<FeatureCommand> {
        oteryn_client_ui::skill_progress::request(self).map(|payload| FeatureCommand {
            capability: if self.complete_selected { 370 } else { 279 },
            kind: if self.complete_selected { 640 } else { 495 },
            payload,
        })
    }
    fn ended(&mut self) {
        self.pending = false;
        self.error = Some(tr!("Session ended"));
    }
}
panel!(party_effects, PartyEffectsPanel, 280, 496);
panel!(potion_durations, PotionDurationsPanel, 381, 662);

#[derive(Default)]
struct PanelActions {
    refresh: bool,
    skills: [bool; 8],
}
unsafe extern "system" fn window_proc(hwnd: HWND, message: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    // SAFETY: the window's creating thread owns the refresh flag until DestroyWindow.
    unsafe {
        if message == WM_COMMAND && (w >> 16) & 0xffff == BN_CLICKED as usize {
            let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut PanelActions;
            if !actions.is_null() {
                let id = w & 0xffff;
                if id == 1 {
                    (*actions).refresh = true;
                }
                if let Some(index) = id.checked_sub(10)
                    && index < 8
                {
                    (*actions).skills[index] = true;
                }
            }
            return 0;
        }
        DefWindowProcW(hwnd, message, w, l)
    }
}
fn wide(text: &str) -> Vec<u16> {
    text.replace('\0', "\u{fffd}")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}

pub(super) fn open_window<P: StatusPanel>(
    link: &PlayLink,
    title: &'static str,
    get: fn(&mut Mailbox) -> &mut P,
) {
    {
        let mut mailbox = lock(&link.mailbox);
        if get(&mut mailbox).open() {
            return;
        }
        get(&mut mailbox).set_open(true);
    }
    let weak = Arc::downgrade(&link.mailbox);
    let failed = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("status-panel".into())
        .spawn(move || run(weak, commands, title, get))
        .is_err()
    {
        close(&failed, get);
    }
}
fn close<P: StatusPanel>(weak: &Weak<std::sync::Mutex<Mailbox>>, get: fn(&mut Mailbox) -> &mut P) {
    if let Some(mailbox) = weak.upgrade() {
        get(&mut lock(&mailbox)).set_open(false);
    }
}
fn run<P: StatusPanel>(
    weak: Weak<std::sync::Mutex<Mailbox>>,
    commands: tokio::sync::mpsc::UnboundedSender<PlayCommand>,
    title: &str,
    get: fn(&mut Mailbox) -> &mut P,
) {
    // SAFETY: controls and userdata remain confined to this thread; Win32 copies strings.
    // The boxed refresh flag stays alive until after DestroyWindow.
    unsafe {
        let mut bounds =
            crate::window_layout::restore(title, [CW_USEDEFAULT, CW_USEDEFAULT, 600, 620]);
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide(title).as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE | WS_THICKFRAME,
            bounds[0],
            bounds[1],
            bounds[2],
            bounds[3],
            null_mut(),
            null_mut(),
            null_mut(),
            null(),
        );
        if hwnd.is_null() {
            close(&weak, get);
            return;
        }
        let mut refresh_flag = Box::<PanelActions>::default();
        SetWindowLongPtrW(
            hwnd,
            GWLP_USERDATA,
            (&mut *refresh_flag as *mut PanelActions) as isize,
        );
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, window_proc as *const () as isize);
        let text = CreateWindowExW(
            0,
            wide("EDIT").as_ptr(),
            wide("").as_ptr(),
            WS_CHILD
                | WS_VISIBLE
                | WS_VSCROLL
                | ES_MULTILINE as u32
                | ES_READONLY as u32
                | ES_AUTOVSCROLL as u32,
            16,
            16,
            550,
            if P::skill_widget() { 170 } else { 500 },
            hwnd,
            null_mut(),
            null_mut(),
            null(),
        );
        let refresh = CreateWindowExW(
            0,
            wide("BUTTON").as_ptr(),
            wide(tr!("Refresh")).as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP,
            16,
            530,
            120,
            32,
            hwnd,
            1 as _,
            null_mut(),
            null(),
        );
        if text.is_null() || refresh.is_null() {
            DestroyWindow(hwnd);
            close(&weak, get);
            return;
        }
        use windows_sys::Win32::UI::Controls::{
            ICC_PROGRESS_CLASS, INITCOMMONCONTROLSEX, InitCommonControlsEx, PBM_SETPOS,
            PBM_SETRANGE32,
        };
        let mut gauges = Vec::new();
        if P::skill_widget() {
            let common = INITCOMMONCONTROLSEX {
                dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
                dwICC: ICC_PROGRESS_CLASS,
            };
            InitCommonControlsEx(&common);
            for (index, label) in oteryn_client_ui::skill_progress::SKILL_LABELS
                .iter()
                .enumerate()
            {
                let y = 200 + index as i32 * 34;
                let check = CreateWindowExW(
                    0,
                    wide("BUTTON").as_ptr(),
                    wide(label).as_ptr(),
                    WS_CHILD | WS_VISIBLE | BS_AUTOCHECKBOX as u32,
                    16,
                    y,
                    140,
                    26,
                    hwnd,
                    (10 + index) as _,
                    null_mut(),
                    null(),
                );
                let progress = CreateWindowExW(
                    0,
                    wide("msctls_progress32").as_ptr(),
                    wide("").as_ptr(),
                    WS_CHILD | WS_VISIBLE,
                    160,
                    y + 4,
                    380,
                    18,
                    hwnd,
                    null_mut(),
                    null_mut(),
                    null(),
                );
                if check.is_null() || progress.is_null() {
                    DestroyWindow(hwnd);
                    close(&weak, get);
                    return;
                }
                SendMessageW(progress, PBM_SETRANGE32, 0, 10000);
                gauges.push((check, progress));
            }
        }
        let mut held = None;
        let mut refreshed = Instant::now();
        loop {
            let mut rect: RECT = std::mem::zeroed();
            if GetWindowRect(hwnd, &mut rect) != 0 && IsIconic(hwnd) == 0 {
                bounds = [
                    rect.left,
                    rect.top,
                    rect.right - rect.left,
                    rect.bottom - rect.top,
                ];
            }
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
            let mut mailbox = lock(&shared);
            if mailbox.quest_shutdown || !get(&mut mailbox).open() {
                break;
            }
            let panel = get(&mut mailbox);
            for (index, toggled) in refresh_flag.skills.iter_mut().enumerate() {
                if std::mem::take(toggled) {
                    panel.toggle_skill(index);
                }
            }
            for ((check, progress), (_, percent, visible)) in gauges.iter().zip(panel.gauges()) {
                SendMessageW(*check, BM_SETCHECK, usize::from(visible), 0);
                SendMessageW(*progress, PBM_SETPOS, usize::from(percent), 0);
                ShowWindow(*progress, if visible { SW_SHOW } else { SW_HIDE });
            }
            if std::mem::take(&mut refresh_flag.refresh)
                || refreshed.elapsed() >= Duration::from_secs(1)
            {
                refreshed = Instant::now();
                if let Some(command) = panel.request()
                    && commands.send(PlayCommand::Feature(command)).is_err()
                {
                    panel.ended();
                }
            }
            let rendered = (panel.text(), panel.ready());
            if held.as_ref() != Some(&rendered) {
                SetWindowTextW(text, wide(&rendered.0).as_ptr());
                EnableWindow(refresh, i32::from(rendered.1));
                held = Some(rendered);
            }
            drop(mailbox);
            std::thread::sleep(Duration::from_millis(30));
        }
        crate::window_layout::remember(title, bounds);
        if IsWindow(hwnd) != 0 {
            DestroyWindow(hwnd);
        }
        close(&weak, get);
    }
}
