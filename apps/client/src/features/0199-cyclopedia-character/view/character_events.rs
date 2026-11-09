use super::*;

#[derive(Default)]
pub(super) struct Actions(pub(super) Vec<usize>);

pub(super) unsafe extern "system" fn proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_COMMAND {
            let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            let id = w & 0xffff;
            let note = (w >> 16) & 0xffff;
            if !actions.is_null()
                && (*actions).0.len() < 32
                && matches!(id, 101..=119 | 140 | 145..=150 | 152 | 153 | 180..=185)
                && note == BN_CLICKED as usize
            {
                (*actions).0.push(id);
            }
            return 0;
        }
        DefWindowProcW(hwnd, msg, w, l)
    }
}

pub(super) fn close(weak: &WeakMailbox) {
    if let Some(shared) = weak.upgrade() {
        lock(&shared).cyclopedia_character.open = false;
    }
}

pub(in crate::play) fn open_window(link: &PlayLink) {
    {
        let mut m = lock(&link.mailbox);
        if m.cyclopedia_character.open {
            return;
        }
        m.cyclopedia_character.open = true;
    }
    link.cyclopedia_character_request(super::super::target::refresh_intent(
        &link.cyclopedia_character_panel(),
    ));
    let weak = Arc::downgrade(&link.mailbox);
    let rollback = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("0199-cyclopedia-character-panel".into())
        .spawn(move || run(weak, commands))
        .is_err()
    {
        close(&rollback);
    }
}
