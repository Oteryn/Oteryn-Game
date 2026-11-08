use oteryn_client_runtime::raw_feature::{CommandStatus,FeatureCommand,GatedCommandResult,house_wrap as wire};
use oteryn_client_ui::house_furniture_wrap::{Intent,Panel};

pub(in crate::play) fn command(panel: &Panel,intent: Intent) -> Result<FeatureCommand,String> {
    let page=panel.catalogue.as_ref().map_or(0,|catalogue|catalogue.page);
    let payload=match intent {
        Intent::Refresh=>wire::encode_catalogue_query(page).ok_or("Invalid furniture page")?,
        Intent::Previous=>wire::encode_catalogue_query(page.saturating_sub(1)).ok_or("Invalid furniture page")?,
        Intent::Next=> {
            if !panel.catalogue.as_ref().is_some_and(|catalogue|catalogue.more) {return Err("No more furniture".into());}
            wire::encode_catalogue_query(page.checked_add(1).ok_or("Invalid furniture page")?).ok_or("Invalid furniture page")?
        }
        Intent::Apply=> {
            let index=panel.choice.trim().parse::<usize>().ok().and_then(|value|value.checked_sub(1)).ok_or("Choose a furniture number from the list")?;
            panel.catalogue.as_ref().and_then(|catalogue|catalogue.request(index)).ok_or("Refresh and choose furniture from the list")?.encode()
        }
    };
    Ok(FeatureCommand {capability:207,kind:wire::COMMAND_WRAP,payload})
}
pub(in crate::play) fn apply(panel: &mut Panel,command: &FeatureCommand,result: &GatedCommandResult) {
    if !panel.pending || panel.pending_payload!=command.payload {return;}
    panel.pending=false; panel.pending_payload.clear();
    let accepted=matches!(result.status,CommandStatus::Accepted|CommandStatus::DuplicateReplay);
    if wire::decode_catalogue_query(&command.payload).is_some() && accepted {
        if let Some(catalogue)=wire::Catalogue::decode(&result.payload)
            && wire::decode_catalogue_query(&command.payload)==Some(catalogue.page) {
            panel.catalogue=Some(catalogue); panel.choice.clear(); panel.feedback="Choose furniture from the list".into();
        } else {panel.feedback="Invalid furniture catalogue".into();}
        return;
    }
    panel.feedback=match wire::Reply::decode(&result.payload) {
        Some(wire::Reply::Committed(revision)) if accepted && wire::Request::decode(&command.payload).is_some()=> {
            panel.catalogue=None;
            format!("Furniture updated (revision {revision}). Refresh the list")
        }
        Some(wire::Reply::Refused(reason)) if !accepted=> {
            panel.catalogue=None;
            match reason {
                wire::Refusal::NotInHouse=>"Enter your house first",
                wire::Refusal::NoAccess=>"You cannot edit this house",
                wire::Refusal::NotWrapable=>"This item cannot be wrapped or placed",
                wire::Refusal::Stale=>"Furniture or house access changed. Refresh the list",
                wire::Refusal::Blocked=>"The placement is blocked. Move and refresh the list",
                wire::Refusal::InventoryFull=>"Your backpack is full",
            }.into()
        }
        _=>"Invalid furniture response".into(),
    };
}

#[cfg(test)]
#[path = "tests/e2e_i10.rs"]
mod e2e_i10;
