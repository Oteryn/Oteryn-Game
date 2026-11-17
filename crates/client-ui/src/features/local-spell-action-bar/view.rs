use oteryn_client_runtime::spell::ActorVitals;
use oteryn_renderer::{AtlasImage, BatchError, QuadInstance};

/// oteryn-choice: show the server's reported remaining cooldown, without claiming local cast authority.
#[must_use]
pub fn panel_text(
    panel: &super::ActionBarPanel,
    catalog: &oteryn_client_runtime::spell_catalog::CatalogState,
) -> String {
    use std::fmt::Write;
    let mut text = String::from(tr!(
        "Assign server cast indices (empty clears a slot). Ctrl+Shift+U: catalogue\r\n"
    ));
    for (slot, field) in panel.bar.fields().iter().enumerate() {
        let entry = field.parse::<u32>().ok().and_then(|id| {
            catalog
                .entries()
                .iter()
                .find(|entry| entry.cast_index == id)
        });
        let remaining = field.parse().ok().map_or(0, |spell| {
            panel.bar.remaining_ms(
                spell,
                entry.map_or(&[], |entry| entry.cooldown_groups.as_slice()),
            )
        });
        let _ = lwrite!(
            text,
            "{}: {} {} | Server cooldown: {} ms\r\n",
            slot + 1,
            field,
            entry.map_or("", |entry| entry.name.as_str()),
            if panel.bar.cooldowns.is_some() {
                remaining.to_string()
            } else {
                tr!("unknown").into()
            }
        );
    }
    text.push_str(&panel.feedback);
    text
}

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

/// Compact shell HUD beside the spell hotkeys. Unknown vitals never imply a full bar.
#[must_use]
pub fn vitals_text(vitals: Option<&ActorVitals>) -> String {
    let Some(vitals) = vitals else {
        return tr!("HP —   MP —").to_owned();
    };
    fn bar(value: u32, max: u32) -> String {
        let filled = if max == 0 {
            0
        } else {
            u64::from(value.min(max)) * 10 / u64::from(max)
        };
        (0..10)
            .map(|i| if i < filled { '█' } else { '░' })
            .collect()
    }
    lfmt!(
        "HP [{}] {}/{}   MP [{}] {}/{}   Soul: {}   Harmony: {}/5   Serene: {}",
        bar(vitals.health, vitals.max_health),
        vitals.health,
        vitals.max_health,
        bar(vitals.mana, vitals.max_mana),
        vitals.mana,
        vitals.max_mana,
        vitals.soul,
        vitals.harmony,
        if vitals.serene {
            tr!("active")
        } else {
            tr!("inactive")
        }
    )
}

/// Labels for the docked row. Cooldown knowledge always comes from the server snapshot.
#[must_use]
pub fn slot_labels(
    panel: &super::ActionBarPanel,
    catalog: &oteryn_client_runtime::spell_catalog::CatalogState,
) -> Vec<String> {
    let elapsed = panel.cooldown_received_at.map_or(0, |received| {
        u64::try_from(received.elapsed().as_millis()).unwrap_or(u64::MAX)
    });
    slot_labels_after(panel, catalog, elapsed)
}
fn slot_labels_after(
    panel: &super::ActionBarPanel,
    catalog: &oteryn_client_runtime::spell_catalog::CatalogState,
    elapsed: u64,
) -> Vec<String> {
    panel
        .bar
        .fields()
        .iter()
        .enumerate()
        .map(|(slot, field)| {
            let spell = field.parse().ok();
            let entry = catalog
                .entries()
                .iter()
                .find(|entry| Some(entry.cast_index) == spell);
            let cooldown = spell.and_then(std::num::NonZeroU32::new).map(|spell| {
                panel
                    .bar
                    .remaining_ms(
                        spell,
                        entry.map_or(&[], |entry| entry.cooldown_groups.as_slice()),
                    )
                    .saturating_sub(elapsed)
            });
            let state = match (panel.bar.cooldowns.as_ref(), cooldown) {
                (_, None) => "Empty".into(),
                (None, _) => "Cooldown ?".into(),
                (Some(_), Some(0)) => "Ready".into(),
                (Some(_), Some(ms)) => format!("{ms} ms"),
            };
            format!(
                "{}: {}\n{state}",
                slot + 1,
                entry.map_or(field.as_str(), |entry| entry.name.as_str())
            )
        })
        .collect()
}

#[path = "view/cooldown.rs"]
mod cooldown;
pub use cooldown::{CooldownIcon, cooldown_icons};
