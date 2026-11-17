//! Received lower-floor effects stay within their terrain painter range.
use super::*;
type FloorVisuals = Vec<(i16, Vec<QuadInstance>, u8)>;
pub(super) fn project(play: &PlayView, frames: &[Option<Vec<crate::map::Draw>>], now: Instant) -> Result<FloorVisuals, BatchError> {
    let mut result = Vec::new();
    if !play.preferences().show_effects {
        return Ok(result);
    }
    let floors = play.local.world().draw_floors();
    let visibility = play.preferences().game_window;
    for (cue, frame) in play.visuals.iter().zip(frames) {
        if !visibility.visible(&cue.kind) {
            continue;
        }
        let (floor, source) = match cue.kind {
            PresentationEventKind::MagicEffect {
                position, source, ..
            } => (position.floor, source),
            PresentationEventKind::Missile {
                from, to, source, ..
            } if from.floor == to.floor => (from.floor, source),
            _ => continue,
        };
        if floor == play.floor || !floors.contains(&floor) { continue; }
        let Some(frame) = frame else { continue; };
        let opacity = play.preferences().effect_opacity;
        let alpha = if matches!(cue.kind, PresentationEventKind::MagicEffect { effect_id, .. } if play.local.world().effects().is_boss_area(effect_id)) { play.preferences().boss_area_opacity } else { match source {
            oteryn_session::PresentationSource::Own => opacity[0],
            oteryn_session::PresentationSource::Others => opacity[1],
            oteryn_session::PresentationSource::Creatures => opacity[2],
            oteryn_session::PresentationSource::Global => 100,
        } };
        if alpha == 0 { continue; }
        let shift = i32::from(floor) - i32::from(play.floor);
        let project = |mut at: oteryn_session::ActorPosition| { at.x = at.x.saturating_sub(shift); at.y = at.y.saturating_sub(shift); at };
        let mut batch = SpriteBatch::new();
        match cue.kind {
            PresentationEventKind::MagicEffect { position, .. } => visual::sprites(&mut batch, play.scene.view(), play.scene.atlas(), project(position), frame)?,
            PresentationEventKind::Missile { from,to,.. } => visual::missile(&mut batch, play.scene.view(), play.scene.atlas(), project(from), project(to), now.saturating_duration_since(cue.received).as_secs_f32(), Some(frame))?,
            _ => {}
        }
        if !batch.is_empty() { result.push((floor,batch.instances().to_vec(),alpha)); }
    }
    Ok(result)
}
