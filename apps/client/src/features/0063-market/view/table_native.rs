//! Report tables: all HWNDs, text buffers and callbacks stay on the creating thread.
use super::*;
use windows_sys::Win32::UI::Controls::*;
pub(super) struct Table(pub(super) HWND);
impl Table {
    pub(super) unsafe fn create(
        parent: HWND,
        id: usize,
        b: [i32; 4],
        columns: &[(&str, i32)],
    ) -> Option<Self> {
        unsafe {
            let init = INITCOMMONCONTROLSEX {
                dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
                dwICC: ICC_LISTVIEW_CLASSES,
            };
            InitCommonControlsEx(&init);
            let hwnd = control(
                parent,
                "SysListView32",
                "",
                id,
                b,
                WS_BORDER | WS_TABSTOP | LVS_REPORT | LVS_SINGLESEL | LVS_SHOWSELALWAYS,
            );
            if hwnd.is_null() {
                return None;
            }
            SendMessageW(
                hwnd,
                LVM_SETEXTENDEDLISTVIEWSTYLE,
                0,
                (LVS_EX_FULLROWSELECT | LVS_EX_GRIDLINES | LVS_EX_DOUBLEBUFFER) as isize,
            );
            for (i, (label, width)) in columns.iter().enumerate() {
                let mut text = wide(label);
                let mut column: LVCOLUMNW = std::mem::zeroed();
                column.mask = LVCF_TEXT | LVCF_WIDTH;
                column.cx = *width;
                column.pszText = text.as_mut_ptr();
                SendMessageW(
                    hwnd,
                    LVM_INSERTCOLUMNW,
                    i,
                    (&column as *const LVCOLUMNW) as isize,
                );
            }
            Some(Self(hwnd))
        }
    }
    pub(super) unsafe fn rows(&self, rows: &[Vec<String>]) {
        unsafe {
            SendMessageW(self.0, LVM_DELETEALLITEMS, 0, 0);
            for (i, fields) in rows.iter().enumerate() {
                for (j, text) in fields.iter().enumerate() {
                    let mut text = wide(text);
                    let mut item: LVITEMW = std::mem::zeroed();
                    item.mask = LVIF_TEXT;
                    item.iItem = i as i32;
                    item.iSubItem = j as i32;
                    item.pszText = text.as_mut_ptr();
                    SendMessageW(
                        self.0,
                        if j == 0 {
                            LVM_INSERTITEMW
                        } else {
                            LVM_SETITEMW
                        },
                        0,
                        (&item as *const LVITEMW) as isize,
                    );
                }
            }
        }
    }
    pub(super) unsafe fn selection(&self) -> Option<usize> {
        let row =
            unsafe { SendMessageW(self.0, LVM_GETNEXTITEM, usize::MAX, LVNI_SELECTED as isize) };
        usize::try_from(row).ok()
    }
    pub(super) unsafe fn select(&self, index: usize) {
        unsafe {
            let mut item: LVITEMW = std::mem::zeroed();
            item.stateMask = LVIS_SELECTED;
            item.state = LVIS_SELECTED;
            SendMessageW(
                self.0,
                LVM_SETITEMSTATE,
                index,
                (&item as *const LVITEMW) as isize,
            );
        }
    }
}
pub(super) unsafe fn selection_notification(l: LPARAM) -> Option<usize> {
    if l == 0 {
        return None;
    }
    unsafe {
        let header = &*(l as *const NMHDR);
        if header.code == LVN_ITEMCHANGED {
            let change = &*(l as *const NMLISTVIEW);
            if change.uNewState & LVIS_SELECTED != 0 && change.uOldState & LVIS_SELECTED == 0 {
                return Some(header.idFrom);
            }
        }
    }
    None
}
pub(super) struct Offers {
    pub(super) buy: Table,
    pub(super) sell: Table,
    shown: [Vec<([u8; 16], u32)>; 2],
}
impl Offers {
    pub(super) unsafe fn create(parent: HWND) -> Option<Self> {
        unsafe {
            let columns = [
                ("Item", 70),
                ("Amount", 65),
                ("Unit price", 85),
                ("Total", 100),
                ("State", 65),
                ("Visibility", 100),
            ];
            control(parent, "STATIC", "Buy offers", 0, [10, 80, 335, 22], 0);
            control(parent, "STATIC", "Sell offers", 0, [355, 80, 335, 22], 0);
            Some(Self {
                buy: Table::create(parent, 1, [10, 105, 335, 260], &columns)?,
                sell: Table::create(parent, 14, [355, 105, 335, 260], &columns)?,
                shown: [Vec::new(), Vec::new()],
            })
        }
    }
    pub(super) unsafe fn selected<'a>(
        &self,
        rows: &'a [oteryn_client_runtime::market::market::MarketRow],
        id: usize,
    ) -> Option<&'a oteryn_client_runtime::market::market::MarketRow> {
        unsafe {
            let (table, side) = if id == 14 {
                (&self.sell, 2)
            } else {
                (&self.buy, 1)
            };
            oteryn_client_ui::market::table_selection(
                rows,
                &self.shown[(side - 1) as usize],
                table.selection()?,
            )
        }
    }
    pub(super) unsafe fn refresh(&mut self, panel: &oteryn_client_ui::market::MarketPanel) {
        unsafe {
            for (table, side) in [(&self.buy, 1), (&self.sell, 2)] {
                let rows: Vec<_> = panel
                    .rows
                    .iter()
                    .filter(|r| {
                        panel.side_filter == 0 || r.side == 0 || r.side == panel.side_filter
                    })
                    .filter(|r| r.side == side || (side == 1 && r.side == 0))
                    .collect();
                let fields: Vec<_> = rows
                    .iter()
                    .map(|r| {
                        vec![
                            r.definition.to_string(),
                            r.amount.to_string(),
                            r.price.to_string(),
                            (u128::from(r.amount) * u128::from(r.price)).to_string(),
                            match r.state {
                                1 => "Open",
                                2 => "Ended",
                                5 => "Credit held",
                                _ => "Catalogue",
                            }
                            .into(),
                            if r.anonymous { "Anonymous" } else { "Public" }.into(),
                        ]
                    })
                    .collect();
                self.shown[(side - 1) as usize] =
                    rows.iter().map(|r| (r.offer, r.definition)).collect();
                table.rows(&fields);
                if let Some(index) = rows
                    .iter()
                    .position(|r| Some(r.offer) == panel.selected_offer)
                {
                    table.select(index);
                }
            }
        }
    }
}
