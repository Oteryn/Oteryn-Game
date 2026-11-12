use oteryn_client_runtime::raw_feature::{
    CommandStatus, FeatureCommand, GatedCommandResult, house_wrap as wire,
};
use oteryn_client_ui::house_furniture_wrap::{Intent, Panel};

pub(in crate::play) fn command(panel: &Panel, intent: Intent) -> Result<FeatureCommand, String> {
    let page = panel
        .catalogue
        .as_ref()
        .map_or(0, |catalogue| catalogue.page);
    let payload = match intent {
        Intent::Refresh => {
            wire::encode_catalogue_query(page).ok_or(tr!("Invalid furniture page"))?
        }
        Intent::Previous => wire::encode_catalogue_query(page.saturating_sub(1))
            .ok_or(tr!("Invalid furniture page"))?,
        Intent::Next => {
            if !panel
                .catalogue
                .as_ref()
                .is_some_and(|catalogue| catalogue.more)
            {
                return Err(tr!("No more furniture").into());
            }
            wire::encode_catalogue_query(page.checked_add(1).ok_or(tr!("Invalid furniture page"))?)
                .ok_or(tr!("Invalid furniture page"))?
        }
        Intent::Apply => {
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
                .and_then(|catalogue| catalogue.request(index))
                .ok_or(tr!("Refresh and choose furniture from the list"))?
                .encode()
        }
    };
    Ok(FeatureCommand {
        capability: 207,
        kind: wire::COMMAND_WRAP,
        payload,
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
    if wire::decode_catalogue_query(&command.payload).is_some() && accepted {
        if let Some(catalogue) = wire::Catalogue::decode(&result.payload)
            && wire::decode_catalogue_query(&command.payload) == Some(catalogue.page)
        {
            panel.catalogue = Some(catalogue);
            panel.choice.clear();
            panel.feedback = tr!("Choose furniture from the list").into();
        } else {
            panel.feedback = tr!("Invalid furniture catalogue").into();
        }
        return;
    }
    panel.feedback = match wire::Reply::decode(&result.payload) {
        Some(wire::Reply::Committed(revision))
            if accepted && wire::Request::decode(&command.payload).is_some() =>
        {
            panel.catalogue = None;
            lfmt!(
                "Furniture updated (revision {revision}). Refresh the list",
                revision = revision
            )
        }
        Some(wire::Reply::Refused(reason)) if !accepted => {
            panel.catalogue = None;
            match reason {
                wire::Refusal::NotInHouse => tr!("Enter your house first"),
                wire::Refusal::NoAccess => tr!("You cannot edit this house"),
                wire::Refusal::NotWrapable => tr!("This item cannot be wrapped or placed"),
                wire::Refusal::Stale => tr!("Furniture or house access changed. Refresh the list"),
                wire::Refusal::Blocked => {
                    tr!("The placement is blocked. Move and refresh the list")
                }
                wire::Refusal::InventoryFull => tr!("Your backpack is full"),
            }
            .into()
        }
        _ => tr!("Invalid furniture response").into(),
    };
}
