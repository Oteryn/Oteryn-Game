//! At most 200 immutable GUI fonts, retained for process lifetime for posted WM_SETFONT messages.
use std::collections::BTreeMap;
use windows_sys::Win32::Graphics::Gdi::{
    CreateFontIndirectW, DEFAULT_GUI_FONT, GetObjectW, GetStockObject, LOGFONTW,
};

#[derive(Default)]
pub(super) struct UiFonts(BTreeMap<u16, usize>);
impl UiFonts {
    pub(super) fn get(&mut self, percentage: u16) -> Option<usize> {
        let percentage = percentage.clamp(1, 200);
        if let Some(font) = self.0.get(&percentage) {
            return Some(*font);
        }
        unsafe {
            let mut description: LOGFONTW = std::mem::zeroed();
            if GetObjectW(
                GetStockObject(DEFAULT_GUI_FONT),
                std::mem::size_of::<LOGFONTW>() as i32,
                &mut description as *mut _ as *mut _,
            ) == 0
            {
                return None;
            }
            description.lfHeight =
                (i64::from(description.lfHeight) * i64::from(percentage) / 100) as i32;
            if description.lfHeight == 0 {
                description.lfHeight = -1;
            }
            description.lfWidth =
                (i64::from(description.lfWidth) * i64::from(percentage) / 100) as i32;
            let font = CreateFontIndirectW(&description);
            if font.is_null() {
                return None;
            }
            self.0.insert(percentage, font as usize);
            Some(font as usize)
        }
    }
}
