//! Logical UI rectangles are independent of world zoom and fitted to the monitor work area.
pub const FIXED_PIXEL_FONT_PROPERTY: &str = "Oteryn.FixedPixelFont";

pub fn scale_ui_rect(rect: [i32; 4], percentage: u16) -> [i32; 4] {
    rescale_ui_rect(rect, 100, percentage.clamp(1, 200))
}

pub fn rescale_ui_rect(rect: [i32; 4], from: u16, to: u16) -> [i32; 4] {
    rect.map(|value| {
        (i64::from(value) * i64::from(to) / i64::from(from.max(1)))
            .clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
    })
}

pub fn fit_ui_scale(requested: u16, size: [i32; 2], available: [i32; 2]) -> u16 {
    if size.into_iter().chain(available).any(|value| value <= 0) {
        return requested.clamp(1, 200);
    }
    let fit = (i64::from(available[0]) * 100 / i64::from(size[0]))
        .min(i64::from(available[1]) * 100 / i64::from(size[1]));
    requested.clamp(1, 200).min(fit.clamp(1, 200) as u16)
}
