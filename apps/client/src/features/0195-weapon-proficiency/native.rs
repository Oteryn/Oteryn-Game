//! Native controls; parsing and intent selection are shared with Linux tests.
use oteryn_client_runtime::raw_feature::FeatureCommand;
use oteryn_client_ui::weapon_proficiency::{Intent, WeaponProficiencyPanel, request};

pub(super) fn display(p: &WeaponProficiencyPanel) -> String {
    format!(
        "{}\r\n{}\r\n{}",
        if p.pending {
            "Waiting for the server"
        } else if !p.selected {
            "Feature unavailable in this session"
        } else {
            "Ready"
        },
        p.feedback.unwrap_or(""),
        p.result.as_deref().unwrap_or("")
    )
}

pub(super) fn submit(
    p: &mut WeaponProficiencyPanel,
    id: usize,
    values: [&str; 6],
    clear: bool,
) -> Option<FeatureCommand> {
    if p.pending || !p.selected {
        return None;
    }
    let intent = match id {
        101 => Intent::SelectPerk,
        102 => Intent::Modify,
        103 => Intent::RankUp,
        104 => Intent::OrbRank,
        105 => Intent::ReshapeOffer,
        106 => Intent::ReshapeChoose,
        107 => Intent::DeclineOffer,
        108 => Intent::Clear,
        _ => return None,
    };
    let parsed = (|| {
        Some((
            values[0].trim().parse::<u32>().ok()?,
            values[1].trim().parse::<u8>().ok()?,
            values[2].trim().parse::<u8>().ok()?,
            values[3].trim().parse::<u8>().ok()?,
            values[4].trim().parse::<u64>().ok()?,
            values[5].trim().parse::<u8>().ok()?,
        ))
    })();
    let Some((weapon, level, perk, slot, revision, choice)) = parsed else {
        p.feedback = Some("Enter valid numeric proficiency fields");
        return None;
    };
    if level > 6 || (!clear && perk == u8::MAX) || choice > 2 {
        p.feedback = Some("Enter level 0-6, perk 0-254 and choice 0-2");
        return None;
    }
    p.weapon = weapon;
    p.level = level;
    p.perk = perk;
    p.slot = slot;
    p.revision = revision;
    p.shaping_choice = choice;
    p.clear_perk = clear;
    request(p, intent)
}

#[cfg(windows)]
pub(in crate::play) use window::open_window;
#[cfg(windows)]
mod window {
    // HWND operations stay on the creating thread, following the daily reward panel.
    #![allow(unsafe_code)]
    use super::*;
    use crate::combat_input::PlayCommand;
    use crate::play::{PlayLink, mailbox::lock};
    use std::ptr::{null, null_mut};
    use std::sync::{Arc, Weak};
    use std::time::Duration;
    use tokio::sync::mpsc::UnboundedSender;
    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    type WeakMailbox = Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>;
    #[derive(Default)]
    struct Actions(Vec<usize>);
    unsafe extern "system" fn proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
        unsafe {
            if msg == WM_COMMAND {
                let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
                let id = w & 0xffff;
                let note = (w >> 16) & 0xffff;
                if !actions.is_null() && matches!(id, 101..=108) && note == BN_CLICKED as usize {
                    (*actions).0.push(id);
                }
                return 0;
            }
            DefWindowProcW(hwnd, msg, w, l)
        }
    }
    fn wide(text: &str) -> Vec<u16> {
        text.replace('\0', "\u{fffd}")
            .encode_utf16()
            .chain(Some(0))
            .collect()
    }
    pub(in crate::play) fn open_window(link: &PlayLink) {
        {
            let mut m = lock(&link.mailbox);
            if m.weapon_proficiency.open {
                return;
            }
            m.weapon_proficiency.open = true;
        }
        let weak = Arc::downgrade(&link.mailbox);
        let rollback = weak.clone();
        let commands = link.commands.clone();
        if std::thread::Builder::new()
            .name("0195-weapon-proficiency-panel".into())
            .spawn(move || run(weak, commands))
            .is_err()
        {
            close(&rollback);
        }
    }
    fn run(weak: WeakMailbox, commands: UnboundedSender<PlayCommand>) {
        unsafe {
            let hwnd = CreateWindowExW(
                0,
                wide("STATIC").as_ptr(),
                wide("Weapon proficiency").as_ptr(),
                WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                740,
                650,
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
            let fields = [
                ("Weapon id", "1"),
                ("Level (0-6)", "0"),
                ("Perk index", "0"),
                ("Slot (1-2)", "1"),
                ("Revision", "1"),
                ("Reshape choice (0-2)", "0"),
            ];
            let mut edits = [null_mut(); 6];
            for (i, (label, value)) in fields.iter().enumerate() {
                let y = 10 + i as i32 * 32;
                control(hwnd, "STATIC", label, 301 + i, [10, y, 235, 25], 0);
                edits[i] = control(
                    hwnd,
                    "EDIT",
                    value,
                    201 + i,
                    [250, y, 440, 25],
                    WS_BORDER | WS_TABSTOP,
                );
            }
            let clear = control(
                hwnd,
                "BUTTON",
                "Clear selected perk",
                210,
                [10, 205, 250, 25],
                BS_AUTOCHECKBOX as u32 | WS_TABSTOP,
            );
            for (i, label) in [
                "Select perk",
                "Modify",
                "Rank up",
                "Orb rank",
                "Reshape offer",
                "Choose reshape",
                "Decline offer",
                "Clear modification",
            ]
            .iter()
            .enumerate()
            {
                control(
                    hwnd,
                    "BUTTON",
                    label,
                    101 + i,
                    [
                        10 + (i % 4) as i32 * 175,
                        240 + (i / 4) as i32 * 35,
                        165,
                        28,
                    ],
                    WS_TABSTOP,
                );
            }
            let output = control(
                hwnd,
                "EDIT",
                "",
                400,
                [10, 355, 690, 240],
                ES_MULTILINE as u32 | ES_READONLY as u32 | WS_BORDER | WS_VSCROLL,
            );
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
                for action in std::mem::take(&mut actions.0) {
                    let values = edits.map(read_text);
                    if let Some(command) = submit(
                        &mut mailbox.weapon_proficiency,
                        action,
                        values.each_ref().map(String::as_str),
                        SendMessageW(clear, BM_GETCHECK, 0, 0) == 1,
                    ) && commands.send(PlayCommand::Feature(command)).is_err()
                    {
                        mailbox.weapon_proficiency.pending = false;
                        mailbox.weapon_proficiency.feedback = Some("The session has ended");
                    }
                }
                SetWindowTextW(output, wide(&display(&mailbox.weapon_proficiency)).as_ptr());
                drop(mailbox);
                std::thread::sleep(Duration::from_millis(25));
            }
            DestroyWindow(hwnd);
            close(&weak);
            drop(actions);
        }
    }
    fn control(
        parent: HWND,
        class: &str,
        label: &str,
        id: usize,
        rect: [i32; 4],
        style: u32,
    ) -> HWND {
        unsafe {
            CreateWindowExW(
                0,
                wide(class).as_ptr(),
                wide(label).as_ptr(),
                WS_CHILD | WS_VISIBLE | style,
                rect[0],
                rect[1],
                rect[2],
                rect[3],
                parent,
                id as *mut _,
                null_mut(),
                null(),
            )
        }
    }
    fn close(weak: &WeakMailbox) {
        if let Some(shared) = weak.upgrade() {
            lock(&shared).weapon_proficiency.open = false;
        }
    }

    fn read_text(hwnd: HWND) -> String {
        // SAFETY: hwnd is an edit owned by this thread; the buffer includes its terminator.
        unsafe {
            let len = GetWindowTextLengthW(hwnd).max(0);
            let mut buffer = vec![0; len as usize + 1];
            let read = GetWindowTextW(hwnd, buffer.as_mut_ptr(), len + 1).max(0);
            String::from_utf16_lossy(&buffer[..read as usize])
        }
    }
}
