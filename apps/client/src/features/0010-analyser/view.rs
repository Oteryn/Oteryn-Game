use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn analyser_panel(&self) -> oteryn_client_ui::analyser::Panel {
        lock(&self.mailbox).analyser.clone()
    }
}
#[cfg(windows)]
pub(in crate::play) fn open_window(link: &PlayLink) {
    use crate::play::feature_0115_mail::view::native::{Form, open};
    open(
        link,
        Form {
            title: "Analysers",
            labels: &[
                "XP still needed for your level goal (blank disables)",
                "Item number from loot or supply list",
                "Unit item value (gold)",
            ],
            buttons: &[
                "Overview",
                "Hunting",
                "Experience",
                "Loot",
                "Supplies",
                "Damage",
                "Healing",
                "Impact",
                "Input",
                "Hunting session",
                "Party hunt",
                "Loot tracker",
                "Kill tracker",
                "Reset session",
                "Apply level goal",
                "Apply item price",
                "Resolve item info",
            ],
            snapshot: |mailbox| {
                let panel = &mailbox.analyser;
                (
                    panel.open,
                    false,
                    vec![
                        panel.goal_input.clone(),
                        panel.price_input[0].clone(),
                        panel.price_input[1].clone(),
                    ],
                    oteryn_client_ui::analyser::panel_text(panel),
                )
            },
            set_open: |mailbox, value| mailbox.analyser.open = value,

            submit: |mailbox, commands, action, values| {
                let tools = oteryn_client_ui::analyser::Tool::ALL.len();
                if action < oteryn_client_ui::analyser::Tool::CYCLOPEDIA.len() {
                    super::input::select_tool(&mut mailbox.analyser, action);
                } else if let Some(tool) = oteryn_client_ui::analyser::Tool::ALL.get(action) {
                    oteryn_client_ui::analyser::select_tool(&mut mailbox.analyser, tool.key());
                } else if action == tools {
                    super::input::select_tool(
                        &mut mailbox.analyser,
                        oteryn_client_ui::analyser::Tool::CYCLOPEDIA.len(),
                    );
                } else if action == tools + 1
                    && let Some(value) = values.first()
                {
                    oteryn_client_ui::analyser::set_xp_goal(&mut mailbox.analyser, value);
                } else if action == tools + 2
                    && let [_, item, price] = values.as_slice()
                {
                    oteryn_client_ui::analyser::set_item_price(&mut mailbox.analyser, item, price);
                } else if action == tools + 3 {
                    super::input::resolve_items(mailbox, commands);
                }
            },
        },
    );
}
