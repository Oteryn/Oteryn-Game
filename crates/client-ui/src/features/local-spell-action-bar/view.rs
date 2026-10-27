use oteryn_client_runtime::spell::ActorVitals;
use oteryn_renderer::{AtlasImage, BatchError, QuadInstance};

pub const BAR_WIDTH_PX: f32 = 120.0;
pub const BAR_HEIGHT_PX: f32 = 10.0;
const BAR_GAP_PX: f32 = 4.0;

/// Fill width in pixels: `value / max` clamped to the bar, zero when `max` is zero.
#[must_use]
pub fn fill_px(value: u32, max: u32) -> f32 {
    if max == 0 {
        return 0.0;
    }
    // Vitals are small; f32 precision is ample for a 120 px bar.
    #[allow(clippy::cast_precision_loss)]
    let ratio = (value.min(max) as f32) / (max as f32);
    ratio * BAR_WIDTH_PX
}

/// Four quads at `origin` (pixels): health track and fill, then mana track and fill, drawn from
/// solid atlas cells (`track`, `health`, `mana`). Nothing without vitals.
pub fn vitals_bars(
    atlas: &AtlasImage,
    vitals: Option<&ActorVitals>,
    origin: [f32; 2],
    (track, health, mana): (u16, u16, u16),
) -> Result<Vec<QuadInstance>, BatchError> {
    let Some(vitals) = vitals else {
        return Ok(Vec::new());
    };
    let mut quads = Vec::with_capacity(4);
    for (row, (value, max, cell)) in [
        (vitals.health, vitals.max_health, health),
        (vitals.mana, vitals.max_mana, mana),
    ]
    .into_iter()
    .enumerate()
    {
        #[allow(clippy::cast_precision_loss)]
        let y = origin[1] + (row as f32) * (BAR_HEIGHT_PX + BAR_GAP_PX);
        quads.push(QuadInstance {
            position: [origin[0], y],
            size: [BAR_WIDTH_PX, BAR_HEIGHT_PX],
            uv: atlas.uv_rect(track)?,
        });
        quads.push(QuadInstance {
            position: [origin[0], y],
            size: [fill_px(value, max), BAR_HEIGHT_PX],
            uv: atlas.uv_rect(cell)?,
        });
    }
    Ok(quads)
}

#[cfg(test)]
#[path = "tests/view.rs"]
mod tests;
