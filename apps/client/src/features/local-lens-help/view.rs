//! Lens help: a mode in which a click on a part of the game window reads its help text.
use crate::play::{
    HudPanel, PlayLink, PlayView,
    mailbox::lock,
};
use oteryn_client_ui::lens_help;

impl PlayView {
    /// The catalog key of the element under a window point, if the lens knows it: a HUD pane,
    /// the minimap, a creature or the game window itself. `root` is the window's client size.
    #[must_use]
    pub fn lens_key(&self, point: [f32; 2], root: [u16; 2]) -> Option<&'static str> {
        let [x, y] = point;
        let hit = |[left, top, width, height]: [f32; 4]| {
            x >= left && y >= top && x < left + width && y < top + height
        };
        let preferences = self.preferences();
        let layout = preferences.effective_hud_layout();
        for (panel, key) in [
            (HudPanel::Resources, "resources"),
            (HudPanel::Combat, "combat"),
            (HudPanel::Conditions, "conditions"),
            (HudPanel::Monk, "monk"),
        ] {
            if layout
                .rect(panel, root, preferences.interface_scale)
                .is_some_and(|rect| hit(rect.map(f32::from)))
            {
                return Some(key);
            }
        }
        if self.live_map
            && preferences.panel_visible("minimap")
            && hit([
                0.0,
                0.0,
                (oteryn_client_ui::minimap::COLUMNS * oteryn_client_ui::minimap::TILE_PX) as f32,
                (oteryn_client_ui::minimap::ROWS * oteryn_client_ui::minimap::TILE_PX) as f32,
            ])
        {
            return Some("minimap");
        }
        let tile = self.scene().view().screen_to_tile(x, y)?;
        let creature = self.entities.iter().any(|entity| {
            entity.kind == oteryn_session::EntityKind::Creature
                && entity.position.floor == self.floor
                && entity.position.x == tile.x
                && entity.position.y == tile.y
        });
        Some(if creature { "creature" } else { "world" })
    }
}

impl PlayLink {
    /// Switch the lens mode and return whether it is on.
    pub fn lens_toggle(&self) -> bool {
        lock(&self.mailbox).lens.toggle()
    }
    #[must_use]
    pub fn lens_active(&self) -> bool {
        lock(&self.mailbox).lens.active
    }
    /// A click in lens mode: show the help of the element (or that nothing is known there) and
    /// leave the mode. The text goes to the console feedback line.
    pub fn lens_click(&self, key: Option<&str>) {
        {
            let mut mailbox = lock(&self.mailbox);
            mailbox.lens.finish();
            let text = key
                .and_then(lens_help::help_text)
                .unwrap_or_else(|| lens_help::mode_text(false));
            mailbox.chat.feedback = Some(text.into());
        }
        self.open_chat();
    }
}
