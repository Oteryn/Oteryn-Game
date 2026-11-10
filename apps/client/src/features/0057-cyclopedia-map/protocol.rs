use crate::play::feature_0115_mail::protocol::accepted;
use oteryn_client_runtime::{
    cyclopedia_map as wire,
    raw_feature::{FeatureCommand, GatedCommandResult},
};
use oteryn_client_ui::cyclopedia_map::{Intent, Panel};
pub(in crate::play) fn command(panel: &Panel, intent: Intent) -> Result<FeatureCommand, String> {
    if intent == Intent::Atlas {
        return Ok(FeatureCommand {
            capability: 57,
            kind: 72,
            payload: panel
                .atlas_query
                .encode()
                .map_err(|_| "Invalid atlas view")?,
        });
    }
    let operation = match intent {
        Intent::Query => 0,
        Intent::Atlas => return Err("Use the atlas query".into()),
        Intent::RegionAtlas | Intent::AtlasNext => 5,
        Intent::Start => 1,
        Intent::Shuffle => 2,
        Intent::Find => 3,
        Intent::Donate => 4,
    };
    let amount = if intent == Intent::Donate {
        panel.fields[0]
            .trim()
            .parse()
            .map_err(|_| tr!("Enter donation gold (1-10000000)"))?
    } else if intent == Intent::AtlasNext {
        panel
            .region_atlas
            .as_ref()
            .and_then(|page| page.next)
            .ok_or("The complete atlas is loaded")?
    } else {
        0
    };
    let (kind, payload) = wire::request(operation, amount)
        .map_err(|_| tr!("Invalid map request or donation amount"))?;
    Ok(FeatureCommand {
        capability: wire::CAPABILITY_CYCLOPEDIA_MAP_V1,
        kind,
        payload,
    })
}
pub(in crate::play) fn apply(
    panel: &mut Panel,
    command: &FeatureCommand,
    result: &GatedCommandResult,
) {
    if command.kind == wire::COMMAND_TYPE_CYCLOPEDIA_SPEED_QUERY {
        panel.speed_pending = false;
        panel.speed_bonus = if accepted(result) {
            <[u8; 2]>::try_from(result.payload.as_slice())
                .map(u16::from_le_bytes)
                .unwrap_or(0)
        } else {
            0
        };
        return;
    }
    if super::input::atlas::apply(panel, command, result) {
        return;
    }
    if !panel.pending || panel.pending_kind != command.kind {
        return;
    }
    panel.pending = false;

    if !accepted(result) {
        panel.feedback = tr!("The server refused the request").into();
        return;
    }
    if command.payload.first() == Some(&128) {
        match wire::atlas::Atlas::decode(&result.payload) {
            Ok(atlas) => {
                let Ok(query) = wire::atlas::Query::decode(&command.payload) else {
                    panel.feedback = "Invalid atlas request".into();
                    return;
                };
                if atlas.query.zoom != query.zoom
                    || atlas.query.page != query.page
                    || (query.floor != 127 && atlas.query.floor != query.floor)
                    || ((query.x, query.y) != (65535, 65535)
                        && (atlas.query.x, atlas.query.y) != (query.x, query.y))
                {
                    panel.feedback = "Different atlas view returned".into();
                    return;
                }

                let completed = atlas
                    .progress
                    .as_ref()
                    .is_some_and(|progress| progress.discovered && progress.found == 7)
                    && panel
                        .map
                        .as_ref()
                        .is_some_and(|previous| previous.found < 7);
                panel.floor = atlas.query.floor;
                panel.map = atlas.progress.clone();
                panel.summary = if let Some(progress) = &atlas.progress {
                    format!(
                        "Exploration revision {} | Discovered {} | Found {}/7 | Area {}%",
                        progress.revision,
                        progress.discovered,
                        progress.found,
                        progress.area_percent
                    )
                } else {
                    "Exploration unavailable at the current position".into()
                };
                panel.atlas_query = atlas.query;
                panel.atlas = Some(atlas);
                panel.feedback = if completed {
                    tr!("Subarea discovered! All seven points of interest found.").into()
                } else {
                    "Atlas updated".into()
                };
            }
            Err(_) => panel.feedback = "Invalid atlas reply".into(),
        }
        return;
    }
    if let Ok(intent) = wire::decode_request(command.kind, &command.payload)
        && intent.operation == 5
    {
        let Ok(page) = wire::region_atlas::decode(&result.payload) else {
            panel.feedback = "Invalid atlas reply".into();
            return;
        };
        if page.next.is_some_and(|next| next <= intent.amount)
            || intent.amount != 0
                && panel.region_atlas.as_ref().and_then(|p| p.next) != Some(intent.amount)
        {
            panel.feedback = "Stale atlas cursor".into();
            return;
        }
        let mut combined = page;
        if intent.amount != 0
            && let Some(previous) = &panel.region_atlas
        {
            combined.runs.splice(0..0, previous.runs.iter().copied());
            let mut areas: std::collections::BTreeMap<_, _> =
                previous.areas.iter().map(|a| (a.id, a.clone())).collect();
            for area in combined.areas {
                areas.insert(area.id, area);
            }
            combined.areas = areas.into_values().collect();
        }
        if combined.runs.len() > 262_144 || combined.areas.len() > 1024 {
            panel.feedback = "Atlas geometry limit exceeded".into();
            return;
        }
        if intent.amount == 0
            && let Some(run) = combined.runs.first()
        {
            panel.floor = run.floor;
        }
        panel.atlas_after = intent.amount;
        panel.region_atlas = Some(combined);
        panel.feedback = "Atlas updated".into();
        return;
    }
    let Ok(state) = wire::decode_result(&result.payload) else {
        panel.feedback = tr!("Invalid cyclopedia map reply").into();
        return;
    };
    panel.feedback = match state.code {
        1 => tr!("Invalid map request"),
        2 => tr!("Premium required"),
        3 => tr!("Area already discovered"),
        4 => tr!("No exploration in progress"),
        5 => tr!("Insufficient reachable pool"),
        6 => tr!("Insufficient funds"),
        7 => tr!("Area currently improved"),
        _ => {
            let completed = state.discovered && state.found == 7
                && panel.map.as_ref().is_some_and(|previous| previous.found < 7);
            panel.summary = format!("Current exploration area | Revision {} | Discovered: {}\r\nFound {}/7 | Area {}% | Pool {} | Epoch {} | Improved: {}\r\nVisible points: {:?}", state.revision, state.discovered, state.found, state.area_percent, state.pool, state.epoch, state.improved, state.visible);
            panel.map = Some(state);
            if completed { tr!("Subarea discovered! All seven points of interest found.") }
            else { tr!("Cyclopedia map updated") }
        }
    }.into();
}

#[cfg(test)]
#[path = "tests/e2e_w12.rs"]
mod e2e_w12;
