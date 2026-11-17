use super::Panel;
use oteryn_client_runtime::raw_feature::house_wrap::Action;
pub fn panel_text(panel: &Panel) -> String {
    let mut text = String::from(tr!("House furniture\r\n"));
    if let Some(catalogue) = &panel.catalogue {
        text.push_str(&lfmt!(
            "Page {}. Placement: {}, {}, floor {}\r\n",
            catalogue.page + 1,
            catalogue.position[0],
            catalogue.position[1],
            catalogue.position[2]
        ));
        for (index, item) in catalogue.entries.iter().enumerate() {
            text.push_str(&format!(
                "{}. {} — {}\r\n",
                index + 1,
                item.name,
                match item.action {
                    Action::Wrap => tr!("Wrap"),
                    Action::Unwrap => tr!("Place"),
                }
            ));
        }
        if catalogue.entries.is_empty() {
            text.push_str(tr!("No furniture to wrap or place\r\n"));
        }
    }
    text.push_str(if panel.pending {
        tr!("Waiting for the server")
    } else if !panel.selected {
        tr!("Unavailable in this session")
    } else {
        &panel.feedback
    });
    text
}
