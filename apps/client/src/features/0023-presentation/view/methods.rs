impl PlayView {
    pub(in crate::play) fn receive_presentation(
        &mut self,
        events: &[PresentationEventKind],
        now: Instant,
    ) {
        self.visuals
            .retain(|cue| now.saturating_duration_since(cue.received) < VISUAL_LIFETIME);
        self.visuals.extend(events.iter().map(|&kind| VisualCue {
            kind,
            received: now,
        }));
        let excess = self.visuals.len().saturating_sub(MAX_VISUAL_CUES);
        self.visuals.drain(..excess);
    }

    pub(in crate::play) fn advance_presentation(&mut self, now: Instant) -> Result<(), BatchError> {
        if self.visuals.is_empty() && !self.tutorial_overlay.enabled {
            return Ok(());
        }
        self.visuals
            .retain(|cue| now.saturating_duration_since(cue.received) < VISUAL_LIFETIME);
        self.project_presentation(now)
    }

    pub(in crate::play) fn project_presentation(&mut self, now: Instant) -> Result<(), BatchError> {
        // Palette pixels remain emissive: overlays must be legible in dark areas.
        let atlas = self.local.world();
        let atlas = atlas.atlas();
        let bars = self.preferences().hud_bars;
        let effects = self.preferences().show_effects;
        let health_circle = self.preferences().health_circle;
        let creature_bars = self.preferences().creature_bars;
        let harmony = self.preferences().show_harmony;
        let serene = self.preferences().show_serene;
        let (view, batch) = self.scene.presentation_base()?;
        if bars && let Some(vitals) = &self.vitals {
            for (row, value, max, color) in [
                (
                    0.0,
                    vitals.health,
                    vitals.max_health,
                    PresentationColor::Red,
                ),
                (1.0, vitals.mana, vitals.max_mana, PresentationColor::Blue),
            ] {
                let at = [8.0, 56.0 + row * 14.0];
                pixel(
                    batch,
                    view,
                    atlas,
                    at,
                    [120.0, 10.0],
                    PresentationColor::LightGrey,
                )?;
                pixel(
                    batch,
                    view,
                    atlas,
                    at,
                    [
                        oteryn_client_ui::spell_action_bar::fill_px(value, max),
                        10.0,
                    ],
                    color,
                )?;
            }
        }
        if let Some(vitals) = &self.vitals {
            if harmony {
                for segment in 0..5_u32 {
                    pixel(
                        batch,
                        view,
                        atlas,
                        [8.0 + segment as f32 * 24.0, 86.0],
                        [20.0, 8.0],
                        if segment < vitals.harmony.min(5) {
                            PresentationColor::Yellow
                        } else {
                            PresentationColor::LightGrey
                        },
                    )?;
                }
            }
            if serene {
                pixel(
                    batch,
                    view,
                    atlas,
                    [8.0, 100.0],
                    [120.0, 8.0],
                    if vitals.serene {
                        PresentationColor::LightGreen
                    } else {
                        PresentationColor::LightGrey
                    },
                )?;
            }
        }
        if health_circle && let Some(vitals) = &self.vitals {
            let [x, y] = view.tile_to_screen(self.own);
            let half = view.tile_px() as f32 / 2.0;
            let radius = half + 8.0;
            for (side, value, max, color) in [
                (
                    1.0,
                    vitals.health,
                    vitals.max_health,
                    PresentationColor::Red,
                ),
                (-1.0, vitals.mana, vitals.max_mana, PresentationColor::Blue),
            ] {
                let filled = if max == 0 {
                    0
                } else {
                    (u64::from(value.min(max)) * 16 / u64::from(max)) as u32
                };
                for segment in 0..16_u32 {
                    let angle = (segment as f32 + 0.5) * std::f32::consts::PI / 16.0;
                    pixel(
                        batch,
                        view,
                        atlas,
                        [
                            x + half + side * radius * angle.sin() - 1.5,
                            y + half - radius * angle.cos() - 1.5,
                        ],
                        [3.0, 3.0],
                        if segment < filled {
                            color
                        } else {
                            PresentationColor::LightGrey
                        },
                    )?;
                }
            }
        }
        if creature_bars {
            for entity in &self.entities {
                let oteryn_session::EntityDetail::Actor { health_percent, .. } = entity.detail
                else {
                    continue;
                };
                let tile = TileCoord::new(entity.position.x, entity.position.y);
                if entity.position.floor != self.floor || !view.contains(tile) {
                    continue;
                }
                let [x, y] = view.tile_to_screen(tile);
                let at = [x + 2.0, y + view.tile_px() as f32 - 5.0];
                let width = (view.tile_px() as f32 - 4.0).max(1.0);
                pixel(
                    batch,
                    view,
                    atlas,
                    at,
                    [width, 3.0],
                    PresentationColor::DarkRed,
                )?;
                pixel(
                    batch,
                    view,
                    atlas,
                    at,
                    [width * f32::from(health_percent.min(100)) / 100.0, 3.0],
                    PresentationColor::LightGreen,
                )?;
                if let Some(icon) = self
                    .party_icons
                    .iter()
                    .find(|icon| icon.entity == entity.entity)
                {
                    for (offset, value, pattern, skull) in [
                        (0.0, icon.shield, [5_u8, 7, 2], false),
                        (8.0, icon.skull, [7_u8, 5, 2], true),
                    ] {
                        if value == 0 {
                            continue;
                        }
                        let color = party_icon_rgb(value, skull);
                        for (row, bits) in pattern.into_iter().enumerate() {
                            for col in 0..3 {
                                if bits & (4 >> col) != 0 {
                                    pixel_rgb(
                                        batch,
                                        view,
                                        atlas,
                                        [x + offset + col as f32 * 2.0, y + row as f32 * 2.0],
                                        [2.0, 2.0],
                                        color,
                                    )?;
                                }
                            }
                        }
                    }
                }
            }
        }
        draw_tutorial(batch, view, atlas, &self.tutorial_overlay, now)?;
        if !effects {
            return Ok(());
        }
        let mut lines = std::collections::BTreeMap::new();
        for cue in &self.visuals {
            let age = now.saturating_duration_since(cue.received);
            if age >= VISUAL_LIFETIME {
                continue;
            }
            let progress = age.as_secs_f32() / VISUAL_LIFETIME.as_secs_f32();
            match cue.kind {
                PresentationEventKind::ValueText {
                    position,
                    primary,
                    secondary,
                    ..
                } if position.floor == self.floor => {
                    let tile = TileCoord::new(position.x, position.y);
                    if !view.contains(tile) {
                        continue;
                    }
                    let [x, y] = view.tile_to_screen(tile);
                    let center = x + view.tile_px() as f32 / 2.0;
                    let line = lines.entry((position.x, position.y)).or_insert(0_u32);
                    let top = y - progress * 32.0 - 12.0 * *line as f32;
                    *line += 1 + u32::from(secondary.is_some());
                    number(batch, view, atlas, [center, top], primary)?;
                    if let Some(value) = secondary {
                        number(batch, view, atlas, [center, top - 12.0], value)?;
                    }
                }
                PresentationEventKind::MagicEffect {
                    position,
                    effect_id,
                    ..
                } if position.floor == self.floor && progress < 0.5 => {
                    let tile = TileCoord::new(position.x, position.y);
                    if !view.contains(tile) {
                        continue;
                    }
                    let [x, y] = view.tile_to_screen(tile);
                    let half = view.tile_px() as f32 / 2.0;
                    let radius = 4.0 + progress * 24.0;
                    // Crystal utils_definitions.hpp: FIREWORK_YELLOW/RED/BLUE = 29/30/31.
                    let color = match effect_id {
                        1 | 14 | 30 => PresentationColor::Red,
                        7 => PresentationColor::Orange,
                        16 => PresentationColor::PastelRed,
                        24 => PresentationColor::SkyBlue,
                        27 => PresentationColor::White,
                        28 => PresentationColor::LightGreen,
                        31 => PresentationColor::Blue,
                        _ => PresentationColor::Yellow,
                    };
                    if matches!(effect_id, 29..=31) {
                        for [dx, dy] in [
                            [-1.0, 0.0],
                            [1.0, 0.0],
                            [0.0, -1.0],
                            [0.0, 1.0],
                            [-0.7, -0.7],
                            [-0.7, 0.7],
                            [0.7, -0.7],
                            [0.7, 0.7],
                        ] {
                            pixel(
                                batch,
                                view,
                                atlas,
                                [x + half + dx * radius, y + half + dy * radius],
                                [3.0, 3.0],
                                color,
                            )?;
                        }
                        continue;
                    }
                    for [dx, dy, w, h] in [
                        [-radius, -radius, radius * 2.0, 2.0],
                        [-radius, radius, radius * 2.0, 2.0],
                        [-radius, -radius, 2.0, radius * 2.0],
                        [radius, -radius, 2.0, radius * 2.0],
                    ] {
                        pixel(
                            batch,
                            view,
                            atlas,
                            [x + half + dx, y + half + dy],
                            [w, h],
                            color,
                        )?;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}
