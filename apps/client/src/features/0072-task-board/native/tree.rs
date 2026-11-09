//! Thread-owned native tree; lParam carries only a checked path number, never a Rust pointer.
#![allow(
    unsafe_code,
    reason = "Win32 controls require thread-owned raw handles"
)]
use super::{control, wide};
use oteryn_client_ui::task_board::{TaskBoardPanel, shop_offer_choice, tree_nodes};
use windows_sys::Win32::{
    Foundation::HWND,
    UI::{Controls::*, WindowsAndMessaging::*},
};

pub(super) unsafe fn create(parent: HWND) -> HWND {
    // SAFETY: the parent and its children are created and used on this thread.
    unsafe {
        let classes = INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_TREEVIEW_CLASSES,
        };
        if InitCommonControlsEx(&classes) == 0 {
            return std::ptr::null_mut();
        }
        control(
            parent,
            "SysTreeView32",
            "",
            301,
            [490, 10, 300, 380],
            WS_BORDER | WS_TABSTOP | TVS_HASLINES | TVS_HASBUTTONS | TVS_LINESATROOT,
        )
    }
}
unsafe fn tag(tree: HWND) -> Option<isize> {
    unsafe {
        let handle = SendMessageW(tree, TVM_GETNEXTITEM, TVGN_CARET as usize, 0);
        if handle == 0 {
            return None;
        }
        let mut item = TVITEMW {
            mask: TVIF_PARAM,
            hItem: handle,
            ..Default::default()
        };
        (SendMessageW(tree, TVM_GETITEMW, 0, (&mut item as *mut TVITEMW) as isize) != 0)
            .then_some(item.lParam)
    }
}
pub(super) unsafe fn selected_path(tree: HWND) -> Option<u8> {
    unsafe { u8::try_from(tag(tree)?).ok().filter(|path| *path < 4) }
}
pub(super) unsafe fn refresh(tree: HWND, panel: &TaskBoardPanel) {
    // SAFETY: SendMessage copies each label before its Vec is dropped; only integers are retained.
    unsafe {
        let wanted = tag(tree).unwrap_or(-1);
        SendMessageW(tree, TVM_DELETEITEM, 0, TVI_ROOT);
        let mut handles = Vec::new();
        let mut selected = 0;
        for node in tree_nodes(panel) {
            let mut label = wide(&node.label);
            let value = node.path.map_or(-1, isize::from);
            let insert = TVINSERTSTRUCTW {
                hParent: node
                    .parent
                    .and_then(|i| handles.get(i).copied())
                    .unwrap_or(TVI_ROOT),
                hInsertAfter: TVI_LAST,
                Anonymous: TVINSERTSTRUCTW_0 {
                    item: TVITEMW {
                        mask: TVIF_TEXT | TVIF_PARAM,
                        pszText: label.as_mut_ptr(),
                        lParam: value,
                        ..Default::default()
                    },
                },
            };
            let handle = SendMessageW(
                tree,
                TVM_INSERTITEMW,
                0,
                (&insert as *const TVINSERTSTRUCTW) as isize,
            );
            handles.push(handle);
            if value == wanted && selected == 0 {
                selected = handle;
            }
            if node.path.is_none() {
                SendMessageW(tree, TVM_EXPAND, TVE_EXPAND as usize, handle);
            }
        }
        if selected != 0 {
            SendMessageW(tree, TVM_SELECTITEM, TVGN_CARET as usize, selected);
            SendMessageW(tree, TVM_EXPAND, TVE_EXPAND as usize, selected);
        }
    }
}
pub(super) unsafe fn refresh_shop(shop: HWND, panel: &TaskBoardPanel, ids: &mut Vec<u16>) {
    unsafe {
        let previous = usize::try_from(SendMessageW(shop, CB_GETCURSEL, 0, 0))
            .ok()
            .and_then(|row| ids.get(row).copied());
        SendMessageW(shop, CB_RESETCONTENT, 0, 0);
        ids.clear();
        if let Some(reply) = &panel.weekly {
            for offer in &reply.shop {
                let label = shop_offer_choice(offer, reply.points);
                SendMessageW(shop, CB_ADDSTRING, 0, wide(&label).as_ptr() as isize);
                ids.push(offer.index);
            }
        }
        if let Some(row) = previous.and_then(|id| ids.iter().position(|value| *value == id)) {
            SendMessageW(shop, CB_SETCURSEL, row, 0);
        }
    }
}
