//! Reorder display rows by identity; labels come from the same catalogues as the panels.
use super::*;
use oteryn_client::play::ReferencePreferences;

pub(super) struct OrderControls {
    labels: [HWND; 2],
    lists: [HWND; 2],
    buttons: [HWND; 4],
}
impl OrderControls {
    pub(super) unsafe fn new(hwnd: HWND) -> Option<Self> {
        unsafe {
            let labels = ["Skills: select then move", "Conditions: select then move"]
                .map(|label| control(hwnd, "STATIC", label, 0, [10, 220, 330, 22], 0));
            MoveWindow(labels[1], 380, 220, 330, 22, 1);
            let lists = [10, 380].map(|x| {
                control(
                    hwnd,
                    "LISTBOX",
                    "",
                    0,
                    [x, 245, 260, 120],
                    WS_BORDER | WS_VSCROLL | LBS_NOTIFY as u32,
                )
            });
            let buttons = std::array::from_fn(|index| {
                control(
                    hwnd,
                    "BUTTON",
                    if index % 2 == 0 { "Up" } else { "Down" },
                    241 + index,
                    [
                        280 + (index / 2) as i32 * 370,
                        245 + (index % 2) as i32 * 40,
                        65,
                        30,
                    ],
                    0,
                )
            });
            if labels
                .iter()
                .chain(lists.iter())
                .chain(buttons.iter())
                .any(|h| h.is_null())
            {
                return None;
            }
            Some(Self {
                labels,
                lists,
                buttons,
            })
        }
    }
    pub(super) unsafe fn show(&self, visible: bool) {
        unsafe {
            for handle in self
                .labels
                .iter()
                .chain(self.lists.iter())
                .chain(self.buttons.iter())
            {
                ShowWindow(*handle, if visible { SW_SHOW } else { SW_HIDE });
            }
        }
    }
    pub(super) unsafe fn reorder(
        &self,
        action: usize,
        preferences: &mut ReferencePreferences,
    ) -> bool {
        unsafe {
            let Some(button) = action.checked_sub(241).filter(|value| *value < 4) else {
                return false;
            };
            let list = self.lists[button / 2];
            let Ok(index) = usize::try_from(SendMessageW(list, LB_GETCURSEL, 0, 0)) else {
                return false;
            };
            let down = button % 2 == 1;
            if !preferences.move_status_entry(button < 2, index, down) {
                return false;
            }
            SendMessageW(
                list,
                LB_SETCURSEL,
                if down { index + 1 } else { index - 1 },
                0,
            );
            true
        }
    }
    pub(super) unsafe fn refresh(&self, preferences: &ReferencePreferences) {
        unsafe {
            for (list, skills) in self.lists.into_iter().zip([true, false]) {
                let selected = SendMessageW(list, LB_GETCURSEL, 0, 0).max(0) as usize;
                SendMessageW(list, LB_RESETCONTENT, 0, 0);
                let order = if skills {
                    &preferences.skill_order
                } else {
                    &preferences.condition_order
                };
                let default: Vec<u8> = if skills {
                    (0..8).collect()
                } else {
                    (1..=27).collect()
                };
                for id in if order.is_empty() { &default } else { order } {
                    let label = if skills {
                        oteryn_client_ui::skill_progress::SKILL_LABELS[usize::from(*id)]
                    } else {
                        oteryn_client_ui::condition_status::icon_label(*id).1
                    };
                    SendMessageW(list, LB_ADDSTRING, 0, wide(label).as_ptr() as isize);
                }
                SendMessageW(list, LB_SETCURSEL, selected, 0);
            }
        }
    }
}
