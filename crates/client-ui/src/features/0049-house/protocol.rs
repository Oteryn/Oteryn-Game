use super::Panel;
use oteryn_client_runtime::{
    house::{self, Action},
    raw_feature::FeatureCommand,
};
pub fn request(
    panel: &mut Panel,
    action: Option<&Action>,
    next: bool,
) -> Result<FeatureCommand, &'static str> {
    if !panel.selected {
        return Err(tr!("Unavailable in this session"));
    }
    if panel.pending.is_some() {
        return Err(tr!("Waiting for the server"));
    }
    let command = if let Some(action) = action {
        let row = panel
            .page
            .rows
            .get(panel.index)
            .ok_or(tr!("Select a house"))?;
        house::mutation(panel.world, &row.house, row.acl_revision, action)?
    } else {
        let cursor = if next {
            Some(panel.page.cursor.as_deref().ok_or(tr!("No next page"))?)
        } else {
            None
        };
        house::query(panel.world, &panel.town, cursor)?
    };
    panel.pending_acl = matches!(
        action,
        Some(Action::Guests(_) | Action::Subowners(_) | Action::Door { .. })
    );
    panel.pending_index = panel.index;
    panel.pending = Some(command.clone());
    Ok(command)
}
