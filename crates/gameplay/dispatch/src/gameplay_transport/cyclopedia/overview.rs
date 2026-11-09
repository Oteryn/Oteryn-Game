//! T079: overview cells and public markers from the same admitted World as exploration.
use super::*;
use wire::atlas::{Area, Atlas, HEIGHT, Marker, Query, WIDTH};
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn cyclopedia_atlas_wire(
        &self,
        admitted: AdmittedSession,
        id: u64,
        payload: &[u8],
    ) -> GuildReply {
        let Ok(mut query) = Query::decode(payload) else {
            return GuildReply::Rejected;
        };
        let Some(fence) = admitted.item_fence else {
            return GuildReply::Rejected;
        };
        if !admitted
            .continuity
            .selected_capabilities
            .contains(wire::CAPABILITY_CYCLOPEDIA_MAP_V1)
        {
            return GuildReply::Rejected;
        }
        let Some(current) =
            super::super::character_gameplay_fence::current(self, admitted.game_session_id, false)
                .await
        else {
            return GuildReply::Rejected;
        };
        if current.item_fence() != fence
            || fence.game_session_id != admitted.game_session_id
            || fence.connection_generation.get() != admitted.continuity.connection_generation
        {
            return GuildReply::Rejected;
        }
        let progress = match self.cyclopedia_basic_wire(admitted, id, 72, &[]).await {
            GuildReply::Payload(bytes) => match wire::decode_result(&bytes) {
                Ok(value) => Some(value),
                Err(_) => return GuildReply::Rejected,
            },
            GuildReply::Rejected => None,
            GuildReply::Unavailable => return GuildReply::Unavailable,
        };
        let Some(actor) = admitted.runtime_actor else {
            return GuildReply::Rejected;
        };
        let mut runtime = self.runtime.lock().await;
        let binding = runtime.binding();
        if admitted.world_id != binding.world_id()
            || Some(admitted.channel_id) != binding.channel_id()
            || fence.runtime_scope != binding.runtime_scope()
            || fence.scope_ownership_generation != binding.scope_generation()
            || runtime.owner_fence().is_err()
            || !runtime
                .player_control_facts(actor, admitted.game_session_id)
                .is_ok_and(|f| f.control_loss.is_none())
        {
            return GuildReply::Rejected;
        }
        let Some(position) =
            super::super::bundle_map_position(&mut runtime, self.movement_cells(), actor)
        else {
            return GuildReply::Rejected;
        };
        let npcs: Vec<_> = runtime
            .npc_atlas_positions()
            .map(|(key, at)| (key.to_owned(), at))
            .collect();
        drop(runtime);
        let base = position.map.overlay.base();
        let step = 1u32 << query.zoom;
        if query.floor == 127 {
            let Ok(floor) = i8::try_from(position.at.floor) else {
                return GuildReply::Rejected;
            };
            query.floor = floor;
        }
        if query.x == u16::MAX && query.y == u16::MAX {
            let (Ok(x), Ok(y)) = (u16::try_from(position.at.x), u16::try_from(position.at.y))
            else {
                return GuildReply::Rejected;
            };
            query.x = x.saturating_sub((WIDTH as u32 * step / 2).min(65535) as u16);
            query.y = y.saturating_sub((HEIGHT as u32 * step / 2).min(65535) as u16);
        }
        let floors = base.tiles().fold(0u16, |mask, (floor, _, _, _)| {
            mask | (1u16 << (-floor as u8))
        });
        let mut cells = vec![0; WIDTH * HEIGHT];
        let mut ids = std::collections::BTreeSet::new();
        for run in base
            .subareas
            .runs
            .iter()
            .filter(|run| run.floor == query.floor)
        {
            let Some(row) = u32::from(run.y)
                .checked_sub(u32::from(query.y))
                .map(|v| v / step)
                .filter(|r| *r < HEIGHT as u32)
            else {
                continue;
            };
            let left = u32::from(run.begin).max(u32::from(query.x));
            let right = u32::from(run.end).min(u32::from(query.x) + WIDTH as u32 * step);
            if left >= right {
                continue;
            }
            ids.insert(run.subarea);
            for col in (left - u32::from(query.x)) / step..=(right - 1 - u32::from(query.x)) / step
            {
                let cell = &mut cells[row as usize * WIDTH + col as usize];
                if *cell == 0 {
                    *cell = run.subarea;
                }
            }
        }
        let mut areas = Vec::new();
        for a in &base.subareas.areas {
            if ids.contains(&a.id)
                || base
                    .subareas
                    .areas
                    .iter()
                    .any(|child| ids.contains(&child.id) && child.parent == Some(a.id))
            {
                areas.push(Area {
                    id: a.id,
                    parent: a.parent.unwrap_or(0),
                    name: label(&a.name),
                });
            }
        }
        if areas.len() > 64 {
            return GuildReply::Rejected;
        }
        let inside = |x: u16, y: u16| {
            u32::from(x) >= u32::from(query.x)
                && u32::from(y) >= u32::from(query.y)
                && u32::from(x) < u32::from(query.x) + WIDTH as u32 * step
                && u32::from(y) < u32::from(query.y) + HEIGHT as u32 * step
        };
        let mut markers = Vec::new();
        for (key, at) in npcs {
            if -at.floor == i16::from(query.floor)
                && let (Ok(x), Ok(y)) = (u16::try_from(at.x), u16::try_from(at.y))
                && inside(x, y)
            {
                markers.push(Marker {
                    kind: 1,
                    x,
                    y,
                    label: label(&key),
                });
            }
        }
        for (floor, x, y, _) in base
            .tiles()
            .filter(|(floor, x, y, _)| *floor == query.floor && inside(*x, *y))
        {
            let pos = crate::map::overlay::TilePos { x, y, floor };
            if position.map.floors.contains(pos) || position.map.floors.teleport(pos).is_some() {
                markers.push(Marker {
                    kind: 2,
                    x,
                    y,
                    label: if let Some(to) = position.map.floors.teleport(pos) {
                        format!("Passage to {},{},{}", to.x, to.y, to.floor)
                    } else {
                        "Floor passage".into()
                    },
                });
            }
        }
        for &(x, y, floor) in progress.iter().flat_map(|p| &p.visible) {
            if floor == query.floor && inside(x, y) {
                markers.push(Marker {
                    kind: 3,
                    x,
                    y,
                    label: "Visible exploration point".into(),
                });
            }
        }
        markers.sort_by(|a, b| (a.kind, a.y, a.x, &a.label).cmp(&(b.kind, b.y, b.x, &b.label)));
        let Ok(marker_total) = u32::try_from(markers.len()) else {
            return GuildReply::Rejected;
        };
        let markers = markers
            .into_iter()
            .skip(usize::from(query.page) * 64)
            .take(64)
            .collect();
        let atlas = Atlas {
            query,
            digest: base.digest(),
            floors,
            progress,
            cells,
            areas,
            markers,
            marker_total,
        };
        match atlas.encode() {
            Ok(bytes) => GuildReply::Payload(bytes),
            Err(_) => GuildReply::Rejected,
        }
    }
}
fn label(s: &str) -> String {
    let mut end = s.len().min(128);
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    s[..end].chars().filter(|c| !c.is_control()).collect()
}
