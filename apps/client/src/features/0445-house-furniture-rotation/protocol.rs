use oteryn_client_runtime::raw_feature::{
    CommandStatus, FeatureCommand, GatedCommandResult, house_rotation as wire,
};
use oteryn_client_ui::house_furniture_rotation::{Intent, Panel};

pub(in crate::play) fn command(panel: &Panel, intent: Intent) -> Result<FeatureCommand, String> {
    let page = panel
        .catalogue
        .as_ref()
        .map_or(0, |catalogue| catalogue.page);
    let request = match intent {
        Intent::Refresh => wire::Request::Query(page),
        Intent::Previous => wire::Request::Query(page.saturating_sub(1)),
        Intent::Next => {
            if !panel
                .catalogue
                .as_ref()
                .is_some_and(|catalogue| catalogue.more)
            {
                return Err(tr!("No more furniture").into());
            }
            wire::Request::Query(page.checked_add(1).ok_or(tr!("Invalid furniture page"))?)
        }
        Intent::Rotate => {
            let index = panel
                .choice
                .trim()
                .parse::<usize>()
                .ok()
                .and_then(|value| value.checked_sub(1))
                .ok_or(tr!("Choose a furniture number from the list"))?;
            panel
                .catalogue
                .as_ref()
                .and_then(|catalogue| catalogue.rotate(index))
                .ok_or(tr!("Refresh and choose furniture from the list"))?
        }
    };
    Ok(FeatureCommand {
        capability: wire::CAPABILITY_HOUSE_FURNITURE_ROTATION_V1,
        kind: wire::COMMAND,
        payload: request.encode().ok_or(tr!("Invalid furniture request"))?,
    })
}
pub(in crate::play) fn apply(
    panel: &mut Panel,
    command: &FeatureCommand,
    result: &GatedCommandResult,
) {
    if !panel.pending || panel.pending_payload != command.payload {
        return;
    }
    panel.pending = false;
    panel.pending_payload.clear();
    let accepted = matches!(
        result.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    );
    match (
        wire::Request::decode(&command.payload),
        wire::Reply::decode(&result.payload),
    ) {
        (Some(wire::Request::Query(page)), Some(wire::Reply::Catalogue(catalogue)))
            if accepted && page == catalogue.page =>
        {
            panel.catalogue = Some(catalogue);
            panel.choice.clear();
            panel.feedback = tr!("Choose furniture from the list").into();
        }
        (Some(wire::Request::Rotate(_)), Some(wire::Reply::Applied(disposition))) => {
            // Crystal src/game/game.cpp:3054-3057: missing transform targets can leave the item unchanged.
            // Always obtain fresh authoritative revisions instead of predicting a revision increment.
            panel.catalogue = None;
            panel.choice.clear();
            panel.feedback = match disposition {
                wire::UseDisposition::Committed if accepted => {
                    tr!("Furniture rotated. Refresh the list")
                }
                _ => tr!("Furniture or house access changed. Refresh the list"),
            }
            .into();
        }
        _ => {
            panel.catalogue = None;
            panel.feedback = tr!("Invalid furniture response. Refresh the list").into();
        }
    }
}
