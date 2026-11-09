use super::*;
pub(super) struct Pane {
    pub id: String,
    pub base: [i32; 2],
    pub children: Vec<(usize, [i32; 4])>,
}
pub(super) unsafe extern "system" fn child(hwnd: HWND, data: LPARAM) -> i32 {
    unsafe {
        let pane = &mut *(data as *mut Pane);
        let parent = GetParent(hwnd);
        let mut info: WINDOWINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<WINDOWINFO>() as u32;
        GetWindowInfo(parent, &mut info);
        let mut bounds = rect(hwnd);
        bounds[0] -= info.rcClient.left;
        bounds[1] -= info.rcClient.top;
        pane.children.push((hwnd as usize, bounds));
        1
    }
}
pub(super) unsafe extern "system" fn procedure(
    hwnd: HWND,
    msg: u32,
    w: WPARAM,
    l: LPARAM,
    _id: usize,
    data: usize,
) -> LRESULT {
    unsafe {
        let pane = &*(data as *const Pane);
        match msg {
            WM_ENTERSIZEMOVE => {
                lock().moving.insert(hwnd as usize);
            }
            WM_EXITSIZEMOVE => {
                let root = lock().root as HWND;
                let mut info: WINDOWINFO = std::mem::zeroed();
                info.cbSize = std::mem::size_of::<WINDOWINFO>() as u32;
                GetWindowInfo(root, &mut info);
                let mut bounds = rect(hwnd);
                bounds[0] -= info.rcClient.left;
                bounds[1] -= info.rcClient.top;
                let mut host = lock();
                let size = host.size;
                host.layout.dragged(&pane.id, bounds, size);
                host.moving.remove(&(hwnd as usize));
                save(&host);
            }
            WM_SIZE => {
                let mut rect: RECT = std::mem::zeroed();
                GetClientRect(hwnd, &mut rect);
                for (child, [x, y, w, h]) in &pane.children {
                    MoveWindow(
                        *child as HWND,
                        x.saturating_mul(rect.right) / pane.base[0],
                        y.saturating_mul(rect.bottom) / pane.base[1],
                        w.saturating_mul(rect.right) / pane.base[0],
                        h.saturating_mul(rect.bottom) / pane.base[1],
                        1,
                    );
                }
            }
            WM_CONTEXTMENU => {
                let menu = CreatePopupMenu();
                if !menu.is_null() {
                    for (i, label) in ["Dock left", "Dock right", "Float"].into_iter().enumerate() {
                        AppendMenuW(menu, MF_STRING, 33000 + i, wide(label).as_ptr());
                    }
                    let bounds = rect(hwnd);
                    let action = TrackPopupMenu(
                        menu,
                        TPM_RETURNCMD | TPM_NONOTIFY,
                        bounds[0] + 12,
                        bounds[1] + 24,
                        0,
                        hwnd,
                        std::ptr::null(),
                    );
                    DestroyMenu(menu);
                    let dock = match action {
                        33000 => Some(Dock::Left),
                        33001 => Some(Dock::Right),
                        33002 => Some(Dock::Floating),
                        _ => None,
                    };
                    if let Some(dock) = dock {
                        let mut host = lock();
                        host.layout.dock(&pane.id, dock);
                        save(&host);
                    }
                }
                return 0;
            }
            WM_NCDESTROY => {
                RemoveWindowSubclass(hwnd, Some(procedure), 0x033);
                lock().windows.remove(&(hwnd as usize));
                lock().moving.remove(&(hwnd as usize));
                drop(Box::from_raw(data as *mut Pane));
            }
            _ => {}
        }
        DefSubclassProc(hwnd, msg, w, l)
    }
}
