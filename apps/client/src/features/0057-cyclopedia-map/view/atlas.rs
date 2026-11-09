use crate::play::{
    PlayLink,
    feature_0115_mail::view::native::{Form, open},
};
#[path = "atlas_window.rs"]
mod window;
pub(in crate::play) fn show_map(
    panel: &oteryn_client_ui::cyclopedia_map::Panel,
    commands: &tokio::sync::mpsc::UnboundedSender<crate::combat_input::PlayCommand>,
) {
    if panel.atlas.bounds.is_some() {
        window::open(panel, commands);
    }
}
pub(in crate::play) fn open_window(link: &PlayLink) {
    open(
        link,
        Form {
            title: "World atlas — Ctrl+Alt+Shift+F11",
            labels: &[],
            buttons: &[
                "Regions",
                "Previous area",
                "Next area",
                "Subareas",
                "Top-level regions",
                "Load map",
                "Floor up",
                "Floor down",
                "Region layer",
                "POI layer",
                "Refresh visible POIs",
                "Show graphical map",
                "Next area page",
                "Previous area page",
            ],
            snapshot: |mailbox| {
                let panel = &mailbox.properties.cyclopedia_map;
                window::refresh(panel);
                (
                    panel.atlas.open,
                    panel.pending,
                    vec![],
                    oteryn_client_ui::cyclopedia_map::atlas_view::text(panel),
                )
            },
            set_open: |mailbox, value| mailbox.properties.cyclopedia_map.atlas.open = value,
            submit: |mailbox, commands, action, _| {
                super::super::input::atlas::submit(mailbox, commands, action)
            },
        },
    );
}
