use crate::container_tree::ContainerPanel;
/// The existing native browser displays the negotiated House container rows.
pub fn panel_text(panel: &ContainerPanel) -> String {
    let text = crate::container_tree::view::panel_text(panel);
    if panel.house_capacity_v2
        && let Some(view) = panel.current()
        && view.capacity > 20
        && view.browse.is_none()
    {
        text.replacen(
            &lfmt!("Container {}", view.container_handle),
            tr!("House container"),
            1,
        )
    } else {
        text
    }
}
