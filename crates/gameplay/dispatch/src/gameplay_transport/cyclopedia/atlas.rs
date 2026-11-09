//! Static atlas facts from the actual admitted bundle; exploration POIs stay on the legacy query.
use super::*;
use oteryn_protocol_oteryn::cyclopedia::atlas::{
    self as wire, Area, AreaPage, GeometryPage, Reply, Request, Run,
};
use std::collections::HashSet;
pub(super) async fn dispatch(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    payload: &[u8],
) -> GuildReply {
    let Ok(request) = wire::decode_request(payload) else {
        return GuildReply::Rejected;
    };
    let runtime = owner.runtime.lock().await;
    if !super::super::battle_instinct_status::actor_is_current(&runtime, admitted)
        || super::super::item_ref_admission::pinned_index(&runtime, owner.active_generation)
            .is_none()
    {
        return GuildReply::Rejected;
    }
    drop(runtime);
    let Some(map) = owner.bundle_map() else {
        return GuildReply::Rejected;
    };
    project(&map.overlay.base().subareas, request)
        .and_then(|reply| wire::encode_reply(&reply).ok())
        .map_or(GuildReply::Rejected, GuildReply::Payload)
}
fn project(table: &oteryn_world_bundle::subarea::Table, request: Request) -> Option<Reply> {
    wire::encode_request(request).ok()?;
    if request.floor == 1 {
        if request.area != 0
            && !table
                .areas
                .iter()
                .any(|area| area.id == request.area && area.parent.is_none())
        {
            return None;
        }
        let present: HashSet<_> = table.runs.iter().map(|run| run.subarea).collect();
        let parents: HashSet<_> = table
            .areas
            .iter()
            .filter(|area| present.contains(&area.id))
            .filter_map(|area| area.parent)
            .collect();
        let mut areas: Vec<_> = table
            .areas
            .iter()
            .filter(|area| {
                area.parent.unwrap_or(0) == request.area
                    && (present.contains(&area.id) || parents.contains(&area.id))
            })
            .collect();
        areas.sort_by_key(|area| area.id);
        let after = usize::from(request.after);
        if after > areas.len() {
            return None;
        }
        let more = areas.len() - after > wire::AREA_PAGE;
        let areas: Vec<_> = areas
            .into_iter()
            .skip(after)
            .take(wire::AREA_PAGE)
            .map(|area| Area {
                id: area.id,
                parent: area.parent.unwrap_or(0),
                name: area.name.clone(),
            })
            .collect();
        let next = if more {
            Some((after + areas.len()).try_into().ok()?)
        } else {
            None
        };
        return Some(Reply::Areas(AreaPage {
            parent: request.area,
            after: request.after,
            next,
            areas,
        }));
    }
    let selected = table.areas.iter().find(|area| area.id == request.area)?;
    let members: HashSet<_> = table
        .areas
        .iter()
        .filter(|area| {
            area.id == selected.id
                || (selected.parent.is_none() && area.parent == Some(selected.id))
        })
        .map(|area| area.id)
        .collect();
    let runs: Vec<_> = table
        .runs
        .iter()
        .filter(|run| run.floor == request.floor && members.contains(&run.subarea))
        .collect();
    if runs.len() > wire::MAX_RUNS {
        return None;
    }
    let bounds = if runs.is_empty() {
        [0; 4]
    } else {
        [
            runs.iter().map(|r| r.begin).min()?,
            runs.iter().map(|r| r.y).min()?,
            runs.iter().map(|r| r.end).max()?.checked_sub(1)?,
            runs.iter().map(|r| r.y).max()?,
        ]
    };
    let offset = usize::from(request.after) * wire::RUN_PAGE;
    if offset > runs.len() {
        return None;
    }
    let next = if runs.len() - offset > wire::RUN_PAGE {
        Some(request.after.checked_add(1)?)
    } else {
        None
    };
    let total = runs.len().try_into().ok()?;
    let runs = runs
        .into_iter()
        .skip(offset)
        .take(wire::RUN_PAGE)
        .map(|r| Run {
            y: r.y,
            begin: r.begin,
            end: r.end,
            subarea: r.subarea,
        })
        .collect();
    Some(Reply::Geometry(GeometryPage {
        area: request.area,
        floor: request.floor,
        page: request.after,
        next,
        total,
        bounds,
        runs,
    }))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn atlas_projects_bundle_regions_subareas_floors_and_complete_geometry_pages()
    -> Result<(), Box<dyn std::error::Error>> {
        use oteryn_world_bundle::subarea::{Area as NativeArea, Run as NativeRun, Table};
        let area = |id, parent, name: &str| NativeArea {
            id,
            parent,
            name: name.into(),
            key: format!("oteryn:area.{id}"),
            source_record_sha256: "1".repeat(64),
            town_id: None,
            hometown: false,
            temple: None,
        };
        let mut table = Table {
            areas: vec![
                area(1, None, "Harbor"),
                area(2, Some(1), "Inner harbor"),
                area(3, None, "Highlands"),
            ],
            ..Table::default()
        };
        table.runs = (0..1025)
            .map(|y| NativeRun {
                floor: -7,
                y,
                begin: 10,
                end: 20,
                subarea: 2,
            })
            .collect();
        table.runs.push(NativeRun {
            floor: -6,
            y: 2000,
            begin: 40,
            end: 50,
            subarea: 3,
        });
        let Reply::Areas(roots) = project(
            &table,
            Request {
                area: 0,
                floor: 1,
                after: 0,
            },
        )
        .ok_or("regions")?
        else {
            return Err("wrong region reply".into());
        };
        assert_eq!(roots.areas.iter().map(|a| a.id).collect::<Vec<_>>(), [1, 3]);
        let Reply::Areas(children) = project(
            &table,
            Request {
                area: 1,
                floor: 1,
                after: 0,
            },
        )
        .ok_or("subareas")?
        else {
            return Err("wrong subarea reply".into());
        };
        assert_eq!(children.areas[0].id, 2);
        let Reply::Geometry(first) = project(
            &table,
            Request {
                area: 1,
                floor: -7,
                after: 0,
            },
        )
        .ok_or("map")?
        else {
            return Err("wrong geometry reply".into());
        };
        assert_eq!(
            (first.runs.len(), first.total, first.next),
            (1024, 1025, Some(1))
        );
        assert_eq!(first.bounds, [10, 0, 19, 1024]);
        let Reply::Geometry(last) = project(
            &table,
            Request {
                area: 1,
                floor: -7,
                after: 1,
            },
        )
        .ok_or("continuation")?
        else {
            return Err("wrong continuation".into());
        };
        assert_eq!((last.runs.len(), last.next), (1, None));
        let Reply::Geometry(empty) = project(
            &table,
            Request {
                area: 1,
                floor: -6,
                after: 0,
            },
        )
        .ok_or("floor")?
        else {
            return Err("wrong floor reply".into());
        };
        assert_eq!(empty.total, 0);
        assert_eq!(wire::MAX_RUNS, oteryn_world_bundle::subarea::MAX_RUNS);
        assert_eq!(wire::MAX_AREAS, oteryn_world_bundle::subarea::MAX_AREAS);
        Ok(())
    }
}
