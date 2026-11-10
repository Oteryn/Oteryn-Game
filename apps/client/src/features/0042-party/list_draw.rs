//! Owner-drawn Party List rows: portrait, name, health and mana, and nearby skull.
use super::*;
use windows_sys::Win32::Foundation::RECT;
pub(super) unsafe fn draw(actions: &Actions, item: &DRAWITEMSTRUCT) {
    // SAFETY: WM_DRAWITEM provides a live DC and all rows/assets belong to this window thread.
    unsafe {
        if item.CtlID != MEMBERS as u32 {
            return;
        }
        let Some((_, row)) = actions.rows.get(item.itemID as usize) else {
            if let Some((_, name)) = (item.itemID as usize)
                .checked_sub(actions.rows.len())
                .and_then(|index| actions.legacy.get(index))
            {
                fill(item.hDC, item.rcItem, 0x181818);
                let label = wide(name);
                SetTextColor(item.hDC, 0xffffff);
                SetBkMode(item.hDC, TRANSPARENT as i32);
                TextOutW(
                    item.hDC,
                    item.rcItem.left + 50,
                    item.rcItem.top + 2,
                    label.as_ptr(),
                    label.len().saturating_sub(1) as i32,
                );
            }
            return;
        };
        fill(
            item.hDC,
            item.rcItem,
            if item.itemState & ODS_SELECTED != 0 {
                0x484028
            } else {
                0x181818
            },
        );
        let x = item.rcItem.left;
        let y = item.rcItem.top;
        let image = actions
            .portraits
            .borrow_mut()
            .as_mut()
            .and_then(|assets| assets.image(row));
        if let Some(image) = image {
            let mut bitmap: BITMAPINFO = std::mem::zeroed();
            bitmap.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            bitmap.bmiHeader.biWidth = 128;
            bitmap.bmiHeader.biHeight = -128;
            bitmap.bmiHeader.biPlanes = 1;
            bitmap.bmiHeader.biBitCount = 32;
            bitmap.bmiHeader.biCompression = BI_RGB;
            StretchDIBits(
                item.hDC,
                x,
                y,
                48,
                48,
                32,
                32,
                64,
                64,
                image.rgba.as_ptr().cast(),
                &bitmap,
                DIB_RGB_COLORS,
                SRCCOPY,
            );
        }
        let label = format!(
            "{}{}{}",
            if row.summon { "↳ " } else { "" },
            row.name,
            row.skull
                .filter(|s| *s != 0)
                .map_or(String::new(), |s| format!(" [skull {s}]"))
        );
        let label = wide(&label);
        SetBkMode(item.hDC, TRANSPARENT as i32);
        SetTextColor(item.hDC, 0xffffff);
        TextOutW(
            item.hDC,
            x + 50,
            y + 2,
            label.as_ptr(),
            label.len().saturating_sub(1) as i32,
        );
        for (percent, top, colour) in [
            (row.health_percent, y + 24, 0x00aa00),
            (row.mana_percent, y + 34, 0xdd6600),
        ] {
            if let Some(percent) = percent {
                fill(
                    item.hDC,
                    RECT {
                        left: x + 50,
                        top,
                        right: x + 250,
                        bottom: top + 6,
                    },
                    0x404040,
                );
                fill(
                    item.hDC,
                    RECT {
                        left: x + 50,
                        top,
                        right: x + 50 + i32::from(percent) * 2,
                        bottom: top + 6,
                    },
                    colour,
                );
            }
        }
    }
}
unsafe fn fill(dc: HDC, rect: RECT, colour: u32) {
    // SAFETY: the brush is created, used with the live draw DC, and deleted in this call.
    unsafe {
        let brush = CreateSolidBrush(colour);
        FillRect(dc, &rect, brush);
        DeleteObject(brush);
    }
}
