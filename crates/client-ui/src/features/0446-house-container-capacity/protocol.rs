use crate::container_tree::ContainerPanel;
use oteryn_client_runtime::container_tree::ContainerViews;
/// Crystal data/items/items.xml:26689-26699: the two lizard weapon racks hold25 items.
/// The session decoder negotiates this bound; ordinary capability14 remains20.
pub fn project(panel: &mut ContainerPanel, views: Option<ContainerViews>) -> bool {
    let limit = if panel.house_capacity_v2 { 25 } else { 20 };
    if views.as_ref().is_some_and(|views| {
        views.views.iter().any(|view| {
            view.browse.is_none()
                && (usize::from(view.capacity) > limit || view.entries.len() > limit)
        })
    }) {
        panel.feedback = Some(tr!("Cannot display this container"));
        return false;
    }
    panel.apply(views);
    true
}
