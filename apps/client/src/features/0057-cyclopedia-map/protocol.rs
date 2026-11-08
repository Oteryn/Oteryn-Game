use crate::play::feature_0115_mail::protocol::accepted;
use oteryn_client_runtime::{
    cyclopedia_map as wire,
    raw_feature::{FeatureCommand, GatedCommandResult},
};
use oteryn_client_ui::cyclopedia_map::{Intent, Panel};
pub(in crate::play) fn command(panel: &Panel, intent: Intent) -> Result<FeatureCommand, String> {
    let operation = match intent {
        Intent::Query => 0,
        Intent::Atlas | Intent::AtlasNext => 5,
        Intent::Start => 1,
        Intent::Shuffle => 2,
        Intent::Find => 3,
        Intent::Donate => 4,
    };
    let amount = if intent == Intent::Donate {
        panel.fields[0]
            .trim()
            .parse()
            .map_err(|_| "Enter donation gold (1-10000000)")?
    } else if intent == Intent::AtlasNext {
        panel
            .atlas
            .as_ref()
            .and_then(|page| page.next)
            .ok_or("The complete atlas is loaded")?
    } else {
        0
    };
    let (kind, payload) =
        wire::request(operation, amount).map_err(|_| "Invalid map request or donation amount")?;
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
    if !panel.pending || panel.pending_kind != command.kind {
        return;
    }
    panel.pending = false;

    if !accepted(result) {
        panel.feedback = "The server refused the request".into();
        return;
    }
    if let Ok(intent) = wire::decode_request(command.kind, &command.payload)
        && intent.operation == 5
    {
        let Ok(page) = wire::atlas::decode(&result.payload) else {
            panel.feedback = "Invalid atlas reply".into();
            return;
        };
        if page.next.is_some_and(|next| next <= intent.amount)
            || intent.amount != 0
                && panel.atlas.as_ref().and_then(|p| p.next) != Some(intent.amount)
        {
            panel.feedback = "Stale atlas cursor".into();
            return;
        }
        let mut combined = page;
        if intent.amount != 0
            && let Some(previous) = &panel.atlas
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
        panel.atlas = Some(combined);
        panel.feedback = "Atlas updated".into();
        return;
    }
    let Ok(state) = wire::decode_result(&result.payload) else {
        panel.feedback = "Invalid cyclopedia map reply".into();
        return;
    };
    panel.feedback = match state.code {
        1 => "Invalid map request",
        2 => "Premium required",
        3 => "Area already discovered",
        4 => "No exploration in progress",
        5 => "Insufficient reachable pool",
        6 => "Insufficient funds",
        7 => "Area currently improved",
        _ => {
            panel.summary = format!("Area 1 | Revision {} | Discovered: {}\r\nFound {}/7 | Area {}% | Pool {} | Epoch {} | Improved: {}\r\nVisible points: {:?}", state.revision, state.discovered, state.found, state.area_percent, state.pool, state.epoch, state.improved, state.visible);
            panel.map = Some(state);
            "Cyclopedia map updated"
        }
    }.into();
}
