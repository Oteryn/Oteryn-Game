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
        let (frames, changed) = if self.preferences().show_effects {
            self.scene.visual_frames(&self.visuals, now, self.floor)?
        } else {
            (Vec::new(), false)
        };
        if changed {
            self.weather_scene();
            self.light_scene()?;
        }
        let resource_options = resources::Options::from(self.preferences());
        let effects = self.preferences().show_effects;
        let own_ref = self.scene.own_identity().and_then(|identity| {
            self.appearances
                .iter()
                .find(|entry| entry.entity.entity.identity == identity)
                .map(|entry| entry.entity.entity)
        });
        let age = now.saturating_duration_since(self.party_icon_epoch);
        let actor_options = actor_hud::Options::new(self.preferences(), self.floor, own_ref, age);
        let (view, atlas, batch) = self.scene.presentation_base()?;
        resources::draw(
            batch,
            view,
            atlas,
            self.vitals.as_ref(),
            self.own,
            resource_options,
        )?;
        actor_hud::draw(
            batch,
            view,
            atlas,
            self.own,
            &self.entities,
            &self.party_icons,
            actor_options,
        )?;
        draw_tutorial(batch, view, atlas, &self.tutorial_overlay, now)?;
        if !effects {
            return Ok(());
        }
        let mut lines = std::collections::BTreeMap::new();
        for (index, cue) in self.visuals.iter().enumerate() {
            let age = now.saturating_duration_since(cue.received);
            if age >= VISUAL_LIFETIME {
                continue;
            }
            let progress = age.as_secs_f32();
            let frame = frames.get(index).and_then(Option::as_deref);
            match cue.kind {
                PresentationEventKind::ValueText {
                    position,
                    primary,
                    secondary,
                    ..
                } if position.floor == self.floor && progress < 1.0 => {
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
                } if position.floor == self.floor => {
                    if let Some(draws) = frame {
                        visual::sprites(batch, view, atlas, position, draws)?;
                        continue;
                    }
                    if progress >= 0.5 {
                        continue;
                    }
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
                PresentationEventKind::SpellWords {
                    position, spell, ..
                } if position.floor == self.floor && progress < 1.0 => {
                    if let Some(words) = self.spell_words.get(&spell) {
                        visual::words(batch, view, atlas, position, words, progress)?;
                    }
                }
                PresentationEventKind::Missile { from, to, .. }
                    if from.floor == self.floor && to.floor == self.floor =>
                {
                    visual::missile(batch, view, atlas, from, to, progress, frame)?;
                }
                _ => {}
            }
        }
        Ok(())
    }
}
