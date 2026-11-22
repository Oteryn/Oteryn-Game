//! Exact-colour Bevy bars over the same confirmed actors as the retained scene.
use crate::play::PlayView;
use oteryn_renderer::TileCoord;
use oteryn_session::EntityDetail;

pub(crate) struct CreatureHealthBar {
    pub at: [f32; 2],
    pub width: f32,
    pub health: u8,
}

impl PlayView {
    pub(crate) fn creature_health_bars(&self) -> Vec<CreatureHealthBar> {
        let mut bars = Vec::new();
        let view = self.scene.view();
        let scale = f32::from(self.preferences().interface_scale) / 100.;
        let mut push = |tile: TileCoord, health: u8, offset: [f32; 2]| {
            if view.contains(tile) {
                let [x, y] = view.tile_to_screen(tile);
                bars.push(CreatureHealthBar {
                    at: [
                        x + offset[0] + 2. * scale,
                        (y + offset[1] - 4. * scale).max(10. * scale),
                    ],
                    width: (view.tile_px() as f32 - 4. * scale).max(1.),
                    health: health.min(100),
                });
            }
        };
        let own_identity = self.scene.own_identity();
        let mut own_shown = false;
        for entity in
            self.entities
                .iter()
                .chain(
                    self.appearances
                        .iter()
                        .map(|entry| &entry.entity)
                        .filter(|entity| {
                            !self
                                .entities
                                .iter()
                                .any(|held| held.entity == entity.entity)
                        }),
                )
        {
            let (EntityDetail::Actor { health_percent, .. }
            | EntityDetail::ActorWithIcons { health_percent, .. }) = entity.detail
            else {
                continue;
            };
            let is_own = own_identity == Some(entity.entity.identity);
            if is_own {
                push(self.own, health_percent, [0.; 2]);
                own_shown = true;
            } else if entity.position.floor == self.floor {
                push(
                    TileCoord::new(entity.position.x, entity.position.y),
                    health_percent,
                    {
                        let [dx, dy] = self.scene.actor_offset(entity.entity);
                        let [cx, cy] = self
                            .camera_offset(std::time::Instant::now())
                            .unwrap_or([0.; 2]);
                        [dx + cx, dy + cy]
                    },
                );
            }
        }
        if !own_shown && let Some(vitals) = self.vitals.as_ref().filter(|v| v.max_health > 0) {
            push(
                self.own,
                (u64::from(vitals.health.min(vitals.max_health)) * 100
                    / u64::from(vitals.max_health)) as u8,
                [0.; 2],
            );
        }
        bars
    }
}
