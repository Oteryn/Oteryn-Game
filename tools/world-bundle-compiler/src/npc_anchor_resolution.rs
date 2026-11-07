//! Read-only authoring aid: admit nearby source roaming cells through the normal compiler.
use oteryn_world_bundle_compiler::{
    Error,
    bundle::{BuildClass, Identity},
    compile::{Input, realize_npcs, realize_spawns},
    npc::MAX_PLACEMENTS_PER_NPC,
    source::{project_regions, registry, world_extent},
};
use serde_json::{Value, json};
use std::{collections::BTreeSet, path::Path};

pub fn run(root: &Path, keys: &[String]) -> Result<Value, Error> {
    let fail = |message: &str| Error::Format(message.into());
    if keys.is_empty() || keys.len() > 6 || keys.iter().collect::<BTreeSet<_>>().len() != keys.len()
    {
        return Err(fail("request one to six distinct NPC keys"));
    }
    let bytes = std::fs::read(
        root.join("tools/content-schema/npc-authoring/samples/promotion-candidates-v1.json"),
    )
    .map_err(|e| fail(&e.to_string()))?;
    let capture: Value = serde_json::from_slice(&bytes).map_err(|e| fail(&e.to_string()))?;
    let mut anchors = Vec::new();
    for key in keys {
        let row = capture["candidates"]
            .as_array()
            .ok_or_else(|| fail("candidates"))?
            .iter()
            .find(|row| row["identity"]["key"].as_str() == Some(key))
            .ok_or_else(|| fail("unknown captured NPC"))?;
        let placements = row["placements"]
            .as_array()
            .ok_or_else(|| fail("placements"))?;
        if placements.len() != 1 {
            return Err(fail("requires one unambiguous captured anchor"));
        }
        let p = &placements[0]["position"];
        let x = p["x"]
            .as_u64()
            .and_then(|v| u16::try_from(v).ok())
            .ok_or_else(|| fail("x"))?;
        let y = p["y"]
            .as_u64()
            .and_then(|v| u16::try_from(v).ok())
            .ok_or_else(|| fail("y"))?;
        let z = p["z"]
            .as_u64()
            .and_then(|v| u8::try_from(v).ok())
            .filter(|v| *v <= 15)
            .ok_or_else(|| fail("z"))?;
        let radius = row["movement"]["walk_radius"]
            .as_u64()
            .filter(|v| *v <= 4)
            .ok_or_else(|| fail("requires bounded captured walk radius"))?
            as u16;
        anchors.push((key.clone(), x, y, z, radius));
    }
    let regions = anchors
        .iter()
        .flat_map(|(_, x, y, _, r)| {
            [
                (x.saturating_sub(*r) / 256, y.saturating_sub(*r) / 256),
                (x.saturating_add(*r) / 256, y.saturating_sub(*r) / 256),
                (x.saturating_sub(*r) / 256, y.saturating_add(*r) / 256),
                (x.saturating_add(*r) / 256, y.saturating_add(*r) / 256),
            ]
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let mut project = project_regions(root, &regions)?;
    let (resolver, palette) = registry(root, &project)?;
    let originals = project.families.npc_placements.clone();
    let mut admitted = Vec::new();
    let mut rejected = Vec::new();
    for offset in (0..81).step_by(MAX_PLACEMENTS_PER_NPC) {
        if !anchors
            .iter()
            .any(|(_, _, _, _, r)| usize::from(2 * r + 1).pow(2) > offset)
        {
            break;
        }
        project.families.npc_placements = originals.clone();
        for (key, x, y, z, r) in &anchors {
            let template = project
                .families
                .npc_placements
                .iter()
                .find(|p| &p.npc == key)
                .cloned()
                .ok_or_else(|| fail("requires existing placement declaration"))?;
            project.families.npc_placements.retain(|p| &p.npc != key);
            let mut cells = (x.saturating_sub(*r)..=x.saturating_add(*r))
                .flat_map(|cx| {
                    (y.saturating_sub(*r)..=y.saturating_add(*r)).map(move |cy| (cx, cy))
                })
                .collect::<Vec<_>>();
            cells.sort_by_key(|(cx, cy)| {
                let dx = cx.abs_diff(*x);
                let dy = cy.abs_diff(*y);
                (dx.max(dy), dx + dy, *cy, *cx)
            });
            // Authoring probes obey the same per-NPC bound as runtime placements.
            for (cx, cy) in cells.into_iter().skip(offset).take(MAX_PLACEMENTS_PER_NPC) {
                let mut candidate = template.clone();
                candidate.key = format!(
                    "oteryn:npc_placement.{}.x{cx}_y{cy}_z{z}",
                    key.trim_start_matches("oteryn:npc.")
                );
                candidate.cell = (cx, cy, *z);
                project.families.npc_placements.push(candidate);
            }
        }

        let input = Input {
            regions: &project.regions,
            palette: &palette,
            identity: Identity::default(),
            world: world_extent(root)?,
            build_class: BuildClass::NonProduction,
            draft_areas: Vec::new(),
            families: &project.families,
        };
        let (spawns, _) = realize_spawns(&input, &resolver)?;
        let (table, report) = realize_npcs(&input, &resolver, &spawns)?;
        admitted.extend(table.npcs);
        rejected.extend(report.held);
    }
    let mut resolved = Vec::new();
    for (key, x, y, z, r) in anchors {
        let rows = admitted.iter().filter(|row| row.key == key);
        let p = rows
            .flat_map(|row| &row.placements)
            .min_by_key(|p| {
                let dx = p.x.abs_diff(x);
                let dy = p.y.abs_diff(y);
                (dx.max(dy), dx + dy, p.y, p.x)
            })
            .ok_or_else(|| {
                fail(&format!(
                    "no admitted placement for {key}: {:?}",
                    rejected.iter().filter(|p| p.npc == key).collect::<Vec<_>>()
                ))
            })?;
        eprintln!("admitted {key}: {},{},{}", p.x, p.y, z);
        resolved.push(json!({"npc":key,"captured_spawn_anchor":{"x":x,"y":y,"z":z},"walk_radius":r,"position":{"x":p.x,"y":p.y,"z":z},"rejected_candidates":rejected.iter().filter(|p|p.npc==key).count()}));
    }
    Ok(json!({"resolved":resolved}))
}
