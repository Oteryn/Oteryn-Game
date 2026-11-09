pub use oteryn_client_runtime::discovery::inspection::*;
mod input;
mod protocol;
mod state;
mod view;
pub use input::open_key;
pub use protocol::{apply_result, request, request_page, request_sources};
pub use state::Panel;
pub use view::{catalogue_text, panel_text};

#[cfg(test)]
mod catalogue_tests {
    use super::*;
    use oteryn_client_runtime::server_mechanic::{CommandStatus, ServerMechanicResponse};
    #[test]
    fn catalogue_retains_confirmed_templates_and_searches_attributes()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut panel = Panel {
            selected: true,
            ..Default::default()
        };
        request(&mut panel, Request::Catalogue(7)).ok_or("request")?;
        let reply = Reply {
            status: 1,
            requests: vec![],
            catalogue: None,
            items: vec![InspectedItem {
                slot: 0,
                definition_ref: 7,
                count: 1,
                tier: 0,
                name: "Silver sword".into(),
                attributes: vec![
                    ("Weapon Type".into(), "sword, two-handed".into()),
                    ("attack".into(), "25".into()),
                ],
            }],
        };
        apply_result(
            &mut panel,
            &ServerMechanicResponse {
                command_id: 1,
                server_sequence: 1,
                status: CommandStatus::Accepted,
                payload: encode_reply(&reply).map_err(|_| "reply")?,
            },
        );
        panel.search = "SILVER".into();
        panel.category = "Sword".into();
        assert!(catalogue_text(&panel).contains("attack: 25"));
        request(&mut panel, Request::Catalogue(8)).ok_or("second request")?;
        assert!(catalogue_text(&panel).contains("Silver sword"));
        panel.category = "Armor".into();
        assert!(!catalogue_text(&panel).contains("Silver sword"));
        Ok(())
    }
    #[test]
    fn item_categories_are_selected_from_confirmed_attributes() {
        let mut panel = Panel {
            selected: true,
            ..Default::default()
        };
        panel.catalogue.insert(
            7,
            InspectedItem {
                slot: 0,
                definition_ref: 7,
                count: 1,
                tier: 0,
                name: "Silver sword".into(),
                attributes: vec![("Weapon Type".into(), "sword, two-handed".into())],
            },
        );
        panel.select_category(false);
        assert_eq!(panel.category, "sword");
        assert!(catalogue_text(&panel).contains("Silver sword"));
        panel.search = "missing".into();
        assert!(catalogue_text(&panel).contains("No items match"));
        panel.select_category(false);
        assert!(panel.category.is_empty());
        panel.select_category(true);
        assert_eq!(panel.category, "sword");
    }
}

#[cfg(test)]
#[path = "tests/catalogue.rs"]
mod page_tests;
